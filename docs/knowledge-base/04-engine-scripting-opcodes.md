# 04 — Scripting Engine: TTM and ADS Opcodes

> Johnny Castaway's content is **bytecode-driven**. There are two levels:
> - **TTM** (*"Tableau"/Text-Tableau Movie*) = **per-scene animation** bytecode
>   (draws sprites/primitives, sets delays, plays sound). It lives in the `TT3:` chunk of a
>   `.TTM`.
> - **ADS** (*Animation/Director Script*) = **scene sequencing** bytecode —
>   decides *which* TTMs to play, with conditionals and randomness. It lives in the `SCR:`
>   chunk of an `.ADS`.
>
> Faithfully reproducing these two interpreters = reproducing ~95% of the behavior.
> This is the "crown jewel" of the reverse engineering.
>
> **Primary source:** `jc_reborn` (`ttm.c`, `ads.c`, `dump.c`) — the most complete and
> JC-specific. **Cross-references:** JCOS (`Instruction.cs` has the exact operands
> via a disassembler), castaway/dgds-viewer (`process.*`), ScummVM (`dgds.cpp`).
> Complete tables: [`raw/jc_reborn-notes.md`](raw/jc_reborn-notes.md) §3–§4 and
> [`raw/jcos-csharp-notes.md`](raw/jcos-csharp-notes.md).

---

## 1. Instruction encoding

### TTM (`ttm.c:141`)
```
u16 opcode (LE)
numArgs = opcode & 0x000F          ; number of int16 operands
op      = opcode & 0xFFF0          ; the opcode itself (low nibble zeroed)
IF numArgs == 0x0F:                ; special case: 1 STRING operand
    reads byte pairs (UTF-16-ish) until the 00 00 pair (ASCIIZ, even padding)
ELSE:
    reads numArgs × int16 (LE)
```
**Golden rule:** the low nibble of the opcode is the **argument count**; `0xF`
means "one string argument" (used by `LOAD_*`).

### ADS (`ads.c`)
```
u16 opcode (LE)
IF (opcode & 0xFF00) == 0:         ; small value
    it is a TAG/ID (push of a scene/sequence id)
ELSE:
    it is a 16-bit opcode; the number of operands is FIXED per opcode (does not come from the nibble)
```

---

## 2. TTM opcode table

As implemented in `jc_reborn` (column "Effect"). `[dump-only]` = recognized by the
disassembler but **without a runtime handler** in jc_reborn (probably irrelevant for
JC, but it exists in the data). The hex already includes the count nibble.

| Opcode | Mnemonic | Args | Effect |
|---|---|---:|---|
| `0x001F` | SAVE_BACKGROUND | str | [dump-only] |
| `0x0080` | DRAW_BACKGROUND | 0 | stub (frees image slots) |
| `0x0110` | PURGE | 0 | end/loop control: if `sceneTimer` active → jumps to the previous tag; otherwise marks the thread as finished |
| `0x0FF0` | **UPDATE** | 0 | **yield**: ends the current step, presents the frame |
| `0x1021` | **SET_DELAY** | 1 | `timer = delay = max(arg, 4)` — frame delay in **ticks** |
| `0x1051` | SET_BMP_SLOT | 1 | selects the spritesheet slot for the next draws |
| `0x1061` | SET_PALETTE_SLOT | 1 | recognized, no-op (single global palette) |
| `0x1101` | LOCAL_TAG | 1 | tag marker (bookmark at load) |
| `0x1111` | TAG | 1 | scene/label marker (bookmark at load) |
| `0x1121` | TTM_UNKNOWN_1 | 1 | defines a region used before SAVE_IMAGE1/CLEAR_SCREEN; no-op |
| `0x1201` | **GOTO_TAG** | 1 | `nextGotoOffset = ttmFindTag(arg)` (deferred jump) → loops |
| `0x2002` | SET_COLORS | 2 | `fgColor=arg0; bgColor=arg1` |
| `0x2012` | SET_FRAME1 | 2 | always (0,0) near LOAD_IMAGE; no-op |
| `0x2022` | TIMER | 2 | `delay = timer = (arg0+arg1)/2` (approximation) |
| `0x4004` | SET_CLIP_ZONE | 4 | `grSetClipZone(x1,y1,x2,y2)` |
| `0x4110` | FADE_OUT | 0 | [dump-only] |
| `0x4120` | FADE_IN | 0 | [dump-only] |
| `0x4204` | COPY_ZONE_TO_BG | 4 | blits a region onto the persistent "saved zones" layer |
| `0x4214` | SAVE_IMAGE1 | 4 | defines a redraw zone (effectively no-op) |
| `0xA002` | DRAW_PIXEL | 2 | `grDrawPixel(x,y,fgColor)` |
| `0xA054` | SAVE_ZONE | 4 | `grSaveZone` (GJGULIVR.TTM only) |
| `0xA064` | RESTORE_ZONE | 4 | `grRestoreZone` |
| `0xA0A4` | DRAW_LINE | 4 | Bresenham line `(x1,y1,x2,y2,fgColor)` |
| `0xA104` | DRAW_RECT | 4 | filled rectangle `(x,y,w,h,fgColor)` |
| `0xA404` | DRAW_CIRCLE | 4 | circle `(x,y,w,h,fg,bg)` |
| `0xA504` | **DRAW_SPRITE** | 4 | `grDrawSprite(x,y,spriteNo,imageNo)` |
| `0xA510` | DRAW_SPRITE1 | 0 | [dump-only] |
| `0xA524` | **DRAW_SPRITE_FLIP** | 4 | horizontally mirrored sprite |
| `0xA530` | DRAW_SPRITE3 | 0 | [dump-only] |
| `0xA601` | CLEAR_SCREEN | 1 | clears this thread's layer |
| `0xB606` | DRAW_SCREEN | 6 | recognized, no-op |
| `0xC020` | LOAD_SAMPLE | 0 | [dump-only] |
| `0xC030` | SELECT_SAMPLE | 0 | [dump-only] |
| `0xC040` | DESELECT_SAMPLE | 0 | [dump-only] |
| `0xC051` | **PLAY_SAMPLE** | 1 | `soundPlay(arg0)` |
| `0xC060` | STOP_SAMPLE | 0 | [dump-only] |
| `0xF01F` | **LOAD_SCREEN** | str | loads a `.SCR` as background |
| `0xF02F` | **LOAD_IMAGE** | str | loads a `.BMP` into the selected slot |
| `0xF05F` | LOAD_PALETTE | str | recognized, no-op (global palette already loaded) |

### How an animation runs
A "scene" is an entry point (tag) within the TTM. Each step of `ttmPlay()`
typically: `CLEAR_SCREEN` on the thread's transparent layer → draws sprites/primitives
of **one frame** → `UPDATE` (yield). The scheduler waits `delay` ticks and re-enters;
`GOTO_TAG`/`PURGE` create loops. The final composition is layer over layer
(background → saved zones → each thread → holiday layer).

---

## 3. ADS opcode table

From `jc_reborn` (`ads.c` runtime + `dump.c` names). Args in 16-bit words.

| Opcode | Mnemonic | Args | Meaning |
|---|---|---:|---|
| `0x1070` | IF_LASTPLAYED_LOCAL | 2 | "if last played (slot,tag)" local; enqueues a local chunk (ACTIVITY.ADS tag 7 only) |
| `0x1330` | IF_UNKNOWN_1 | 2 | guard, synonym of IF_NOT_RUNNING; ignored at runtime |
| `0x1350` | **IF_LASTPLAYED** | 2 | **reactive trigger**: the chunk runs when scene (slot,tag) ends |
| `0x1360` | IF_NOT_RUNNING | 2 | if (slot,tag) running → skips the block |
| `0x1370` | IF_IS_RUNNING | 2 | skips the block unless (slot,tag) is running |
| `0x1420` | AND | 0 | boolean AND of conditions |
| `0x1430` | OR | 0 | boolean OR (`inOrBlock`) |
| `0x1510` | PLAY_SCENE | 0 | "closing brace" of a conditional block |
| `0x1520` | ADD_SCENE_LOCAL | 5 | adds a scene enqueued by a local trigger |
| `0x2005` | **ADD_SCENE** | 4 | **creates a TTM thread** (slot,tag,arg3,?). In a RANDOM block → weighted candidate |
| `0x2010` | STOP_SCENE | 3 | stops a scene by (slot,tag) |
| `0x2014` | UNKNOWN_5 | 0 | recognized by the dumper |
| `0x3010` | **RANDOM_START** | 0 | start of a weighted random selection block |
| `0x3020` | NOP | 1 | "do nothing" candidate (the weight is the arg) |
| `0x30FF` | **RANDOM_END** | 0 | picks and executes **one** candidate by weight |
| `0x4000` | UNKNOWN_6 | 3 | BUILDING.ADS tag 7 only; ignored |
| `0xF010` | FADE_OUT | 0 | recognized, no-op at runtime |
| `0xF200` | GOSUB_TAG | 1 | calls another tag's chunk inline (e.g. STAND.ADS→tag 14) |
| `0xFFFF` | **END** | 0 | end of sequence → requests stop |
| `0xFFF0` | END_IF | 0 | closes an IF block |
| (other) | `:TAG n` | 0 | any other value = tag/label id |

### `ADD_SCENE` semantics (arg3) — **load-bearing**
- `arg3 < 0` → plays **for `-arg3` ticks** (`sceneTimer`);
- `arg3 > 0` → plays **`arg3` times** (`sceneIterations`);
- `arg3 == 0` → plays **once** until natural end.

### ADS execution model (reactive chaining)
1. `adsLoad()` pre-scans the script: marks tags and the **trigger chunks**
   (`IF_LASTPLAYED`/`IF_NOT_RUNNING`).
2. Plays the initial chunk → `ADD_SCENE` creates TTM threads.
3. When a TTM thread **finishes**, `adsPlayTriggeredChunks()` fires any chunk
   whose `IF_LASTPLAYED (slot,tag)` matches — **this is how one animation chains to the
   next**.
4. `RANDOM_START … RANDOM_END` picks one operation by **weight** (sums the weights,
   `rand()%total`, walks the cumulative distribution) — it is the basis of "Johnny chooses
   randomly what to do".

---

## 4. Divergences between implementations (watch out when porting)

The reimplementations do not always agree (they were independent reverse engineering). Where
they diverge, **prefer `jc_reborn`** (JC-specific) and validate by running against the data:

| Topic | jc_reborn (JC) | ScummVM (RotD/HoC) | JCOS / castaway |
|---|---|---|---|
| DELAY unit | tick = **20 ms** (`events.c:108`); `SET_DELAY` in ticks | `0x1020`: `delay += arg*10` ms | JCOS: ×20 ms |
| `0xA100` | **DRAW_RECT** (filled) | "SET (bmp) WINDOW" | castaway: DRAW_RECT; JCOS: SET_WINDOW0 |
| `0x4000` (TTM) | SET_CLIP_ZONE | SET WINDOW | castaway: SET_CLIP_REGION |
| Nibble = count | yes | yes | yes (JCOS masks `& 0xfff0`) |
| LZW: table size | 4096 | allocates 16384 (real cap 12 bits=4096) | JCOS: 4096 |
| ADS opcodes implemented | almost all | only `0x2005` (rest stub) | most |
| TTM×ADS opcode collisions | resolved by context | — | castaway: first match; viewer: last-key |

> **Real collisions:** `0x2010`, `0x4000`, `0xF010` mean different things in TTM
> vs ADS. The interpreter must dispatch according to the **type of script** in effect.

> **JCOS as a dictionary:** JCOS's `Instruction.cs` has a `ToString()` that spells out the
> operands of ~60 opcodes — it is the best reference for **names and operand tuples**.
> Sprites: JCOS confirms `0xA500/0xA520 = DRAW_SPRITE/DRAW_SPRITE_FLIP` (the `2` = mirrored)
> and that `DRAW_SPRITE1/3` (`0xA510/0xA530`) throw "not implemented".

---

## 5. Implications for Wilson Reborn

1. **Transcribe both VMs** (TTM and ADS) with the tables above. They are small and well
   understood.
2. **Preserve the load-bearing conventions:** the sign of ADD_SCENE's `arg3`; the tri-state
   `isRunning` (0 free / 1 running / 2 finished-pending / 3 static-background); nibble =
   count; `0xF`=string.
3. **Dispatch by context** (TTM vs ADS) to resolve opcode collisions.
4. **Standardize the tick at 20 ms** and the **cooperative variable-timestep scheduler**
   (see [05](05-arquitetura-do-engine.md) §loop).
5. **Possible modernization:** the `[dump-only]` opcodes (FADE_IN/OUT, sound C0xx,
   SAVE_BACKGROUND) can be **actually implemented** in Wilson Reborn for extra fidelity,
   since modern hardware allows it (jc_reborn ignored them for simplicity).
