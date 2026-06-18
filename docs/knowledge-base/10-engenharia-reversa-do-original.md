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
> [parity audit](09-paridade-e-easter-eggs.md); and (7) a **full instruction-level disassembly**
> (capstone 16-bit, NE relocations resolved — §9) that resolved the remaining open questions
> instruction-by-instruction (§10). *(§§1–8 below were written before that pass, when only
> `objdump` was available and the observational logic was judged by constants + behaviour; §§9–10
> supersede the few caveats that the deep disassembly later settled.)*

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
| `calcpath` (pathfinding adjacency) | **route table FOUND + ported byte-faithfully**: 6×6 weighted route streams at `seg14:0x0A94`→`0x0362–0x0AA2` (see §10.3) | **Binary proof** ✅ (decoded, diverged from the old reconstruction, now ported) |
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

### 2.2 `calcpath` — byte-checked, then ported byte-faithfully ✅ (see §10.3)
**Resolved and ported (2026-06-18).** The original is table-driven: a **6×6 matrix of word
pointers** (start × dest, **1-based**) at `seg14:0x0A94`, each → a **route stream**
(`seg14:0x0362–0x0AA2`) of marker-delimited, **weighted** `(next_spot, weight)` choices per
cursor spot (lookup `seg4:03ac`). The byte-check found our old `WALK_MATRIX[7][6][6]` (the
jc_reborn reconstruction) **diverged on all 30 pairs** — a second-order *unweighted* model vs
the original's first-order *weighted, per-trip-curated* one (clearest case: the original never
walks 5→3 directly, always via 4). **Fixed:** `path.rs` now ports the original's weighted route
streams (extracted into `calcpath_data.rs` by `extract_calcpath.py`); `calc_path` is the
original's step-wise weighted walk. Full detail in §10.3.

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
| ~~MEDIUM~~ resolved | Scene transitions | **The original does a HARD CUT** (single `StretchBlt`/`BitBlt`, `seg8:06ce`) — so our hard cut is **faithful, not a gap** (§10.2). jc_reborn's `grFadeOut()` wipe is the reimplementation's own embellishment. The original *does* carry a **dormant** LFSR dissolve (dead code, `[0x1ebf]≡0`) — the basis for our opt-in "dissolve" transition. |
| ~~LOW~~ done | Unused intro/end screens | `INTRO.SCR` is now displayed once at startup (`Show::enable_intro`, default-on `intro` config / `--no-intro`, matching the original's `Introduction` toggle). `THEEND.SCR` still only appears as the day-11 TTM, not as a standalone closing sequence (like jc_reborn) — a minor presentation difference. |
| LOW | `0x0080` DRAW_BACKGROUND = **undocumented** no-op | Handled correctly by the catch-all (like jc_reborn's stub, which says "no-op; frees slots"), but **190× across 36 files** and **castaway/dgds-viewer/JCOS treat it as a background redraw**. If one day a visual bug appears, it is the #1 divergence point to investigate (disassemble the original's `0x0080` handler). |
| LOW | `0xA054` SAVE_ZONE = no-op | While the pair `0xA064` RESTORE_ZONE is implemented. Faithful to jc_reborn (there it is also a near-stub); used 1× (GJGULIVR.TTM). No known visible defect. |
| (soft) | holiday/drift/scheduling **boundary constants** | Not located as immediates (§2.3) — jc_reborn's observational RE, faithfully ported (behaviour matches the bible). *(`calcpath` is no longer here — its route table was **found**, §2.2/§10.3.)* |

## 7. Verdict

- **On the original DATA: high confidence that nothing is missing.** 100% of the TTM
  (30/30) and ADS (18/18) opcodes used are handled; 179/179 resources parse; 66/66 scenes
  build; the 11-day arc, day-beats, easter eggs, 23 sounds and the 4 holidays (= the maximum that
  the data allows) are present and covered by tests. **The bible's "July 4" pending item
  is resolved (absent from the original).**
- **On the observational LOGIC:** `walk_data` is **byte-perfect**, and the **`calcpath` route
  table was found** in the binary (§10.3) — so the routing *mechanism* is now binary-confirmed
  (waypoint bytes pending a future check). Only the **holiday/drift boundary constants** remain
  unproven (not co-located immediates; behaviour matches the bible). The full **capstone
  disassembly** (§9–§10) took this well beyond the earlier `objdump`-only reach.
- **Actionable parity improvements** (revised by the disassembly, §9–§10): the only real
  binary-confirmed gap was the **intro** (a real resource with an `Introduction` toggle) —
  now **implemented** (`Show::enable_intro`, default-on `intro` config / `--no-intro`). The **MCI**
  audio path is **dead code** (§9.4) — not a gap. **Scene transitions are a hard cut** in the
  original (§10.2) — our hard cut is faithful; the dormant LFSR dissolve we found is the basis for
  an *opt-in* effect. The **animation rate is 16 ms/tick** (§10.1), now adopted.

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
- **Loop/time → RESOLVED: 16 ms/tick** (see §10.1). `SetTimer(…, 50 ms, …)` pumps `WM_TIMER`
  (0x0113), but the advance is **paced by real time** (`GetCurrentTime`): a `seg9` scheduler derives
  a **4 ms** master unit (`1000 / (13 × 18)` → `[0x45de]`, the ~18.2 Hz PC-timer constant) and the
  animation callback runs every 4th one ⇒ **16 ms/tick** (~62.5 Hz), frame-rate-independent. (The
  earlier suspect `[0x2e14]` was a red herring — statically 0, dead code.) jc_reborn approximates
  20 ms/tick; **we now use the original's 16 ms** (`wilson_engine::MS_PER_TICK`). This was the last
  open timing number — **closed.**
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
- **Fade/wipe between scenes → RESOLVED: the shipped original does a HARD CUT** (see §10.2). Every
  scene change and the intro present through one primitive (`seg8:06ce` = a single
  `StretchBlt`/`BitBlt`), no fade/wipe. So **our hard cut is faithful** — not a gap. *Discovery:* the
  binary **does** contain a transition effect — a random-order **LFSR tiled dissolve** (`seg12:198a`,
  tap-mask table at `seg14:0x27fe`) — but it is **dead code**, gated by `[0x1ebf]` which is statically
  0 (and its blit fn-ptrs are null). It was compiled in but disabled. (This is the basis for our
  opt-in "dissolve" transition — resurrecting the original's own effect, not jc_reborn's wipe.)
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
**implemented**. **MCI** was **resolved as dead code** (§9.4), not a gap. The remaining open items were
then **closed by the 2026-06-18 deep dive (§10):** the **exact time rate = 16 ms/tick** (§10.1),
**scene transitions = hard cut** (and a dormant LFSR dissolve discovered, §10.2), and **`calcpath` is
a real route table** at `seg14:0x0A94` (§10.3). Only the holiday/drift *boundary constants* remain
untraced (behaviour matches the bible; faithful to jc_reborn, without per-byte proof).

---

## 10. Deep dive (2026-06-18) — three open questions resolved

A second disassembly pass to close the remaining unknowns, each re-derived directly from the
original binary (tools in [`../reverse-engineering/`](../reverse-engineering/README.md); raw
listing kept local). Addresses are `segN:offset` (CODE) / `seg14` (DATA; file = offset + 0x17E00).

### 10.1 Animation timing = **16 ms/tick** (was the last open number)
The pacing is **not** in the `[0x2e14]` helper (that is statically 0 / dead). It lives in a small
`seg9` timer scheduler. The rate is **hardcoded** (no `SCRANTIC.INI` override):
- `seg9:076a` is called once with the literal **13** (`seg2:150a mov ax,0xd`). It computes
  `1000 / (13 × 18)` — `imul dx(=0x12)` then `idiv` of `0x3e8(=1000)` — and stores **`[0x45de] = 4` ms**
  (`seg9:0791`). The `18` is the classic ~18.2 Hz PC-timer constant.
- The animation callback is registered with multiplier **4** (`seg2:1516 mov ax,4`); the scheduler does
  `[0x45de] × 4` (`seg9:083d imul`) ⇒ **entry interval = 16 ms** (`seg9:0843`).
- The pump (`seg9:089a`, from the `WM_TIMER` handler; `SetTimer` period = `0x32` = 50 ms) fires each
  entry while its 32-bit next-fire stamp `≤ GetCurrentTime`, advancing `+16` ⇒ **frame-rate-independent
  16 ms/tick** (~62.5 Hz). A TTM `wait N` is therefore `N × 16 ms`.

⇒ We adopt **`wilson_engine::MS_PER_TICK = 16`** (was 20, jc_reborn's approximation). Confidence: **HIGH**
(single call site, literal constants, full math verified). Note: `idiv` truncates `1000/234 = 4`, so the
unit is exactly 16 ms with sub-frame jitter absorbed by gating on real time.

### 10.2 Scene transitions = **hard cut** (+ a dormant LFSR dissolve)
The universal present primitive **`seg8:06ce`** (called from seg2/3/5/11/12/13) rounds the rect to 8-px
and does **one** `StretchBlt` (2× scale, ROP SRCCOPY) or `BitBlt` (1×) from the offscreen buffer to the
screen — **no band loop, no per-frame reveal**. `INTRO.SCR` is presented the same way (`seg2:14e4`). So
**every scene change and the intro are hard cuts** — our behaviour is faithful. *Intro timing:* at
`seg2:1502` the original blits `INTRO.SCR` to the screen and **the very next instruction** (`seg2:150a`)
starts the 16 ms timer and enters the loop — there is **no intro hold timer**; the title simply stays
up until the first scene is drawn (on 1992 disk hardware that load took a few seconds). So a fixed hold
is our approximation; the desktop now gates engine-advance on a per-frame deadline (`pace::FramePacer`)
so spurious OS redraws can't make the intro/first frames flash by (regression fixed 2026-06-18).
- *Discovery:* a transition effect **exists but is disabled**. `seg12:198a` branches on `[0x1ebf]`
  (`seg12:19c7`): if 0 → plain `seg8:06ce`; if ≠0 → a **random-order tiled dissolve** driven by an LFSR
  (`shr`/`xor`), with per-cell blit fn-ptrs `[0x40b0]/[0x40b2]` and a per-frame timing gate. The LFSR
  **tap-mask table** at `seg14:0x27fe` = `3, 6, C, 14, 30, 60, B8, 110, 240, 500` (textbook maximal-length
  feedback masks for 2–11-bit registers). But `[0x1ebf]` is **statically 0 with zero writes anywhere**, and
  the fn-ptrs are null — so the dissolve **never runs** (likely an authoring/debug feature left compiled-in).
  Confidence: **HIGH**. ⇒ This is the basis for our **opt-in "dissolve" transition**: resurrecting the
  original's own effect (default off = faithful hard cut).
  - *Re-verified (2026-06-18) by a **whole-binary** byte scan* (not just the 74.5% recursive disasm): the
    word `0x1ebf` appears in **10 instructions — all reads** (9× `cmp byte [0x1ebf], 0` gating the dissolve
    in `seg7`/`seg12`, 1× `mov al,[0x1ebf]`); **0 writes** (no `a2/a3/c6 06/c7 06/88 06/89 06 …`). So the
    gate is provably never set ⇒ the dissolve is dead **everywhere, including leaving the intro** (`INTRO.SCR`
    hard-cuts into the first scene). A user's memory of "the intro always dissolved" is the *coded-but-disabled*
    effect, not the shipped behaviour. (We expose it **opt-in**, and `--transition dissolve`/`?dissolve` now
    also covers the intro→first-scene boundary.)

### 10.3 `calcpath` = a real **6×6 word-pointer route table**
Routing is **table-driven** (not geometric). Lookup `seg4:03ac`, called as `route([0x30e0]=start,
[0x30e2]=dest)`: `route_ptr = *(word*)(0x0A94 + start*12 + dest*2)` (`seg4:03c7 mov dx,0xc` row stride 12,
`shl dx,1` for dest, base `0x0A94`). Indices are **1-based** (0 = invalid → why our reimpl uses `[7]`).
The **route-pointer table** at `seg14:0x0A94` (file `0x18894`), 6×6 words, diagonal = 0:
```
        d1     d2     d3     d4     d5     d6
 s1: 0000   0362   03a4   03ea   042c   046a
 s2: 04b0   0000   04ea   0528   0566   05a0
 s3: 05de   0620   0000   065a   0698   06d2
 s4: 0714   0752   078c   0000   07c6   0800
 s5: 083a   087c   08ba   08f8   0000   0936
 s6: 0970   09b2   09f0   0a32   0a6c   0000
```
Each entry points into a **route stream** (`seg14:0x0362–0x0AA2`) of marker-delimited (`-1…-7`) `(value,
value)` waypoint pairs consumed by the walk animator; per-spot 3-byte records are at `seg14:0x02DE`, and
anim-base tables at `seg14:0x0347`/`0x034E`. There is **no single 252-entry contiguous matrix** (which is
why §2.2's earlier scan missed it).

**Byte-check + port (done).** Each section is a list of **`(value, weight)`** pairs; a `value` →
`(start_hdg, end_hdg, next_spot)` at `0x02DE + value*3`, so a section is the weighted next-spot
choices for that cursor. The selection is a **step-wise weighted walk**: at the current spot, roll
against its section (weights, summed per section — usually 100, but at least one section, s1→d5
cursor 1, sums to 150), move to the chosen `next_spot`, repeat to the destination. Compared to our
old `WALK_MATRIX` (jc_reborn's second-order *unweighted* reconstruction), the original **diverged on
all 30 pairs**; the clearest structural error was a spurious direct **5→3** hop (the original always
detours via 4). We **ported it faithfully**: `extract_calcpath.py` decodes the 30 streams into
`calcpath_data.rs` (`ROUTE_STREAMS`), and `path.rs::calc_path` is the weighted walk. *(Aside: this
exposed a latent RNG bug — `Rng::new` left xorshift64's high bits unmixed for tiny seeds, so the
first draw was always 0; now splitmix64-scrambled.)* Confidence: **HIGH** (routes match the
disassembly; the engine runs the real data clean with the new routing).
