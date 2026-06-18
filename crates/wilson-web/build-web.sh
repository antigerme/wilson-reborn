#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Build the Wilson Reborn web bundle: compile the engine to wasm32 and generate the JS
# bindings into ./web/ (next to index.html). Then serve ./web/ over HTTP.
#
# Prerequisites are checked (and the wasm target is auto-added) below; you only need:
#   cargo install wasm-bindgen-cli      # version must match the `wasm-bindgen` crate
#
# Usage: ./build-web.sh [release|debug]   (default: release)
set -euo pipefail
cd "$(dirname "$0")"

profile="${1:-release}"
flag=""
[ "$profile" = "release" ] && flag="--release"
target="wasm32-unknown-unknown"
have() { command -v "$1" >/dev/null 2>&1; }

# ---- preflight: the prerequisites people miss (the missing target is the #1 trip-up) ----
have cargo || {
    echo "error: 'cargo' not found — install Rust (rustup recommended: https://rustup.rs)." >&2
    exit 1
}
if have rustup; then
    if ! rustup target list --installed 2>/dev/null | grep -qx "$target"; then
        echo "==> '$target' target missing — adding it (rustup target add $target)"
        rustup target add "$target"
    fi
else
    # No rustup: we can't add the target ourselves; warn with the exact fix.
    echo "warning: 'rustup' not found — cannot auto-add the '$target' target. If the build" >&2
    echo "         fails with \"can't find crate for core/std\", install the target for your" >&2
    echo "         toolchain (with rustup: 'rustup target add $target')." >&2
fi

echo "[1/2] cargo build ($profile) → $target"
cargo build $flag --target "$target" -p wilson-web

wasm="../../target/$target/$profile/wilson_web.wasm"
echo "[2/2] wasm-bindgen → web/"
if ! have wasm-bindgen; then
    echo "error: 'wasm-bindgen' not found. Install the CLI (its version must match the" >&2
    echo "       wasm-bindgen crate): cargo install wasm-bindgen-cli" >&2
    exit 1
fi
wasm-bindgen --target web --no-typescript --out-dir web "$wasm"

echo
echo "Done. Serve the page locally, e.g.:"
echo "    python3 -m http.server -d \"$(pwd)/web\" 8000"
echo "then open http://localhost:8000/ and pick your RESOURCE.MAP + RESOURCE.001."
