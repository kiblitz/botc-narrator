#!/usr/bin/env sh
# Local merge gate (replaces CI): formatting, lints and tests for the engine,
# the simulator site, the multiplayer server and bridge, and the deploy
# scripts. Leaves the stamp scripts/deploy.sh requires.
set -eu
cd "$(dirname "$0")/.."
bash scripts/stamp.sh clear
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
sh scripts/build-web.sh
(cd site && npm ci --silent && npx playwright test)
(cd server/e2e && npm ci --silent && npx playwright test)
out=$(bash scripts/check-deploy.sh | tee /dev/stderr | tail -n 1)
bash scripts/stamp.sh "${out#box_tests }"
