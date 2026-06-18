<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# wilson-web — Wilson Reborn in the browser (WASM)

Runs the **headless engine** ([`wilson-engine`]) in a browser via WebAssembly: you pick your
own `RESOURCE.MAP` + `RESOURCE.001`, and the engine renders the life of Johnny Castaway to a
`<canvas>` — the same code as the desktop app, no server, nothing uploaded.

> The game data is **copyright** Sierra/Dynamix and is **never** bundled here. Bring your own
> originals (e.g. from the Internet Archive's `scrantic-run.zip`). The page reads them locally.

## How it works
- The crate is `wasm32`-only (an empty lib on other targets, so it doesn't affect the desktop
  build). It exposes a tiny `Wilson` class to JS: `new Wilson(map, data, seed, nowSecs)`,
  `frame(nowSecs) → RGBA bytes`, `delay_ms()`, `width()/height()`, `enable_dissolve()`.
- Rust returns raw RGBA; the page (`web/index.html`) wraps it in `ImageData` and draws it,
  pacing frames by `delay_ms()` (the engine's 16 ms/tick). No `web-sys` dependency — just
  `wasm-bindgen`.
- The clock comes from JS (`Date.now()`), so the day/night cycle and the 11-day arc work.
- Audio is desktop-only for now (the browser build is silent).

## Build & run
```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli        # version must match the wasm-bindgen crate

./build-web.sh                        # compiles to wasm + generates web/wilson_web.js
python3 -m http.server -d web 8000    # any static server works
# open http://localhost:8000/ and pick RESOURCE.MAP + RESOURCE.001
```

The generated `web/wilson_web.js` + `wilson_web_bg.wasm` are build outputs (git-ignored);
only `index.html` is committed.

[`wilson-engine`]: ../wilson-engine
