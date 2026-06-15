#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Build self-contained (data-embedded) Wilson Reborn binaries for PERSONAL use, from a
# Linux host. You supply the original game data; it is read only at build time and baked
# into the binaries (never written into the repo). The resulting binaries contain the
# copyright game data — keep them for yourself, do not redistribute publicly.
#
# Usage:
#   scripts/build-embedded.sh <data-dir> [out-dir]
#     <data-dir>  folder with RESOURCE.MAP + RESOURCE.001 (+ optional soundN.wav)
#     [out-dir]   output dir (default: target/embedded)
#
# From Linux it builds:
#   * Linux   x86_64  -> wilson-linux-x86_64                  (native)
#   * Windows x86_64  -> wilson.exe + wilson.scr             (cross via mingw-w64)
# macOS (binary + .saver) must be built ON a Mac — cross-compiling Apple targets from
# Linux needs the macOS SDK (osxcross); see docs/INSTALL.md.
set -euo pipefail

DATA="${1:-}"
if [ -z "$DATA" ]; then
    echo "usage: scripts/build-embedded.sh <data-dir> [out-dir]" >&2
    exit 2
fi
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="${2:-$ROOT/target/embedded}"

if [ ! -f "$DATA/RESOURCE.MAP" ]; then
    echo "error: '$DATA/RESOURCE.MAP' not found — point <data-dir> at your original data" >&2
    exit 1
fi
DATA_ABS="$(cd "$DATA" && pwd)"
mkdir -p "$OUT"
export WILSON_EMBED_DATA="$DATA_ABS"
echo "==> Embedding data from: $DATA_ABS"
echo "==> Output dir:          $OUT"

# --- Linux (native) ---------------------------------------------------------------
echo
echo "==> [Linux] building native embedded binary"
cargo build --release -p wilson --features embed-data --manifest-path "$ROOT/Cargo.toml"
cp "$ROOT/target/release/wilson" "$OUT/wilson-linux-x86_64"
echo "    ok -> $OUT/wilson-linux-x86_64"

# --- Windows (cross via mingw-w64) ------------------------------------------------
WIN_TARGET="x86_64-pc-windows-gnu"
echo
if ! rustup target list --installed 2>/dev/null | grep -qx "$WIN_TARGET"; then
    echo "==> [Windows] skipped — target '$WIN_TARGET' not installed. To enable:"
    echo "      rustup target add $WIN_TARGET"
    echo "      sudo dnf install -y mingw64-gcc        # Fedora"
    echo "      # (Debian/Ubuntu: sudo apt-get install -y gcc-mingw-w64-x86-64)"
elif ! command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1; then
    echo "==> [Windows] skipped — mingw linker not found. Install it:"
    echo "      sudo dnf install -y mingw64-gcc        # Fedora"
else
    echo "==> [Windows] building cross embedded binary (.exe + .scr)"
    # Static mingw runtime so the .exe/.scr is standalone (no libwinpthread/libgcc DLLs).
    CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc \
        RUSTFLAGS="${RUSTFLAGS:-} -C target-feature=+crt-static" \
        cargo build --release -p wilson --features embed-data \
        --target "$WIN_TARGET" --manifest-path "$ROOT/Cargo.toml"
    cp "$ROOT/target/$WIN_TARGET/release/wilson.exe" "$OUT/wilson.exe"
    cp "$OUT/wilson.exe" "$OUT/wilson.scr"
    echo "    ok -> $OUT/wilson.exe and $OUT/wilson.scr"
fi

# --- macOS ------------------------------------------------------------------------
echo
echo "==> [macOS] not built from Linux. On a Mac, run:"
echo "      WILSON_EMBED_DATA='$DATA_ABS' cargo build --release -p wilson --features embed-data"
echo "      # and crates/wilson-saver/macos/build-saver.sh for an embedded .saver"

echo
echo "==> Done. Files in: $OUT"
echo "    NOTE: these binaries embed the copyright game data — personal use only."
