<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Wilson Reborn architecture (a map for devs and AIs)

This document gives the **mental model** of the project: how the data becomes pixels on
screen, the role of each crate/module, the correspondence with `jc_reborn` (the reference)
and **how to validate** without hunting for bugs by eye. Read it alongside `CLAUDE.md`.

## The pipeline, from disk to screen

```
RESOURCE.MAP + RESOURCE.001  (original 1992 game data — copyright, not versioned)
        │
        ▼
  wilson-dgds        decodes the DGDS/SCRANTIC format:
                     map/archive, RLE/LZW, chunks, palette, BMP/SCR/TTM/ADS,
                     and the TTM/ADS bytecode → typed instructions. Zero deps. Pure.
        │  (Archive, Palette, Ttm, Ads, ResourceMap, find_ci, …)
        ▼
  wilson-engine      runs the scripts against an indexed Surface (headless):
                     • ttm_exec  — interprets one frame of a TTM thread (opcodes → drawing)
                     • ads_vm    — schedules up to 10 concurrent TTM threads (the "scene player")
                     • story     — the director: 63 scenes, 11-day cycle, tide/night/holiday,
                                   island drift + the VARPOS_OK/LEFT_ISLAND filter
                     • walk/path — Johnny walks between "spots" (pathfinding + animation)
                     • island    — draws the island/water/clouds/raft in the background
                     • show      — ties it together: plans a run, walks, plays the scene, composes frames
        │  (Show::next_frame → Frame { surface, delay_ticks, sounds })
        ▼
  wilson (app)       winit + softbuffer: opens the window and, on each timer tick, draws the
                     Frame (Surface → RGBA via the palette → 4:3 scaling with letterbox). It
                     loads the original data (--data/auto), plays soundN.wav, persists day/stats.
```

`wilson-saver` is the same engine exposed via FFI for the native macOS screensaver.

## The frame-production loop (the heart)

`Show::next_frame` mirrors `jc_reborn`'s `storyPlay`:

1. **Plan a run** (`Director::plan_run`): pick the final scene, the chain of ambient scenes
   (6–20), and the island state (tide/night/holiday/raft + `x_pos/y_pos` drift).
2. For each scene: **walk** Johnny to the spot (`Walker`), then **play** the ADS scene
   (`AdsVm`) over the island background.
3. `AdsVm::next_frame` runs **one scheduler iteration**: advances the threads whose timer hit
   zero, composes `background → saved zones → thread layers`, and returns `delay_ticks = mini`
   (the smallest pending delay among the active threads).
4. No more scenes → plan the next run.

## Timing (the part that has bitten us)

- Unit: **1 tick = 20 ms**. The frame carries `delay_ticks`; the app waits
  `frame_delay_ms(ticks) = ticks * 20 * 100 / speed` ms (`config.rs`).
- This is **identical** to `jc_reborn`: `eventsWaitTick(delay)` does `delay *= 20`
  (`events.c`), with `grUpdateDelay = mini` (`ads.c`).
- ⚠️ The pacing is only correct if the app **honors** `delay_ticks`. The winit loop must
  redraw **only** when the timer fires (`StartCause::ResumeTimeReached`), never on every
  `AboutToWait` (that bypasses `WaitUntil` and runs too fast). See `main.rs`.

## Island drift (the "balloon off-screen" part)

- The island can drift: `island_from_scene` picks `x_pos/y_pos` (ranges from `story.c`).
- When the island is offset, the director requires scenes with the **`VARPOS_OK`** flag
  (`wanted |= VARPOS_OK`, = `story.c:230`): only scenes that still look right when offset.
- The foreground (Johnny + scene props) is drawn at `ttmDx/ttmDy = x_pos/y_pos` (`+272` if the
  scene has `LEFT_ISLAND`), so Johnny follows the island. See `ads_offset`.

## How to validate (instead of hunting for bugs by eye)

**Automatic safety net (in CI, without the original data):**

- `cargo test -p wilson-engine` — including `engine_run_stays_live_and_paced`: it runs
  thousands of frames and requires that it never panics, always emits 640×480, **keeps
  animating** (does not freeze) and keeps a **human pace**.

**Deep validation (local, with the original data):**

```bash
WILSON_DATA_DIR=<dir-with-the-data> cargo test -p wilson-engine real_data_long_run_invariants -- --nocapture
```

Simulates ~20 min of playback while advancing the calendar and requires: 640×480 frames,
**100% opaque** (no leaked TRANSPARENT — the "magenta water" class of bug), live animation,
human pace and the **day advancing**.

**Visual review (a human or AI looks at a sampled montage):**

```bash
# render ~1h of a run and save 1 frame every ~30s (fits on disk)
cargo run -p wilson-engine --example render_run -- <data-dir> /tmp/out 27000 225 1
# turn it into a montage (or a short mp4) with ffmpeg:
ffmpeg -pattern_type glob -i '/tmp/out/*.ppm' -vf 'scale=240:180,tile=8x8' /tmp/montage.png
```

The frames are exactly what the app shows. Looking at the montage catches gross visual bugs
(island off-screen, wrong color, freeze) that invariants don't capture.

> Honest limitation: without a reference video of the original (which is non-deterministic),
> we can't compare pixel-by-pixel. The combination of **invariants + montage review + the
> occasional real-data test** is what keeps things "100% functional" without constant manual
> inspection.
