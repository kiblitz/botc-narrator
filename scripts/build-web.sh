#!/usr/bin/env sh
# Builds the simulator WebAssembly into site/pkg.
set -eu
cd "$(dirname "$0")/.."
wasm-pack build web --release --target web --out-dir ../site/pkg --no-typescript --no-pack
