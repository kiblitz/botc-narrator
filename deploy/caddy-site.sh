#!/usr/bin/env bash
#
# Install botc's Caddy site config on the shared box and have Caddy take it --
# or leave Caddy's files as they were.
#
#     botc-caddy-site <site config>
#
# Installed at /usr/local/sbin/botc-caddy-site by setup-vps.sh. Root runs it:
# botc-apply on every deploy, and setup-vps.sh. No sudoers rule names it.
#
# The box hosts more than one app, so Caddy's one config file belongs to the box
# and imports every app's site config from /etc/caddy/sites/. This installs
# botc's, validates the shared Caddyfile with every app's site imported -- so a
# botc change that breaks or clashes with another app's is refused -- and
# reloads Caddy, holding the lock every app's deploy takes to change Caddy
# throughout.
#
# This never writes the shared Caddyfile. slot1's box/bootstrap.sh is what puts
# that file on the box; a box without it is refused here, because a site config
# installed beside a Caddyfile that imports nothing would be read by nobody.
#
# It is botc's own rather than a tool for every app on the box: the site
# config's name, the paths and the log tag are botc's. Another app ships its
# own, keeping to the same rules -- the lock above all -- rather than sharing a
# script across repos that deploy independently.
#
# Tested by deploy/tests/caddy_site_test.sh.

set -euo pipefail

# Both overrides exist for the tests, and neither reaches a real run: ROOT is
# empty in production, botc-apply's sudoers rule resets the environment, and
# setup-vps.sh clears both with `env -u` before running this.
readonly ROOT="${BOTC_DEPLOY_ROOT:-}"
# How long to wait for another app's deploy to finish with Caddy.
readonly LOCK_SECONDS="${BOTC_CADDY_SITE_LOCK_SECONDS:-60}"

# The app this is for: what its site config and everything it keeps are named.
readonly APP=botc
readonly APP_TAG=botc
readonly LOG_TAG="${APP_TAG}-caddy-site"
readonly CADDY_USER=caddy
readonly CADDY_UNIT=caddy
readonly CONFIG_MODE=644
readonly SITES_DIR_MODE=755
readonly ROOT_ONLY_MODE=700
# flock takes a lock through any open file, even one opened only to read, so
# only root may open it.
readonly LOCK_MODE=600
readonly LOCK_UMASK=077
# The group and other write bits.
readonly WRITABLE_BY_OTHERS=022

readonly CADDYFILE="${ROOT}/etc/caddy/Caddyfile"
# How the shared Caddyfile pulls sites in. The box's copy is slot1's
# box/Caddyfile; this line is the record that the layout is in place.
readonly SHARED_IMPORT='import sites/*.caddy'
readonly SITES_DIR="${ROOT}/etc/caddy/sites"
readonly SITE_CONFIG_NAME="${APP}.caddy"
readonly SITE_CONFIG="${SITES_DIR}/${SITE_CONFIG_NAME}"
# Root-only, and outside the sites directory so the import never reads anything
# kept here.
readonly STATE_DIR="${ROOT}/var/lib/${APP_TAG}-caddy-site"
# Each run keeps what it replaces in a fresh directory named with this. These
# are not cleared by later runs: one is left behind when Caddy's files could not
# be put back, or when a run was killed outright, and then it may hold the only
# copy of what was there.
readonly RUN_PREFIX="${STATE_DIR}/run."
# A copy of the site config Caddy last took. The one in the sites directory is
# not always that -- a run cut off after installing it, or a copy made by hand,
# leaves one Caddy never loaded -- so only a staged config matching both counts
# as unchanged.
readonly TAKEN_SITE_CONFIG="${STATE_DIR}/${SITE_CONFIG_NAME}.taken"
# Every app's deploy holds this while it changes its site config and reloads
# Caddy, so two deploys at once cannot validate or reload each other's half-made
# change.
readonly LOCK_DIR="${ROOT}/run/lock"
readonly LOCK="${LOCK_DIR}/caddy-sites.lock"

# Once started, this runs to its end. Stopped between installing a file and
# putting it back, it would leave Caddy's config half-changed on disk. PIPE as
# well: run by a deploy whose SSH connection has dropped, the next message
# written would otherwise be its last.
#
# What this runs inherits the ignored signals too. That is left so: `caddy
# validate` and `systemctl reload` finish in moments, and stopping either
# partway is no better for being done by a hang-up.
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

if [[ $# -ne 1 ]]; then
	say "usage: $0 <site config>"
	exit 2
fi
readonly STAGED="$1"

if [[ -L "${STAGED}" || ! -f "${STAGED}" || ! -s "${STAGED}" ]]; then
	say "not a plain, non-empty file: ${STAGED}"
	exit 1
fi

# `caddy`, `runuser`, `systemctl`, `cmp`, `flock`, `stat`, `mv` and `logger`
# come from PATH: root's own, or botc-apply's, which its sudoers rule pins.

# The lock means something only where no other account can create files: there,
# one could take it first and hold it for good, or leave a file of its own where
# it goes. botc-apply takes its own lock the same way; each is installed on its
# own, so they cannot share this.
lock_dir_state="$(stat -c '%u %a' "${LOCK_DIR}")"
read -r lock_dir_owner lock_dir_mode <<<"${lock_dir_state}"
if [[ "${lock_dir_owner}" -ne 0 ]] || ((8#${lock_dir_mode} & 8#${WRITABLE_BY_OTHERS})); then
	say "${LOCK_DIR} is writable by accounts other than root, so no lock in it can be trusted; Caddy was left alone"
	exit 1
fi

# Created root-only, and made so if something older left it readable by all.
previous_umask="$(umask)"
umask "${LOCK_UMASK}"
exec {lock_fd}>"${LOCK}"
umask "${previous_umask}"
chmod "${LOCK_MODE}" "${LOCK}"
if ! flock --wait "${LOCK_SECONDS}" "${lock_fd}"; then
	say "another deploy held ${LOCK} for ${LOCK_SECONDS}s; Caddy was left alone (if none is running, something opened the lock while others could read it, and a reboot clears it)"
	exit 1
fi

# Everything from here on is decided under the lock: another app's deploy
# changes exactly what is read next, and a decision made before it finished
# could skip a reload the box needs.
if [[ ! -f "${CADDYFILE}" ]] || ! grep -qxF "${SHARED_IMPORT}" "${CADDYFILE}"; then
	# A site config installed beside a Caddyfile that imports nothing would be
	# read by nobody, and the deploy would "succeed" with the routing unchanged.
	say "${CADDYFILE} has no '${SHARED_IMPORT}' line of its own: this box is not on the shared Caddy layout, which slot1's box/bootstrap.sh installs; Caddy was left alone"
	exit 1
fi

# A reload closes the WebSockets Caddy proxies, for every app on the box.
# botc's own games are among them. Most deploys change no routing,
# and should not cost anyone their connection.
if [[ -f "${SITE_CONFIG}" && -f "${TAKEN_SITE_CONFIG}" ]] &&
	cmp -s "${STAGED}" "${SITE_CONFIG}" && cmp -s "${STAGED}" "${TAKEN_SITE_CONFIG}"; then
	say "site config unchanged; Caddy was left alone"
	exit 0
fi

log "installing a changed site config"

# install_in_place <file> <target>: root's, CONFIG_MODE, and in with one rename,
# so a copy cut short never leaves Caddy half a file to read at its next start.
# The draft's name starts with a dot and does not end in .caddy, so the import
# never picks it up. `-T`, so a directory at the target is an error rather than
# somewhere to put the file inside of.
#
# apply.sh installs its binary much the same way. Each script is installed on
# its own, so neither can source a helper from the other.
install_in_place() {
	local draft
	draft="$(dirname "$2")/.$(basename "$2").new"
	install -m "${CONFIG_MODE}" -o root -g root "$1" "${draft}" || return 1
	if ! mv -fT "${draft}" "$2"; then
		rm -f "${draft}"
		return 1
	fi
}

# How far Caddy's files have got:
#
#   untouched   nothing on disk has changed
#   installed   the new site config is in place
#   reloading   Caddy has been asked to take it
#   taken       Caddy has taken it
#
# Anything short of taken is put back on the way out, however this exits.
progress=untouched
WORK=""

# put_back: what was replaced, back in place. Fails if it could not be.
put_back() {
	if [[ "${HAD_SITE_CONFIG}" == true ]]; then
		install_in_place "${PREVIOUS_SITE_CONFIG}" "${SITE_CONFIG}"
	else
		rm -f "${SITE_CONFIG}"
	fi
}

# finish: runs on every exit. Whatever fails in here, the rest still has to run.
finish() {
	set +e
	local keep_work=false
	case "${progress}" in
	installed)
		if put_back; then
			report "what was there before is back in place"
		else
			keep_work=true
		fi
		;;
	reloading)
		# Once more, although the reload just failed: one systemd reports as
		# failed -- timed out, say -- may still have reached Caddy, and loading
		# what is back on disk costs nothing if it did not. Unless this reload
		# goes through, what Caddy runs is not known, and the record of what it
		# took is forgotten so the next run reloads.
		if put_back; then
			report "what was there before is back in place"
			if ! systemctl reload "${CADDY_UNIT}"; then
				rm -f "${TAKEN_SITE_CONFIG}"
				report "Caddy would not reload with that either; the next run will reload Caddy again"
			fi
		else
			rm -f "${TAKEN_SITE_CONFIG}"
			keep_work=true
		fi
		;;
	esac
	if [[ "${keep_work}" == false ]]; then
		rm -rf ${WORK:+"${WORK}"}
	else
		report "could not put Caddy's files back; what they were is kept in ${WORK}"
	fi
}
trap finish EXIT

install -d -m "${ROOT_ONLY_MODE}" -o root -g root "${STATE_DIR}"
WORK="$(mktemp -d "${RUN_PREFIX}XXXXXX")"
readonly WORK
readonly PREVIOUS_SITE_CONFIG="${WORK}/${SITE_CONFIG_NAME}"
readonly VALIDATE_LOG="${WORK}/validate.log"

# Keep what is about to be replaced, and record whether there was anything to
# keep rather than guess it from the backup later.
HAD_SITE_CONFIG=false
if [[ -f "${SITE_CONFIG}" ]]; then
	cp -p "${SITE_CONFIG}" "${PREVIOUS_SITE_CONFIG}"
	HAD_SITE_CONFIG=true
fi
readonly HAD_SITE_CONFIG

progress=installed
install -d -m "${SITES_DIR_MODE}" -o root -g root "${SITES_DIR}"
install_in_place "${STAGED}" "${SITE_CONFIG}"

# Validate before reloading: a bad config would take every site on the box down.
# Refusing the reload is not enough on its own -- Caddy keeps serving the old
# config, but reads the files on disk at its next start, so a bad one left there
# would take the sites down at the next reboot instead.
#
# As Caddy's own account, because validating opens the log files the config
# names and creates any that are missing: created by root, Caddy could not open
# them afterwards and would fail to start. Without the lock, which a process of
# that account has no business holding open.
#
# Into a file, and passed on only if it fails: Caddy always has something to
# say, and it dies on a write to a connection that has dropped even though this
# script ignores SIGPIPE -- which would read as a config it refused.
if ! runuser -u "${CADDY_USER}" -- caddy validate --config "${CADDYFILE}" --adapter caddyfile \
	{lock_fd}>&- >"${VALIDATE_LOG}" 2>&1 </dev/null; then
	say "$(<"${VALIDATE_LOG}")"
	report "Caddy refused the site config alongside the box's other sites (its output is above); what was there before is being put back, and Caddy was left alone"
	exit 1
fi

progress=reloading
if ! systemctl reload "${CADDY_UNIT}"; then
	report "Caddy would not reload with the new site config; what was there before is being put back"
	exit 1
fi
progress=taken

# Caddy runs the change from here on, and nothing below undoes it or fails the
# run: a caller told otherwise would stop halfway through a deploy. What cannot
# be done is said, and left for the next run or a person.

# What Caddy now runs, for the next run's unchanged check. Only once it has
# taken it: a run that stops short leaves the old record, and the next run loads
# the config again. A record that cannot be written is removed, for the same
# reason.
if ! install_in_place "${SITE_CONFIG}" "${TAKEN_SITE_CONFIG}"; then
	rm -f "${TAKEN_SITE_CONFIG}" || true
	report "could not record which site config Caddy took; the next run will reload Caddy again"
fi

report "site config installed; Caddy was reloaded"
