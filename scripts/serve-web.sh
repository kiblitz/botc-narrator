#!/usr/bin/env sh
# Builds and serves the simulator at http://localhost:${PORT:-8080}
set -eu
cd "$(dirname "$0")/.."
sh scripts/build-web.sh
node site/serve.mjs
