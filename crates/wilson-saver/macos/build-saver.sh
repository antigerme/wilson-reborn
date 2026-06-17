#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Build the macOS `WilsonReborn.saver` bundle: the Rust `wilson-saver` staticlib linked
# into an Obj-C ScreenSaverView. Run on macOS with Xcode command-line tools + a Rust
# toolchain. Output: <out>/WilsonReborn.saver (default: target/saver/).
set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)" # crates/wilson-saver/macos
ROOT="$(cd "$DIR/../../.." && pwd)"                 # repo root
OUT="${1:-$ROOT/target/saver}"
NAME="WilsonReborn"
BUNDLE="$OUT/$NAME.saver"

echo "==> Building Rust staticlib (release)"
cargo build --release -p wilson-saver --manifest-path "$ROOT/Cargo.toml"
STATICLIB="$ROOT/target/release/libwilson_saver.a"
[ -f "$STATICLIB" ] || {
  echo "missing staticlib: $STATICLIB" >&2
  exit 1
}

echo "==> Assembling bundle at $BUNDLE"
rm -rf "$BUNDLE"
mkdir -p "$BUNDLE/Contents/MacOS"
cp "$DIR/Info.plist" "$BUNDLE/Contents/Info.plist"
mkdir -p "$BUNDLE/Contents/Resources"
# App icon (our own art; see crates/wilson/assets/make_icon.py). Referenced by
# CFBundleIconFile in Info.plist.
cp "$ROOT/crates/wilson/assets/wilson.icns" "$BUNDLE/Contents/Resources/wilson.icns"

echo "==> Compiling + linking ScreenSaverView"
# The Rust staticlib pulls in std, which on macOS needs CoreFoundation/Security/iconv.
clang -bundle -fobjc-arc -mmacosx-version-min=11.0 \
  -o "$BUNDLE/Contents/MacOS/$NAME" \
  "$DIR/WilsonRebornView.m" \
  "$STATICLIB" \
  -framework ScreenSaver -framework Cocoa -framework QuartzCore \
  -framework CoreFoundation -framework Security -framework SystemConfiguration \
  -liconv

echo "==> Built $BUNDLE"
