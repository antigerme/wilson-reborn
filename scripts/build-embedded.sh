#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Build self-contained (data-embedded) Wilson Reborn binaries for PERSONAL use, from a
# Linux host. You supply the original game data; it is read only at build time and baked
# into the binaries (never written into the repo). The resulting binaries contain the
# copyright game data — keep them for yourself, do not redistribute publicly.
#
# This builds the DESKTOP binaries only (Linux/Windows/macOS). The browser (WASM) build is
# separate and never embeds data — see crates/wilson-web/build-web.sh.
#
# Usage:
#   scripts/build-embedded.sh [--check] [--fetch-ia [--i-accept-legal-responsibility]] [<data-dir>] [out-dir]
#     --check     only run the preflight diagnostic (no build, no download), then exit
#     --fetch-ia  DOWNLOAD the original data from the Internet Archive instead of passing
#                 <data-dir>. Opt-in only; it is COPYRIGHT data, for PERSONAL USE ONLY —
#                 it prints a loud legal warning and asks you to type "I ACCEPT". It is
#                 hard-blocked in CI. (--i-accept-legal-responsibility pre-accepts the
#                 warning for non-interactive runs — same legal responsibility applies.)
#     <data-dir>  folder with RESOURCE.MAP + RESOURCE.001 (+ optional soundN.wav)
#                 (omit when using --fetch-ia)
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

# Internet Archive source for --fetch-ia (the preserved original screensaver). The exact
# bytes are pinned by SHA-256 so a changed / wrongly-served file is rejected.
IA_URL="https://archive.org/download/johnny-castaway-screensaver/scrantic-run.zip"
IA_SHA256="111c384aa44fc810c0453f524d8c02dee58ea3358ee788316b2fc1f2059afc56"

# ---- argument parsing (flags may appear anywhere) --------------------------------
CHECK_ONLY=0
FETCH_IA=0
IA_ACCEPTED=0
POSITIONAL=()
for a in "$@"; do
    case "$a" in
        --check | -n | --dry-run) CHECK_ONLY=1 ;;
        --fetch-ia) FETCH_IA=1 ;;
        --i-accept-legal-responsibility) IA_ACCEPTED=1 ;;
        -h | --help)
            cat <<'USAGE'
Usage: scripts/build-embedded.sh [--check] [--fetch-ia [--i-accept-legal-responsibility]] [<data-dir>] [out-dir]
  --check      only run the preflight diagnostic (no build, no download), then exit
  --fetch-ia   download the ORIGINAL data from the Internet Archive instead of passing
               <data-dir>. Opt-in; COPYRIGHT data; PERSONAL USE ONLY. Prints a loud legal
               warning and asks you to type "I ACCEPT". NEVER runs in CI.
               (--i-accept-legal-responsibility pre-accepts it, for non-interactive runs.)
  <data-dir>   folder with RESOURCE.MAP + RESOURCE.001 (+ optional soundN.wav)
               (omit when using --fetch-ia)
  out-dir      output dir (default: target/embedded)

Builds (from Linux): Linux x86_64 (native) + Windows x86_64 (.exe/.scr via mingw-w64).
macOS must be built on a Mac (see docs/INSTALL.md).
USAGE
            exit 0
            ;;
        *) POSITIONAL+=("$a") ;;
    esac
done
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# With --fetch-ia there is no <data-dir> argument, so the only positional is [out-dir].
if [ "$FETCH_IA" -eq 1 ]; then
    DATA=""
    OUT="${POSITIONAL[0]:-$ROOT/target/embedded}"
else
    DATA="${POSITIONAL[0]:-}"
    OUT="${POSITIONAL[1]:-$ROOT/target/embedded}"
fi

have() { command -v "$1" >/dev/null 2>&1; }

# ---- --fetch-ia: LOUD legal gate + pinned download -------------------------------
# Runs only for a real build (never under --check) and is HARD-BLOCKED in CI, so that no
# automation ever downloads or bakes the copyright data into an artifact. On success it
# leaves the extracted data in $IA_TMP and points $DATA at it; $IA_TMP is wiped on exit.
IA_TMP=""
fetch_from_ia() {
    if [ -n "${CI:-}" ] || [ -n "${GITHUB_ACTIONS:-}" ] || [ -n "${GITHUB_RUN_ID:-}" ]; then
        echo "error: --fetch-ia is refused in CI environments — copyright data must never be" >&2
        echo "       downloaded or embedded into artifacts by automation." >&2
        exit 2
    fi
    local t
    for t in curl unzip sha256sum; do
        have "$t" || { echo "error: --fetch-ia needs '$t' installed (missing)." >&2; exit 2; }
    done

    cat >&2 <<'BANNER'

################################################################################
################################################################################
##                                                                            ##
##   ##   ##   AVISO LEGAL  /  LEGAL WARNING   ##   ##                         ##
##                                                                            ##
##   !!!  LEIA TUDO ANTES DE CONTINUAR  /  READ THIS IN FULL  !!!             ##
##                                                                            ##
################################################################################
################################################################################

  ----------------------------- ENGLISH --------------------------------------
  The "Johnny Castaway" game data is COPYRIGHTED material
  (© Sierra On-Line / Dynamix; rights now held by Activision / Microsoft).
  It is NOT free software, NOT public domain, and was NEVER officially released
  as freeware. "Abandonware" is NOT a legal license.

  --fetch-ia DOWNLOADS that copyrighted data from the Internet Archive ONTO
  YOUR machine and BAKES IT INTO the binary you are about to build. The
  resulting binary CONTAINS THE COPYRIGHTED GAME.

  BY USING --fetch-ia YOU AGREE THAT:
    * The built binary is for YOUR OWN STRICTLY PERSONAL USE.
    * You will NOT distribute, share, upload, publish, sell, mirror or otherwise
      redistribute it. Doing so is COPYRIGHT INFRINGEMENT and is SOLELY YOUR
      responsibility.
    * You use this option ONLY if you are legally entitled to a copy.

  Wilson Reborn, its authors and contributors merely automate a download you
  could perform by hand. THEY PROVIDE NO WARRANTY AND ACCEPT NO LIABILITY OR
  RESPONSIBILITY WHATSOEVER. YOU — the person running this command — ASSUME ALL
  LEGAL RISK AND SOLE RESPONSIBILITY for downloading, building with and using
  this data.

  ---------------------------- PORTUGUES -------------------------------------
  Os dados do jogo "Johnny Castaway" sao material PROTEGIDO POR DIREITO AUTORAL
  (© Sierra/Dynamix; hoje Activision/Microsoft). NAO sao software livre, NAO sao
  dominio publico e NUNCA foram liberados oficialmente. "Abandonware" NAO e
  licenca.

  O --fetch-ia BAIXA esses dados protegidos do Internet Archive PARA A SUA
  maquina e os EMBUTE no binario que voce vai gerar. O binario resultante
  CONTEM O JOGO PROTEGIDO.

  AO USAR --fetch-ia VOCE CONCORDA QUE:
    * O binario gerado e para o SEU USO ESTRITAMENTE PESSOAL.
    * Voce NAO vai distribuir, compartilhar, publicar, enviar, vender nem
      espelhar o binario. Fazer isso e VIOLACAO DE DIREITO AUTORAL e e
      responsabilidade EXCLUSIVAMENTE SUA.
    * Voce so usa esta opcao se tem direito legal a uma copia.

  O Wilson Reborn e seus autores apenas automatizam um download que voce poderia
  fazer a mao. NAO HA GARANTIA E NAO HA QUALQUER RESPONSABILIDADE. VOCE — quem
  executa este comando — ASSUME TODO O RISCO E A RESPONSABILIDADE LEGAL
  EXCLUSIVA por baixar, compilar e usar estes dados.

################################################################################

BANNER

    if [ "$IA_ACCEPTED" -eq 1 ]; then
        echo "  Acceptance recorded via --i-accept-legal-responsibility." >&2
    elif [ -t 0 ]; then
        printf '  To proceed you must type exactly  I ACCEPT  (anything else aborts): ' >&2
        local reply=""
        read -r reply || true
        if [ "$reply" != "I ACCEPT" ]; then
            echo "  Aborted — acceptance not given. Nothing was downloaded." >&2
            exit 3
        fi
    else
        echo "error: --fetch-ia was not accepted. Re-run interactively and type 'I ACCEPT'," >&2
        echo "       or pass --i-accept-legal-responsibility to accept non-interactively." >&2
        exit 3
    fi

    IA_TMP="$(mktemp -d "${TMPDIR:-/tmp}/wilson-ia.XXXXXX")"
    # The downloaded copyright data is transient — wipe it whenever the script exits.
    trap 'rm -rf "$IA_TMP"' EXIT
    local zip="$IA_TMP/scrantic-run.zip"
    echo "==> [--fetch-ia] downloading $IA_URL" >&2
    curl -fL --retry 3 --max-time 300 -o "$zip" "$IA_URL"
    local got
    got="$(sha256sum "$zip" | cut -d' ' -f1)"
    if [ "$got" != "$IA_SHA256" ]; then
        echo "error: SHA-256 mismatch for scrantic-run.zip — refusing to use it." >&2
        echo "       expected $IA_SHA256" >&2
        echo "       got      $got" >&2
        exit 4
    fi
    echo "==> [--fetch-ia] checksum OK; extracting" >&2
    mkdir -p "$IA_TMP/data"
    unzip -oq "$zip" -d "$IA_TMP/data"
    DATA="$IA_TMP/data"
    echo "==> [--fetch-ia] data ready in a temporary dir (auto-removed on exit)." >&2
}

# Perform the download for a real build (never during --check).
if [ "$FETCH_IA" -eq 1 ] && [ "$CHECK_ONLY" -ne 1 ]; then
    fetch_from_ia
fi

# ---- preflight diagnostic --------------------------------------------------------
# Each check prints [ok]/[--] and, when missing, the exact command to fix it. Nothing
# here aborts the script (every probe is guarded), so it is safe under `set -e`.
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
elif [ "$FETCH_IA" -eq 1 ]; then
    echo "  [ok] original data       --fetch-ia: will be DOWNLOADED (copyright; personal use)"
else
    echo "  [--] original data       not found at '${DATA:-<none>}/RESOURCE.MAP'"
    echo "                           → pass the folder with RESOURCE.MAP + RESOURCE.001"
    echo "                           → or use --fetch-ia (downloads it; see its warning)"
fi
if [ "$FETCH_IA" -eq 1 ]; then
    if have curl && have unzip && have sha256sum; then
        echo "  [ok] fetch tools         curl + unzip + sha256sum present"
    else
        echo "  [--] fetch tools         --fetch-ia needs curl, unzip and sha256sum"
    fi
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
    echo "==> --check: diagnostic only, nothing built or downloaded."
    exit 0
fi

# ---- from here we actually build; require the data dir ---------------------------
if [ "$data_ok" -ne 1 ]; then
    echo "error: '${DATA:-<none>}/RESOURCE.MAP' not found — point <data-dir> at your original data" >&2
    echo "       (or use --fetch-ia to download it; run with --check to see the full preflight)" >&2
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
echo "    NOTE: these binaries embed the copyright game data — personal use only, do not redistribute."
