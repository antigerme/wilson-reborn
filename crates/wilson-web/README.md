<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# wilson-web — Wilson Reborn in the browser (WASM)

Runs the **headless engine** ([`wilson-engine`]) in a browser via WebAssembly — the same code
as the desktop app, no server. Two modes:

- **Bring your own data** (default): the page asks for your `RESOURCE.MAP` + `RESOURCE.001`,
  read **locally** (nothing uploaded). Safe to host.
- **Self-contained** (`embed-data` feature, personal use): the `RESOURCE.*` are baked into the
  `.wasm` at build time, so the page just runs — no file picker.

> The game data is **copyright** Sierra/Dynamix and is **never** committed here. For the
> self-contained build you supply your own originals (read only at build time); the resulting
> bundle contains the game — **personal use only, do not host/redistribute it.**

## How it works
- The crate is `wasm32`-only (an empty lib on other targets, so it doesn't affect the desktop
  build). It exposes a tiny `Wilson` class to JS: `new Wilson(map, data, seed, nowSecs)`,
  `Wilson.embedded(seed, nowSecs)` (only in an `embed-data` build), `frame(nowSecs) → RGBA`,
  `delay_ms()`, `width()/height()`, `enable_dissolve()`, plus `has_embedded_data()` so the page
  knows whether to auto-start or show the picker.
- Rust returns raw RGBA; the page (`web/index.html`) wraps it in `ImageData` and draws it,
  pacing frames by `delay_ms()` (the engine's 16 ms/tick). No `web-sys` dependency — just
  `wasm-bindgen`.
- The clock comes from JS (`Date.now()`), so the day/night cycle and the 11-day arc work.
- Audio is desktop-only for now (the browser build is silent).

## Build & run
```sh
cargo install wasm-bindgen-cli        # version must match the wasm-bindgen crate
                                      # (the wasm32 target is auto-added by build-web.sh)

# Bring-your-own-data page:
./build-web.sh
python3 -m http.server -d web 8000    # open http://localhost:8000/ and pick RESOURCE.MAP + .001

# Self-contained page (personal use) — bake your data in:
WILSON_EMBED_DATA=<dir-with-RESOURCE.*> ./build-web.sh   # then serve web/ as above — it just runs
```

(From the packager: `scripts/build-embedded.sh --web` — self-contained when given a `<data-dir>`,
bring-your-own without one.) The generated `web/wilson_web.js` + `wilson_web_bg.wasm` are build
outputs (git-ignored); only `index.html` is committed.

[`wilson-engine`]: ../wilson-engine
