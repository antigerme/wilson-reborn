# 06 — Reference Projects (comparison, reuse and licenses)

> Evaluation of the 5 projects in `repos/` as a reference for Wilson Reborn: what each
> one does best, what to reuse and the license implications. Detailed technical notes
> on each are in [`raw/`](raw/).

> **⚠️ The 5 projects are NO LONGER vendored in `repos/`** — they were removed from the
> public repository (do not redistribute third-party code or copyright material). To
> consult them, clone the upstreams:
>
> - **jc_reborn** (jno6809): <https://github.com/jno6809/jc_reborn>
> - **JCOS** / Johnny-Castaway-Open-Source (nivs1978): <https://github.com/nivs1978/Johnny-Castaway-Open-Source>
> - **castaway** (xesf): <https://github.com/xesf/castaway>
> - **dgds-viewer** (xesf): <https://github.com/xesf/dgds-viewer>
> - **dgds** (ScummVM): <https://github.com/scummvm/scummvm> (`engines/dgds`)
>
> The `repos/...` references in this document and in the KB point to the paths of **those**
> projects.

---

## 1. Comparison table

| Project | Language/Stack | License | State | Main strength |
|---|---|---|---|---|
| **jc_reborn** | C99 + SDL2 | **GPLv3** | Playable, "every scene works" (w/ inaccuracies) | **Best gameplay blueprint**: near-complete VMs, walk, scheduler, island, day/holiday |
| **dgds (ScummVM)** | C++ (ScummVM) | **GPLv2+** | Exploratory WIP (RotD/HoC) | **DGDS format authority**: chunks, RLE/LZW, fonts, sound/MIDI, hash |
| **JCOS** | C# / WinForms (.NET) | **GPLv3** | WIP, Windows-only, no day cycle | **Best format+opcode docs** (English names, operands); extracted the `.wav` |
| **castaway** | JS (ES Modules), Canvas | see `LICENSE` | Web WIP | **SCRANTIC metadata** (scene names, story/day), improvements roadmap |
| **dgds-viewer** | JS + React + Electron | see `LICENSE` | Multi-game viewer | **Best JS tooling**: defensive parsers, disassembler, inspection UI |

---

## 2. Individual evaluation

### 2.1 `jc_reborn` — the gameplay base ⭐
**It is Wilson Reborn's primary reference.** It loads the original data and reproduces the
behavior by interpreting TTM/ADS. Clean 4-layer architecture (I/O, VM, backend,
logic). It implements what no other has together: **walk between spots, random scene
scheduler, island/cloud drawing, 11-day cycle and holidays**.

- **Reuse:** opcode semantics (§[04](04-engine-scripting-opcodes.md)); the 3
  data tables (`story_data.h`, `walk_data.h`, `calcpath_data.h`); the
  director/scheduler logic; the day/night/tide/holiday logic.
- **Caveats:** walk, scheduler, island positioning and some zone ops
  (`grSaveImage1`, `grSaveZone`) are **observational approximations** (the author admits it); a
  disassembly of the original would refine them. The render re-blits everything per frame (no dirty-rect).
- **License:** GPLv3 — copying code contaminates; reusing the documented *insights*
  (this KB) does not.

### 2.2 `dgds` (ScummVM) — the format authority ⭐
A ScummVM engine for the DGDS family. **It does not cover JC** in `detection_tables.h` (it lists only
Rise of the Dragon and Heart of China, which use `VOLUME.RMF` instead of `RESOURCE.MAP`), but
the **chunk format, compression, fonts, palette and sound are shared**.

- **Reuse:** the rigorous container spec (the 0x80000000 bit, the packed-chunk
  prefix), exact RLE/LZW (incl. the `_cacheBits` nuance), BIN/VGA plane
  recombination, the `<<2` palette, and `dgdsHash()` (if one day supporting other DGDS games).
- **Caveats:** many ADS/TTM opcodes are **stubs** (`warning("Unimplemented")`) — for
  JC opcode semantics, `jc_reborn` is better. `VQT:` is not decoded.
- **License:** GPLv2+ (ScummVM).

### 2.3 `JCOS` — the pioneer and opcode dictionary
The first complete decoding of the data (2015). `Instruction.cs` has a `ToString()`
that **spells out the operands of ~60 opcodes** — the best reference for names/tuples.
It also **extracted the 24 `.wav`** that jc_reborn reuses.

- **Reuse:** the opcode dictionary; the ADS/TTM/BMP/SCR/PAL grammar; the
  `.wav`; the Excel disassembly exporter (`Excel.cs`).
- **Caveats:** WinForms **Windows-only**, hardcoded `C:\SIERRA\SCRANTIC` path, **no
  day/holiday logic** (random selector of 4 scenes), debug rectangles on top,
  fixed 16-color palette instead of the parsed `.PAL`, 2 sprite opcodes not
  implemented.
- **License:** GPLv3.

### 2.4 `castaway` — metadata and roadmap
A web port (Canvas). The only one with the **SCRANTIC story/scene layer**:
`metadata/scenes.mjs` (descriptive names of the ACTIVITY scenes), `story.mjs` (day
counter), `palette.mjs`. It has an **improvements roadmap** very aligned with your goals
(see [07](07-plano-do-port-moderno.md) §enhancements).

- **Reuse:** the **scene descriptions** (GAG DIVES, NATIVE, GULL READING…); the
  roadmap; the Canvas rendering approach (if the target is web).
- **Caveats:** `story.mjs` still picks a **uniformly random** scene (without the real day
  schedule); RLE2 throws an error.

### 2.5 `dgds-viewer` — the best JS tooling
A more elaborate version of castaway, as a **generic viewer** of DGDS resources (all
5 games), with React/Electron and a **live disassembler** (`ScriptCode.jsx`).
A more robust interpreter (`process.js`, O(1) dispatch).

- **Reuse:** the **parsers** (clean, dependency-free, `DataView`); the disassembler
  to **inspect/debug assets** during development.
- **Caveats:** the interpreter (`process.*`) is the weak point (mutable global state, not
  re-entrant, many NOPs, ADS scheduling with bugs) — use it as a **semantic reference**,
  rewrite from scratch.

---

## 3. Recommended reuse strategy

1. **Knowledge, not code (default).** This KB captures the *insights* (formats,
   opcodes, logic) in a license-independent way. Reimplementing from it keeps
   Wilson Reborn free from GPL contamination — useful if you want to choose the license.
2. **JS parsers** (`dgds-viewer`) are the easiest to port if the target is
   web/TypeScript.
3. **Data tables** (`story_data.h`, `walk_data.h`, `calcpath_data.h` from jc_reborn):
   they are **reconstructed data/facts**. Reusing them speeds things up a lot; assess the license
   implication (data vs creative code) or **re-extract** from `SCRANTIC.SCR`.
4. **Disassembler** (`dgds-viewer` or `jc_reborn dump`): indispensable for validating the
   port against the real data.
5. **Cross-validation:** run the same scene in ≥2 implementations and compare — the best
   way to resolve the opcode divergences ([04](04-engine-scripting-opcodes.md) §4).

---

## 4. License matrix (summary)

| Origin | License | Implication if you **copy code** |
|---|---|---|
| jc_reborn, JCOS | GPLv3 | a derivative work must be GPLv3 |
| dgds (ScummVM) | GPLv2+ | a derivative work must be GPL |
| castaway, dgds-viewer | see `LICENSE` in the upstream | check before copying |
| **This KB (docs)** | — | reimplementing from facts/documentation does not create a derivative work of the code |

> **The game data** (`RESOURCE.*`, sprites, sounds) remains **copyright
> Sierra/Dynamix** — no project redistributes it (see
> [01 §legal note](01-historia-e-creditos.md)). Wilson Reborn's license and asset
> decision is in [07](07-plano-do-port-moderno.md).
