#!/usr/bin/env bash
#
# What the deploy script tests share: a scratch machine to run a real script
# against, stubs for the commands that would touch a real one, assertions, and
# the runner.
#
# A test file sources this, defines `new_machine` (which calls
# `new_bare_machine` first) and its `test_*` functions, and ends with
# `run_tests`. Sourcing it needs root, and sets the EXIT trap to `cleanup`: a
# test file must not set one of its own. Every function and constant here is
# made readonly, so a test file that redefines one fails at once rather than
# quietly changing what the other tests run against. The fixture's state --
# `MACHINE`, `STATUS` and the rest -- cannot be: a test file leaves what it does
# not use from here alone.
#
# What a test file uses from here:
#
#   constants   `DEPLOY_DIR`, `CADDY_SITE_SCRIPT`;
#               `CADDY_USER`, `CADDY_RELOAD`, `ANY_RELOAD`, `ANY_RESTART`,
#               `SHARED_IMPORT`; the modes `EXECUTABLE_MODE`, `CONFIG_MODE`,
#               `ROOT_ONLY_MODE`, `LOCK_MODE` and `SHARED_LOCK_DIR_MODE`;
#               `LONG_HOLD_SECONDS`; the stub markers `INVALID_MARKER`,
#               `BROKEN_MARKER`, `UNSTARTABLE_MARKER` and `CADDY_ERROR`, and
#               what the stubs record, `RELOADED_WITHOUT_LOCK` and
#               `LOCK_HANDED_ON`; the paths on the machine, `BINARY_PATH`,
#               `CADDYFILE_PATH`, `SITES_PATH`, `SITE_CONFIG_PATH`,
#               `OTHER_SITE_PATH`, `LOCK_DIR_PATH`, `LOCK_PATH`,
#               `CADDY_SITE_STATE_PATH` and `TAKEN_SITE_CONFIG_PATH`; the
#               contents it gives them, `OLD_SITE_CONFIG`, `NEW_SITE_CONFIG`,
#               `OTHER_SITE_CONFIG` and `UNSHARED_CADDYFILE`
#   fixture     `new_bare_machine`, `install_shared_layout`, `new_scratch_dir`,
#               `replace_with_link`, `hold_lock`, `run_on_machine`,
#               `run_on_machine_unheard`; `MACHINE`, `RELOAD_FAILS`,
#               `LOGGER_FAILS`, `MV_FAILS_FROM`, `LOCK_SECONDS`
#   assertions  `check`, and the predicates after it
#   runner      `run_tests`
#
# and what it keeps to itself: `REAL_FIND`, `REAL_MV`, `CADDY_UNIT`,
# `CADDY_SITE_PATH`, `WRITABLE_BY_OTHERS`, `SETUID_SETGID_BITS`,
# `CADDY_CHATTER`, `DEFAULT_LOCK_SECONDS`, `LOCK_POLLS`, `LOCK_POLL_SECONDS`,
# `STUBS`, `CALLS`, `SEARCHES`, `STATUS`, `SCRATCH`, `BACKGROUND`,
# `TEST_FAILED`, `cleanup`, `on_machine`.
#
# Beyond the stubs, it uses the real `flock`, `stat`, `mkfifo`, `find`, `mv` and
# `readlink` from PATH.
#
# Everything defined here is used by the files that source it, which shellcheck
# cannot see when it checks this file on its own.
# shellcheck disable=SC2034

if [[ $EUID -ne 0 ]]; then
	echo "run as root: sudo bash $0" >&2
	exit 1
fi

DEPLOY_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly DEPLOY_DIR
readonly CADDY_SITE_SCRIPT="${DEPLOY_DIR}/caddy-site.sh"
REAL_FIND="$(command -v find)"
readonly REAL_FIND
REAL_MV="$(command -v mv)"
readonly REAL_MV

readonly CADDY_USER=caddy
readonly CADDY_UNIT=caddy
readonly CADDY_RELOAD="systemctl reload ${CADDY_UNIT}"
# What no_call_mentions looks for to rule out any reload, or any restart.
readonly ANY_RELOAD="systemctl reload"
readonly ANY_RESTART="systemctl restart"
# What the stubs react to: a Caddy config containing INVALID fails validation;
# a binary containing BROKEN starts and then falls over, and one containing
# UNSTARTABLE cannot be started at all.
readonly INVALID_MARKER=INVALID
readonly BROKEN_MARKER=BROKEN
readonly UNSTARTABLE_MARKER=UNSTARTABLE
readonly CADDY_ERROR="caddy stub: the config is invalid"
# What the Caddy stub says about a valid config: like Caddy, it is never silent.
readonly CADDY_CHATTER="caddy stub: using the config"
# What the stubs record when the Caddy lock is not where it belongs: not held
# while Caddy reloads, or handed on to a command run as Caddy's account.
readonly RELOADED_WITHOUT_LOCK="systemctl reload ran without the Caddy lock held"
readonly LOCK_HANDED_ON="runuser was handed the Caddy lock"

# The modes the scripts give what they install.
readonly EXECUTABLE_MODE=755
readonly CONFIG_MODE=644
# A directory no account but root can enter, and a lock only root can open.
readonly ROOT_ONLY_MODE=700
readonly LOCK_MODE=600
# The group and other write bits, and the setuid and setgid ones.
readonly WRITABLE_BY_OTHERS=022
readonly SETUID_SETGID_BITS=6000
# /run/lock as some distributions have it: any account may create files there.
readonly SHARED_LOCK_DIR_MODE=1777

# Where the scripts keep things, relative to the machine's root.
readonly BINARY_PATH=usr/local/bin/botc-server
readonly CADDYFILE_PATH=etc/caddy/Caddyfile
readonly SITES_PATH=etc/caddy/sites
readonly SITE_CONFIG_PATH="${SITES_PATH}/botc.caddy"
readonly OTHER_SITE_PATH="${SITES_PATH}/another-app.caddy"
readonly CADDY_SITE_PATH=usr/local/sbin/botc-caddy-site
readonly LOCK_DIR_PATH=run/lock
readonly LOCK_PATH="${LOCK_DIR_PATH}/caddy-sites.lock"
readonly CADDY_SITE_STATE_PATH=var/lib/botc-caddy-site
readonly TAKEN_SITE_CONFIG_PATH="${CADDY_SITE_STATE_PATH}/botc.caddy.taken"

# Another run, holding a lock from the background long enough to outlast one
# that gives up.
readonly LONG_HOLD_SECONDS=30

# How the shared Caddyfile pulls sites in. botc ships no Caddyfile of its own
# -- the shared one belongs to the box -- so this is the whole of what the
# fixture puts there, and the line botc-caddy-site recognises the layout by.
readonly SHARED_IMPORT='import sites/*.caddy'
# A Caddyfile that imports no sites, as a box slot1's box/bootstrap.sh has
# never put on the shared layout has. botc refuses such a box.
readonly UNSHARED_CADDYFILE="botc.kiblitz.xyz { a whole site, importing nothing }"
readonly OLD_SITE_CONFIG="old site config"
readonly NEW_SITE_CONFIG="new site config"
readonly OTHER_SITE_CONFIG="another app's site config"

# How long botc-caddy-site waits for the Caddy lock unless a test says otherwise.
readonly DEFAULT_LOCK_SECONDS=1
# How often, and how many times, hold_lock looks for the lock it took.
readonly LOCK_POLL_SECONDS=0.05
readonly LOCK_POLLS=100

# --- fixture ------------------------------------------------------------------

MACHINE=""
STUBS=""
CALLS=""
SEARCHES=""
RELOAD_FAILS=0
# Set to 1, every logger call fails, as it can when journald is down.
LOGGER_FAILS=0
# Set to part of a path, every mv naming a path with it in fails.
MV_FAILS_FROM=""
# Where CI stages a deploy, and the uid standing in for the deploy account's.
readonly STAGING_DIR_PATH=var/lib/botc-deploy/staging
readonly PRIVATE_DIR_MODE=700
readonly DEFAULT_STAGING_OWNER=1002
STAGING_OWNER="${DEFAULT_STAGING_OWNER}"
LOCK_SECONDS="${DEFAULT_LOCK_SECONDS}"
STATUS=0
SCRATCH=()
# Processes a test started in the background, which cleanup stops.
BACKGROUND=()

# cleanup: stops what a test left in the background and removes its scratch
# directories.
cleanup() {
	local pid
	for pid in "${BACKGROUND[@]}"; do
		kill "${pid}" 2>/dev/null || true
		wait "${pid}" 2>/dev/null || true
	done
	BACKGROUND=()
	rm -rf "${SCRATCH[@]}"
	SCRATCH=()
}
trap cleanup EXIT

# new_scratch_dir <variable>: makes a directory that cleanup removes, and puts
# its path in the named variable.
new_scratch_dir() {
	local dir
	dir="$(mktemp -d)"
	SCRATCH+=("${dir}")
	printf -v "$1" '%s' "${dir}"
}

# new_bare_machine: a machine with nothing on it yet, and stubs that record
# what they were asked to do. Sets MACHINE (the fake filesystem root), STUBS,
# CALLS and SEARCHES.
new_bare_machine() {
	new_scratch_dir MACHINE
	new_scratch_dir STUBS
	CALLS="${STUBS}/calls"
	SEARCHES="${STUBS}/searches"
	RELOAD_FAILS=0
	LOGGER_FAILS=0
	MV_FAILS_FROM=""
	LOCK_SECONDS="${DEFAULT_LOCK_SECONDS}"
	: >"${CALLS}"
	mkdir -p "${MACHINE}/tmp" "${MACHINE}/var/lib" "${MACHINE}/${LOCK_DIR_PATH}"
	# As setup-vps.sh leaves it: the deploy account's alone. There is no such
	# account here, so the tests tell apply.sh who owns it.
	STAGING_OWNER="${DEFAULT_STAGING_OWNER}"
	install -d -m "${PRIVATE_DIR_MODE}" -o "${STAGING_OWNER}" -g "${STAGING_OWNER}" \
		"${MACHINE}/${STAGING_DIR_PATH}"

	# A reload also records whether the Caddy lock was free to take -- which it
	# must not be, while whoever asked for the reload holds it.
	cat >"${STUBS}/systemctl" <<'STUB'
#!/usr/bin/env bash
echo "systemctl $*" >>"${STUB_CALLS}"
case "$1" in
restart) ! grep -q "${STUB_UNSTARTABLE_MARKER}" "${STUB_BINARY}" ;;
is-active) ! grep -q -e "${STUB_BROKEN_MARKER}" -e "${STUB_UNSTARTABLE_MARKER}" "${STUB_BINARY}" ;;
reload)
	if flock --nonblock "${STUB_LOCK}" true; then
		echo "${STUB_RELOADED_WITHOUT_LOCK}" >>"${STUB_CALLS}"
	fi
	[[ "${STUB_RELOAD_FAILS}" -eq 0 ]]
	;;
esac
STUB
	# Talks on stderr either way, as Caddy does; a write that fails fails it.
	cat >"${STUBS}/caddy" <<'STUB'
#!/usr/bin/env bash
echo "caddy $*" >>"${STUB_CALLS}"
if grep -rq "${STUB_INVALID_MARKER}" "${STUB_CADDY_DIR}"; then
	echo "${STUB_CADDY_ERROR}" >&2
	exit 1
fi
echo "${STUB_CADDY_CHATTER}" >&2
STUB
	# Records who a command would have run as, and whether it was handed the
	# Caddy lock, then runs it.
	cat >"${STUBS}/runuser" <<'STUB'
#!/usr/bin/env bash
echo "runuser $*" >>"${STUB_CALLS}"
for fd in /proc/$$/fd/*; do
	if [[ "$(readlink "${fd}")" == "${STUB_LOCK}" ]]; then
		echo "${STUB_LOCK_HANDED_ON}" >>"${STUB_CALLS}"
	fi
done
while [[ "$1" != -- ]]; do shift; done
shift
exec "$@"
STUB
	# Records who else could reach what is being searched -- the mode of the
	# directory it sits in -- then does the real search.
	cat >"${STUBS}/find" <<'STUB'
#!/usr/bin/env bash
echo "$(stat -c '%a' "$(dirname "$1")") $1" >>"${STUB_SEARCHES}"
exec "${STUB_REAL_FIND}" "$@"
STUB
	# Fails a move naming what MV_FAILS_FROM picks out, as a full or failing
	# disk would; does every other move for real.
	cat >"${STUBS}/mv" <<'STUB'
#!/usr/bin/env bash
if [[ -n "${STUB_MV_FAILS_FROM}" ]]; then
	for arg in "$@"; do
		if [[ "${arg}" == *"${STUB_MV_FAILS_FROM}"* ]]; then
			echo "mv stub: failing to move ${arg}" >&2
			exit 1
		fi
	done
fi
exec "${STUB_REAL_MV}" "$@"
STUB
	cat >"${STUBS}/logger" <<'STUB'
#!/usr/bin/env bash
[[ "${STUB_LOGGER_FAILS}" -eq 0 ]]
STUB
	chmod "${EXECUTABLE_MODE}" "${STUBS}/systemctl" "${STUBS}/caddy" \
		"${STUBS}/runuser" "${STUBS}/find" "${STUBS}/mv" "${STUBS}/logger"
}

# install_shared_layout: the shared Caddy layout as slot1's
# box/bootstrap.sh leaves a box -- a Caddyfile that does nothing but import the
# sites directory -- with botc-caddy-site and botc's site config installed and
# recorded as the one Caddy took, plus another app's site config beside it.
install_shared_layout() {
	mkdir -p "${MACHINE}/${SITES_PATH}" "${MACHINE}/$(dirname "${CADDY_SITE_PATH}")"
	install -d -m "${ROOT_ONLY_MODE}" "${MACHINE}/${CADDY_SITE_STATE_PATH}"
	printf '%s\n' "${SHARED_IMPORT}" >"${MACHINE}/${CADDYFILE_PATH}"
	printf '%s\n' "${OLD_SITE_CONFIG}" >"${MACHINE}/${SITE_CONFIG_PATH}"
	printf '%s\n' "${OLD_SITE_CONFIG}" >"${MACHINE}/${TAKEN_SITE_CONFIG_PATH}"
	printf '%s\n' "${OTHER_SITE_CONFIG}" >"${MACHINE}/${OTHER_SITE_PATH}"
	install -m "${EXECUTABLE_MODE}" "${CADDY_SITE_SCRIPT}" "${MACHINE}/${CADDY_SITE_PATH}"
}

# replace_with_link <path>: turns a file on the machine into a link to a copy
# of itself somewhere else, as the deploy account could in its staging area.
replace_with_link() {
	mv "${MACHINE}/$1" "${MACHINE}/tmp/elsewhere"
	ln -s "${MACHINE}/tmp/elsewhere" "${MACHINE}/$1"
}

# hold_lock <path> <seconds>: takes the lock at the path on the machine from
# the background, as another run of a deploy script would, and returns once it
# is held. cleanup lets it go.
hold_lock() {
	local lock="${MACHINE}/$1" polls
	(
		exec {held_fd}>"${lock}"
		flock "${held_fd}"
		exec sleep "$2"
	) &
	BACKGROUND+=("$!")
	for ((polls = 0; polls < LOCK_POLLS; polls++)); do
		if ! flock --nonblock "${lock}" true; then
			return 0
		fi
		sleep "${LOCK_POLL_SECONDS}"
	done
	echo "the background run never took ${lock}" >&2
	return 1
}

# on_machine <script> [argument...]: runs a real deploy script against the
# scratch machine with the stubs first on PATH.
on_machine() {
	env PATH="${STUBS}:${PATH}" \
		BOTC_DEPLOY_ROOT="${MACHINE}" BOTC_APPLY_SETTLE_SECONDS=0 \
		BOTC_CADDY_SITE_LOCK_SECONDS="${LOCK_SECONDS}" \
		BOTC_APPLY_STAGING_OWNER="${STAGING_OWNER}" \
		STUB_CALLS="${CALLS}" STUB_SEARCHES="${SEARCHES}" \
		STUB_REAL_FIND="${REAL_FIND}" STUB_REAL_MV="${REAL_MV}" \
		STUB_BINARY="${MACHINE}/${BINARY_PATH}" \
		STUB_CADDY_DIR="${MACHINE}/$(dirname "${CADDYFILE_PATH}")" \
		STUB_LOCK="${MACHINE}/${LOCK_PATH}" \
		STUB_INVALID_MARKER="${INVALID_MARKER}" STUB_BROKEN_MARKER="${BROKEN_MARKER}" \
		STUB_UNSTARTABLE_MARKER="${UNSTARTABLE_MARKER}" \
		STUB_RELOADED_WITHOUT_LOCK="${RELOADED_WITHOUT_LOCK}" \
		STUB_LOCK_HANDED_ON="${LOCK_HANDED_ON}" \
		STUB_CADDY_ERROR="${CADDY_ERROR}" STUB_CADDY_CHATTER="${CADDY_CHATTER}" \
		STUB_RELOAD_FAILS="${RELOAD_FAILS}" STUB_LOGGER_FAILS="${LOGGER_FAILS}" \
		STUB_MV_FAILS_FROM="${MV_FAILS_FROM}" \
		bash "$@"
}

# run_on_machine <script> [argument...]: runs it on the machine. Sets STATUS to
# its exit code, for `succeeded` and `failed`; what it printed is for
# `output_mentions`.
run_on_machine() {
	STATUS=0
	on_machine "$@" >"${STUBS}/output" 2>&1 || STATUS=$?
}

# run_on_machine_unheard <script> [argument...]: runs it on the machine with
# everything it prints going to a pipe nobody can read -- as when the SSH
# connection a deploy runs over has dropped. Sets STATUS to its exit code.
run_on_machine_unheard() {
	local fifo="${STUBS}/unheard" both unheard
	: >"${STUBS}/output"
	mkfifo "${fifo}"
	# Held open for reading as well, the FIFO lets itself be opened for writing
	# without waiting for a reader; closed again, it has none, and never will.
	exec {both}<>"${fifo}"
	exec {unheard}>"${fifo}"
	exec {both}>&-
	STATUS=0
	on_machine "$@" >&"${unheard}" 2>&1 || STATUS=$?
	exec {unheard}>&-
}

# --- assertions ---------------------------------------------------------------

TEST_FAILED=false

# check <description> <predicate> [argument...]: records a failure, described,
# unless the predicate holds.
check() {
	local description="$1"
	shift
	if ! "$@"; then
		echo "    FAIL: ${description}"
		TEST_FAILED=true
	fi
}

contents_are() { [[ "$(cat "${MACHINE}/$1" 2>/dev/null)" == "$2" ]]; }
contents_are_not() { ! contents_are "$@"; }
shared_caddyfile_is_untouched() { contents_are "${CADDYFILE_PATH}" "${SHARED_IMPORT}"; }
succeeded() { [[ "${STATUS}" -eq 0 ]]; }
failed() { [[ "${STATUS}" -ne 0 ]]; }
# was_called matches a whole recorded command; no_call_mentions refuses any
# recorded command containing the text, so ANY_RELOAD covers every unit.
was_called() { grep -qxF "$1" "${CALLS}"; }
no_call_mentions() { ! grep -qF "$1" "${CALLS}"; }
called_times() { [[ "$(grep -cxF "$1" "${CALLS}")" -eq "$2" ]]; }
nothing_was_called() { [[ ! -s "${CALLS}" ]]; }
output_mentions() { grep -qF "$1" "${STUBS}/output"; }
owned_by_root() { [[ "$(stat -c '%u:%g' "${MACHINE}/$1")" == "0:0" ]]; }
exists() { [[ -e "${MACHINE}/$1" ]]; }
does_not_exist() { [[ ! -e "${MACHINE}/$1" && ! -L "${MACHINE}/$1" ]]; }
mode_is() { [[ "$(stat -c '%a' "${MACHINE}/$1")" == "$2" ]]; }
nothing_writable_by_others() { [[ -z "$(find "${MACHINE}/$1" -perm "/${WRITABLE_BY_OTHERS}")" ]]; }
nothing_setuid() { [[ -z "$(find "${MACHINE}/$1" -perm "/${SETUID_SETGID_BITS}")" ]]; }
no_symlinks_under() { [[ -z "$(find "${MACHINE}/$1" -type l)" ]]; }
# Something was searched, and every search ran inside a root-only directory.
searched_only_where_root_alone_can_reach() {
	[[ -s "${SEARCHES}" ]] && ! grep -qv "^${ROOT_ONLY_MODE} " "${SEARCHES}"
}

# --- runner -------------------------------------------------------------------

# run_tests: runs every test_* function on a fresh machine, and fails unless at
# least one ran and none failed.
run_tests() {
	if ! declare -F new_machine >/dev/null; then
		echo "$0 defines no new_machine for lib.sh's run_tests to call" >&2
		exit 1
	fi

	local failures=0 tests=0 test_fn
	for test_fn in $(declare -F | awk '{print $3}' | grep '^test_'); do
		tests=$((tests + 1))
		TEST_FAILED=false
		new_machine
		"${test_fn}"
		if [[ "${TEST_FAILED}" == true ]]; then
			failures=$((failures + 1))
			echo "FAIL ${test_fn}"
			sed 's/^/    | /' "${STUBS}/output"
		else
			echo "ok   ${test_fn}"
		fi
		cleanup
	done

	echo
	echo "${tests} tests, ${failures} failed"
	# A rename that stopped the runner finding tests must not read as a pass.
	[[ "${tests}" -gt 0 && "${failures}" -eq 0 ]]
}

# Everything above, and nothing a test file defines after sourcing this.
mapfile -t lib_functions < <(declare -F | awk '{print $3}')
readonly -f "${lib_functions[@]}"
unset lib_functions
