#!/usr/bin/env sh
# Soak test: plays random games over every script until stopped (or for
# --games N / --minutes M); writes failures.jsonl and summary.{json,md} to
# --out (default C:/Users/Gabriel Lee/Projects/botc-soak). See src/soak.rs.
set -eu
cd "$(dirname "$0")/.."
cargo build --release --bin soak
exec target/release/soak "$@"
