#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Build the Wilson Reborn web bundle: compile the engine to wasm32 and generate the JS
# bindings into ./web/ (next to index.html). Then serve ./web/ over HTTP.
#
# Prerequisites:
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli      # version must match the `wasm-bindgen` crate
set -euo pipefail
cd "$(dirname "$0")"

profile="${1:-release}"
flag=""; [ "$profile" = "release" ] && flag="--release"

echo "[1/2] cargo build ($profile) → wasm32"
cargo build $flag --target wasm32-unknown-unknown -p wilson-web

wasm="../../target/wasm32-unknown-unknown/$profile/wilson_web.wasm"
echo "[2/2] wasm-bindgen → web/"
if ! command -v wasm-bindgen >/dev/null 2>&1; then
  echo "error: wasm-bindgen not found. Install it with:"
  echo "       cargo install wasm-bindgen-cli"
  exit 1
fi
wasm-bindgen --target web --no-typescript --out-dir web "$wasm"

echo
echo "Done. Serve the page locally, e.g.:"
echo "    python3 -m http.server -d \"$(pwd)/web\" 8000"
echo "then open http://localhost:8000/ and pick your RESOURCE.MAP + RESOURCE.001."
