#!/bin/sh
# botc storyteller bridge, in one line (the host's lobby shows it with the
# room's pairing code):
#
#   curl -fsSL https://botc.kiblitz.xyz/bridge.sh | sh -s -- <PAIRING-CODE>
#
# Fetches bridge.mjs from the same server and runs it with Node; any other
# arguments (--server, --codex-home, -- COMMAND...) go to it.
set -eu

# The site, as in bridge.mjs and app.js's SITE.
server=https://botc.kiblitz.xyz
# Where to fetch bridge.mjs from: the user's --server, if any, before `--`
# (after it come the storyteller command's own arguments).
previous=
for arg in "$@"; do
	[ "$arg" = -- ] && break
	[ "$previous" = --server ] && server=$arg
	previous=$arg
done

if ! command -v node >/dev/null 2>&1; then
	echo "botc-bridge: needs Node 22 or later (https://nodejs.org)" >&2
	exit 1
fi
# Node needs the .mjs name to run it as a module.
dir=$(mktemp -d "${TMPDIR:-/tmp}/botc-bridge.XXXXXX")
trap 'rm -rf "$dir"' EXIT
trap 'exit 130' INT TERM HUP
script="$dir/bridge.mjs"
curl -fsSL "$server/bridge.mjs" -o "$script"
# Not exec: the trap removes the script when the bridge stops.
# --server first: the user's own, in "$@", comes later and wins, and anything
# after `--` stays the command's.
node "$script" --server "$server" "$@"
