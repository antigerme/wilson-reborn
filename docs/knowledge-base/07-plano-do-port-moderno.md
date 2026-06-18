# 07 — Modern Port Plan (Wilson Reborn)

> An action-oriented synthesis: how to build a **modern, extremely portable
> (Windows/Linux) clone, at better resolutions and with improvements**, **without losing any
> resource** of the original. Recommendations + roadmap. Decisions marked 🟦 are points to
> confirm with you.

---

## 1. Goals (of the project)

1. **Extreme portability** Windows/Linux (and, as a bonus, web/macOS when possible).
2. **Modern language**.
3. **Better resolutions** than the original's fixed 640×480 / 16 colors.
4. **Improvements** beyond the original (without breaking it).
5. **Total parity** with the original — all events, gags, history, easter eggs,
   holidays (see [checklist in bible §14](02-biblia-de-conteudo.md#14-parity-checklist-lose-nothing-summary)).

---

## 2. Recommended stack 🟦

**Primary recommendation: Rust** + a *pixel buffer* presented by GPU (`pixels`/`wgpu`),
mirroring the original's **layered blitter** model.

Why: a single static binary, **trivial cross-compile** (Win/Linux/macOS), **runs on
WebAssembly** (a "free" web version), memory-safe, great for a low-consumption long-running
process. The original engine is a 2D surface compositor — an accelerated
*framebuffer* (`pixels` over `wgpu`) preserves the composition logic 1:1
(background → zones → threads → holiday) and still provides scaling/HiDPI.

**Strong alternatives** (all meet the goals):

| Stack | Pros | Cons |
|---|---|---|
| **Rust + `pixels`/`wgpu`** (rec.) | single binary, WASM, perf, safe | learning curve |
| **Rust + `macroquad`** | simple API, native web, easy to package | less fine control |
| **Go + Ebitengine** | dead-simple builds and cross-compile, WASM, 2D ready | larger binaries; GC |
| **TypeScript + Canvas + Tauri** | reuses `castaway`/`dgds-viewer`; web-first; Tauri ≪ Electron | desktop depends on WebView |
| **C + SDL2** (= jc_reborn) | reuses almost everything from jc_reborn | "less modern" C; GPLv3; no easy web |

> If goal #1 is **least effort by reusing ready-made code**, the path is
> **TypeScript** (`dgds-viewer` parsers + `castaway` metadata) or **C/SDL2**
> (a conceptual fork of jc_reborn). If it is **the best long-term product**, **Rust**.

---

## 3. Proposed architecture

Mirror jc_reborn's 4 clean layers ([05 §10](05-arquitetura-do-engine.md)):

```
┌─────────────────────────────────────────────────────────────┐
│ Platform: Win screensaver (.scr) │ Linux (XScreenSaver/      │
│ standalone/Wayland) │ standalone app │ web (WASM) │ wallpaper │
├─────────────────────────────────────────────────────────────┤
│ Render/Audio backend (trait/interface): pixels|wgpu|canvas    │
├─────────────────────────────────────────────────────────────┤
│ Game logic: director (story), walk, island, day/holiday       │
├─────────────────────────────────────────────────────────────┤
│ Bytecode VMs: TTM + ADS interpreter                           │
├─────────────────────────────────────────────────────────────┤
│ Data I/O: RESOURCE.MAP/.001 parser + RLE/LZW + types          │
└─────────────────────────────────────────────────────────────┘
```
An **abstract backend** (a `Renderer`/`Audio` interface) lets the same core run on
desktop, web and as a screensaver — key to "extreme portability".

---

## 4. Resolution independence (the "run better")

The original is **640×480, 16 colors, bitmap sprites**. Strategies (combinable):

1. **Integer nearest-neighbor scaling** (pixel-perfect): renders into the 640×480 buffer
   and scales ×2/×3/×N to the screen — faithful, sharp, trivial. *MVP.*
2. **Scaling to any resolution / HiDPI** with a filter option (nearest vs
   smooth) and *letterboxing* to keep the aspect ratio.
3. **Repositioning on large screens:** the engine already draws the island in a random position
   (`VARPOS_OK`); widening the ranges for widescreen/4K screens gives more "sea space".
4. **HD asset pack (future):** since the sprites are small and stylized, you can
   **redraw them in high resolution** (or vectorize) an optional "HD asset pack", keeping
   the original pixel art as the default.
5. **Multi-monitor** and **native resolution** detected automatically.

---

## 5. Assets and legal strategy 🟦

The original data (`RESOURCE.*`, sprites, sounds) is **copyright Sierra/Dynamix**
([01 §legal note](01-historia-e-creditos.md)). Options:

- **(A) BYO data (the clones' default):** Wilson Reborn is the free *engine*; the user
  provides their `RESOURCE.MAP`/`RESOURCE.001` (which they own). Simple and legally safe.
- **(B) Recreated asset pack:** **new** art/sounds made from scratch, redistributable →
  a 100% standalone and legal version. More work, but it is the path to distributing
  "complete".
- **(C) Hybrid:** engine + a loader that accepts **both** the original data **and** a
  recreated asset pack (its own format, e.g. JSON + PNG/sprites + ogg). Recommended:
  it opens both doors.

> The 3 sets that **do not** come from `RESOURCE.001` (`story_data.h`, `walk_data.h`,
> `calcpath_data.h`) need to be ported/recreated either way
> ([03 §8](03-dados-originais-e-formatos.md#8-data-that-is-not-in-resource001)).

---

## 6. Per-platform packaging

- **Windows:** a `.scr` is just an `.exe` that responds to `/s` (show), `/p` (preview),
  `/c` (config). The core compiles to `.exe` and exposes these arguments.
- **Linux:** fullscreen standalone (classic screensaver mode = "any key quits"); and/or
  integration with **XScreenSaver** (windowed mode via `-window-id`) and Wayland (ext-idle).
- **Standalone app / "live wallpaper":** the same binary, windowed/desktop mode.
- **Web (WASM):** an in-browser demo (as castaway/dgds-viewer already do).
- **macOS (bonus):** a `.saver` bundle if desired.

---

## 7. Improvements roadmap (without breaking the original) 🟦

Combining the `castaway` roadmap + opportunities from this research. Everything **optional/
configurable**, with a 100% faithful "classic mode" as the default.

**Visual/time**
- A real **24h day/night cycle** (instead of 8h), optionally based on **geolocation**
  (real sunrise/sunset).
- **Real tides** by location; **moving** clouds; extra waves/parallax.
- **HD resolutions**, multi-monitor, HiDPI, optional HD asset pack.

**Content**
- **Extensible/configurable holidays** (the table is small —
  [bible §9](02-biblia-de-conteudo.md#9-anniversary-dates--holidays-annivers--storyc-logic));
  investigate the **July 4** mentioned by Wikipedia and add regional dates (e.g.:
  Brazilian holidays).
- **"Classic bugs" mode** as an easter egg (giant island, dozens of Johnnys —
  [bible §12](02-biblia-de-conteudo.md#12-original-bugs-cataloged-in-bugs)).
- Play the **full story in sequence** (not just by real day) — "story" mode.

**Quality of life**
- **Configuration UI** (speed, sound, cycle, holidays, scale/filter, monitors).
- **Statistics** (hours played, activities seen) — castaway's idea.
- **Accelerate time** / jump to a story day (great for testing and for the user
  to see everything).

---

## 8. Phased plan

| Phase | Deliverable | Focus |
|---|---|---|
| **0 — Foundation** | `RESOURCE.MAP/.001` parser + RLE/LZW; resource dump (validate against `jc_reborn dump`) | [03](03-dados-originais-e-formatos.md) |
| **1 — VMs** | TTM + ADS interpreters; play **one scene** in isolation | [04](04-engine-scripting-opcodes.md) |
| **2 — Render** | Layered pixel-buffer backend + palette + sprites + sound; play a scene with audio | [05](05-arquitetura-do-engine.md) §7–8 |
| **3 — Island & walk** | Background/tide/night/clouds/raft; spots A–F + pathfinding + walk animation | [05](05-arquitetura-do-engine.md) §5–6 |
| **4 — Director** | `storyPlay`: 11-day cycle, scene selection, holidays → **parity** | [02](02-biblia-de-conteudo.md), [05](05-arquitetura-do-engine.md) §4 |
| **5 — Package** | Windows `.scr` + standalone Linux + (web) | §6 |
| **6 — Improvements** | HD resolutions, 24h day/night, config UI, etc. | §7 |

**Faithful MVP = end of Phase 5.** Validate parity with the
[bible checklist §14](02-biblia-de-conteudo.md#14-parity-checklist-lose-nothing-summary).

---

## 9. Open questions / risks

| Item | Detail | Resolution path |
|---|---|---|
| **11 vs ~120 days** | Engines use an **11-day** cycle; Wikipedia says ~120 | Follow the data (11), keep it configurable; investigate `scrantic.ini`/`NumDays` |
| **Independence Day** | Cited by Wikipedia, absent from the site/jc_reborn | Look for art/scene in the data; extensible holiday table |
| **RLE2 (method 3)** | Only JCOS mentions it; unused(?) | Implement only if it shows up in the data |
| **VQT:** | VQ images not decoded by ScummVM | Check whether JC uses it; otherwise, ignore |
| **jc_reborn approximations** | walk/scheduler/island position are observational | Refine via **disassembly of `SCRANTIC.SCR`** if you want 100% |
| **Opcode divergences** | DELAY ×10/×20, `0xA100` rect vs window | Validate by running against the data ([04 §4](04-engine-scripting-opcodes.md#4-divergences-between-implementations-watch-out-when-porting)) |
| **Non-resource data** | `walk/story/calcpath` come from the exe/observation | Port verbatim or re-extract from `SCRANTIC.SCR` (offset 0x188ea) |

---

## 10. Decisions to confirm 🟦
1. **Language/stack** (Rust recommended; alternatives in §2).
2. **Asset strategy** (BYO data / recreated asset pack / hybrid — §5).
3. **MVP scope** (faithful parity first vs. include improvements right away).
4. **License** of Wilson Reborn (affects how much to reuse from the GPL projects — [06 §4](06-projetos-de-referencia.md#4-license-matrix-summary)).
