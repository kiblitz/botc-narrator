#!/usr/bin/env bash
#
# Checks that the deploy files naming the same thing name it the same way.
#
# The workflow, scripts/deploy.sh, botc-apply, botc-caddy-site, setup-vps.sh,
# the unit and the site config each restate some path, name or port, because
# each is installed on its own and cannot read the others. Nothing else notices
# when one of them changes alone: a deploy would stage files botc-apply never
# looks for, install a binary the unit does not run, unpack a web client Caddy
# does not serve, or be refused by a sudoers rule naming a different command.
#
# Needs nothing but a checkout; scripts/check-deploy.sh runs it before every
# merge:
#
#     bash deploy/tests/files_agree.sh

set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../.."

readonly WORKFLOW=.github/workflows/deploy.yml
readonly CHECKS=scripts/check-deploy.sh
readonly DEPLOY_SCRIPT=scripts/deploy.sh
readonly APPLY=deploy/apply.sh
readonly CADDY_SITE=deploy/caddy-site.sh
readonly SETUP=deploy/setup-vps.sh
readonly UNIT=deploy/botc.service
readonly SITE_CONFIG=deploy/botc.caddy
readonly TEST_LIB=deploy/tests/lib.sh
readonly ROUTES=server/src/route.rs

failures=0

# agree <what> <value> <value>...: says whether every value is the same, and
# none of them is empty.
agree() {
	local what="$1" first="$2" value
	shift
	for value in "$@"; do
		if [[ -z "${value}" || "${value}" != "${first}" ]]; then
			echo "::error::the deploy files disagree on ${what}: $(printf '[%s] ' "$@")"
			failures=$((failures + 1))
			return 0
		fi
	done
	echo "${what}: ${first}"
}

# workflow_env <NAME>: the deploy workflow's `NAME: value`.
workflow_env() { sed -n "s/^  $1: //p" "${WORKFLOW}"; }

# constant <script> <NAME>: the script's `readonly NAME=value`, with the other
# constants it refers to filled in and the tests' ROOT left out.
constant() {
	local value ref reference replacement
	value="$(sed -n "s/^readonly $2=//p" "$1" | tr -d "\"'")"
	while [[ "${value}" =~ \$\{([A-Z_]+)\} ]]; do
		ref="${BASH_REMATCH[1]}"
		reference="\${${ref}}"
		replacement=""
		if [[ "${ref}" != ROOT ]]; then
			replacement="$(constant "$1" "${ref}")"
		fi
		value="${value//"${reference}"/"${replacement}"}"
	done
	printf '%s' "${value}"
}

# unit_setting <Key>: the unit's `Key=value`.
unit_setting() { sed -n "s/^$1=//p" "${UNIT}"; }
# unit_environment <NAME>: the unit's `Environment=NAME=value`.
unit_environment() { unit_setting Environment | sed -n "s/^$1=//p"; }
# unit_environment_names: the name of every setting the unit passes on.
unit_environment_names() { unit_setting Environment | sed -n 's/=.*//p'; }
# route_constant <NAME>: a `pub const NAME: &str = "value";` out of route.rs.
route_constant() { sed -n "s/^pub const $1: &str = \"\(.*\)\";.*/\1/p" "${ROUTES}"; }
# route_names: the name of every path constant route.rs defines.
route_names() { sed -n 's/^pub const \([A-Z_]*\): &str = "\/.*";.*/\1/p' "${ROUTES}"; }
# routed <path>: "matched" when one of Caddy's proxied matchers (@api, @ws)
# covers that path, which is what sends it to the server instead of answering it
# with the web client.
routed() {
	awk -v path="$1" '/^[ 	]*@(api|ws) path/ { for (i = 3; i <= NF; i++) {
		p = $i; star = sub(/\*$/, "", p)
		if ((star && substr(path, 1, length(p)) == p) || path == p) { print "matched"; exit }
	} }' "${SITE_CONFIG}"
}
# streamed <path>: "streamed" when that path is matched by @ws, the matcher whose
# proxy keeps sockets open (flush_interval, stream_close_delay).
streamed() {
	awk -v path="$1" '/^[ 	]*@ws path/ { for (i = 3; i <= NF; i++) {
		p = $i; star = sub(/\*$/, "", p)
		if ((star && substr(path, 1, length(p)) == p) || path == p) { print "streamed"; exit }
	} }' "${SITE_CONFIG}"
}

for staged in STAGED_BINARY STAGED_SITE_CONFIG STAGED_WWW; do
	agree "where ${staged} is staged" "$(workflow_env "${staged}")" "$(constant "${APPLY}" "${staged}")"
done
# scripts/deploy.sh is the workflow done by hand, and while Actions cannot run
# it is the only one of the two that deploys anything -- so it restates the
# workflow's whole environment. Hold them together name by name.
for name in DEPLOY_HOST DEPLOY_USER SITE_HOST SERVICE APPLY CADDY_SITE \
	HEALTH_URL API_PATH SYSTEMCTL STAGED_BINARY STAGED_SITE_CONFIG STAGED_WWW \
	WEB_DIR SERVER_PACKAGE HEALTH_ATTEMPTS HEALTH_RETRY_SECONDS HEALTH_TIMEOUT_SECONDS \
	PUBLIC_CHECK_SECONDS; do
	agree "the deploy's ${name}" "$(workflow_env "${name}")" \
		"$(constant "${DEPLOY_SCRIPT}" "${name}")"
done
agree "the site config the workflow ships" "${SITE_CONFIG}" \
	"$(sed -n 's|^ *scp \(deploy/[^ ]*\) .*STAGED_SITE_CONFIG"$|\1|p' "${WORKFLOW}")"
agree "the site config deploy.sh ships" "${SITE_CONFIG}" "$(constant "${DEPLOY_SCRIPT}" SITE_CONFIG)"
agree "the server package" "$(workflow_env SERVER_PACKAGE)" \
	"$(sed -n 's/^name = "\(.*\)"$/\1/p' server/Cargo.toml | head -1)"
agree "the server binary" "$(constant "${APPLY}" BINARY)" "$(constant "${SETUP}" BINARY)" \
	"$(unit_setting ExecStart)" "/usr/local/bin/$(workflow_env SERVER_BINARY)"
agree "the account the service runs as" "$(constant "${SETUP}" SERVICE_USER)" \
	"$(unit_setting User)" "$(unit_setting Group)"
agree "the web root" "$(constant "${APPLY}" WEB_ROOT)" "$(constant "${SETUP}" WEB_ROOT)" \
	"$(sed -n 's/^\s*root \* //p' "${SITE_CONFIG}")"
agree "the web client's modes" "$(constant "${APPLY}" WEB_MODES)" "$(constant "${SETUP}" WEB_MODES)"
agree "the service" "$(workflow_env SERVICE)" "$(constant "${APPLY}" SERVICE)" \
	"$(constant "${SETUP}" SERVICE)"
agree "the deploy script" "$(workflow_env APPLY)" "$(constant "${SETUP}" APPLY)"
agree "the site config's name" "$(basename "${SITE_CONFIG}")" "$(constant "${SETUP}" SITE_CONFIG_NAME)" \
	"$(constant "${CADDY_SITE}" SITE_CONFIG_NAME)" "$(basename "$(constant "${TEST_LIB}" SITE_CONFIG_PATH)")"
agree "the Caddy site script" "$(workflow_env CADDY_SITE)" "$(constant "${APPLY}" CADDY_SITE)" \
	"$(constant "${SETUP}" CADDY_SITE)"
agree "the deploy account" "$(workflow_env DEPLOY_USER)" "$(constant "${SETUP}" DEPLOY_USER)"
agree "the site's hostname" "$(workflow_env SITE_HOST)" "$(constant "${SETUP}" SITE_HOST)" \
	"$(sed -n 's/^\([a-z0-9.-]*\) {$/\1/p' "${SITE_CONFIG}")"
# The log file's name is the hostname, and its directory is the one slot1's
# box/bootstrap.sh makes Caddy's and setup-vps.sh checks for.
agree "Caddy's log file" "$(sed -n 's|^\s*output file .*/\([^/]*\)\.log {$|\1|p' "${SITE_CONFIG}")" "$(workflow_env SITE_HOST)"
agree "Caddy's log directory" "$(constant "${SETUP}" CADDY_LOG_DIR)" \
	"$(sed -n 's|^\s*output file \(.*\)/[^/]* {$|\1|p' "${SITE_CONFIG}")"
# Both proxies (JSON and WebSockets) go to the one listener; `sort -u` collapses
# them, so if they ever disagree this arrives as two lines and fails.
agree "where the service listens" "$(unit_environment BIND_ADDR)" \
	"$(sed -n 's/^\s*reverse_proxy \([^ ]*\).*/\1/p' "${SITE_CONFIG}" | sort -u)" \
	"$(workflow_env HEALTH_URL | sed -n 's|^http://\([^/]*\)/health$|\1|p')"

# --- the paths the server and Caddy have to agree on ---------------------------
#
# server/src/route.rs is where these are defined; everything else restates them.
# An unrouted path is the failure worth spending this on: Caddy answers it with
# the web client, so it looks like a perfectly good 200 rather than a 404.
agree "the health endpoint" "$(route_constant HEALTH)" \
	"$(workflow_env HEALTH_URL | sed -n 's|^http://[^/]*\(/.*\)$|\1|p')"
agree "the API path the deploy smoke-tests" "$(route_constant SCRIPTS)" "$(workflow_env API_PATH)"
# Every path route.rs defines, read out of it rather than listed here, so a route
# added there is held to Caddy without anybody remembering this file. The ones
# named are those the deploy cannot do without: named, a route.rs that lost one
# -- or that this could not read at all -- fails here instead of passing with
# nothing to check.
for path_name in $({
	printf '%s\n' HEALTH SCRIPTS PLAY BRIDGE
	route_names
} | sort -u); do
	agree "Caddy routes ${path_name} to the app" "matched" \
		"$(routed "$(route_constant "${path_name}")")"
done
# WebSockets need the streaming proxy; a socket behind the plain one would be
# cut off by Caddy's buffering and by every other app's reload.
for path_name in PLAY BRIDGE; do
	agree "Caddy streams ${path_name}" "streamed" "$(streamed "$(route_constant "${path_name}")")"
done
# The web client's own files and pages must not be: Caddy serves those.
for page in / /index.html /app.js /style.css /ABCD; do
	routing="$(routed "${page}")"
	agree "Caddy leaves ${page} to the web client" "unmatched" "${routing:-unmatched}"
done

agree "the staging area" "$(constant "${APPLY}" STAGING_DIR)" "$(constant "${SETUP}" STAGING_DIR)" \
	"/$(constant "${TEST_LIB}" STAGING_DIR_PATH)"
agree "the staging area's mode" "$(constant "${SETUP}" PRIVATE_DIR_MODE)" \
	"$(constant "${TEST_LIB}" PRIVATE_DIR_MODE)"

# botc keeps nothing on disk and holds no secret: rooms live in memory and
# nobody signs in. These must not exist at all.
if [[ -n "$(unit_setting EnvironmentFile)" || -n "$(unit_setting StateDirectory)" ]] ||
	unit_environment_names | grep -Eqi '(KEY|SECRET|TOKEN|PASSWORD|DATABASE)'; then
	echo "::error::the unit names a secret or on-disk state; botc has neither"
	failures=$((failures + 1))
else
	echo "the unit holds no secret and no state"
fi

# --- the shared box -----------------------------------------------------------
#
# botc ships no Caddyfile: the shared one belongs to the box, and slot1's
# box/bootstrap.sh is what writes it. What botc restates is the import line it
# recognises the layout by -- in the script that refuses a box without it, the
# one that provisions botc, the tests, and the local check.
agree "the shared Caddyfile's import" "$(constant "${CADDY_SITE}" SHARED_IMPORT)" \
	"$(constant "${SETUP}" SHARED_IMPORT)" "$(constant "${TEST_LIB}" SHARED_IMPORT)"
agree "scripts/check-deploy.sh builds its Caddyfile from that same line" "deploy/caddy-site.sh" \
	"$(sed -n 's|^ *shared_import=.*SHARED_IMPORT.* \(deploy/caddy-site\.sh\))$|\1|p' "${CHECKS}")"
# The sudoers rule names the status command exactly, and sudo refuses any other.
agree "the systemctl the deploy account may run" "$(workflow_env SYSTEMCTL)" \
	"$(constant "${SETUP}" SYSTEMCTL)"
agree "Caddy's account" "$(constant "${SETUP}" CADDY_USER)" "$(constant "${CADDY_SITE}" CADDY_USER)" \
	"$(constant "${TEST_LIB}" CADDY_USER)"
agree "Caddy's unit" "$(constant "${SETUP}" CADDY_UNIT)" "$(constant "${CADDY_SITE}" CADDY_UNIT)" \
	"$(constant "${TEST_LIB}" CADDY_UNIT)"
# Each script that takes a lock checks and makes it on its own, the same way.
agree "botc-apply's lock" "$(constant "${APPLY}" APPLY_LOCK)" "$(constant "${SETUP}" APPLY_LOCK)"
agree "a lock's mode" "$(constant "${APPLY}" LOCK_MODE)" "$(constant "${CADDY_SITE}" LOCK_MODE)" \
	"$(constant "${SETUP}" LOCK_MODE)" "$(constant "${TEST_LIB}" LOCK_MODE)"
agree "the umask a lock is created with" "$(constant "${APPLY}" LOCK_UMASK)" \
	"$(constant "${CADDY_SITE}" LOCK_UMASK)" "$(constant "${SETUP}" LOCK_UMASK)"
agree "what makes a lock directory untrustworthy" "$(constant "${APPLY}" WRITABLE_BY_OTHERS)" \
	"$(constant "${CADDY_SITE}" WRITABLE_BY_OTHERS)" "$(constant "${SETUP}" WRITABLE_BY_OTHERS)" \
	"$(constant "${TEST_LIB}" WRITABLE_BY_OTHERS)"
# setup-vps.sh runs botc-caddy-site with every test-only setting it reads unset.
# shellcheck disable=SC2016 # the ${ is what is looked for, not expanded
agree "the test-only settings botc-caddy-site reads" \
	"$(grep -o '\${BOTC_[A-Z_]*' "${CADDY_SITE}" | cut -c3- | sort -u | tr '\n' ' ')" \
	"$(grep -o -- '-u BOTC_[A-Z_]*' "${SETUP}" | cut -c4- | sort -u | tr '\n' ' ')"
# The tests' own paths into the scratch machine -- BINARY_PATH and the like --
# are not checked here: one that drifted from the scripts fails the tests.

exit "$((failures > 0))"
