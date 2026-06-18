<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# wilson-web — Wilson Reborn in the browser (WASM)

Runs the **headless engine** ([`wilson-engine`]) in a browser via WebAssembly — the same code
as the desktop app, no server. Two modes:

- **Bring your own data** (default): drop your `RESOURCE.MAP` + `RESOURCE.001` (and
  `SCRANTIC.EXE` for sound), **or a `scrantic-run.zip` / `scrantic-installer.zip`** — read
  **locally** (nothing uploaded). Optionally **💾 Save** it in the browser (IndexedDB, opt-in,
  with *Forget*) so you don't pick it again. Safe to host.
- **Self-contained** (`embed-data` feature, personal use): the `RESOURCE.*` (+ sounds) are baked
  into the `.wasm` at build time, so the page just runs — no file picker.

Sound is **on by default** (🔊/🔇 + volume). A **⛶ fullscreen** button keeps the canvas
black-letterboxed and holds a **Screen Wake Lock**; the UI/cursor fade when idle. The display
matches the desktop defaults (**`scale=fit` + `filter=linear`**). URL options mirror the desktop
CLI: `?fullscreen&scale=fit|stretch|integer&filter=linear|nearest&speed=200&day=5&dissolve&story`
`&story_secs=60&daynight=real&intro=0&intro_secs=3&mute&volume=50&seed=…`. (`?dissolve` also
dissolves the intro→first-scene; the intro hold is `?intro_secs`, default 3 s.)

> The game data is **copyright** Sierra/Dynamix and is **never** committed here. For the
> self-contained build you supply your own originals (read only at build time); the resulting
> bundle contains the game — **personal use only, do not host/redistribute it.**

## How it works
- The crate is `wasm32`-only (an empty lib on other targets, so it doesn't affect the desktop
  build). It exposes to JS: an `Options` struct (`seed`, `day`, `speed`, `dissolve`, `intro`,
  `story`, `story_secs`, `real_daynight`) and a `Wilson` class with `Wilson.create(map, data,
  nowSecs, opts)`, `Wilson.from_zip(zipBytes, nowSecs, opts)`, `Wilson.embedded(nowSecs, opts)`
  (embed-data only), `frame(nowSecs) → RGBA`, `delay_ms()` (speed-scaled), `width()/height()`,
  and the sound API below — plus `has_embedded_data()` so the page knows whether to auto-start.
- `from_zip` reads the zip **in wasm** (the `zip` crate, deflate via miniz_oxide) and decompresses
  the installer's DCL members (`RESOURCE.00$`, `SCRANTIC.SC$`) via `wilson_dgds`, like `--data`.
- Rust returns raw RGBA; the page (`web/index.html`) wraps it in `ImageData` and draws it,
  pacing frames by `delay_ms()` (the engine's 16 ms/tick). No `web-sys` dependency — just
  `wasm-bindgen`.
- The clock comes from JS (`Date.now()`), so the day/night cycle and the 11-day arc work.
- **Sound (on by default):** the engine emits per-frame sound cues; `take_sounds()` drains them
  and `sound_wav(id)` returns the WAV bytes, which the page plays via the **Web Audio API**. The
  effects live inside `SCRANTIC.EXE` (not `RESOURCE.*`): baked in for an `embed-data` build, or
  loaded at runtime via `set_sound_data(exeBytes)` when the user supplies `SCRANTIC.EXE`
  (`has_sound()` reports availability). A 🔊/🔇 button toggles mute. **Browsers block audio until
  the first user interaction** (a security policy — the desktop has no such limit), so the page
  starts sound on the first click/keypress/tap/pointer and the 🔊 control **pulses** until then
  (it doesn't pretend sound is already playing). Sound stays on by default; the user mutes if they
  want.

## Build & run
```sh
cargo install wasm-bindgen-cli        # version must match the wasm-bindgen crate
                                      # (the wasm32 target is auto-added by build-web.sh)

# Bring-your-own-data page:
./build-web.sh
python3 -m http.server -d web 8000    # open http://localhost:8000/ and drop your files or a .zip

# Self-contained page (personal use) — bake your data in:
WILSON_EMBED_DATA=<dir-with-RESOURCE.*> ./build-web.sh   # then serve web/ as above — it just runs
```

(From the packager: `scripts/build-embedded.sh --web` — self-contained when given a `<data-dir>`,
bring-your-own without one.) The generated `web/wilson_web.js` + `wilson_web_bg.wasm` are build
outputs (git-ignored); only `index.html` is committed.

## Testing

[`e2e/`](e2e/README.md) drives the built page in a **real headless Chrome** (Playwright), like a
user: with an embedded build it checks the engine **renders** and that **sound actually plays**
after a click (the browser autoplay gate); with a bring-your-own build it's a data-free **smoke**
(loads + wasm inits + no JS errors). CI runs the smoke (job `web-e2e`); the full render+sound test
is local (it needs the copyright data). See the testing rule in the repo's `CLAUDE.md`.

[`wilson-engine`]: ../wilson-engine
