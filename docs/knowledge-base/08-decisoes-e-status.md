# 08 — Project Decisions and Status

> **Consolidated** state of the project (firm decisions + where we are). Update on every
> increment. For the chronological history, see [`../PROJECT-LOG.md`](../PROJECT-LOG.md).

## Firm decisions (summarized ADR)

| # | Decision | Choice | Rationale |
|---|---|---|---|
| 1 | **Language/stack** | **Rust** (Cargo workspace) | single binary, cross-compile Win/Linux + WASM, safe, ideal for a long-running screensaver process |
| 2 | **Assets** | **100% original** | uses **only** the user's original files (`RESOURCE.MAP`/`RESOURCE.001`), via `--data <dir>` or auto-detection. **No recreated pack** (revised on 2026-06-15: the recreated art was removed — it did not reach the desired quality; focus on total parity with the original data). |
| 3 | **Scope** | **All improvements** | but delivered in **100% functional increments** |
| 4 | **License** | **GPL-3.0-or-later** | allows reusing jc_reborn/JCOS (GPLv3) + ScummVM (GPLv2+); castaway/dgds-viewer per each upstream's `LICENSE` (check before copying code). GPLv3 chosen |

## Permanent processes (agreed with the user)
- Always go from a **100% working** point to another **100% working** one.
- **Complete tests/validations** + **GitHub CI** (Ubuntu + Windows + Fedora). Red CI ⇒ resolve.
- **Follow PRs** (conflicts and CI) and resolve. The user does a squash merge and deletes the branch;
  I can open a PR when the branch matures. Always a new branch per increment.
- **Document everything** (knowledge-base, this file and PROJECT-LOG) so as not to lose memory.

## Target architecture (summary — see [07](07-plano-do-port-moderno.md))
Layers: data I/O → VMs (TTM/ADS) → render/audio backend → game logic →
platforms (`.scr` Win / Linux / standalone / web).

Planned crates:
- `wilson-dgds` — formats + decompression + resources. **(resource layer complete)**
- `wilson-engine` — TTM/ADS VMs + director/story + walk + island + **integration (`Show`)**.
  **✅ complete headless engine** (from `RESOURCE.*` to a stream of composed frames).
- `wilson` — app/window (winit + **softbuffer**, CPU) + loader for the original
  `RESOURCE.*` (`--data` or auto-detection; no recreated pack). **✅ live window running.**
  (`softbuffer` was chosen over `pixels/wgpu`: lighter, no GPU stack, faster CI —
  fits the engine, which already produces a CPU buffer.)

## Status (roadmap)

| Phase | Description | State |
|---|---|---|
| KB | Knowledge base | ✅ completed (merged) |
| **0** | **Data layer** (`RESOURCE.*`, RLE/LZW, chunks, PAL) | ✅ completed (PR #2) |
| **1a** | **`.BMP/.SCR/.TTM/.ADS` parsers + `Archive`** | ✅ completed (PR #3) |
| **1b** | **Decode TTM/ADS bytecode → instructions (disassembler)** | ✅ completed (PR #4) |
| **1c** | **Executable TTM interpreter (headless, 1 thread) + `Surface`** | ✅ completed (PR #5) |
| **1d** | **ADS scheduler (multi-thread + composition + RANDOM/triggers)** | ✅ completed (PR #6) |
| **1e** | **Director (11-day story, selection, island state: tide/night/raft/holiday)** | ✅ completed (PR #7) |
| **1f** | **Pathfinding between spots (2nd-order adjacency matrix + routes)** | ✅ completed (PR #8) |
| **1g** | **Walk animation (`walk_data.h` frames + `Walker` state machine)** | ✅ completed (PR #9) |
| **1h** | **Island rendering (background, raft, clouds, waves, holiday props)** | ✅ completed (PR #10) — **Phase 1 (headless engine) complete** |
| **2a** | **Integration (`Show`): director + island + walk + ADS → frame stream** | ✅ completed (PR #11) |
| **2b** | **`wilson` app: live window (winit + softbuffer) + `RESOURCE.*` loader** | ✅ completed (PR #12) |
| **2c** | **Validation against REAL data (gated test) + 4:3 scaling (letterbox)** | ✅ completed — **the engine renders the original Johnny** |
| 2d | Polish: sound, day persistence, config/options (fullscreen, scale, speed) | ✅ completed — sound · persistence · config/options |
| 3 | Packaging (Win/Linux/web/WASM) → **playable parity** with the original data | 🟡 **in progress** — ✅ **release CI** (`release.yml`: Windows `wilson.scr` + Linux binary); ✅ **self-contained build** (feature `embed-data`: original data embedded in the binary, runs without `--data`; only at compile time, never in the repo — personal use due to copyright); ✅ **first public release `v0.2.0`** (`release.yml` published the Windows `.scr`/`.exe`, Linux and macOS binaries + the `.saver` bundle to the GitHub Release); ✅ **web/WASM** (`wilson-web`: the engine compiled to wasm32, runs in a browser with bring-your-own `RESOURCE.*` — loose files **or a `.zip`** run/installer, or embedded; **sound** on by default 🔊+volume (Web Audio), **fullscreen ⛶** + black bg + Wake Lock, **URL options** mirroring the desktop CLI [`scale=fit`+`filter=linear` defaults, `speed/day/dissolve/story/daynight/intro/mute/volume/seed`], opt-in **save-in-browser** (IndexedDB); CI builds the wasm target) |
| 4 | Improvements (24h day/night, config, statistics, etc.) | 🟡 **in progress** — ✅ config/options · ✅ **24h day/night** · ✅ **statistics** · ✅ **parity audit** ([09](09-paridade-e-easter-eggs.md)) · ✅ **loader robustness** · ✅ **render/timing audit vs jc_reborn** (palette/tick/SET_DELAY check out; holiday z-order fixed) · ✅ **100% opcode coverage** (saved-zone layer `COPY_ZONE_TO_BG`/`RESTORE_ZONE` for the giant cargo ship; no opcode from the real data is ignored anymore) · ✅ **reverse engineering of the original** ([10](10-engenharia-reversa-do-original.md); reproducible tools in [`../reverse-engineering/`](../reverse-engineering/README.md)) · ✅ **intro screen** (`INTRO.SCR` at startup + `Introduction` toggle) · ✅ **MCI resolved as dead code** (not a gap) · ✅ **knowledge base in English** |

> **Pivot 2026-06-15:** the **recreated pack** (procedural art: island/palm/Johnny/Mary/
> Suzy/visitors/easter eggs etc.) was **removed** — it did not reach the desired quality.
> The focus became **100% the original files** (total parity already validated with
> `--data`). The engine, the window, sound, config, persistence, statistics and the
> packaging remain.

## Real data validation ✅
Validated against the **authentic** `RESOURCE.001` (md5 `374e6d05…`): 180 resources
(pal=1, bmp=117, scr=10, ttm=41, ads=10), **LZW + ~37 thousand TTM/ADS instructions
decoded without error**, and hundreds of rendered frames (the original Johnny appears
correctly). Captured by a **gated integration test** (skipped in CI, without copyright
data):
```sh
WILSON_DATA_DIR=/path/to/dist cargo test -p wilson-dgds --test real_data -- --nocapture
```
> The original data and copyright files (`RESOURCE.*`, `dist.zip`, `.msi`) are **not**
> redistributed in the repository nor in the public releases; the app **requires** the user's
> data via `--data`/auto-detection (or, for personal use, embedded in the binary via the
> `embed-data` feature — bytes read only at compile time, never versioned).

### History (before validation)
The tests used only synthetic fixtures; the byte-exact validation of the LZW/parsers against
a real `RESOURCE.001` was planned as an optional integration test via an environment variable
pointing to the original data.
