#!/usr/bin/env bash
#
# Tests for deploy/caddy-site.sh, installed on the VPS as botc-caddy-site.
#
# It is how botc changes Caddy on a box other apps share, so the ways
# it goes wrong reach past botc: a broken or clashing site config left on disk
# takes every site down at Caddy's next start, a needless reload closes every
# app's proxied sockets, and two apps changing Caddy at once can each reload the
# other's half-made change. Worse still would be writing the shared Caddyfile,
# which belongs to the box; several tests below exist only to pin that down.
#
# Each test runs the real script against a scratch machine, with `systemctl`,
# `caddy`, `runuser`, `find`, `mv` and `logger` stubbed by lib.sh.
#
# Needs root, because the script installs files owned by root:
#
#     sudo bash deploy/tests/caddy_site_test.sh

set -euo pipefail

TESTS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly TESTS_DIR
# shellcheck source-path=SCRIPTDIR source=lib.sh
source "${TESTS_DIR}/lib.sh"

readonly STAGED_PATH=tmp/botc.caddy
# A shared Caddyfile someone has since given a global option, for another app.
readonly EDITED_SHARED_CADDYFILE="{
	email someone@example.com
}
${SHARED_IMPORT}"

# Another app's deploy, holding the Caddy lock from the background only briefly,
# and a run patient enough to outlast it.
readonly SHORT_HOLD_SECONDS=1
readonly PATIENT_LOCK_SECONDS=10

# --- fixture ------------------------------------------------------------------

new_machine() {
	new_bare_machine
	install_shared_layout
}

# As a box that slot1's box/bootstrap.sh has never put on the shared
# layout: a Caddyfile that imports nothing, no sites directory, and nothing
# botc-caddy-site ever kept. botc never moves such a box -- it refuses it.
make_it_a_box_off_the_shared_layout() {
	printf '%s\n' "${UNSHARED_CADDYFILE}" >"${MACHINE}/${CADDYFILE_PATH}"
	rm -rf "${MACHINE:?}/${SITES_PATH}" "${MACHINE:?}/${CADDY_SITE_STATE_PATH}"
}

# stage <site config contents>: what a deploy stages.
stage() { printf '%s\n' "$1" >"${MACHINE}/${STAGED_PATH}"; }

# What a deploy runs, and what setup-vps.sh runs: the site config, and nothing
# else. This script takes no Caddyfile: the shared one is the box's.
install_site() { run_on_machine "${CADDY_SITE_SCRIPT}" "${MACHINE}/${STAGED_PATH}"; }

validated_as_caddy() {
	was_called "runuser -u ${CADDY_USER} -- caddy validate --config ${MACHINE}/${CADDYFILE_PATH} --adapter caddyfile"
}

# --- tests: installing a site config ------------------------------------------

test_a_changed_site_config_is_installed_and_caddy_reloaded() {
	stage "${NEW_SITE_CONFIG}"
	install_site
	check "exits 0" succeeded
	check "installs the site config" contents_are "${SITE_CONFIG_PATH}" "${NEW_SITE_CONFIG}"
	check "makes the site config root's" owned_by_root "${SITE_CONFIG_PATH}"
	check "makes the site config ${CONFIG_MODE}" mode_is "${SITE_CONFIG_PATH}" "${CONFIG_MODE}"
	check "records it as the one Caddy took" contents_are "${TAKEN_SITE_CONFIG_PATH}" "${NEW_SITE_CONFIG}"
	check "leaves the shared Caddyfile alone" shared_caddyfile_is_untouched
	check "leaves another app's site config alone" contents_are "${OTHER_SITE_PATH}" "${OTHER_SITE_CONFIG}"
	check "reloads Caddy once" called_times "${CADDY_RELOAD}" 1
	check "reloads only while holding the Caddy lock" no_call_mentions "${RELOADED_WITHOUT_LOCK}"
}

test_the_shared_caddyfile_is_validated_as_the_caddy_user() {
	# The shared file, not botc's alone, so a clash with another app's site is
	# caught. As Caddy's own account, because validating creates any missing log
	# file the config names, and one created by root is one Caddy cannot open.
	stage "${NEW_SITE_CONFIG}"
	install_site
	check "exits 0" succeeded
	check "validates the shared Caddyfile through runuser as ${CADDY_USER}" validated_as_caddy
	# Held open by a process of Caddy's account, the lock would outlive this
	# run for as long as that process did, and block every app's deploy.
	check "does not hand the Caddy lock to Caddy's account" no_call_mentions "${LOCK_HANDED_ON}"
}

test_an_unchanged_site_config_leaves_caddy_alone() {
	# A reload closes every proxied socket on the box, and most deploys change
	# no routing at all.
	stage "${OLD_SITE_CONFIG}"
	install_site
	check "exits 0" succeeded
	check "neither validates nor reloads" nothing_was_called
}

test_a_site_config_caddy_never_took_is_loaded_even_if_unchanged() {
	# The file on disk is not always what Caddy runs: a run cut off after
	# installing it, or a copy made by hand, leaves one Caddy never took. Only
	# what Caddy has taken counts as unchanged.
	rm "${MACHINE}/${TAKEN_SITE_CONFIG_PATH}"
	stage "${OLD_SITE_CONFIG}"
	install_site
	check "exits 0" succeeded
	check "validates it" validated_as_caddy
	check "reloads Caddy once" called_times "${CADDY_RELOAD}" 1
	check "records it as the one Caddy took" contents_are "${TAKEN_SITE_CONFIG_PATH}" "${OLD_SITE_CONFIG}"
}

test_an_invalid_site_config_is_not_left_on_disk() {
	# Caddy keeps serving its old config when a reload is refused, but the files
	# on disk are what it reads at its next start -- so a bad one left there
	# takes every site down at the next reboot instead of now.
	stage "${INVALID_MARKER}"
	install_site
	check "exits non-zero" failed
	check "puts the previous site config back" contents_are "${SITE_CONFIG_PATH}" "${OLD_SITE_CONFIG}"
	check "keeps the previous one recorded as taken" contents_are "${TAKEN_SITE_CONFIG_PATH}" "${OLD_SITE_CONFIG}"
	check "leaves another app's site config alone" contents_are "${OTHER_SITE_PATH}" "${OTHER_SITE_CONFIG}"
	check "does not reload Caddy" no_call_mentions "${ANY_RELOAD}"
	check "shows Caddy's own explanation" output_mentions "${CADDY_ERROR}"
}

test_an_invalid_site_config_is_put_back_with_nobody_watching() {
	# A deploy whose SSH connection has dropped: what it writes next goes
	# nowhere, and must not stop it before what it installed is put back.
	stage "${INVALID_MARKER}"
	run_on_machine_unheard "${CADDY_SITE_SCRIPT}" "${MACHINE}/${STAGED_PATH}"
	check "exits non-zero" failed
	check "puts the previous site config back" contents_are "${SITE_CONFIG_PATH}" "${OLD_SITE_CONFIG}"
}

test_a_site_config_installed_with_nobody_watching_still_succeeds() {
	stage "${NEW_SITE_CONFIG}"
	run_on_machine_unheard "${CADDY_SITE_SCRIPT}" "${MACHINE}/${STAGED_PATH}"
	check "exits 0" succeeded
	check "installs the site config" contents_are "${SITE_CONFIG_PATH}" "${NEW_SITE_CONFIG}"
}

test_a_stale_backup_is_never_restored() {
	# A backup left by an earlier run is not a copy of what is there now. With
	# no site config in place, a refused one must leave none, not resurrect
	# whatever the last run happened to replace.
	stage "${NEW_SITE_CONFIG}"
	install_site
	rm "${MACHINE}/${SITE_CONFIG_PATH}"
	stage "${INVALID_MARKER}"
	install_site
	check "exits non-zero" failed
	check "leaves no site config rather than an old one" does_not_exist "${SITE_CONFIG_PATH}"
}

test_a_failed_reload_puts_the_previous_site_config_back() {
	RELOAD_FAILS=1
	stage "${NEW_SITE_CONFIG}"
	install_site
	check "exits non-zero" failed
	check "puts the previous site config back" contents_are "${SITE_CONFIG_PATH}" "${OLD_SITE_CONFIG}"
	check "reloads again to go back to it" called_times "${CADDY_RELOAD}" 2
	# Both reloads failed, and a failed one may still have reached Caddy: what
	# it runs is not known, so the next run must not skip a reload on the
	# strength of the old record.
	check "forgets which site config Caddy took" does_not_exist "${TAKEN_SITE_CONFIG_PATH}"
	check "reloads only while holding the Caddy lock" no_call_mentions "${RELOADED_WITHOUT_LOCK}"
}

# --- tests: the shared Caddyfile is the box's ---------------------------------

test_a_box_without_the_shared_layout_is_refused() {
	# A site config installed beside a Caddyfile that imports nothing would be
	# read by nobody, and the deploy would "succeed" with the routing unchanged.
	# botc does not move the box onto the layout -- slot1's
	# box/bootstrap.sh does, and this says so.
	make_it_a_box_off_the_shared_layout
	stage "${NEW_SITE_CONFIG}"
	install_site
	check "exits non-zero" failed
	check "leaves the Caddyfile alone" contents_are "${CADDYFILE_PATH}" "${UNSHARED_CADDYFILE}"
	check "installs no site config" does_not_exist "${SITE_CONFIG_PATH}"
	check "creates no sites directory" does_not_exist "${SITES_PATH}"
	check "runs no Caddy or systemctl command" nothing_was_called
	check "says which script puts a box on the layout" output_mentions "box/bootstrap.sh"
}

test_a_box_without_a_caddyfile_at_all_is_refused() {
	# A fresh box, or one whose Caddyfile was removed by hand. Writing one is
	# not botc's to do, so there is nothing to do but refuse.
	make_it_a_box_off_the_shared_layout
	rm "${MACHINE}/${CADDYFILE_PATH}"
	stage "${NEW_SITE_CONFIG}"
	install_site
	check "exits non-zero" failed
	check "writes no Caddyfile" does_not_exist "${CADDYFILE_PATH}"
	check "installs no site config" does_not_exist "${SITE_CONFIG_PATH}"
	check "runs no Caddy or systemctl command" nothing_was_called
}

test_a_commented_out_import_does_not_count() {
	printf '# %s\n' "${SHARED_IMPORT}" >"${MACHINE}/${CADDYFILE_PATH}"
	stage "${NEW_SITE_CONFIG}"
	install_site
	check "exits non-zero" failed
	check "leaves the site config alone" contents_are "${SITE_CONFIG_PATH}" "${OLD_SITE_CONFIG}"
	check "runs no Caddy or systemctl command" nothing_was_called
}

test_an_edited_shared_caddyfile_is_left_exactly_as_it_is() {
	# The box's Caddyfile is the box's: a global option added there for another
	# app's sake must survive every botc deploy, byte for byte. This is the
	# rule slot0's ticket-to-ride broke before its D-0024.
	printf '%s\n' "${EDITED_SHARED_CADDYFILE}" >"${MACHINE}/${CADDYFILE_PATH}"
	stage "${NEW_SITE_CONFIG}"
	install_site
	check "exits 0" succeeded
	check "leaves the shared Caddyfile as it was" contents_are "${CADDYFILE_PATH}" "${EDITED_SHARED_CADDYFILE}"
	check "installs the site config" contents_are "${SITE_CONFIG_PATH}" "${NEW_SITE_CONFIG}"
	check "reloads Caddy once" called_times "${CADDY_RELOAD}" 1
}

test_a_refused_site_config_leaves_the_shared_caddyfile_alone() {
	printf '%s\n' "${EDITED_SHARED_CADDYFILE}" >"${MACHINE}/${CADDYFILE_PATH}"
	stage "${INVALID_MARKER}"
	install_site
	check "exits non-zero" failed
	check "leaves the shared Caddyfile as it was" contents_are "${CADDYFILE_PATH}" "${EDITED_SHARED_CADDYFILE}"
}

# --- tests: staging, locking and logging --------------------------------------

test_a_symlinked_staged_file_is_refused() {
	stage "${NEW_SITE_CONFIG}"
	replace_with_link "${STAGED_PATH}"
	install_site
	check "exits non-zero" failed
	check "leaves the site config alone" contents_are "${SITE_CONFIG_PATH}" "${OLD_SITE_CONFIG}"
	check "runs no Caddy or systemctl command" nothing_was_called
}

test_a_lock_directory_other_accounts_can_write_to_is_refused() {
	# Where any account can create files, one could take the Caddy lock first
	# and hold it for good, or leave a file of its own where the lock goes.
	chmod "${SHARED_LOCK_DIR_MODE}" "${MACHINE}/${LOCK_DIR_PATH}"
	stage "${NEW_SITE_CONFIG}"
	install_site
	check "exits non-zero" failed
	check "leaves the site config alone" contents_are "${SITE_CONFIG_PATH}" "${OLD_SITE_CONFIG}"
	check "runs no Caddy or systemctl command" nothing_was_called
	check "names the directory" output_mentions "${MACHINE}/${LOCK_DIR_PATH}"
}

test_a_lock_other_accounts_could_open_is_made_root_only() {
	# flock takes a lock through any open file, even one opened only to read:
	# an account that could read the Caddy lock could hold it, and block every
	# app's deploy.
	printf '' >"${MACHINE}/${LOCK_PATH}"
	chmod "${CONFIG_MODE}" "${MACHINE}/${LOCK_PATH}"
	stage "${NEW_SITE_CONFIG}"
	install_site
	check "exits 0" succeeded
	check "leaves the lock where only root can open it" mode_is "${LOCK_PATH}" "${LOCK_MODE}"
}

test_a_deploy_gives_up_while_another_holds_the_caddy_lock() {
	hold_lock "${LOCK_PATH}" "${LONG_HOLD_SECONDS}"
	stage "${NEW_SITE_CONFIG}"
	install_site
	check "exits non-zero" failed
	check "leaves the site config alone" contents_are "${SITE_CONFIG_PATH}" "${OLD_SITE_CONFIG}"
	check "runs no Caddy or systemctl command" nothing_was_called
	check "names the lock it waited for" output_mentions "${LOCK_PATH}"
}

test_a_deploy_waits_for_another_to_release_the_caddy_lock() {
	hold_lock "${LOCK_PATH}" "${SHORT_HOLD_SECONDS}"
	LOCK_SECONDS="${PATIENT_LOCK_SECONDS}"
	stage "${NEW_SITE_CONFIG}"
	install_site
	check "exits 0" succeeded
	check "installs the site config" contents_are "${SITE_CONFIG_PATH}" "${NEW_SITE_CONFIG}"
}

test_the_layout_is_decided_under_the_lock() {
	# Another app's deploy changes exactly what the layout check reads. Deciding
	# before the lock is held would mean acting on what was true beforehand.
	hold_lock "${LOCK_PATH}" "${LONG_HOLD_SECONDS}"
	make_it_a_box_off_the_shared_layout
	stage "${NEW_SITE_CONFIG}"
	install_site
	check "exits non-zero" failed
	# Off the layout and unable to take the lock: the lock is what it must
	# report, because it never got as far as looking at the Caddyfile.
	check "names the lock rather than the layout" output_mentions "${LOCK_PATH}"
}

test_a_record_that_cannot_be_written_does_not_undo_a_taken_change() {
	# Once Caddy has reloaded, the change has happened. Failing to write down
	# what it took must not read as a refusal: a deploy would stop halfway,
	# leaving a binary and web client that never got installed.
	MV_FAILS_FROM=".taken"
	stage "${NEW_SITE_CONFIG}"
	install_site
	check "exits 0" succeeded
	check "installs the site config" contents_are "${SITE_CONFIG_PATH}" "${NEW_SITE_CONFIG}"
	check "reloads Caddy once" called_times "${CADDY_RELOAD}" 1
	check "forgets the previous record, which no longer holds" does_not_exist "${TAKEN_SITE_CONFIG_PATH}"
	check "leaves no draft of the record" \
		does_not_exist "$(dirname "${TAKEN_SITE_CONFIG_PATH}")/.$(basename "${TAKEN_SITE_CONFIG_PATH}").new"
	check "says the next run will reload Caddy again" output_mentions "reload Caddy again"
}

test_a_site_config_that_cannot_be_installed_leaves_no_draft() {
	# The draft is named so the shared Caddyfile's import never picks it up, but
	# one left behind would still be a stale copy in Caddy's own directory.
	MV_FAILS_FROM="${SITE_CONFIG_PATH}"
	stage "${NEW_SITE_CONFIG}"
	install_site
	check "exits non-zero" failed
	check "leaves no draft beside the site config" \
		does_not_exist "$(dirname "${SITE_CONFIG_PATH}")/.$(basename "${SITE_CONFIG_PATH}").new"
	check "does not reload Caddy" no_call_mentions "${ANY_RELOAD}"
}

test_a_site_config_is_installed_when_logging_fails() {
	LOGGER_FAILS=1
	stage "${NEW_SITE_CONFIG}"
	install_site
	check "exits 0" succeeded
	check "installs the site config" contents_are "${SITE_CONFIG_PATH}" "${NEW_SITE_CONFIG}"
}

run_tests
