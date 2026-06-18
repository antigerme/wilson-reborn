<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# wilson-web e2e — the page in a real headless Chrome

A Playwright test that drives the built web page in a **real headless Chrome, like a user**, to
validate what unit tests can't: that the engine actually **renders in the browser** and that
**sound actually plays** after a user gesture (the browser autoplay gate).

It adapts to whatever bundle is in [`../web`](../web):

| Bundle (how you built it) | What `run.mjs` does |
|---|---|
| **embedded** — `WILSON_EMBED_DATA=<dir-with-RESOURCE.*> ../build-web.sh` | **Full test**: canvas renders non-black (engine runs), audio is `suspended` before a gesture, then a click resumes it and a **sound buffer actually starts** (`?seed=0&speed=400`, so cues are deterministic + fast). |
| **bring-your-own** — `../build-web.sh` (no data) | **Smoke test**: the page loads, the wasm module initialises, the picker is present, and no JS errors are raised. (No game data → no render/sound check.) This is what **CI** runs (the data is copyright, so it can't ship to CI). |

## Run it

```sh
cd crates/wilson-web/e2e
npm install
npx playwright install chromium     # one-time: download the browser

# Full test (proves sound), using your own originals:
WILSON_EMBED_DATA=/path/to/RESOURCE-dir ../build-web.sh
node run.mjs

# Smoke test (no data), same as CI:
../build-web.sh
node run.mjs
```

`run.mjs` serves `../web` itself and exits non-zero on any failure. `node_modules/` is git-ignored.

> The full test embeds the **copyright** game data into the bundle (personal use only — never
> committed or hosted). CI only runs the data-free smoke; the full render+sound test is a local
> validation step (see the testing rule in the repo's `CLAUDE.md`).
