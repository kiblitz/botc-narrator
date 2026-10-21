#!/usr/bin/env bash
# The deploy half of the local CI: what a `deploy-scripts` CI job would run,
# on the owner's Windows machine (Git Bash). scripts/check.sh runs it before
# every merge; it can also be run alone. Tools that are not installed here are
# skipped with a "skipped:" line.
#
# Its last line is `box_tests ran` or `box_tests skipped`, which check.sh
# hands to scripts/stamp.sh.
set -euo pipefail
cd "$(dirname "$0")/.."

step() { printf '\n== %s\n' "$*"; }
skip() { printf 'skipped: %s\n' "$*"; }
# A skip nobody can miss. The root deploy tests are the one part of the checks
# that can stop covering anything without the run going red, so saying it
# quietly is not enough.
loud() { printf '\n!!!! %s\n' "$*" >&2; }

# The slot1 box, where the root deploy tests run in a throwaway container
# because this machine is Windows (see that step).
readonly BOX=claude@5.161.56.133
readonly BOX_KEY="${HOME}/.ssh/slot1-admin"
# ubuntu:24.04 rather than fedora:44: the tests need `diff`, which the Fedora
# image does not ship.
readonly BOX_IMAGE=docker.io/library/ubuntu:24.04
readonly BOX_CONNECT_SECONDS=15

step "shellcheck"
if command -v shellcheck >/dev/null 2>&1; then
	shellcheck -x deploy/*.sh deploy/tests/*.sh scripts/check-deploy.sh scripts/stamp.sh scripts/deploy.sh
else
	skip "shellcheck not installed"
fi

step "shfmt"
if command -v shfmt >/dev/null 2>&1; then
	shfmt -d deploy/*.sh deploy/tests/*.sh scripts/check-deploy.sh scripts/stamp.sh scripts/deploy.sh
else
	skip "shfmt not installed"
fi

step "deploy files agree"
bash deploy/tests/files_agree.sh

# Caddy must serve the site through the box's shared Caddyfile (its import line
# and nothing else). `adapt`, not `validate`: validate opens log files that
# exist only on the box.
step "caddy serves the site through the shared Caddyfile"
site_host=$(sed -n 's/^  SITE_HOST: //p' .github/workflows/deploy.yml)
shared_import=$(sed -n 's/^readonly SHARED_IMPORT=.\(.*\).$/\1/p' deploy/caddy-site.sh)
if command -v caddy >/dev/null 2>&1; then
	layout=$(mktemp -d)
	trap 'rm -rf "$layout"' EXIT
	mkdir -p "$layout/sites"
	printf '%s\n' "$shared_import" >"$layout/Caddyfile"
	cp deploy/botc.caddy "$layout/sites/"
	if (cd "$layout" && caddy adapt --config Caddyfile --adapter caddyfile) | grep -qF "\"$site_host\""; then
		echo "the shared Caddyfile serves $site_host"
	else
		echo "the shared Caddyfile does not serve $site_host" >&2
		exit 1
	fi
else
	skip "caddy not installed"
fi

# apply.sh and caddy-site.sh are the half of a deploy that runs as root, and
# their tests chown files and stub systemctl, so they need a Linux root that can
# be thrown away afterwards. This machine is neither, so they run on the box,
# which keeps podman for exactly this. Only deploy/ is copied: the tests read
# nothing outside it. The working files are LF (.gitattributes pins *.sh), so
# they run as they are -- a CRLF copy would die as `bash\r`.
step "deploy script tests (root, in a container on the slot1 box)"
box_tests=ran
box_ssh=(ssh -i "$BOX_KEY" -o BatchMode=yes -o IdentitiesOnly=yes
	-o ConnectTimeout="$BOX_CONNECT_SECONDS" "$BOX")
if [ ! -f "$BOX_KEY" ]; then
	box_tests=skipped
	loud "no $BOX_KEY here: deploy/tests/*_test.sh did NOT run."
elif ! "${box_ssh[@]}" true >/dev/null 2>&1; then
	box_tests=skipped
	loud "$BOX is unreachable: deploy/tests/*_test.sh did NOT run." \
		"apply.sh and caddy-site.sh are UNCHECKED by this run -- do not merge" \
		"a change to deploy/ until they have passed somewhere."
else
	stage=$("${box_ssh[@]}" mktemp -d)
	scp -q -i "$BOX_KEY" -o BatchMode=yes -o IdentitiesOnly=yes -r deploy "$BOX:$stage/"
	# The remote half is fed on stdin rather than quoted into the command line:
	# it nests a container command inside an ssh command, and one more layer of
	# quoting is one too many. The staging directory goes whether the tests
	# passed or not; :ro,Z so the container works on its own copy and cannot
	# write to the box.
	"${box_ssh[@]}" bash -s "$stage" "$BOX_IMAGE" <<'REMOTE'
set -euo pipefail
stage=$1
image=$2
trap 'rm -rf "$stage"' EXIT
sudo podman run --rm -v "$stage:/src:ro,Z" "$image" bash -c '
	set -eu
	cp -r /src /work
	cd /work
	for test in deploy/tests/*_test.sh; do
		printf "\n-- %s\n" "$test"
		bash "$test"
	done
'
REMOTE
fi

printf '\nbox_tests %s\n' "$box_tests"
