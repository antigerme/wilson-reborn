#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Build self-contained (data-embedded) Wilson Reborn binaries for PERSONAL use, from a
# Linux host. You supply the original game data; it is read only at build time and baked
# into the binaries (never written into the repo). The resulting binaries contain the
# copyright game data — keep them for yourself, do not redistribute publicly.
#
# Usage:
#   scripts/build-embedded.sh [--check] <data-dir> [out-dir]
#     --check     only run the preflight diagnostic (no build), then exit
#     <data-dir>  folder with RESOURCE.MAP + RESOURCE.001 (+ optional soundN.wav)
#     [out-dir]   output dir (default: target/embedded)
#
# From Linux it builds:
#   * Linux   x86_64  -> wilson-linux-x86_64                  (native)
#   * Windows x86_64  -> wilson.exe + wilson.scr             (cross via mingw-w64)
# macOS (binary + .saver) must be built ON a Mac — cross-compiling Apple targets from
# Linux needs the macOS SDK (osxcross); see docs/INSTALL.md.
#
# It prints a preflight diagnostic first (toolchain, ALSA, Windows target, mingw) so you
# can see — and fix — what is missing before anything compiles. Targets whose prerequisites
# are missing are skipped with a clear hint rather than failing mid-run.
set -euo pipefail

WIN_TARGET="x86_64-pc-windows-gnu"

# ---- argument parsing (flags may appear anywhere) --------------------------------
CHECK_ONLY=0
POSITIONAL=()
for a in "$@"; do
    case "$a" in
        --check | -n | --dry-run) CHECK_ONLY=1 ;;
        -h | --help)
            sed -n '8,18p' "$0" | sed 's/^# \{0,1\}//'
            exit 0
            ;;
        *) POSITIONAL+=("$a") ;;
    esac
done
DATA="${POSITIONAL[0]:-}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="${POSITIONAL[1]:-$ROOT/target/embedded}"

# ---- preflight diagnostic --------------------------------------------------------
# Each check prints [ok]/[--] and, when missing, the exact command to fix it. Nothing
# here aborts the script (every probe is guarded), so it is safe under `set -e`.
have() { command -v "$1" >/dev/null 2>&1; }
data_ok=0
alsa_ok=0
win_target_ok=0
mingw_ok=0

[ -n "$DATA" ] && [ -f "$DATA/RESOURCE.MAP" ] && data_ok=1
pkg-config --exists alsa 2>/dev/null && alsa_ok=1
have rustup && rustup target list --installed 2>/dev/null | grep -qx "$WIN_TARGET" && win_target_ok=1
have x86_64-w64-mingw32-gcc && mingw_ok=1

echo "==> Preflight"
if [ "$data_ok" -eq 1 ]; then
    echo "  [ok] original data       $DATA/RESOURCE.MAP"
else
    echo "  [--] original data       not found at '${DATA:-<none>}/RESOURCE.MAP'"
    echo "                           → pass the folder with RESOURCE.MAP + RESOURCE.001"
fi
if have cargo; then
    echo "  [ok] cargo               $(command -v cargo) ($(cargo --version 2>/dev/null))"
else
    echo "  [--] cargo               missing → install Rust (rustup recommended)"
fi
if have rustup; then
    echo "  [ok] rustup              $(rustup --version 2>/dev/null | head -n1)"
else
    echo "  [--] rustup              missing (needed for the Windows target)"
    echo "                           → sudo dnf install -y rustup && rustup-init -y && source \"\$HOME/.cargo/env\""
fi
if [ "$alsa_ok" -eq 1 ]; then
    echo "  [ok] ALSA dev            present (Linux build links it via the audio feature)"
else
    echo "  [--] ALSA dev            missing → the Linux build will fail in alsa-sys"
    echo "                           → Fedora:        sudo dnf install -y alsa-lib-devel pkgconf-pkg-config"
    echo "                           → Debian/Ubuntu: sudo apt-get install -y libasound2-dev pkg-config"
    echo "                           (or build without sound: cargo ... --no-default-features)"
fi
if [ "$win_target_ok" -eq 1 ]; then
    echo "  [ok] Windows target      $WIN_TARGET installed"
else
    echo "  [--] Windows target      $WIN_TARGET not installed"
    echo "                           → rustup target add $WIN_TARGET"
fi
if [ "$mingw_ok" -eq 1 ]; then
    echo "  [ok] mingw linker        $(command -v x86_64-w64-mingw32-gcc)"
else
    echo "  [--] mingw linker        missing (for the Windows cross-build)"
    echo "                           → Fedora:        sudo dnf install -y mingw64-gcc"
    echo "                           → Debian/Ubuntu: sudo apt-get install -y gcc-mingw-w64-x86-64"
fi
# Plan summary.
win_plan="build"
[ "$win_target_ok" -eq 1 ] && [ "$mingw_ok" -eq 1 ] || win_plan="SKIP (see above)"
echo "  ── plan: Linux = build · Windows = $win_plan · macOS = manual (on a Mac)"
echo

if [ "$CHECK_ONLY" -eq 1 ]; then
    echo "==> --check: diagnostic only, nothing built."
    exit 0
fi

# ---- from here we actually build; require the data dir ---------------------------
if [ "$data_ok" -ne 1 ]; then
    echo "error: '${DATA:-<none>}/RESOURCE.MAP' not found — point <data-dir> at your original data" >&2
    echo "       (run with --check to see the full preflight without building)" >&2
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
[ "$alsa_ok" -eq 1 ] || echo "    (warning: ALSA dev not detected — this may fail in alsa-sys; see preflight)"
cargo build --release -p wilson --features embed-data --manifest-path "$ROOT/Cargo.toml"
cp "$ROOT/target/release/wilson" "$OUT/wilson-linux-x86_64"
echo "    ok -> $OUT/wilson-linux-x86_64"

# --- Windows (cross via mingw-w64) ------------------------------------------------
echo
if [ "$win_target_ok" -ne 1 ]; then
    echo "==> [Windows] skipped — target '$WIN_TARGET' not installed (see preflight)."
elif [ "$mingw_ok" -ne 1 ]; then
    echo "==> [Windows] skipped — mingw linker not found (see preflight)."
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
ls -1 "$OUT" 2>/dev/null | sed 's/^/    /'
echo "    NOTE: these binaries embed the copyright game data — personal use only."
