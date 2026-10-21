#!/usr/bin/env bash
# Manual deployment from the owner's Windows machine, doing what
# .github/workflows/deploy.yml does while GitHub Actions cannot run.
#
# Usage: scripts/deploy.sh [--dry-run]
#   --dry-run  build and run every check, but stop before staging or applying.
#
# No open-room guard (v1): rooms live in memory, so a restart that ships a new
# binary ends every game in progress. An unchanged binary is never restarted
# (deploy/README.md). Deploy when no table is playing.
#
# Deploys only what is on main, and only what has been checked here: the
# worktree must be clean, HEAD must be exactly origin/main, and scripts/check.sh
# must have passed on this very tree (it leaves the stamp this reads). There is
# no override -- with no CI, that stamp is the only thing standing between a
# typo and the running site.
set -euo pipefail
cd "$(dirname "$0")/.."

dry_run=false
for arg in "$@"; do
	case "$arg" in
	--dry-run) dry_run=true ;;
	*)
		echo "usage: scripts/deploy.sh [--dry-run]" >&2
		exit 2
		;;
	esac
done

# The same values as deploy.yml's env.
readonly DEPLOY_HOST=5.161.56.133
readonly DEPLOY_USER=botc-deploy
readonly SITE_HOST=botc.kiblitz.xyz
readonly SERVICE=botc
readonly APPLY=/usr/local/sbin/botc-apply
readonly CADDY_SITE=/usr/local/sbin/botc-caddy-site
readonly HEALTH_URL=http://127.0.0.1:8081/health
readonly API_PATH=/api/scripts
readonly SYSTEMCTL=/usr/bin/systemctl
readonly STAGED_BINARY=/var/lib/botc-deploy/staging/botc-server.new
readonly STAGED_SITE_CONFIG=/var/lib/botc-deploy/staging/botc.caddy.new
readonly STAGED_WWW=/var/lib/botc-deploy/staging/botc-www.tar.gz
readonly HEALTH_ATTEMPTS=5
readonly HEALTH_RETRY_SECONDS=3
readonly HEALTH_TIMEOUT_SECONDS=5
readonly PUBLIC_CHECK_SECONDS=15
# What is built and shipped, as deploy.yml names them.
readonly SERVER_PACKAGE=botc_server
readonly WEB_DIR=server/static
readonly SITE_CONFIG=deploy/botc.caddy
readonly KEY="$HOME/.ssh/botc-deploy"
readonly TOOLS="${SLOT0_TOOLS:-C:/Users/Gabriel Lee/Projects/slot0/tools}"
# Written by scripts/check.sh when everything passed; see "require the checks"
# below.
readonly STAMP=target/local-check/passed

step() { printf '\n== %s\n' "$*"; }
warn() { printf 'warning: %s\n' "$*" >&2; }
ssh_opts=(-i "$KEY" -o BatchMode=yes -o IdentitiesOnly=yes -o StrictHostKeyChecking=yes)
# SC2029: the caller's variables are meant to expand here, not on the box --
# every path the box runs is one of the constants above, so that they are this
# commit's values and not whatever the box happens to have.
# shellcheck disable=SC2029
remote() { ssh "${ssh_opts[@]}" "$DEPLOY_USER@$DEPLOY_HOST" "$@"; }
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

step "deploy key"
test -f "$KEY" || {
	echo "missing deploy key $KEY; not deploying" >&2
	exit 1
}

step "require clean current main"
git fetch --quiet origin
test -z "$(git status --porcelain)" || {
	echo "worktree is not clean" >&2
	exit 1
}
test "$(git rev-parse HEAD)" = "$(git rev-parse origin/main)" || {
	echo "HEAD is not origin/main; deploy only what is on main" >&2
	exit 1
}

# The merge gate is scripts/check.sh, so this refuses anything it has
# not passed on. It vouches for a *tree*, which is why merging a checked branch
# unchanged does not mean checking it again, while a single edited byte does.
step "require the checks to have passed on this tree"
test -f "$STAMP" || {
	echo "no $STAMP: run 'bash scripts/check.sh' before deploying" >&2
	exit 1
}
stamped_tree=$(awk '$1 == "tree" {print $2}' "$STAMP")
test "$stamped_tree" = "$(git rev-parse 'HEAD^{tree}')" || {
	echo "$STAMP is for another tree ($stamped_tree); re-run scripts/check.sh" >&2
	exit 1
}
test "$(awk '$1 == "clean" {print $2}' "$STAMP")" = yes || {
	echo "the checks last passed on a dirty worktree; re-run scripts/check.sh" >&2
	exit 1
}
# Not fatal: the root deploy tests cover deploy/apply.sh and deploy/caddy-site.sh,
# which a deploy does not install anyway (setup-vps.sh does). Worth saying out
# loud all the same.
test "$(awk '$1 == "box_tests" {print $2}' "$STAMP")" = ran ||
	warn "the root deploy tests were skipped when the checks last ran"
awk '$1 == "when" {print "checks passed at " $2}' "$STAMP"

step "build the server"
binary=$(bash "$TOOLS/linux-build.sh" . "$SERVER_PACKAGE" | tail -n 1)
echo "$binary"

step "check the web client"
# Plain static files, no build step. An empty index.html or app.js would deploy
# a blank page.
test -s "$WEB_DIR/index.html"
test -s "$WEB_DIR/app.js"

step "package the web client"
# From the commit, not the working files: this Windows checkout may be CRLF.
git archive --format=tar "HEAD:$WEB_DIR" | gzip >"$work/web.tar.gz"

step "check the box runs this commit's deploy scripts"
# Hashed as committed: this Windows checkout's working files are CRLF.
expected=$(for path in deploy/apply.sh deploy/caddy-site.sh; do git show "HEAD:$path" | sha256sum | awk '{print $1}'; done)
actual=$(remote "sha256sum $APPLY $CADDY_SITE" | awk '{print $1}' || true)
if [ "$expected" = "$actual" ]; then
	echo "the box runs this commit's $APPLY and $CADDY_SITE"
else
	warn "the box's $APPLY or $CADDY_SITE is not this commit's; re-run setup-vps.sh (deploy/README.md)"
fi

step "refuse stale main"
test "$(git ls-remote origin refs/heads/main | cut -f1)" = "$(git rev-parse HEAD)" || {
	echo "origin/main moved; fetch and re-run" >&2
	exit 1
}

if "$dry_run"; then
	printf '\ndry run: stopping before staging and applying\n'
	exit 0
fi

step "ship the binary, the web client and the routing config"
git show "HEAD:$SITE_CONFIG" >"$work/botc.caddy"
scp "${ssh_opts[@]}" "$binary" "$DEPLOY_USER@$DEPLOY_HOST:$STAGED_BINARY"
scp "${ssh_opts[@]}" "$work/botc.caddy" "$DEPLOY_USER@$DEPLOY_HOST:$STAGED_SITE_CONFIG"
scp "${ssh_opts[@]}" "$work/web.tar.gz" "$DEPLOY_USER@$DEPLOY_HOST:$STAGED_WWW"

step "apply the deploy"
remote "sudo $APPLY"

step "verify the service is healthy"
healthy=false
for attempt in $(seq "$HEALTH_ATTEMPTS"); do
	if remote "curl -fsS --max-time $HEALTH_TIMEOUT_SECONDS $HEALTH_URL"; then
		echo
		healthy=true
		break
	fi
	echo "attempt $attempt: not up yet, retrying..."
	sleep "$HEALTH_RETRY_SECONDS"
done
if ! "$healthy"; then
	echo "the service did not answer /health after the deploy" >&2
	remote "sudo $SYSTEMCTL status $SERVICE" || true
	exit 1
fi

# Best-effort, as in deploy.yml (continue-on-error): reported, never fatal.
step "check the public endpoint"
if curl -fsS --max-time "$PUBLIC_CHECK_SECONDS" "https://$SITE_HOST/health"; then
	echo
else
	warn "$SITE_HOST is not answering yet (DNS or TLS still pending?)"
fi

step "check the page is the app, not the placeholder"
body=$(curl -s --max-time "$PUBLIC_CHECK_SECONDS" "https://$SITE_HOST/" || true)
case "$body" in
*"app.js"*) echo "the web client is being served" ;;
"") warn "no response from $SITE_HOST yet" ;;
*) warn "/ is not serving the web client" ;;
esac

step "check the API is routed, not swallowed by the static handler"
body=$(curl -s --max-time "$PUBLIC_CHECK_SECONDS" "https://$SITE_HOST$API_PATH" || true)
case "$body" in
"") warn "no response from $SITE_HOST yet" ;;
*"<!doctype html>"* | *"<html"*) warn "/api is served by the static handler; add the path to @api or @ws in deploy/botc.caddy" ;;
*'"id"'*) echo "routed to the app: $body" ;;
*) warn "unexpected answer from $API_PATH: $body" ;;
esac

printf '\ndeployed %s\n' "$(git rev-parse --short HEAD)"
