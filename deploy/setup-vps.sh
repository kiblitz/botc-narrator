#!/usr/bin/env bash
#
# Provisioning for botc on slot1, the shared Fedora VPS.
#
# Idempotent: safe to re-run after changing the unit file, botc's Caddy site
# config or the deploy scripts. It does NOT install the server binary or the web
# client -- that is the deploy workflow's job, and keeping the two apart means a
# routine deploy can never accidentally rewrite the machine's configuration.
# There is no secret to fill in: nobody signs in to botc.
#
# Run as root on the box, passing the public half of the key GitHub Actions
# will deploy with:
#
#     sudo ./setup-vps.sh "ssh-ed25519 AAAA... botc-deploy"
#
# (Omit the key to reconfigure everything else and leave authorized_keys alone.)
#
# Prerequisites, both of them the box's rather than botc's:
#
#   * botc.kiblitz.xyz must already resolve to this machine, or Caddy cannot
#     complete the ACME HTTP challenge and will serve no certificate.
#   * the box must already be on the shared Caddy layout, which slot1's
#     box/bootstrap.sh installs. This script never writes
#     /etc/caddy/Caddyfile, installs Caddy, or touches the firewall: those
#     belong to the box, and no app owns them.
#
# Unlike apply.sh and caddy-site.sh this has no tests: it provisions a whole
# machine -- accounts, the sudoers rule, the unit -- which a scratch directory
# cannot stand in for. What it shares with every deploy, changing Caddy, it does
# through botc-caddy-site, which is tested.

set -euo pipefail

# What botc's own files on the box are named for.
readonly APP_TAG=botc
readonly SERVICE_USER=botc
readonly SERVICE_SHELL=/usr/sbin/nologin
readonly DEPLOY_USER=botc-deploy
readonly DEPLOY_HOME="/home/${DEPLOY_USER}"
readonly DEPLOY_SHELL=/bin/bash
readonly CADDY_USER=caddy
readonly CADDY_UNIT=caddy
readonly CADDY_LOG_DIR=/var/log/caddy
readonly CADDYFILE=/etc/caddy/Caddyfile
# The line that says the box is on the shared layout. caddy-site.sh checks for
# this same line before it installs anything; checked here too, so a fresh
# provisioning says what is wrong before it creates accounts.
readonly SHARED_IMPORT='import sites/*.caddy'
# What puts a box on that layout, for the messages that refuse one that is not.
readonly BOX_BOOTSTRAP="slot1's box/bootstrap.sh"
readonly SERVICE=botc
readonly UNIT_PATH="/etc/systemd/system/${SERVICE}.service"
# botc's Caddy site config, as shipped beside this script and as installed.
readonly SITE_CONFIG_NAME="${SERVICE}.caddy"
readonly BINARY="/usr/local/bin/${APP_TAG}-server"
readonly WEB_ROOT="/var/www/${APP_TAG}"
readonly PLACEHOLDER_PAGE="${WEB_ROOT}/index.html"
# The modes apply.sh gives every web client it unpacks: readable by all,
# writable by root alone, nothing setuid.
readonly WEB_MODES=u=rwX,go=rX,ug-s
readonly APPLY="/usr/local/sbin/${APP_TAG}-apply"
readonly CADDY_SITE="/usr/local/sbin/${APP_TAG}-caddy-site"
# Where CI stages a deploy: the deploy account's alone, so no other account on
# the box can plant or swap what botc-apply picks up. apply.sh refuses a
# staging directory that is anything else.
readonly STAGING_PARENT="/var/lib/${DEPLOY_USER}"
readonly STAGING_DIR="${STAGING_PARENT}/staging"
readonly PRIVATE_DIR_MODE=700
readonly LOCK_DIR=/run/lock
# Held by botc-apply for a whole deploy, and here while this replaces it and
# restarts the service.
readonly APPLY_LOCK="${LOCK_DIR}/${APP_TAG}-apply.lock"
# Root-owned directories only: what botc-apply finds its tools in.
readonly APPLY_PATH=/usr/sbin:/usr/bin
readonly SYSTEMCTL=/usr/bin/systemctl
readonly SUDOERS_DIR=/etc/sudoers.d
readonly SITE_HOST=botc.kiblitz.xyz
# Where this box's public address is looked up, and how long to wait for it.
readonly IP_LOOKUP_URL=https://api.ipify.org
readonly IP_LOOKUP_SECONDS=10

readonly EXECUTABLE_MODE=755
readonly CONFIG_MODE=644
# A directory anyone may read and only its owner may change.
readonly PUBLIC_DIR_MODE=755
# The config directory and the key in it: readable by the service account,
# writable by root alone, and closed to everybody else on the box.
readonly SSH_DIR_MODE=700
readonly AUTHORIZED_KEYS_MODE=600
readonly SUDOERS_MODE=440
# What the deploy account's key files are created with: only it may read them.
readonly KEY_UMASK=077
# flock takes a lock through any open file, even one opened only to read, so
# only root may open it.
readonly LOCK_MODE=600
readonly LOCK_UMASK=077
# The group and other write bits.
readonly WRITABLE_BY_OTHERS=022

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly SCRIPT_DIR
readonly DEPLOY_PUBKEY="${1:-}"

step() { printf '\n\033[1;32m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33mwarning:\033[0m %s\n' "$*" >&2; }
fail() {
	printf '\033[1;31merror:\033[0m %s\n' "$*" >&2
	exit 1
}

if [[ $EUID -ne 0 ]]; then
	fail "run this as root (sudo $0)"
fi

step "Checking that ${SITE_HOST} points here"
# Caddy's certificate request fails in a confusing way if DNS is not ready, so
# say so plainly up front rather than let it fail later.
this_ip="$(curl -fsS --max-time "${IP_LOOKUP_SECONDS}" "${IP_LOOKUP_URL}" 2>/dev/null || echo unknown)"
resolved="$(getent ahostsv4 "${SITE_HOST}" 2>/dev/null | awk '{print $1}' | sort -u | tr '\n' ' ' || true)"
if [[ -z "${resolved}" ]]; then
	warn "${SITE_HOST} does not resolve yet. Caddy will keep retrying; add the DNS A record and it will pick it up."
elif [[ "${this_ip}" != "unknown" && "${resolved}" != *"${this_ip}"* ]]; then
	warn "${SITE_HOST} resolves to [${resolved}] but this host is ${this_ip}. TLS will not be issued until that matches."
else
	echo "    ${SITE_HOST} -> ${resolved}"
fi

step "Checking what the box provides"
# Caddy, the firewall and the shared Caddyfile belong to the box, and slot1's
# box/bootstrap.sh is what puts them there. botc refuses a box that is not
# ready rather than provision half of one itself: an app that installed Caddy
# would be an app that could reconfigure it out from under every other app on
# the machine.
if ! command -v caddy &>/dev/null; then
	fail "Caddy is not installed. Run ${BOX_BOOTSTRAP} first: it owns Caddy, the firewall and the shared Caddyfile."
fi
if [[ ! -f "${CADDYFILE}" ]] || ! grep -qxF "${SHARED_IMPORT}" "${CADDYFILE}"; then
	fail "${CADDYFILE} has no '${SHARED_IMPORT}' line of its own, so this box is not on the shared Caddy layout and a site config installed here would be read by nobody. Run ${BOX_BOOTSTRAP} first."
fi
# Owned by Caddy's own account, not merely present: validating a site config
# creates any log file it names, and one Caddy cannot create is a config Caddy
# refuses -- which would read here as botc's site config being at fault.
caddy_log_dir_owner="$(stat -c '%U' "${CADDY_LOG_DIR}" 2>/dev/null || true)"
if [[ "${caddy_log_dir_owner}" != "${CADDY_USER}" ]]; then
	fail "${CADDY_LOG_DIR} is ${caddy_log_dir_owner:-missing}, not ${CADDY_USER}'s, so Caddy cannot create the log file botc's site config names and would refuse the config. Run ${BOX_BOOTSTRAP} first."
fi
if ! systemctl is-active --quiet "${CADDY_UNIT}"; then
	fail "${CADDY_UNIT} is not running, and only a running Caddy can be reloaded. Start it, or run ${BOX_BOOTSTRAP}."
fi
# Where botc-apply's lock and the Caddy lock every app shares are kept. A
# lock means something only where no other account can create files: there, one
# could take it first and hold it for good. Checked here, before anything is
# created, just as botc-apply and botc-caddy-site check it on every run.
if [[ ! -d "${LOCK_DIR}" ]]; then
	fail "${LOCK_DIR} is missing, and every app's deploy keeps its locks there. ${BOX_BOOTSTRAP} checks the same; see what it says."
fi
lock_dir_state="$(stat -c '%u %a' "${LOCK_DIR}")"
read -r lock_dir_owner lock_dir_mode <<<"${lock_dir_state}"
if [[ "${lock_dir_owner}" -ne 0 ]] || ((8#${lock_dir_mode} & 8#${WRITABLE_BY_OTHERS})); then
	fail "${LOCK_DIR} is writable by accounts other than root, so no lock in it can be trusted. ${BOX_BOOTSTRAP} refuses such a box too; put ${LOCK_DIR} right before provisioning anything."
fi
echo "    Caddy $(caddy version | head -1), on the shared layout"

step "Checking that no deploy is running"
# botc-apply holds this for a whole deploy. Held here from now on, no deploy
# can start while this replaces botc-apply and the unit and restarts the
# service. Created and held just as botc-apply does -- root-only, in the
# directory checked above.
previous_umask="$(umask)"
umask "${LOCK_UMASK}"
exec {apply_lock_fd}>"${APPLY_LOCK}"
umask "${previous_umask}"
chmod "${LOCK_MODE}" "${APPLY_LOCK}"
if ! flock --nonblock "${apply_lock_fd}"; then
	fail "a deploy holds ${APPLY_LOCK}; re-run this once it has finished"
fi

step "Creating the ${SERVICE_USER} service account"
# No login shell and no home directory: this account exists only to own a
# process, so give it nothing else to be useful for.
if id "${SERVICE_USER}" &>/dev/null; then
	echo "    already exists"
else
	useradd --system --no-create-home --shell "${SERVICE_SHELL}" "${SERVICE_USER}"
	echo "    created"
fi

step "Creating the ${DEPLOY_USER} account for CI"
# Separate from the service account: CI needs a home directory and a shell to
# receive an scp, and the process that serves games should have neither.
if id "${DEPLOY_USER}" &>/dev/null; then
	echo "    already exists"
else
	useradd --create-home --home-dir "${DEPLOY_HOME}" --shell "${DEPLOY_SHELL}" "${DEPLOY_USER}"
	echo "    created"
fi

if [[ -n "${DEPLOY_PUBKEY}" ]]; then
	# Written as the deploy account, not as root: everything under its home is
	# the account's to replace, and root writing there would follow whatever
	# link it left in place of .ssh or authorized_keys -- into /etc, say.
	# From `/`, because the account may not be able to enter the directory this
	# runs from. In a session of its own, so there is no terminal for it to
	# open: a process of the deploy account's could attach to it and use root's.
	# Its output goes to a root-only file rather than a pipe, which such a
	# process could hold open and leave this waiting with the lock held. Nor is
	# it handed the lock.
	key_output="$(mktemp)"
	# shellcheck disable=SC2016 # the inner shell expands them
	if ! (cd / && setsid --wait runuser -u "${DEPLOY_USER}" -- bash -c '
		set -euo pipefail
		umask "$5"
		mkdir -p "$1/.ssh"
		chmod "$3" "$1/.ssh"
		touch "$1/.ssh/authorized_keys"
		chmod "$4" "$1/.ssh/authorized_keys"
		if grep -qxF "$2" "$1/.ssh/authorized_keys"; then
			echo "    deploy key already authorized"
		else
			printf "%s\n" "$2" >>"$1/.ssh/authorized_keys"
			echo "    deploy key authorized"
		fi
	' _ "${DEPLOY_HOME}" "${DEPLOY_PUBKEY}" "${SSH_DIR_MODE}" "${AUTHORIZED_KEYS_MODE}" \
		"${KEY_UMASK}" {apply_lock_fd}>&-) </dev/null >"${key_output}" 2>&1; then
		cat -v "${key_output}"
		rm -f "${key_output}"
		fail "could not authorize the deploy key (see above)"
	fi
	# -v: the deploy account wrote it, and root's terminal is not to be sent
	# control sequences of its choosing.
	cat -v "${key_output}"
	rm -f "${key_output}"
else
	warn "no deploy public key given; leaving ${DEPLOY_USER}'s authorized_keys untouched."
fi

step "Preparing the private staging directory"
# The parent is root's, so the deploy account cannot replace the staging
# directory itself with a link; what is inside is the deploy account's.
if [[ -L "${STAGING_PARENT}" || -L "${STAGING_DIR}" ]]; then
	fail "${STAGING_DIR} or its parent is a link; refusing to hand whatever it points at to ${DEPLOY_USER}"
fi
install -d -m "${PUBLIC_DIR_MODE}" -o root -g root "${STAGING_PARENT}"
install -d -m "${PRIVATE_DIR_MODE}" -o "${DEPLOY_USER}" -g "${DEPLOY_USER}" "${STAGING_DIR}"
echo "    ${STAGING_DIR}"

step "Granting ${DEPLOY_USER} exactly the commands a deploy needs"
# Two complete commands, no wildcards and no arguments: apply a staged deploy,
# and read the service's status. Everything the deploy does is inside
# `botc-apply`, which is root-owned and not writable by the deploy account, so
# a leaked deploy key can trigger a deploy but cannot change what a deploy does.
#
# `botc-apply` finds its tools on PATH and has test-only environment overrides,
# so its rule resets the environment and pins PATH itself rather than trusting
# the box-wide sudo defaults, which are not botc's to rely on.
#
# Drafted under a name sudo skips (it ignores any file in sudoers.d whose name
# contains a '.'), checked there, and only then renamed into place: a malformed
# rule, even for a moment, can lock the whole box out of sudo. A draft a failed
# run leaves behind is skipped the same way.
sudoers_draft="$(mktemp "${SUDOERS_DIR}/.${DEPLOY_USER}.XXXXXX")"
cat >"${sudoers_draft}" <<SUDOERS
Defaults!${APPLY} env_reset, secure_path="${APPLY_PATH}"
${DEPLOY_USER} ALL=(root) NOPASSWD: ${APPLY}
${DEPLOY_USER} ALL=(root) NOPASSWD: ${SYSTEMCTL} status ${SERVICE}
SUDOERS
chmod "${SUDOERS_MODE}" "${sudoers_draft}"
if ! visudo -cf "${sudoers_draft}" >/dev/null; then
	rm -f "${sudoers_draft}"
	fail "the generated sudoers rule was invalid; the previous one was kept"
fi
mv -f "${sudoers_draft}" "${SUDOERS_DIR}/${DEPLOY_USER}"
echo "    sudoers rule validated and installed"

step "Preparing the static web root"
# Caddy serves the web client from here. Until the first deploy unpacks the real
# one, leave something honest in place rather than a bare 404. Root-owned, like
# everything apply.sh unpacks into it: Caddy only has to read it, and nothing
# else should be able to rewrite the page people load.
if [[ -L "${WEB_ROOT}" ]]; then
	fail "${WEB_ROOT} is a link; refusing to change the owner and modes of whatever it points at"
fi
install -d -m "${PUBLIC_DIR_MODE}" -o root -g root "${WEB_ROOT}"
# chown does not follow links, but chmod -R changes each entry by path, and
# would follow one swapped in under it. So root takes every entry, every
# directory is closed to others from the top down -- each before anything inside
# it -- root takes every entry again, for anything created meanwhile, and only
# then are the modes set.
chown -R root:root "${WEB_ROOT}"
find "${WEB_ROOT}" -type d -exec chmod go-w {} \;
chown -R root:root "${WEB_ROOT}"
chmod -R "${WEB_MODES}" "${WEB_ROOT}"
if [[ -f "${PLACEHOLDER_PAGE}" ]]; then
	echo "    ${PLACEHOLDER_PAGE} already present, leaving it alone"
else
	placeholder="$(mktemp)"
	cat >"${placeholder}" <<'PLACEHOLDER'
<!doctype html>
<meta charset="utf-8">
<title>botc</title>
<style>
  body { font: 16px/1.6 system-ui, sans-serif; max-width: 34rem; margin: 4rem auto; padding: 0 1rem; }
  code { background: #f4f4f5; padding: .15em .35em; border-radius: .25rem; }
</style>
<h1>botc</h1>
<p>The server is running. There is no web client here yet.</p>
<p>Health: <a href="/health"><code>/health</code></a></p>
PLACEHOLDER
	install -m "${CONFIG_MODE}" -o root -g root "${placeholder}" "${PLACEHOLDER_PAGE}"
	rm -f "${placeholder}"
	echo "    placeholder ${PLACEHOLDER_PAGE} written"
fi

step "Installing botc's Caddy site config"
# Root-owned and root-writable only, like botc-apply below, which runs it.
install -m "${EXECUTABLE_MODE}" -o root -g root "${SCRIPT_DIR}/caddy-site.sh" "${CADDY_SITE}"
# It takes the Caddy lock, validates the shared Caddyfile with every app's site
# imported, and reloads Caddy -- or leaves Caddy's files as they were, and this
# stops here. It never writes the shared Caddyfile.
#
# Without its test-only settings, which root's environment might happen to
# carry, and without the lock, which it has no business holding.
env -u BOTC_DEPLOY_ROOT -u BOTC_CADDY_SITE_LOCK_SECONDS \
	"${CADDY_SITE}" "${SCRIPT_DIR}/${SITE_CONFIG_NAME}" {apply_lock_fd}>&-

step "Installing the deploy script"
# Root-owned and root-writable only: the deploy account runs this with sudo, so
# being able to edit it would be the same as having a general sudo grant.
install -m "${EXECUTABLE_MODE}" -o root -g root "${SCRIPT_DIR}/apply.sh" "${APPLY}"

step "Installing the systemd unit"
# Compared byte for byte, and a changed unit restarts botc below, during
# which every game in progress ends (rooms live in memory): a re-run after any edit
# to botc.service, a comment included, costs that. Leave its bytes alone
# unless the unit itself has to change.
unit_changed=false
if [[ -f "${UNIT_PATH}" ]] && cmp -s "${SCRIPT_DIR}/${SERVICE}.service" "${UNIT_PATH}"; then
	echo "    unchanged"
else
	install -m "${CONFIG_MODE}" -o root -g root "${SCRIPT_DIR}/${SERVICE}.service" "${UNIT_PATH}"
	systemctl daemon-reload
	unit_changed=true
	echo "    installed"
fi

step "Enabling ${SERVICE}"
systemctl enable "${SERVICE}"

# A restart ends every game in progress, so a re-run restarts the
# service only when its unit changed.
if [[ ! -x "${BINARY}" ]]; then
	warn "${BINARY} is not installed yet, so ${SERVICE} is enabled but not started."
	warn "Push to main (or run the deploy workflow) and it will start on its own."
elif [[ "${unit_changed}" == true ]]; then
	systemctl restart "${SERVICE}"
	echo "    ${SERVICE} restarted for its changed unit"
elif systemctl is-active --quiet "${SERVICE}"; then
	echo "    ${SERVICE} already running with this unit; left alone"
else
	systemctl start "${SERVICE}"
	echo "    ${SERVICE} started"
fi

step "Done"
cat <<EOF
    systemctl status ${SERVICE}
    journalctl -u ${SERVICE} -f
    curl -s https://${SITE_HOST}/health

Host key for the DEPLOY_KNOWN_HOSTS repository secret:
EOF
ssh-keyscan -t ed25519 "$(curl -fsS --max-time "${IP_LOOKUP_SECONDS}" "${IP_LOOKUP_URL}" 2>/dev/null || hostname -I | awk '{print $1}')" 2>/dev/null | sed 's/^/    /'
