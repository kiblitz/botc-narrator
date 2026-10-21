#!/usr/bin/env bash
#
# Tests for deploy/apply.sh.
#
# apply.sh is the part of a deploy that runs as root on the VPS, and the ways it
# goes wrong -- a half-applied deploy, files owned by the wrong account, a
# tarball that writes outside the web root -- stay invisible until a reboot or
# the next deploy trips over them. So each test runs the real script against a
# scratch directory, with `systemctl`, `caddy`, `runuser`, `find`, `mv` and
# `logger` replaced by stubs that record what they were asked to do.
#
# The botc-caddy-site on the scratch machine is the real one: how it changes
# Caddy is tested in caddy_site_test.sh, and these check that a deploy goes
# through it and stops when it refuses. They also check that a refused deploy
# leaves other apps' routing alone, which caddy_site_test.sh covers as well: it
# is the promise a deploy makes to the rest of the box, and repeating it costs a
# line.
#
# Needs root, because apply.sh installs files owned by root:
#
#     sudo bash deploy/tests/apply_test.sh

set -euo pipefail

TESTS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly TESTS_DIR
# shellcheck source-path=SCRIPTDIR source=lib.sh
source "${TESTS_DIR}/lib.sh"

readonly APPLY="${DEPLOY_DIR}/apply.sh"

readonly SERVICE=botc
# A tarball built on a GitHub runner records the runner's uid on every entry.
readonly RUNNER_UID=1001

readonly PREVIOUS_BINARY_PATH="${BINARY_PATH}.previous"
readonly WEB_PARENT_PATH=var/www
readonly WEB_ROOT_PATH="${WEB_PARENT_PATH}/botc"
readonly PREVIOUS_WEB_ROOT_PATH="${WEB_ROOT_PATH}.previous"
readonly STAGED_BINARY_PATH="${STAGING_DIR_PATH}/botc-server.new"
readonly STAGED_SITE_CONFIG_PATH="${STAGING_DIR_PATH}/botc.caddy.new"
readonly STAGED_WWW_PATH="${STAGING_DIR_PATH}/botc-www.tar.gz"
# A staging directory other accounts can get into.
readonly OPEN_DIR_MODE=755
readonly APPLY_LOCK_PATH="${LOCK_DIR_PATH}/botc-apply.lock"
readonly WORK_PARENT_PATH=var/lib/botc-apply
# What a deploy killed outright -- out of memory, say -- leaves behind.
readonly KILLED_RUN_PATH="${WORK_PARENT_PATH}/run.killed"
# Where each deploy unpacks the web client before swapping it in.
readonly UNPACK_PREFIX_PATH="${WEB_PARENT_PATH}/.botc.new."
readonly KILLED_UNPACK_PATH="${UNPACK_PREFIX_PATH}killed"

readonly OLD_BINARY="old binary"
readonly ANCIENT_BINARY="a binary from some much older deploy"
readonly OLD_CLIENT="<!doctype html>old client"
readonly NEW_BINARY="new binary"
readonly NEW_CLIENT="<!doctype html>new client"
# An asset an older build shipped and a newer one no longer does.
readonly STALE_ASSET=frontend-0ld.wasm

# --- fixture ------------------------------------------------------------------

CLIENT=""

# A scratch machine on the shared layout with a working deploy already on it.
new_machine() {
	new_bare_machine
	install_shared_layout

	mkdir -p "${MACHINE}/$(dirname "${BINARY_PATH}")" "${MACHINE}/${WEB_ROOT_PATH}"
	printf '%s\n' "${OLD_BINARY}" >"${MACHINE}/${BINARY_PATH}"
	chmod "${EXECUTABLE_MODE}" "${MACHINE}/${BINARY_PATH}"
	# As the real VPS was found: an earlier deploy left the web root, and an
	# asset a later build no longer ships, owned by the runner's uid.
	printf '%s\n' "${OLD_CLIENT}" >"${MACHINE}/${WEB_ROOT_PATH}/index.html"
	printf 'stale asset\n' >"${MACHINE}/${WEB_ROOT_PATH}/${STALE_ASSET}"
	chown -R "${RUNNER_UID}:${RUNNER_UID}" "${MACHINE}/${WEB_ROOT_PATH}"
}

# A web client build in CLIENT, as a runner would produce it.
new_client() {
	new_scratch_dir CLIENT
	printf '%s\n' "${NEW_CLIENT}" >"${CLIENT}/index.html"
}

# Packs CLIENT into the staged tarball, with the runner's uid on every entry.
pack_client() {
	tar -czf "${MACHINE}/${STAGED_WWW_PATH}" \
		--owner="${RUNNER_UID}" --group="${RUNNER_UID}" -C "${CLIENT}" .
}

# stage <binary contents> <site config contents>: what the workflow stages.
stage() {
	printf '%s\n' "$1" >"${MACHINE}/${STAGED_BINARY_PATH}"
	printf '%s\n' "$2" >"${MACHINE}/${STAGED_SITE_CONFIG_PATH}"
	new_client
	pack_client
}

apply() { run_on_machine "${APPLY}"; }
apply_unheard() { run_on_machine_unheard "${APPLY}"; }

# --- tests --------------------------------------------------------------------

test_a_good_deploy_installs_everything_and_restarts() {
	stage "${NEW_BINARY}" "${NEW_SITE_CONFIG}"
	apply
	check "exits 0" succeeded
	check "installs the new binary" contents_are "${BINARY_PATH}" "${NEW_BINARY}"
	check "keeps the old binary for rollback" contents_are "${PREVIOUS_BINARY_PATH}" "${OLD_BINARY}"
	check "installs the new site config" contents_are "${SITE_CONFIG_PATH}" "${NEW_SITE_CONFIG}"
	check "leaves the shared Caddyfile as setup-vps.sh installed it" shared_caddyfile_is_untouched
	check "leaves another app's site config alone" contents_are "${OTHER_SITE_PATH}" "${OTHER_SITE_CONFIG}"
	check "serves the new web client" contents_are "${WEB_ROOT_PATH}/index.html" "${NEW_CLIENT}"
	check "keeps the old web client for rollback" contents_are "${PREVIOUS_WEB_ROOT_PATH}/index.html" "${OLD_CLIENT}"
	check "drops assets the new build no longer ships" does_not_exist "${WEB_ROOT_PATH}/${STALE_ASSET}"
	check "reloads Caddy" was_called "${CADDY_RELOAD}"
	check "restarts the service" was_called "systemctl restart ${SERVICE}"
	check "clears the staged binary" does_not_exist "${STAGED_BINARY_PATH}"
	check "clears the staged site config" does_not_exist "${STAGED_SITE_CONFIG_PATH}"
	check "clears the staged web client" does_not_exist "${STAGED_WWW_PATH}"
	check "keeps its lock where only root can open it" mode_is "${APPLY_LOCK_PATH}" "${LOCK_MODE}"
}

test_a_deploy_that_changes_no_routing_leaves_caddy_alone() {
	# Most deploys. A reload would close every app's sockets for nothing.
	stage "${NEW_BINARY}" "${OLD_SITE_CONFIG}"
	apply
	check "exits 0" succeeded
	check "installs the new binary" contents_are "${BINARY_PATH}" "${NEW_BINARY}"
	check "does not reload Caddy" no_call_mentions "${ANY_RELOAD}"
	check "restarts the service" was_called "systemctl restart ${SERVICE}"
}

test_a_missing_staged_file_touches_nothing() {
	stage "${NEW_BINARY}" "${NEW_SITE_CONFIG}"
	rm "${MACHINE}/${STAGED_WWW_PATH}"
	apply
	check "exits non-zero" failed
	check "leaves the binary alone" contents_are "${BINARY_PATH}" "${OLD_BINARY}"
	check "leaves the site config alone" contents_are "${SITE_CONFIG_PATH}" "${OLD_SITE_CONFIG}"
	check "runs no Caddy or systemctl command" nothing_was_called
	# A staged file left behind is one a later run could pick up.
	check "still clears what was staged" does_not_exist "${STAGED_BINARY_PATH}"
}

test_a_symlinked_staged_file_is_refused() {
	# The deploy account can put anything in its staging area. A staged file
	# that is really a link to somewhere else -- a file only root can read, say
	# -- must not be installed as root.
	stage "${NEW_BINARY}" "${NEW_SITE_CONFIG}"
	replace_with_link "${STAGED_BINARY_PATH}"
	apply
	check "exits non-zero" failed
	check "leaves the binary alone" contents_are "${BINARY_PATH}" "${OLD_BINARY}"
	check "runs no Caddy or systemctl command" nothing_was_called
}

test_a_staging_directory_open_to_other_accounts_is_refused() {
	# Then any account could have planted what is in it.
	stage "${NEW_BINARY}" "${NEW_SITE_CONFIG}"
	chmod "${OPEN_DIR_MODE}" "${MACHINE}/${STAGING_DIR_PATH}"
	apply
	check "exits non-zero" failed
	check "says why" output_mentions "alone"
	check "leaves the binary alone" contents_are "${BINARY_PATH}" "${OLD_BINARY}"
	check "leaves what was staged where it is" exists "${STAGED_BINARY_PATH}"
	check "runs no Caddy or systemctl command" nothing_was_called
}

test_a_staging_directory_that_is_somebody_elses_is_refused() {
	stage "${NEW_BINARY}" "${NEW_SITE_CONFIG}"
	chown "${RUNNER_UID}" "${MACHINE}/${STAGING_DIR_PATH}"
	apply
	check "exits non-zero" failed
	check "leaves the binary alone" contents_are "${BINARY_PATH}" "${OLD_BINARY}"
	check "runs no Caddy or systemctl command" nothing_was_called
}

test_a_staging_directory_that_is_a_link_is_refused() {
	stage "${NEW_BINARY}" "${NEW_SITE_CONFIG}"
	replace_with_link "${STAGING_DIR_PATH}"
	apply
	check "exits non-zero" failed
	check "leaves the binary alone" contents_are "${BINARY_PATH}" "${OLD_BINARY}"
	check "runs no Caddy or systemctl command" nothing_was_called
}

test_a_box_without_the_shared_layout_is_refused() {
	# A site config installed where nothing imports it would change no routing,
	# and the deploy would report success anyway.
	printf '%s\n' "${UNSHARED_CADDYFILE}" >"${MACHINE}/${CADDYFILE_PATH}"
	stage "${NEW_BINARY}" "${NEW_SITE_CONFIG}"
	apply
	check "exits non-zero" failed
	check "leaves the Caddyfile alone" contents_are "${CADDYFILE_PATH}" "${UNSHARED_CADDYFILE}"
	check "leaves the binary alone" contents_are "${BINARY_PATH}" "${OLD_BINARY}"
	check "leaves the web client alone" contents_are "${WEB_ROOT_PATH}/index.html" "${OLD_CLIENT}"
	check "runs no Caddy or systemctl command" nothing_was_called
}

test_a_refused_site_config_stops_the_deploy() {
	stage "${NEW_BINARY}" "${INVALID_MARKER}"
	apply
	check "exits non-zero" failed
	check "puts the previous site config back" contents_are "${SITE_CONFIG_PATH}" "${OLD_SITE_CONFIG}"
	check "leaves the shared Caddyfile alone" shared_caddyfile_is_untouched
	check "leaves another app's site config alone" contents_are "${OTHER_SITE_PATH}" "${OTHER_SITE_CONFIG}"
	check "leaves the binary alone" contents_are "${BINARY_PATH}" "${OLD_BINARY}"
	check "leaves the web client alone" contents_are "${WEB_ROOT_PATH}/index.html" "${OLD_CLIENT}"
	check "does not reload Caddy" no_call_mentions "${ANY_RELOAD}"
	check "does not restart the service" no_call_mentions "${ANY_RESTART}"
	check "shows Caddy's own explanation" output_mentions "${CADDY_ERROR}"
}

test_a_failed_caddy_reload_leaves_nothing_half_applied() {
	# The half-applied deploy this script exists to prevent: if Caddy will not
	# take the new config, nothing after it may happen either.
	RELOAD_FAILS=1
	stage "${NEW_BINARY}" "${NEW_SITE_CONFIG}"
	apply
	check "exits non-zero" failed
	check "puts the previous site config back" contents_are "${SITE_CONFIG_PATH}" "${OLD_SITE_CONFIG}"
	check "leaves the shared Caddyfile alone" shared_caddyfile_is_untouched
	check "leaves another app's site config alone" contents_are "${OTHER_SITE_PATH}" "${OTHER_SITE_CONFIG}"
	check "leaves the binary alone" contents_are "${BINARY_PATH}" "${OLD_BINARY}"
	check "leaves the web client alone" contents_are "${WEB_ROOT_PATH}/index.html" "${OLD_CLIENT}"
	check "does not restart the service" no_call_mentions "${ANY_RESTART}"
}

test_a_binary_that_does_not_come_up_is_rolled_back() {
	stage "${BROKEN_MARKER} binary" "${NEW_SITE_CONFIG}"
	apply
	check "exits non-zero" failed
	check "restores the previous binary" contents_are "${BINARY_PATH}" "${OLD_BINARY}"
	check "restores the web client that went with it" contents_are "${WEB_ROOT_PATH}/index.html" "${OLD_CLIENT}"
	check "keeps the site config Caddy already took" contents_are "${SITE_CONFIG_PATH}" "${NEW_SITE_CONFIG}"
	check "restarts once for the deploy and once for the rollback" called_times "systemctl restart ${SERVICE}" 2
}

test_a_binary_that_cannot_even_start_is_rolled_back() {
	# The unit is Type=exec, so a binary that cannot be executed at all -- the
	# wrong architecture, a truncated copy -- makes `systemctl restart` itself
	# fail. That is the likeliest broken deploy, and it must still roll back.
	stage "${UNSTARTABLE_MARKER} binary" "${NEW_SITE_CONFIG}"
	apply
	check "exits non-zero" failed
	check "restores the previous binary" contents_are "${BINARY_PATH}" "${OLD_BINARY}"
	check "restores the web client that went with it" contents_are "${WEB_ROOT_PATH}/index.html" "${OLD_CLIENT}"
	check "restarts again for the rollback" called_times "systemctl restart ${SERVICE}" 2
}

test_a_stale_previous_binary_is_never_rolled_back_to() {
	# A backup is only worth restoring if it is what was running. With no binary
	# installed -- a first deploy, or one removed by hand -- a .previous left by
	# some much older deploy must not be what a failed start goes back to.
	rm "${MACHINE}/${BINARY_PATH}"
	printf '%s\n' "${ANCIENT_BINARY}" >"${MACHINE}/${PREVIOUS_BINARY_PATH}"
	chmod "${EXECUTABLE_MODE}" "${MACHINE}/${PREVIOUS_BINARY_PATH}"
	stage "${BROKEN_MARKER} binary" "${NEW_SITE_CONFIG}"
	apply
	check "exits non-zero" failed
	check "does not install the older deploy's binary" contents_are_not "${BINARY_PATH}" "${ANCIENT_BINARY}"
}

test_web_client_files_belong_to_root_not_the_build_machine() {
	# A runner-built tarball carries uid 1001, and root's tar keeps archived
	# owners by default. On the VPS that uid is whichever account was created
	# second, which then owns -- and can rewrite -- the page everybody loads.
	stage "${NEW_BINARY}" "${NEW_SITE_CONFIG}"
	apply
	check "exits 0" succeeded
	check "installs index.html owned by root" owned_by_root "${WEB_ROOT_PATH}/index.html"
	check "installs the web root owned by root" owned_by_root "${WEB_ROOT_PATH}"
}

test_web_client_permissions_are_normalised() {
	# Root's tar also keeps archived modes. A world-writable entry would let any
	# account on the shared box rewrite the page; a setuid one becomes setuid
	# root once root owns it.
	stage "${NEW_BINARY}" "${NEW_SITE_CONFIG}"
	mkdir "${CLIENT}/assets"
	printf 'x\n' >"${CLIENT}/assets/app.js"
	printf 'x\n' >"${CLIENT}/assets/helper"
	chmod 777 "${CLIENT}/assets"
	chmod 666 "${CLIENT}/assets/app.js"
	chmod 4755 "${CLIENT}/assets/helper"
	pack_client
	apply
	check "exits 0" succeeded
	check "leaves nothing in the web root writable by group or others" nothing_writable_by_others "${WEB_ROOT_PATH}"
	check "leaves nothing in the web root setuid or setgid" nothing_setuid "${WEB_ROOT_PATH}"
	check "makes the web root itself ${EXECUTABLE_MODE}" mode_is "${WEB_ROOT_PATH}" "${EXECUTABLE_MODE}"
}

test_a_web_client_containing_a_symlink_is_refused() {
	# A link in one deploy is a path the next deploy's tar would follow as root:
	# ship `x -> /etc`, then `x/cron.d/evil`. The web client is files and
	# directories only.
	stage "${NEW_BINARY}" "${NEW_SITE_CONFIG}"
	ln -s /etc "${CLIENT}/escape"
	pack_client
	apply
	check "exits non-zero" failed
	check "leaves the web client alone" contents_are "${WEB_ROOT_PATH}/index.html" "${OLD_CLIENT}"
	check "leaves no link anywhere under the web parent" no_symlinks_under "${WEB_PARENT_PATH}"
	check "runs no Caddy or systemctl command" nothing_was_called
}

test_a_web_client_containing_a_special_file_is_refused() {
	# A named pipe stands in for the dangerous case, a device file.
	stage "${NEW_BINARY}" "${NEW_SITE_CONFIG}"
	mkfifo "${CLIENT}/pipe"
	pack_client
	apply
	check "exits non-zero" failed
	check "leaves the web client alone" contents_are "${WEB_ROOT_PATH}/index.html" "${OLD_CLIENT}"
	check "runs no Caddy or systemctl command" nothing_was_called
}

test_the_unpacked_web_client_is_checked_where_only_root_can_reach_it() {
	# Root's tar creates whatever the archive holds, a device file included,
	# before anything can check it. Unpacked where other accounts can reach it,
	# one of them could open such a file in the moment before it is refused, and
	# keep it open afterwards. Until the client has passed, only root may reach
	# it.
	stage "${NEW_BINARY}" "${NEW_SITE_CONFIG}"
	apply
	check "exits 0" succeeded
	check "checks it inside a directory only root can enter" searched_only_where_root_alone_can_reach
}

test_a_deploy_nobody_is_watching_any_more_still_rolls_back() {
	# A cancelled workflow or a dropped SSH connection closes the pipe the
	# script's messages go to. The next message must not be its last: a binary
	# that did not come up still has to be rolled back.
	stage "${BROKEN_MARKER} binary" "${NEW_SITE_CONFIG}"
	apply_unheard
	check "exits non-zero" failed
	check "restores the previous binary" contents_are "${BINARY_PATH}" "${OLD_BINARY}"
	check "restores the web client that went with it" contents_are "${WEB_ROOT_PATH}/index.html" "${OLD_CLIENT}"
}

test_a_good_deploy_nobody_is_watching_any_more_still_goes_through() {
	stage "${NEW_BINARY}" "${NEW_SITE_CONFIG}"
	apply_unheard
	check "exits 0" succeeded
	check "installs the new binary" contents_are "${BINARY_PATH}" "${NEW_BINARY}"
	check "serves the new web client" contents_are "${WEB_ROOT_PATH}/index.html" "${NEW_CLIENT}"
}

test_a_second_deploy_while_one_is_running_is_refused() {
	# CI's concurrency group serialises its own runs, not a botc-apply started by
	# hand meanwhile. Two at once would claim each other's staged files and
	# remove each other's rollback copies.
	stage "${NEW_BINARY}" "${NEW_SITE_CONFIG}"
	hold_lock "${APPLY_LOCK_PATH}" "${LONG_HOLD_SECONDS}"
	apply
	check "exits non-zero" failed
	check "leaves the binary alone" contents_are "${BINARY_PATH}" "${OLD_BINARY}"
	check "runs no Caddy or systemctl command" nothing_was_called
	check "leaves what was staged to the deploy already running" contents_are "${STAGED_BINARY_PATH}" "${NEW_BINARY}"
}

test_an_unchanged_binary_is_not_restarted() {
	# A restart ends every game in progress, and most deploys
	# -- docs, the web client, CI -- ship the same binary again.
	stage "${OLD_BINARY}" "${NEW_SITE_CONFIG}"
	apply
	check "exits 0" succeeded
	check "serves the new web client" contents_are "${WEB_ROOT_PATH}/index.html" "${NEW_CLIENT}"
	check "does not restart the service" no_call_mentions "${ANY_RESTART}"
}

test_a_lock_other_accounts_could_open_is_made_root_only() {
	# flock takes a lock through any open file, even one opened only to read:
	# an account that can read the lock can hold it, and block every deploy.
	printf '' >"${MACHINE}/${APPLY_LOCK_PATH}"
	chmod "${CONFIG_MODE}" "${MACHINE}/${APPLY_LOCK_PATH}"
	stage "${NEW_BINARY}" "${NEW_SITE_CONFIG}"
	apply
	check "exits 0" succeeded
	check "leaves the lock where only root can open it" mode_is "${APPLY_LOCK_PATH}" "${LOCK_MODE}"
}

test_a_lock_directory_other_accounts_can_write_to_is_refused() {
	# Where any account can create files, one could create the lock first, as
	# its own, and hold it for good.
	chmod "${SHARED_LOCK_DIR_MODE}" "${MACHINE}/${LOCK_DIR_PATH}"
	stage "${NEW_BINARY}" "${NEW_SITE_CONFIG}"
	apply
	check "exits non-zero" failed
	check "leaves the binary alone" contents_are "${BINARY_PATH}" "${OLD_BINARY}"
	check "runs no Caddy or systemctl command" nothing_was_called
	check "names the directory" output_mentions "${MACHINE}/${LOCK_DIR_PATH}"
}

test_what_a_killed_deploy_left_behind_is_cleared() {
	# A deploy killed outright runs no clean-up of its own, and on a small disk
	# every copy of the web client and binary it leaves adds up.
	mkdir -p "${MACHINE}/${KILLED_RUN_PATH}" "${MACHINE}/${KILLED_UNPACK_PATH}"
	stage "${NEW_BINARY}" "${NEW_SITE_CONFIG}"
	apply
	check "exits 0" succeeded
	check "clears the killed run's claimed files" does_not_exist "${KILLED_RUN_PATH}"
	check "clears the killed run's unpacked web client" does_not_exist "${KILLED_UNPACK_PATH}"
}

test_a_deploy_that_fails_after_swapping_the_web_client_puts_it_back() {
	# Anything can fail once the new web client is in -- a full disk as the
	# binary goes in -- and the client must not be left serving beside a server
	# it was not built for.
	mkdir "${MACHINE}/${BINARY_PATH}.new"
	stage "${NEW_BINARY}" "${NEW_SITE_CONFIG}"
	apply
	check "exits non-zero" failed
	check "puts the previous web client back" contents_are "${WEB_ROOT_PATH}/index.html" "${OLD_CLIENT}"
	check "leaves the binary as it was" contents_are "${BINARY_PATH}" "${OLD_BINARY}"
	check "does not restart the service" no_call_mentions "${ANY_RESTART}"
}

test_a_web_client_that_fails_to_move_in_puts_the_previous_one_back() {
	# Between the swap's two renames there is no web root at all, so a failure
	# at the second must not leave Caddy nothing to serve.
	MV_FAILS_FROM="${UNPACK_PREFIX_PATH}"
	stage "${NEW_BINARY}" "${NEW_SITE_CONFIG}"
	apply
	check "exits non-zero" failed
	check "puts the previous web client back" contents_are "${WEB_ROOT_PATH}/index.html" "${OLD_CLIENT}"
	check "leaves the binary alone" contents_are "${BINARY_PATH}" "${OLD_BINARY}"
	check "does not restart the service" no_call_mentions "${ANY_RESTART}"
}

test_a_web_client_a_killed_deploy_set_aside_is_put_back_first() {
	# A deploy killed between the swap's two renames leaves no web root, and the
	# only working client at .previous -- which the next deploy would otherwise
	# delete before it had anything to roll back to.
	mv "${MACHINE}/${WEB_ROOT_PATH}" "${MACHINE}/${PREVIOUS_WEB_ROOT_PATH}"
	stage "${BROKEN_MARKER} binary" "${NEW_SITE_CONFIG}"
	apply
	check "exits non-zero" failed
	check "rolls back to the web client the killed deploy set aside" contents_are "${WEB_ROOT_PATH}/index.html" "${OLD_CLIENT}"
}

test_a_deploy_goes_through_when_logging_fails() {
	# The journal is a record, not a step: a logger that fails -- journald down,
	# say -- must not stop a deploy, least of all once it has changed things.
	LOGGER_FAILS=1
	stage "${NEW_BINARY}" "${NEW_SITE_CONFIG}"
	apply
	check "exits 0" succeeded
	check "installs the new binary" contents_are "${BINARY_PATH}" "${NEW_BINARY}"
	check "serves the new web client" contents_are "${WEB_ROOT_PATH}/index.html" "${NEW_CLIENT}"
}

test_a_staged_directory_is_refused_where_it_lies() {
	# A directory is a tree the deploy account could rearrange while root
	# copied it. It is refused without being copied.
	stage "${NEW_BINARY}" "${NEW_SITE_CONFIG}"
	rm "${MACHINE}/${STAGED_WWW_PATH}"
	mkdir "${MACHINE}/${STAGED_WWW_PATH}"
	apply
	check "exits non-zero" failed
	check "leaves the directory where it was staged" exists "${STAGED_WWW_PATH}"
	check "leaves the web client alone" contents_are "${WEB_ROOT_PATH}/index.html" "${OLD_CLIENT}"
	check "runs no Caddy or systemctl command" nothing_was_called
}

run_tests
