<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# 10 — Reverse Engineering of the Original (parity report)

> **Goal:** to answer, with evidence, the question *"are we forgetting something?"* —
> by comparing the Wilson Reborn implementation directly against the original binary
> (`SCRANTIC.EXE`/`.SCR`) and against the data (`RESOURCE.001`). It is **legitimate RE of one's
> own copy** for an interoperable reimplementation (the same basis as ScummVM/jc_reborn/JCOS).
>
> **Method.** (1) Parse the NE (Win16) structure of `SCRANTIC.EXE`; (2) locate and
> verify **byte-for-byte** the hardcoded tables against our code; (3) search for
> constants/immediates of our logic in the binary; (4) an exhaustive histogram of **every**
> TTM/ADS opcode used in the real `RESOURCE.001`, cross-checked with the executors; (5) a resource
> inventory; (6) a content audit against the [bible](02-biblia-de-conteudo.md) and the
> [parity audit](09-paridade-e-easter-eggs.md). Available disassembly tool:
> only `objdump` (16-bit) — a deep disassembly of all the logic was not feasible,
> so the observational logic is evaluated by **constants + behavior**, not instruction
> by instruction.

## 1. The original binary (NE structure)

`SCRANTIC.EXE` = `SCRANTIC.SCR` (a Windows 3.x screensaver is a renamed `.exe`), format
**NE (16-bit segmented)**, 295,952 bytes. Internal module: `SCRNATIC`.

- **14 segments:** 13 of **code** + 1 of **data** (#14, 18,368 bytes, file `0x17e00`).
  The hardcoded tables (which do **not** come from `RESOURCE.001`) live in the data segment.
- **Imported API:** `MMSYSTEM` (sound), `GDI` (2D graphics), `KERNEL`, `USER`. Standard
  SCRNSAVE entry points (`ScreenSaverProc`, `ScreenSaverConfigureDialog`, Win3.1 password
  dialogs). **Conclusion:** it is an ordinary GDI+WAV screensaver — **no hidden
  subsystem** (no networking, no special hardware, no dedicated MIDI). Our stack (softbuffer
  2D + `rodio` WAV) covers the same surface. The password dialogs are a dead Win3.1
  feature — not a gap.

## 2. Hardcoded tables — verification against the binary

| Table | Result | Confidence |
|---|---|---|
| **`walk_data`** (walk animation) | **489/489 entries BYTE-IDENTICAL** to the original | **Binary proof** ✅ |
| `calcpath` (pathfinding adjacency) | **does not exist** as a table in the EXE | Reconstruction (soft spot) ⚠️ |
| `story_data` (63 scenes) + holiday/tide/drift/scheduling logic | constants **not** locatable as immediates | Port of jc_reborn; content matches the bible ⚠️ |

### 2.1 `walk_data` — byte-perfect ✅
Located in the data segment (file `0x188ea`). The original struct is **3 words per frame
(stride 6)**: `(sprite_word, x+1, y)`, with **`flip` packed in bit 15 of the `sprite_word`**.
Our `WALK_DATA` (`[flip, x+1, y, sprite]`, inherited from jc_reborn — which **extracts** it from
the binary, via `extract_walk_data.c`) reproduces the **489** entries exactly: `x+1`, `y`,
`sprite` and `flip` check out in 100% of the rows. **No divergence.**

> Note: on a first reading I grouped the fields as `(x+1, y, sprite)` and it *seemed* there was
> a 1-frame lag in the sprite. It was a grouping error — with the correct layout
> (`sprite` **first**), it matches exactly. Recorded so as not to reopen it.

### 2.2 `calcpath` — reconstruction, not verifiable ⚠️
Our adjacency matrix `WALK_MATRIX[7][6][6]` **does not appear** in the binary in any tested
form (bytes/words/transposed). This confirms what jc_reborn itself admits: the
pathfinding was **reconstructed by observation**, not extracted. The original computes the routes
by another mechanism (or stores them in a very different way). **There is no proof of byte-for-byte
parity;** it is a plausible model. Resolving it would require disassembling the `calcpath` routine.

### 2.3 `story_data` + story logic — port of jc_reborn ⚠️
The constants of our logic (holiday ranges `mmdd` like `1028<mmdd<1101`; island drift
ranges `-222+rnd(109)` etc.) **were not located as co-located immediates** in the
code — only the offset `-272` (LEFT_ISLAND) appears. That is: the original expresses these
comparisons in another way (probably `month`/`day` separately, not the `mmdd` that jc_reborn
reformulated). **The behavior matches the bible**, but the exact boundaries (e.g.: whether Halloween
includes Oct 28 or not) are jc_reborn's, **not proven in the binary.** The 63 scenes and the
day→scene map (the *day-beats*) are the documented arc (test `day_beats_match_the_story`).

## 3. Opcode coverage (over the real `RESOURCE.001`)

**Everything the data actually uses is handled** — nothing is silently ignored.

- **TTM: 30 distinct opcodes used → 30 handled.** Implemented (with effect): PURGE,
  UPDATE, SET_DELAY, SET_BMP_SLOT, GOTO_TAG, SET_COLORS, TIMER, SET_CLIP_ZONE,
  COPY_ZONE_TO_BG (38×), DRAW_PIXEL/LINE/RECT/CIRCLE, **DRAW_SPRITE (12,546×)**,
  DRAW_SPRITE_FLIP (2,822×), CLEAR_SCREEN, **PLAY_SAMPLE (535×)**, LOAD_SCREEN, LOAD_IMAGE,
  RESTORE_ZONE, TAG/LOCAL_TAG. No-ops faithful to jc_reborn: SET_PALETTE_SLOT, SET_FRAME1,
  SAVE_IMAGE1, SAVE_ZONE, DRAW_SCREEN, LOAD_PALETTE, `TTM_UNKNOWN_1`, and **`0x0080`
  DRAW_BACKGROUND (190×)**.
- **ADS: 18 opcodes used → 18 handled** (+66 *tags*). The argument counts check out
  (the stream never desyncs; the 10 ADS decode cleanly). Faithful no-ops: `IF_UNKNOWN_1`,
  `UNKNOWN_6`, `FADE_OUT` (in the bytecode).

## 4. Resource inventory — everything parses

179 entries in the map → **1 PAL, 116 BMP, 10 SCR, 41 TTM, 10 ADS, 1 skipped** (`FILES.VIN`,
non-resource, ignored by jc_reborn too). **0 broken `RES:` references; 66/66 ADS scene
tags build and play.** An inventory identical to ours (the 10 `.ADS` from
ACTIVITY…WALKSTUF; 42 `.TTM` names including the easter eggs `GJGULIVR`, `GJLILIPU`,
`SHARK1`, `SJMSSGE`, `SBREAKUP`, `THEEND`).

## 5. Content vs the bible

- **11-day arc (Mary + Suzy):** present and exercised (a long-run saw d1–d11); the day-beats
  match `story.c`.
- **Easter eggs / rare scenes:** present and reachable — THE END (day 11), Gulliver
  (`GJGULIVR`, uses SAVE/RESTORE_ZONE), giant cargo ship (`VISITOR#3`, uses COPY_ZONE_TO_BG),
  shark (`SHARK1`), natives (`GJNAT1/3`), "Terminator" (`GJVIS5/6`). Gags like
  ghost/silver balls/real clock are sub-sequences within the bytecode (they run when
  the script runs).
- **23 sound effects:** confirmed (22 ids referenced by TTM `PLAY_SAMPLE` + the director's day-transition
  cue `sound 0`). Id 17 exists as a WAV but **no TTM references
  it — same as in the original** (it is not a gap of ours).
- **4 holidays:** `HOLIDAY.BMP` has **exactly 4 sub-images** → the data can only
  draw 4 props (Halloween/StPatrick/Christmas/New-Year). **July 4 is genuinely
  absent from the original data** — the bible's pending item is **resolved: there is nothing to
  reproduce.**

## 6. Gaps / what we might be missing

| Sev. | Item | Detail |
|---|---|---|
| **MEDIUM** | **Scene transitions without fade/wipe** | jc_reborn does `grFadeOut()` (5 wipe styles) **between scenes** and in the intro, via the *scene-runner* in C (not in the bytecode — that is why the `0xF010` no-op is correct). Our `Show::go_next_scene` (`show.rs:296`) does a **hard cut**. A **real visual** difference (not a loss of content). *Note: the evidence comes from jc_reborn (a reimplementation); that the 1992 original also did a fade is very likely, but I did not locate the fade routine in the binary.* |
| ~~LOW~~ done | Unused intro/end screens | `INTRO.SCR` is now displayed once at startup (`Show::enable_intro`, default-on `intro` config / `--no-intro`, matching the original's `Introduction` toggle). `THEEND.SCR` still only appears as the day-11 TTM, not as a standalone closing sequence (like jc_reborn) — a minor presentation difference. |
| LOW | `0x0080` DRAW_BACKGROUND = **undocumented** no-op | Handled correctly by the catch-all (like jc_reborn's stub, which says "no-op; frees slots"), but **190× across 36 files** and **castaway/dgds-viewer/JCOS treat it as a background redraw**. If one day a visual bug appears, it is the #1 divergence point to investigate (disassemble the original's `0x0080` handler). |
| LOW | `0xA054` SAVE_ZONE = no-op | While the pair `0xA064` RESTORE_ZONE is implemented. Faithful to jc_reborn (there it is also a near-stub); used 1× (GJGULIVR.TTM). No known visible defect. |
| (soft) | `calcpath`, holiday/drift/scheduling constants | Not verifiable in the binary (§2.2/§2.3) — they are jc_reborn's observational RE, faithfully ported. |

## 7. Verdict

- **On the original DATA: high confidence that nothing is missing.** 100% of the TTM
  (30/30) and ADS (18/18) opcodes used are handled; 179/179 resources parse; 66/66 scenes
  build; the 11-day arc, day-beats, easter eggs, 23 sounds and the 4 holidays (= the maximum that
  the data allows) are present and covered by tests. **The bible's "July 4" pending item
  is resolved (absent from the original).**
- **On the observational LOGIC:** we are as faithful as the best reference (jc_reborn) —
  `walk_data` is **byte-perfect**; `calcpath` and the holiday/drift boundaries are jc_reborn's
  reconstruction, which we ported faithfully, but **byte-for-byte parity with the original cannot be
  proven** without a complete disassembly (beyond the reach of `objdump`).
- **Actionable parity improvements** (revised by the disassembly, §9): the only real
  binary-confirmed gap was the **intro** (a real resource with an `Introduction` toggle) —
  now **implemented** (`Show::enable_intro`, default-on `intro` config / `--no-intro`, matching
  the original's `Introduction` key). The **MCI** audio path turned out to be **dead code**
  (§9.4) — not a gap. *(The "fade between scenes" was **downgraded** — see §9.3: it came from
  jc_reborn, not confirmed in the original.)*

## 8. How to reproduce this analysis

With your own copy of the original in `<dir>` (see [INSTALL](../INSTALL.md)):
```bash
# structure + disassembly (segments, imports, relocations, per-segment .asm): the two
#   reproducible tools in ../reverse-engineering/ (NE parser + capstone disassembler):
SCRANTIC_EXE=<dir>/SCRANTIC.EXE WINE_SPECS_DIR=<specs> python3 ../reverse-engineering/ne.py
SCRANTIC_EXE=<dir>/SCRANTIC.EXE python3 ../reverse-engineering/disasm.py   # → $DISASM_OUT/seg*.asm
# tables: byte-for-byte comparison of WALK_DATA in <dir>/SCRANTIC.EXE, offset 0x188ea, stride 6.
# coverage/inventory/content:
WILSON_DATA_DIR=<dir> cargo run -p wilson-engine --example audit
WILSON_DATA_DIR=<dir> cargo test -p wilson-engine real_data_long_run_invariants -- --nocapture
WILSON_DATA_DIR=<dir> cargo test -p wilson-dgds --test real_data -- --nocapture
```

---

## 9. Complete disassembly of the binary (capstone) — straight from the original

> A deep pass at the user's request and **without depending on jc_reborn**. Tool: our own recursive
> disassembler (**capstone** 16-bit) with the **NE relocations resolved** (ordinal maps
> from Wine's `.spec` files) → each API call and internal `call` is labeled. **255
> functions, 25,732 instructions, ~75% of the code** via recursive descent (the rest is the Borland
> CRT, Windows callback *procs* and data tables). The tools are committed and reproducible
> in [`../reverse-engineering/`](../reverse-engineering/README.md) (`ne.py` + `disasm.py`);
> the raw listing they emit is kept **local** (it is a derivative work of the copyright code —
> it does not go to the repo); only the facts are here.

### 9.1 Architecture (confirmed in the binary)
- **Graphics:** composition in off-screen DCs (`CreateCompatibleDC`/`Bitmap`, `SelectObject`
  ×59, `BitBlt` ×19) + **`StretchBlt` ×11 = scale to the screen** (the original scales, like us).
  GDI vector primitives (`LineTo`/`MoveTo`/`Rectangle`/`Ellipse`/`CreatePen`) = the
  TTM drawing opcodes. **NO palette API** (`CreatePalette`/`RealizePalette`/`AnimatePalette`
  absent) ⇒ **no palette animation**; our approach (`.PAL` → RGB → blit) checks out.
- **Sound:** `sndPlaySound` (MMSYSTEM.2) for the WAVs (`WAVESFX%d`) — the only **live** audio
  path. `mciSendCommand` is imported but is **dead code** (see §9.4): its single call site
  (`seg5:00ab`) only ever issues `MCI_CLOSE` (`mov ax, 0x804`) on a device id (`[0x35ee]`)
  that is **never opened/written** — there is no `MCI_OPEN` (0x803) or `MCI_PLAY` (0x806)
  anywhere in the binary. So there is **no MCI audio path** — our `sndPlaySound`-equivalent
  (`rodio` WAV) covers 100% of the live sound.
- **Loop/time:** `SetTimer(…, 50 ms, …)` pumps `WM_TIMER` (0x0113), but the advance is **paced
  by real time** (`GetCurrentTime`, `elapsed × rate[0x2e14]` in fixed-point /100000 via a 32-bit
  helper `seg1:0302`) ⇒ **it is not a fixed 50 ms/frame** (it is frame-rate-independent). jc_reborn
  approximates with a fixed 20 ms/tick; **the exact rate (`[0x2e14]`, set at init) was not nailed down** —
  it is the only open timing number.
- **Config:** the INI `[ScreenSaver.ScreenAntics]` in `SCRANTIC.INI`: `Sounds`, `Introduction`,
  `Password`/`PasswordProtection`, `CurrentMonth` (persistence).

### 9.2 TTM interpreter (`seg12`) — opcodes read from the binary
Dispatch by **linear search in opcode tables** (the opcode in `[0x46da]`; a **two-pass**
interpreter, flag `[0x46d8]`; bitmap slots in `[0x2638]`/`[0x263e]`). The tables themselves:
- the **C** family, `seg12:0x00fc`: `c01f c02f c031 … c0f4 c102 cf01 cf11` (16 variants; the data
  only uses `c051`=PLAY_SAMPLE — we handle the used subset);
- the **A** family, `0x03bc`: `a002 a0a4 a104 … a5a7 a601 a704 af02 af1f af2f`;
- **A0xx zone**, `0x1900`: `a014 a024 … a054(SAVE) a064(RESTORE) a094 a0b5`;
- **low**, `0x12bd`: `0010 0020 0070 0080 0090 00c0 00e0 0110 0400`;
- alias: the interpreter **remaps** `0x1301→0xc051` and `0x1311→0xc061`.

**`0x0080` (DRAW_BACKGROUND) — RESOLVED:** the handler (`seg12:0806`) **frees the GDI handle of the
current bitmap slot** (`call seg6:1845` = a `DeleteObject` wrapper) and zeroes the slot —
**memory management, ZERO visual output.** Therefore: jc_reborn was **right** ("frees image
slots"); castaway/dgds-viewer/JCOS are wrong to call it a "background redraw"; **and our
no-op is correct vs the ORIGINAL.** *(The LOW doubt of §6 is resolved — it is not a gap.)*

### 9.3 Corrections to the items that came from jc_reborn (not from the original)
- **Fade/wipe between scenes (was "MEDIUM") → DOWNGRADED to NOT-CONFIRMED.** In the binary **there is no
  palette fade** (no palette APIs). A *wipe* via BitBlt is possible, but **was not
  located** in the analyzed code. The fade evidence came from **jc_reborn** (a reimplementation)
  — so **it may not be a gap**. Nailing it down requires analyzing the scene-transition path.
- **Intro (was "LOW") → CONFIRMED as a real resource:** `INTRO.SCR` + the config key
  **`Introduction`** (on/off) exist. We do not display the intro ⇒ **a real gap**,
  binary-confirmed.

### 9.4 New findings (only in the binary)
- **`mciSendCommand` — RESOLVED as dead code (NOT a gap).** It is imported, but the **only**
  call site (`seg5:00ab`) issues just `MCI_CLOSE` (`mov ax, 0x804`), guarded by
  `cmp [0x35ee], 0` / `je`. The device id `[0x35ee]` is **only ever read** there (the disassembly
  has no write to it) and there is **no `MCI_OPEN` (0x803) nor `MCI_PLAY` (0x806)** anywhere —
  so nothing is ever opened or played via MCI, and even the close never fires. It is vestigial
  cleanup scaffolding; **`sndPlaySound` is the sole live audio path**, which we reproduce. So
  the earlier "MCI path we lack" is **not a gap**.
- **Real-time clock** — format `"%2d:%02d %cm"` ⇒ a gag shows the PC's real time.
- The **complete C** family (`c01f…c0f4`, 16 variants of PLAY_SAMPLE) — we handle only the used one.

### 9.5 Post-disassembly verdict
The direct reverse engineering of the binary **strengthened** confidence: it confirmed the architecture and the
set of opcodes, and **resolved `0x0080`** (our no-op is correct vs the original). The single
binary-confirmed gap — the **intro** (a resource with an `Introduction` toggle) — has since been
**implemented**. **MCI** was **resolved as dead code** (§9.4), not a gap. The **fade** was
**downgraded** (not confirmed in the original). The **exact time rate** is the only open number.
`calcpath` and the holiday/drift constants remain in the untraced logic (faithful to jc_reborn,
without binary proof).
