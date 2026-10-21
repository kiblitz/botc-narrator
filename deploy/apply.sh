#!/usr/bin/env bash
#
# Apply a staged deploy -- binary, Caddy site config and web client -- so that
# it either goes through or stops cleanly.
#
# Installed at /usr/local/sbin/botc-apply by setup-vps.sh and run by the deploy
# workflow through a single sudoers entry.
#
# Why this exists as one script rather than a chain of `sudo` calls: a chain
# like
#
#     sudo install binary && sudo install site config && sudo systemctl restart
#
# leaves the machine half-updated when it fails partway -- a new binary
# installed but never started, so the box keeps running the old one and a later
# reboot silently picks up a binary nobody verified. Here the steps cannot be
# separated: a site config Caddy refuses stops the deploy before anything else
# changes, and a failure after that -- a service that does not come back among
# them -- puts the web client and binary back to what was working.
#
# Tested by deploy/tests/apply_test.sh.

set -euo pipefail

# Every path below hangs off ROOT, which is empty in production. The tests point
# it at a scratch directory so the whole script runs without touching a real
# machine. botc-caddy-site, which this runs, inherits
# BOTC_DEPLOY_ROOT and reads BOTC_CADDY_SITE_LOCK_SECONDS from the
# same environment; the tests also say who owns the staging area, below. None
# of these can reach a real deploy: the deploy account runs this only through
# sudo, and the sudoers rule setup-vps.sh writes for it resets the environment
# and pins PATH.
readonly ROOT="${BOTC_DEPLOY_ROOT:-}"
# How long the restarted service gets to fall over. The tests shorten it.
readonly SETTLE_SECONDS="${BOTC_APPLY_SETTLE_SECONDS:-2}"

readonly SERVICE=botc
# What botc's own files on the box are named for.
readonly APP_TAG=botc
readonly LOG_TAG="${APP_TAG}-apply"
# Changes botc's Caddy site config on the box other apps share.
readonly CADDY_SITE="${ROOT}/usr/local/sbin/${APP_TAG}-caddy-site"
readonly SITE_CONFIG_NAME="${SERVICE}.caddy"

readonly BINARY="${ROOT}/usr/local/bin/${APP_TAG}-server"
readonly PREVIOUS_BINARY="${BINARY}.previous"
# Where a binary is written before it is renamed into place.
readonly BINARY_DRAFT="${BINARY}.new"
readonly BINARY_MODE=755
readonly WEB_PARENT="${ROOT}/var/www"
readonly WEB_ROOT_NAME="${APP_TAG}"
readonly WEB_ROOT="${WEB_PARENT}/${WEB_ROOT_NAME}"
readonly PREVIOUS_WEB_ROOT="${WEB_ROOT}.previous"
# Every web client gets these: readable by all, writable by root alone, nothing
# setuid. setup-vps.sh gives an existing web root the same.
readonly WEB_MODES=u=rwX,go=rX,ug-s
# The mode of a directory only root can enter.
readonly ROOT_ONLY_MODE=700
# Root-only. Each run claims what was staged into a fresh directory here.
readonly WORK_PARENT="${ROOT}/var/lib/${APP_TAG}-apply"
readonly WORK_PREFIX="${WORK_PARENT}/run."
# Each run unpacks the web client into a fresh directory beside the web root.
readonly UNPACK_PREFIX="${WEB_PARENT}/.${WEB_ROOT_NAME}.new."
# Held for the whole run. CI's concurrency group serialises its own deploys,
# not a botc-apply started by hand meanwhile, and two at once would claim each
# other's staged files and remove each other's rollback copies. setup-vps.sh
# holds it too, while it replaces this script and restarts the service.
readonly LOCK_DIR="${ROOT}/run/lock"
readonly APPLY_LOCK="${LOCK_DIR}/${APP_TAG}-apply.lock"
# flock takes a lock through any open file, even one opened only to read, so
# only root may open it.
readonly LOCK_MODE=600
readonly LOCK_UMASK=077
# The group and other write bits.
readonly WRITABLE_BY_OTHERS=022

# The deploy account's own directory, made by setup-vps.sh: no other account
# can plant or swap a file in it.
readonly DEPLOY_USER="${APP_TAG}-deploy"
readonly STAGING_DIR="${ROOT}/var/lib/${DEPLOY_USER}/staging"
# The group and other bits, all of them.
readonly OPEN_TO_OTHERS=077
# Far longer than copying the largest thing staged takes, and short enough that
# a copy which will never finish fails the deploy rather than hanging it.
readonly COPY_SECONDS=60
readonly STAGED_BINARY="${STAGING_DIR}/${APP_TAG}-server.new"
readonly STAGED_SITE_CONFIG="${STAGING_DIR}/${SITE_CONFIG_NAME}.new"
readonly STAGED_WWW="${STAGING_DIR}/${APP_TAG}-www.tar.gz"

# Once started, a deploy runs to its end. A dropped SSH connection or a
# cancelled workflow would otherwise stop it between two steps that only make
# sense together: a web root moved away with nothing moved in. PIPE as well as
# HUP: once the connection is gone, the next message written to it would
# otherwise be the script's last.
trap '' HUP INT TERM PIPE

# say <message>: tells whoever is watching, if anyone still is. A message nobody
# can read any more is not a reason to stop.
say() { printf '%s\n' "$*" >&2 || true; }
# log <message>: records it in the journal. A journal that will not take it --
# journald down, say -- is not a reason to stop either.
log() { logger -t "${LOG_TAG}" "$*" || true; }
# report <message>: both.
report() {
	log "$*"
	say "$*"
}

# `logger`, `stat`, `id`, `flock`, `cp`, `mv`, `tar`, `find`, `systemctl` and `cmp`, and the
# tools botc-caddy-site runs, come from PATH: in production the sudoers rule
# pins it to root-owned directories, and the tests fill it with stubs.
log "deploy starting"

# The lock means something only where no other account can create files: there,
# one could take it first and hold it for good. botc-caddy-site takes the Caddy
# lock the same way; each is installed on its own, so they cannot share this.
lock_dir_state="$(stat -c '%u %a' "${LOCK_DIR}")"
read -r lock_dir_owner lock_dir_mode <<<"${lock_dir_state}"
if [[ "${lock_dir_owner}" -ne 0 ]] || ((8#${lock_dir_mode} & 8#${WRITABLE_BY_OTHERS})); then
	say "${LOCK_DIR} is writable by accounts other than root, so no lock in it can be trusted; this deploy changed nothing"
	exit 1
fi

# Created root-only, and made so if something older left it readable by all.
previous_umask="$(umask)"
umask "${LOCK_UMASK}"
exec {apply_lock_fd}>"${APPLY_LOCK}"
umask "${previous_umask}"
chmod "${LOCK_MODE}" "${APPLY_LOCK}"
if ! flock --nonblock "${apply_lock_fd}"; then
	say "another botc-apply or setup-vps.sh holds ${APPLY_LOCK}; this deploy changed nothing (if neither is running, something opened the lock while others could read it, and a reboot clears it)"
	exit 1
fi

install -d -m "${ROOT_ONLY_MODE}" -o root -g root "${WORK_PARENT}"
# A run killed outright -- out of memory, say -- removes nothing it made, and on
# a small disk every copy of the binary and web client it leaves adds up. With
# the lock held, no other run's are among them.
shopt -s nullglob
rm -rf -- "${WORK_PREFIX}"* "${UNPACK_PREFIX}"*
shopt -u nullglob
# One killed between the web client swap's two renames also left no web root,
# and the only working client set aside. Put it back first, before this run
# replaces what was set aside with the client it is about to swap out.
if [[ ! -e "${WEB_ROOT}" && ! -L "${WEB_ROOT}" && -d "${PREVIOUS_WEB_ROOT}" ]]; then
	mv -T "${PREVIOUS_WEB_ROOT}" "${WEB_ROOT}"
	report "put back the web client a killed deploy had set aside"
fi

# install_binary <file>: puts it in place with one rename, so a copy cut short
# -- the disk full, the box out of memory -- never leaves a truncated binary
# where the next boot would start it.
install_binary() {
	install -m "${BINARY_MODE}" -o root -g root "$1" "${BINARY_DRAFT}" || return 1
	if ! mv -f "${BINARY_DRAFT}" "${BINARY}"; then
		rm -f "${BINARY_DRAFT}"
		return 1
	fi
}

# put_back_web_client, put_back_binary: what finish undoes a failed deploy
# with. The site config is not among it: Caddy has already taken it, and
# putting it back would mean another reload for every app on the box.
# Re-running an older deploy puts an older one in place.
put_back_web_client() {
	if [[ ! -d "${PREVIOUS_WEB_ROOT}" ]]; then
		return 0
	fi
	# The failed client is moved aside, where finish removes it, rather than
	# removed in place: that takes a while for a large one, and a run killed
	# partway would leave a half-removed web root the next run takes for a
	# working one.
	if [[ -e "${WEB_ROOT}" || -L "${WEB_ROOT}" ]] && ! mv -T "${WEB_ROOT}" "${FAILED_WEB_ROOT}"; then
		report "could not move the failed web client aside; the previous one is at ${PREVIOUS_WEB_ROOT} until the next deploy"
		return 1
	fi
	if ! mv -T "${PREVIOUS_WEB_ROOT}" "${WEB_ROOT}"; then
		report "could not put the previous web client back; it is at ${PREVIOUS_WEB_ROOT}"
		return 1
	fi
	report "the previous web client was put back"
}
# Restarts the service only on a binary it put back: with nothing to go back
# to, another restart of the one that failed would change nothing.
put_back_binary() {
	if [[ "${HAD_BINARY}" != true ]]; then
		return 0
	fi
	if ! install_binary "${PREVIOUS_BINARY}"; then
		report "could not put the previous binary back; it is at ${PREVIOUS_BINARY}"
		return 1
	fi
	if ! systemctl restart "${SERVICE}"; then
		report "the previous binary was put back, but ${SERVICE} would not restart on it"
		return 1
	fi
	report "the previous binary was put back and ${SERVICE} restarted on it"
}

# How far the deploy has got, so that however it ends, the web client and binary
# are put back if it fails after changing them:
#
#   untouched     nothing here to put back
#   web-swapped   the web client is being, or has been, swapped
#   restarted     the new binary is in too, and the service restarted on it
#   applied       done; nothing is put back
progress=untouched
WORK=""
UNPACK=""
HAD_BINARY=false
# finish: runs on every exit. Stays set -e free: a step that fails while
# putting things back must not stop the steps after it.
finish() {
	local status=$?
	set +e
	if ((status != 0)); then
		case "${progress}" in
		restarted)
			put_back_web_client
			put_back_binary
			;;
		web-swapped)
			put_back_web_client
			;;
		esac
	fi
	rm -rf ${WORK:+"${WORK}"} ${UNPACK:+"${UNPACK}"}
}
trap finish EXIT
WORK="$(mktemp -d "${WORK_PREFIX}XXXXXX")"
readonly WORK
UNPACK="$(mktemp -d "${UNPACK_PREFIX}XXXXXX")"
readonly UNPACK
# Where a rollback moves a web client that failed, for finish to remove.
readonly FAILED_WEB_ROOT="${UNPACK}/failed"

# The staging area is the deploy account's alone, and is refused if it is
# anything else: a link, somebody else's, or open to other accounts. The deploy
# account itself can still rearrange it while this runs, so nothing is trusted
# where it lies. Each staged file is copied where only root can reach it and
# checked there, and the staged one removed, so staging is left empty however
# the deploy ends once it has been accepted. Copied rather than moved: both directories are on one
# filesystem, a move would be a rename, and a file the deploy account still had
# open could be written through after it was checked. `-P` takes a link as a
# link, to be refused below; without `-r` a directory is not copied at all.
# The copy is given COPY_SECONDS: a plain file swapped for a pipe between the
# test and the copy would otherwise hold this script, as root, for ever.
#
# The owner is looked up by name in production. The tests, which cannot create
# accounts, say who it is.
if [[ -L "${STAGING_DIR}" || ! -d "${STAGING_DIR}" ]]; then
	say "${STAGING_DIR} is not a directory; run setup-vps.sh. This deploy changed nothing"
	exit 1
fi
staging_owner_wanted="${BOTC_APPLY_STAGING_OWNER:-$(id -u "${DEPLOY_USER}")}"
staging_state="$(stat -c '%u %a' "${STAGING_DIR}")"
read -r staging_owner staging_mode <<<"${staging_state}"
if [[ "${staging_owner}" -ne "${staging_owner_wanted}" ]] || ((8#${staging_mode} & 8#${OPEN_TO_OTHERS})); then
	say "${STAGING_DIR} is not ${DEPLOY_USER}'s alone, so nothing in it can be trusted; this deploy changed nothing"
	exit 1
fi
readonly STAGED=("${STAGED_BINARY}" "${STAGED_SITE_CONFIG}" "${STAGED_WWW}")
readonly CLAIMED_BINARY="${WORK}/server"
readonly CLAIMED_SITE_CONFIG="${WORK}/${SITE_CONFIG_NAME}"
readonly CLAIMED_WWW="${WORK}/www.tar.gz"
readonly CLAIMED=("${CLAIMED_BINARY}" "${CLAIMED_SITE_CONFIG}" "${CLAIMED_WWW}")
for i in "${!STAGED[@]}"; do
	if [[ -L "${STAGED[i]}" || -f "${STAGED[i]}" ]]; then
		timeout "${COPY_SECONDS}" cp -P -T -- "${STAGED[i]}" "${CLAIMED[i]}"
		rm -f -- "${STAGED[i]}"
	fi
done
# Everything must be there before anything is touched: a missing file is a
# refusal, not a half-applied deploy.
for i in "${!STAGED[@]}"; do
	if [[ -L "${CLAIMED[i]}" || ! -f "${CLAIMED[i]}" || ! -s "${CLAIMED[i]}" ]]; then
		say "not staged as a plain, non-empty file: ${STAGED[i]}"
		exit 1
	fi
done

# Unpack the web client before anything changes, into a fresh directory inside
# one only root can enter. Fresh, because unpacking over the live root would
# keep whatever earlier deploys left in it -- a link among them would be a path
# the next deploy's tar follows as root. Root-only, because tar creates what the
# archive holds before anything can check it, and a device file another account
# opened in that moment would stay open to it after being refused.
#
# There is no size limit: the archive comes from the account that can already
# ship any binary it likes, and whatever it unpacks is removed on every exit.
readonly NEW_WEB_ROOT="${UNPACK}/${WEB_ROOT_NAME}"
mkdir "${NEW_WEB_ROOT}"
tar -xzf "${CLAIMED_WWW}" -C "${NEW_WEB_ROOT}" --no-same-owner --no-same-permissions
if [[ -n "$(find "${NEW_WEB_ROOT}" ! -type f ! -type d -print -quit)" ]]; then
	say "the web client contains something other than files and directories; refusing it"
	exit 1
fi
# The archive records the runner's uid and modes on every entry. On the VPS uid
# 1001 is whichever account happened to be created second; a world-writable
# entry would let any account rewrite the page people load, and a setuid one
# would run as root. Own and mode it all afresh.
chown -R root:root "${NEW_WEB_ROOT}"
chmod -R "${WEB_MODES}" "${NEW_WEB_ROOT}"

# Have Caddy take the site config before the web client or binary changes, so a
# config Caddy will not take stops the deploy while it is still only a config
# change. botc-caddy-site puts back what it replaced when Caddy refuses, and
# leaves Caddy alone when the routing has not changed; once Caddy has taken a
# change, it succeeds even if what comes after -- keeping a record of it --
# does not. It is not handed this run's lock: nothing it starts has any business
# holding that.
if ! "${CADDY_SITE}" "${CLAIMED_SITE_CONFIG}" {apply_lock_fd}>&-; then
	say "Caddy did not take the site config (see above); the binary and web client were left alone"
	exit 1
fi

# Keep the running binary, so a failure from here on has somewhere to go back
# to -- and only that: a .previous left by some older deploy is not what was
# running. A hard link copies nothing, and the rename in install_binary leaves
# it pointing at the old binary.
rm -f "${PREVIOUS_BINARY}"
if [[ -x "${BINARY}" ]]; then
	ln "${BINARY}" "${PREVIOUS_BINARY}"
	HAD_BINARY=true
fi
readonly HAD_BINARY

# Swap the new web client in, keeping the previous one for a rollback. Two
# renames on one filesystem, so the moment without a web root is as short as it
# can be. `-T`, so a directory that appeared at the target is an error rather
# than somewhere to put the new client inside of. The swap counts as begun once
# the old .previous is gone: from then on, whatever has been set aside is what
# goes back.
#
# The client is swapped even when it is the same build, and browsers then fetch
# it again: telling a rebuilt client from a changed one means comparing every
# file, for a download nobody notices.
rm -rf "${PREVIOUS_WEB_ROOT}"
progress=web-swapped
if [[ -d "${WEB_ROOT}" ]]; then
	mv -T "${WEB_ROOT}" "${PREVIOUS_WEB_ROOT}"
fi
mv -T "${NEW_WEB_ROOT}" "${WEB_ROOT}"

# A restart ends every game in progress (rooms live in memory),
# and most deploys -- docs, the web client, CI -- ship the same binary again. A
# running service with that binary is left alone; the web client above is
# swapped either way.
if [[ -x "${BINARY}" ]] && cmp -s "${CLAIMED_BINARY}" "${BINARY}" &&
	systemctl is-active --quiet "${SERVICE}"; then
	progress=applied
	report "applied; the binary is unchanged, so ${SERVICE} was left running"
	exit 0
fi

install_binary "${CLAIMED_BINARY}"
progress=restarted
# The unit is Type=exec, so a binary that cannot be executed at all fails the
# restart itself. Whether the service is up is what decides, below.
systemctl restart "${SERVICE}" || true

# Give it a moment to fall over, if it is going to.
sleep "${SETTLE_SECONDS}"
if ! systemctl is-active --quiet "${SERVICE}"; then
	report "${SERVICE} did not come back; rolling the binary and web client back"
	exit 1
fi

progress=applied
report "applied"
