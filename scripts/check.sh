#!/usr/bin/env sh
# Local merge gate (replaces CI): formatting, lints and tests.
set -eu
cd "$(dirname "$0")/.."
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
sh scripts/build-web.sh
(cd site && npm ci --silent && npx playwright test)
