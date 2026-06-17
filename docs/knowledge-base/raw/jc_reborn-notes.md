# jc_reborn — Deep Technical Notes (reference implementation)

Source of truth for **Wilson Reborn**. `jc_reborn` ("Johnny Reborn") is an open-source C/SDL2
re-implementation of the 1992 Sierra/Dynamix screensaver **Johnny Castaway** (engine internally
called **SCRANTIC**, resources in the Dynamix **DGDS** family). It loads the *original* game data
files (`RESOURCE.MAP` + `RESOURCE.001`) and reproduces the behaviour by interpreting the original
TTM (animation) and ADS (sequencing) bytecode.

- Author: Jeremie GUILLAUME, 2019. License: GPL v3 (`LICENSE`).
- Language: C99, single library dependency `-lSDL2`. Builds on Linux (`Makefile.linux`) and
  Windows/MinGW (`Makefile.MinGW`).
- Status (per `README.md`): "work in progress" but every scene works; walking, scene-scheduling,
  island-drawing algorithms were reverse-engineered by observation, not by disassembly, so they are
  approximate. Predecessor project JCOS (Hans Milling / nivs1978) decoded the file formats and many
  opcodes; jc_reborn added near-complete opcode semantics + walking + scheduler + island rendering.

All file:line references below point into the jc_reborn upstream
(<https://github.com/jno6809/jc_reborn>) — formerly vendored under `repos/jc_reborn/`.

---

## 0. File inventory & build

Translation units compiled (both Makefiles, identical OBJ list):
`jc_reborn.o utils.o uncompress.o resource.o dump.o story.o walk.o calcpath.o ads.o ttm.o island.o
bench.o graphics.o sound.o events.o config.o`.

Not in the build (standalone helper): `extract_walk_data.c` (one-off generator for `walk_data.h`).
Headers with embedded static data tables: `story_data.h`, `walk_data.h`, `calcpath_data.h`.
`misc/adsbeautifier.awk` is a pretty-printer for dumped ADS scripts.

`mytypes.h` defines `uint8/16/32` and `sint8/16/32` over `<stdint.h>`.

---

## 1. Architecture, main loop, timing/frame model

### Entry point — `jc_reborn.c`
`main()` (`jc_reborn.c:152`): `parseArgs()` then **always** `parseResourceFiles("RESOURCE.MAP")`
(line 159), then dispatches on the chosen mode:
- default (no verb) → `argPlayAll`: `graphicsInit(); soundInit(); storyPlay(); ...` (the real
  screensaver, infinite loop).
- `dump` → `dumpAllResources()` (extracts every resource to `./dump/`).
- `bench` → `adsPlayBench()` (fps benchmark).
- `ttm <name>` → `adsPlaySingleTtm(name)` (play one TTM directly).
- `ads <name> <tag>` → `adsInitIsland()` or `adsNoIsland()` then `adsPlay(name, tag)`.

CLI verbs/options parsed in `parseArgs()` (`jc_reborn.c:92`): verbs `help version dump bench ttm
ads`; options set globals: `window`→`grWindowed`, `nosound`→`soundDisabled`, `island`→`argIsland`,
`debug`→`debugMode`, `hotkeys`→`evHotKeysEnabled`. Usage text at `jc_reborn.c:51` documents the
while-playing hot-keys: Esc=quit, Alt+Return=toggle fullscreen, Space=pause, Return=step one frame
when paused, M=toggle max/normal speed.

### The timing model — `events.c`
This is the heartbeat. Everything is measured in **"ticks"**; 1 tick = **20 ms** (`eventsWaitTick`
multiplies `delay *= 20`, `events.c:108`). So the engine's nominal rate is 50 ticks/sec, and a
"delay" value N in a TTM means N×20 ms.

`eventsWaitTick(uint16 delay)` (`events.c:106`): busy-waits (with `SDL_Delay(5)` granularity) until
`SDL_GetTicks() - lastTicks >= delay*20`, while polling SDL events. Honours `paused` /`oneFrame`
(step) and `maxSpeed` (M key bypasses the wait). `eventsProcessEvents()` (`events.c:41`) — if
hot-keys disabled, **any** keypress quits (`exit(255)`) like a real screensaver; if enabled it
handles Space/M/Alt+Return/Return/Esc.

`grUpdateDelay` (global) carries the number of ticks the next `grUpdateDisplay()` should wait. The
display refresh (`graphics.c:175`) calls `eventsWaitTick(grUpdateDelay)` just before
`SDL_UpdateWindowSurface`.

### The ADS main loop — `adsPlay()` (`ads.c:658`), the central scheduler
This is the engine's real main loop while a scene plays. After loading TTMs and the chosen ADS
chunk, `while (numThreads)`:
1. If the background (waves) thread is due (`timer==0`), animate the island shore
   (`islandAnimate`).
2. For each of `MAX_TTM_THREADS` (10) TTM threads whose `timer==0`, call `ttmPlay()`.
3. `grUpdateDisplay(...)` composites + waits.
4. Compute `mini` = minimum of all running threads' `delay`/`timer` (capped at 300); decrement every
   thread's `timer` by `mini`; set `grUpdateDelay = mini`. This is a **cooperative, event-driven,
   variable-timestep** scheduler: it sleeps exactly until the next thread needs servicing rather
   than ticking a fixed frame.
5. Per-thread post-processing: apply pending `nextGotoOffset` jumps; decrement `sceneTimer`
   (ADD_SCENE negative arg = play-for-duration); when a thread ends (`isRunning==2`) either re-arm
   it `sceneIterations` times (positive ADD_SCENE arg = play-N-times) or stop it and fire any
   `IF_LASTPLAYED`/`IF_NOT_RUNNING` chunks triggered by its termination (`adsPlayTriggeredChunks`).

`isRunning` tri-state: `0`=free slot, `1`=running, `2`=finished-this-pass (pending cleanup);
background/holiday threads use `3` = "running but not a normal TTM script" (drawn, not stepped).

### bench.c / adsPlayBench()
`benchInit()` (`bench.c:29`) loads `OCEAN00.SCR` + `BOAT.BMP`. `benchPlay()` draws a moving boat
sprite per layer. `adsPlayBench()` (`ads.c:807`) runs 1-, 4-, then 8-layer tests for 3 s each
(`SDL_GetTicks`) and prints fps — purely a rendering-throughput micro-benchmark.

---

## 2. Resource system — `resource.c` / `resource.h`, decompression `uncompress.c`

### Original data files referenced (exhaustive)
Loaded/looked-up by name anywhere in the code:
- `RESOURCE.MAP` — index file (passed to `parseResourceFiles`, `jc_reborn.c:159`).
- `RESOURCE.001` — the main archive; its name is **read from inside** `RESOURCE.MAP`
  (`mapFile.resFileName`, a 13-byte string at `resource.c:358`), not hardcoded.
- `FILES.VIN` — present in the archive as a `.VIN` resource; deliberately **ignored** (just a file
  list), `resource.c:427`.
- `SCRANTIC.SCR` — the original program/overlay; **not** in `RESOURCE.001`. Read only by the
  standalone `extract_walk_data.c` to extract Johnny's walk animation table (see §9).
- Sound: `sound0.wav` … `sound24.wav` (some indices missing) loaded at runtime by `sound.c`
  (`sprintf("sound%d.wav", i)`), index 11/13/etc absent — see README md5 table.

Resource **names referenced** (these live *inside* `RESOURCE.001`, found via `findXxxResource`):
- `.SCR` (screens): `INTRO.SCR` (`ads.c:857`), `OCEAN00.SCR`/`OCEAN01.SCR`/`OCEAN02.SCR`
  (`island.c:45` `"OCEAN0%d.SCR"`, rand%3; `bench.c:31`), `NIGHT.SCR` (`island.c:41`).
- `.BMP` (sprite sheets): `BOAT.BMP` (`bench.c:32`), `JOHNWALK.BMP` (`ads.c:915`),
  `JOHNNYR.BMP`*? no — walking uses `JOHNWALK.BMP`*, `MRAFT.BMP` (`island.c:57`), `BACKGRND.BMP`
  (`island.c:71`), `HOLIDAY.BMP` (`island.c:204`). All other `.BMP`/`.SCR`/`.PAL` names come
  dynamically from TTM `LOAD_IMAGE`/`LOAD_SCREEN`/`LOAD_PALETTE` opcodes inside the data.
- `.ADS` (scene scripts): `ACTIVITY BUILDING FISHING JOHNNY MARY MISCGAG STAND SUZY VISITOR
  WALKSTUF` (all `.ADS`, enumerated in `story_data.h`).
- `.TTM` names: not hardcoded — each `.ADS` resource has a `RES:` table mapping slot id → `.TTM`
  filename, loaded by `adsPlay()` (`ads.c:671`). Examples named in comments: `GFFFOOD.TTM`,
  `WOULDBE.TTM`, `GJGULIVR.TTM`, `SASKDATE.TTM`, `GJIVS6.TTM`, `SASKDATE.TTM`, `STAND.TTM`(`STAND.ADS`).
- `.PAL`: exactly one palette resource is kept (`MAX_PAL_RESOURCES 1`); `palResources[0]` is the
  global 16-colour palette used everywhere (`graphics.c:137`).

### RESOURCE.MAP layout — `parseMapFile()` (`resource.c:342`)
6 unknown bytes, then a 13-byte string = the data filename (`resFileName`), then `uint16
numEntries`, then `numEntries × { uint32 length; uint32 offset }`. Struct `TMapFile` /
`TMapFileEntry` (`resource.h:28`).

### RESOURCE.001 layout — `parseResourceFile()` (`resource.c:373`)
For each map entry: `fseek(offset)`, read 13-byte `resName` + `uint32 resSize`. The **last 4 chars**
of the name give the type (`.ADS .BMP .PAL .SCR .TTM`), dispatched to a per-type parser. Caps:
`MAX_ADS 100, BMP 200, PAL 1, SCR 20, TTM 100`. Global arrays `adsResources[] bmpResources[]
palResources[] scrResources[] ttmResources[]` + counts. Lookups: `findAdsResource / findBmpResource
/ findScrResource / findTtmResource` (linear `strcmp`; fatal if missing).

### Per-type container formats (all chunk-tagged ASCII 4-byte markers)
- **ADS** `parseAdsResource` (`resource.c:54`): `VER:`+size+5-byte version; `ADS:`+4 unknown;
  `RES:`+size+`uint16 numRes`+ `numRes×{uint16 id, char name[≤40]}`; `SCR:`+ compressed script
  block (`compressedSize = size-5`, `uint8 compressionMethod`, `uint32 uncompressedSize`, then
  `uncompress()`); `TAG:`+size+`uint16 numTags`+`numTags×{uint16 id, char desc[≤40]}`.
- **BMP** `parseBmpResource` (`resource.c:134`): `BMP:`+`uint16 width,height`; `INF:`+size+`uint16
  numImages`+`uint16 widths[numImages]`+`uint16 heights[numImages]`; `BIN:`+compressed pixel block.
  Pixels are 4-bpp packed (2 px/byte), one concatenated stream split per-image by widths/heights.
- **PAL** `parsePalResource` (`resource.c:183`): `PAL:`+size+2 unknown; `VGA:`+4 bytes; then
  **256** `{r,g,b}` triples (6-bit VGA values 0..63). Only first 16 entries are used.
- **SCR** `parseScrResource` (`resource.c:222`): `SCR:`+totalSize+flags; `DIM:`+size+`uint16
  width,height`; `BIN:`+compressed 4-bpp full-screen image.
- **TTM** `parseTtmResource` (`resource.c:269`): `VER:`+version; `PAG:`+`uint32 numPages`+2 unknown;
  `TT3:`+compressed bytecode block; `TTI:`+4 unknown; `TAG:`+`numTags×{uint16 id, char desc[≤40]}`.
  Tags name the "scenes" (entry points) inside the TTM.

### Decompression — `uncompress.c`
`uncompress(f, method, inSize, outSize)` (`uncompress.c:218`) switches on `compressionMethod`:
- **1 = RLE** `uncompressRLE` (`uncompress.c:180`): read control byte; if high bit set
  (`& 0x80`), next byte is repeated `(control & 0x7F)` times; else copy the next `control` literal
  bytes.
- **2 = LZW** `uncompressLZW` (`uncompress.c:77`): classic variable-width LZW. 4096-entry code
  table, codes start at 9 bits (`n_bits=9`), grow to 12; `free_entry` starts 257; **code 256 = clear
  /reset** (re-aligns the bit stream: skips to next `n_bits<<3` boundary, resets to 9 bits,
  `free_entry=256`). Output bounded by `outSize`. `getBits()` reads LSB-first within each byte. Any
  other method → `NULL`.

These two codecs + the chunk-tag container are the entirety of the DGDS format needed.

---

## 3. TTM interpreter — `ttm.c` / `ttm.h`

TTM = "TTM script" = the per-scene animation bytecode (the `TT3:` block). It is a stream of 16-bit
little-endian **opcodes**; the **low nibble of the opcode is the argument count** in 16-bit words
(`numArgs = opcode & 0x000F`), EXCEPT `0x0F` which means "one NUL-terminated string argument, padded
to an even byte count" (`ttm.c:164`, mirrored in `dump.c:313` and the pre-pass in `ttmLoadTtm`).

### Loading & tag bookmarking — `ttmLoadTtm()` (`ttm.c:72`)
Stores the uncompressed bytecode in a `TTtmSlot`, then walks it once to bookmark every tag. Opcodes
`0x1111` (TAG) and `0x1101` (LOCAL_TAG) record `{id=arg, offset}` so `ttmFindTag()` (`ttm.c:52`) can
jump by tag id. Argument skipping uses the same low-nibble rule. `TTtmSlot` (`graphics.h:42`) holds
`data, dataSize, tags[], numTags`, plus up to `MAX_BMP_SLOTS`(6) loaded sprite sheets, each up to
`MAX_SPRITES_PER_BMP`(120) `SDL_Surface*`.

### Execution — `ttmPlay()` (`ttm.c:141`)
Runs from `ttmThread->ip` and executes opcodes until an `UPDATE` (`0x0FF0`) yields, or end-of-data.
Sets `grDx/grDy = ttmDx/ttmDy` (the scene's island offset) at entry so all coordinates are
island-relative. Per-thread render state lives in `TTtmThread` (`graphics.h:56`): `selectedBmpSlot,
fgColor, bgColor, delay, timer, sceneTimer, sceneIterations, nextGotoOffset, ttmLayer`(its own
SDL surface).

#### Complete TTM opcode table (as handled by jc_reborn)
`ttmPlay()` implements the following (hex, mnemonic, args, effect). Opcodes the dumper knows but the
player does not act on are marked **[dump-only]** (recognised by `dump.c` but no runtime handler).

| Opcode | Mnemonic | Args | Effect (jc_reborn) |
|--------|----------|------|--------------------|
| `0x001F` | SAVE_BACKGROUND | str? | **[dump-only]** (`dump.c:333`) |
| `0x0080` | DRAW_BACKGROUND | 0 | no-op stub; comment: frees image slots (`ttm.c:184`) |
| `0x0110` | PURGE | 0 | end/loop control: if `sceneTimer` set → jump to previous tag (`ttmFindPreviousTag`); else mark thread finished (`isRunning=2`) (`ttm.c:189`) |
| `0x0FF0` | UPDATE | 0 | **yield**: end this `ttmPlay()` pass, present frame (`continueLoop=0`) (`ttm.c:197`) |
| `0x1021` | SET_DELAY | 1 | `timer=delay=max(arg,4)` — frame delay in ticks (`ttm.c:202`) |
| `0x1051` | SET_BMP_SLOT | 1 | select sprite-sheet slot for subsequent draws (`ttm.c:207`) |
| `0x1061` | SET_PALETTE_SLOT | 1 | recognised, no-op (`ttm.c:212`) |
| `0x1101` | LOCAL_TAG | 1 | tag marker, no runtime effect (bookmarked at load) (`ttm.c:216`) |
| `0x1111` | TAG | 1 | scene/label marker, no runtime effect (`ttm.c:220`) |
| `0x1121` | TTM_UNKNOWN_1 | 1 | defines region id used before SAVE_IMAGE1 / by CLEAR_SCREEN; no-op (`ttm.c:224`) |
| `0x1201` | GOTO_TAG | 1 | set `nextGotoOffset = ttmFindTag(arg)` (deferred jump) (`ttm.c:231`) |
| `0x2002` | SET_COLORS | 2 | `fgColor=arg0; bgColor=arg1` (`ttm.c:237`) |
| `0x2012` | SET_FRAME1 | 2 | always (0,0) near LOAD_IMAGE; no-op (`ttm.c:243`) |
| `0x2022` | TIMER | 2 | `delay=timer=(arg0+arg1)/2` (approximate) (`ttm.c:249`) |
| `0x4004` | SET_CLIP_ZONE | 4 | `grSetClipZone(x1,y1,x2,y2)` (`ttm.c:256`) |
| `0x4110` | FADE_OUT | 0 | **[dump-only]** (`dump.c:393`) |
| `0x4120` | FADE_IN | 0 | **[dump-only]** (`dump.c:397`) |
| `0x4204` | COPY_ZONE_TO_BG | 4 | `grCopyZoneToBg` — blit region into persistent "saved zones" layer (`ttm.c:261`) |
| `0x4214` | SAVE_IMAGE1 | 4 | `grSaveImage1` (effectively no-op; defines redraw zone) (`ttm.c:266`) |
| `0xA002` | DRAW_PIXEL | 2 | `grDrawPixel(x,y,fgColor)` (`ttm.c:273`) |
| `0xA054` | SAVE_ZONE | 4 | `grSaveZone` (only GJGULIVR.TTM) (`ttm.c:278`) |
| `0xA064` | RESTORE_ZONE | 4 | `grRestoreZone` (frees saved-zones layer) (`ttm.c:284`) |
| `0xA0A4` | DRAW_LINE | 4 | `grDrawLine(x1,y1,x2,y2,fgColor)` Bresenham (`ttm.c:290`) |
| `0xA104` | DRAW_RECT | 4 | `grDrawRect(x,y,w,h,fgColor)` filled (`ttm.c:295`) |
| `0xA404` | DRAW_CIRCLE | 4 | `grDrawCircle(x,y,w,h,fg,bg)` (`ttm.c:300`) |
| `0xA504` | DRAW_SPRITE | 4 | `grDrawSprite(x,y,spriteNo,imageNo)` (`ttm.c:305`) |
| `0xA510` | DRAW_SPRITE1 | 0 | **[dump-only]** (`dump.c:437`) |
| `0xA524` | DRAW_SPRITE_FLIP | 4 | horizontally mirrored sprite (`ttm.c:310`) |
| `0xA530` | DRAW_SPRITE3 | 0 | **[dump-only]** (`dump.c:445`) |
| `0xA601` | CLEAR_SCREEN | 1 | `grClearScreen` (clears this thread's layer) (`ttm.c:315`) |
| `0xB606` | DRAW_SCREEN | 6 | recognised, no-op (`ttm.c:321`) |
| `0xC020` | LOAD_SAMPLE | 0 | **[dump-only]** (`dump.c:457`) |
| `0xC030` | SELECT_SAMPLE | 0 | **[dump-only]** (`dump.c:461`) |
| `0xC040` | DESELECT_SAMPLE | 0 | **[dump-only]** (`dump.c:465`) |
| `0xC051` | PLAY_SAMPLE | 1 | `soundPlay(arg0)` (`ttm.c:325`) |
| `0xC060` | STOP_SAMPLE | 0 | **[dump-only]** (`dump.c:473`) |
| `0xF01F` | LOAD_SCREEN | str | `grLoadScreen(name)` set background (`ttm.c:330`) |
| `0xF02F` | LOAD_IMAGE | str | `grLoadBmp(selectedBmpSlot, name)` (`ttm.c:335`) |
| `0xF05F` | LOAD_PALETTE | str | recognised, no-op (global palette already loaded) (`ttm.c:340`) |

How animation works: a scene is a TTM "tag" entry point. Each `ttmPlay()` pass typically
`CLEAR_SCREEN`s the thread's transparent layer, draws sprites/primitives for one frame, then
`UPDATE` yields. The scheduler waits `delay` ticks, then re-enters; `GOTO_TAG`/`PURGE` create loops.
Final composition is layer-on-layer (background → saved zones → each thread layer → holiday layer).

---

## 4. ADS interpreter — `ads.c` / `ads.h`

ADS = the higher-level **scene-sequencing** bytecode (the `SCR:` block of an `.ADS` resource). An
ADS resource also carries a `RES:` table (slot→TTM filename) and a `TAG:` table (named sequence
entry points). ADS opcodes are 16-bit; arg counts are fixed per opcode (NOT nibble-encoded).

### Two-phase model
1. `adsLoad()` (`ads.c:89`) pre-scans the script: bookmarks all top-level tags (`adsTags[]`), finds
   the requested start `tag`'s offset, and **bookmarks the trigger chunks** — the `IF_LASTPLAYED`
   (`0x1350`) and leading `IF_NOT_RUNNING` (`0x1360`) clauses that belong to that tag — into
   `adsChunks[]`. These are the reactive rules fired when a TTM scene ends.
2. `adsPlayChunk()` (`ads.c:445`) executes a chunk: ADD_SCENE/STOP_SCENE spawn/stop TTM threads;
   IF_* are guards; RANDOM_START…RANDOM_END pick one weighted operation.

`adsPlay()` (`ads.c:658`): load all TTMs from the `RES:` table into `ttmSlots[id]`, `adsLoad()`,
play the start chunk, then run the scheduler loop (described in §1). When a thread terminates,
`adsPlayTriggeredChunks()` (`ads.c:633`) replays any chunk whose `IF_LASTPLAYED`/`IF_NOT_RUNNING`
matches the just-finished `(slot,tag)` — this is how one TTM animation chains into the next.

#### Complete ADS opcode table
From `adsPlayChunk()` (runtime), `adsLoad()` (pre-scan arg sizes), and `dump.c:241` (names). Arg
counts are in 16-bit words.

| Opcode | Mnemonic | Args | Meaning / effect |
|--------|----------|------|------------------|
| `0x1070` | IF_LASTPLAYED_LOCAL | 2 | local "if last played (slot,tag)" overriding globals; queues a local chunk (`ads.c:462`). Only ACTIVITY.ADS tag 7. |
| `0x1330` | IF_UNKNOWN_1 | 2 | guard, synonym of IF_NOT_RUNNING; ignored at runtime (`ads.c:474`) |
| `0x1350` | IF_LASTPLAYED | 2 | reactive trigger: chunk runs when scene (slot,tag) last finished. In chunk exec acts as a block terminator unless in OR (`ads.c:484`) |
| `0x1360` | IF_NOT_RUNNING | 2 | if (slot,tag) currently running → skip block (`inSkipBlock`) (`ads.c:495`) |
| `0x1370` | IF_IS_RUNNING | 2 | skip block unless (slot,tag) running (`ads.c:502`) |
| `0x1420` | AND | 0 | boolean AND of conditions (`ads.c:508`) |
| `0x1430` | OR | 0 | boolean OR; sets `inOrBlock` (`ads.c:512`) |
| `0x1510` | PLAY_SCENE | 0 | "closing brace" of a conditional block; ends chunk unless skipping (`ads.c:517`) |
| `0x1520` | ADD_SCENE_LOCAL | 5 | add a scene queued by a local trigger; args (?,slot,tag,arg3,?) → `adsAddScene(args[1],args[2],args[3])` (`ads.c:529`) |
| `0x2005` | ADD_SCENE | 4 | spawn TTM thread (slot,tag,arg3,?). In RANDOM block → weighted candidate (`ads.c:547`) |
| `0x2010` | STOP_SCENE | 3 | stop scene by (slot,tag). In RANDOM block → weighted candidate (`ads.c:560`) |
| `0x2014` | UNKNOWN_5 | 0 | recognised by dumper; pre-scan size 0 (`dump.c:252`, `ads.c:156`) |
| `0x3010` | RANDOM_START | 0 | begin weighted-random selection block (`ads.c:573`) |
| `0x3020` | NOP | 1 | weighted "do nothing" candidate (the weight is the arg) (`ads.c:579`) |
| `0x30FF` | RANDOM_END | 0 | pick & execute one candidate by weight (`adsRandomEnd`) (`ads.c:586`) |
| `0x4000` | UNKNOWN_6 | 3 | only BUILDING.ADS tag 7; ignored (`ads.c:592`) |
| `0xF010` | FADE_OUT | 0 | recognised, no-op at runtime (`ads.c:597`) |
| `0xF200` | GOSUB_TAG | 1 | call another tag's chunk inline (`adsPlayChunk(... adsFindTag(arg))`); used by STAND.ADS→tag 14 (`ads.c:601`) |
| `0xFFFF` | END | 0 | end of sequence → request stop (unless skipping) (`ads.c:610`) |
| `0xFFF0` | END_IF | 0 | closes an IF block (`ads.c:620`) |
| (other) | :TAG n | 0 | any other value = a tag/label id (`ads.c:166` default; `dump.c:261`) |

### Weighted random — `adsRandomPickOp()` (`ads.c:314`)
Sums all candidate `weight`s, picks `rand()%total`, walks the cumulative distribution. Candidate
types `OP_ADD_SCENE / OP_STOP_SCENE / OP_NOP`. This drives the "engine randomly chooses what Johnny
does next" behaviour inside a scene.

### Thread management
`adsAddScene()` (`ads.c:219`) finds a free `ttmThreads[]` slot (skips duplicates), inits it
(`delay=4`, `fgColor=bgColor=0x0F`), sets `ip = ttmFindTag(slot,tag)`, allocates `ttmLayer =
grNewLayer()`. `arg3` semantics: **negative** → `sceneTimer = -arg3` (play for that many ticks);
**positive** → `sceneIterations = arg3-1` (play that many times); `0` → play once to natural end.
`adsStopScene` frees the layer and decrements `numThreads`.

Other ADS modes: `adsPlaySingleTtm()` (`ads.c:426`, play one TTM linearly), `adsPlayIntro()`
(`ads.c:855`, show `INTRO.SCR` then `grFadeOut`), `adsInitIsland`/`adsReleaseIsland`/`adsNoIsland`
(background setup), `adsPlayWalk()` (`ads.c:912`, run the walking animation between two spots — see
§9), `adsPlayBench()` (§1).

`MAX_TTM_SLOTS=10`, `MAX_TTM_THREADS=10`, `MAX_ADS_CHUNKS=100`, `MAX_RANDOM_OPS=10`.

---

## 5. Graphics — `graphics.c` / `graphics.h`

- **Resolution**: fixed logical `640×480`, 32-bpp SDL window (`SCREEN_WIDTH/HEIGHT`,
  `graphics.h:26`). `graphicsInit()` (`graphics.c:113`) creates the window (fullscreen unless
  `grWindowed`), hides cursor in fullscreen, loads `palResources[0]` palette, seeds `srand(time)`,
  calls `eventsInit()`. No scaling logic beyond centering via `grScreenOrigin` (which is `{0,0}`
  since logical==window size).
- **Palette**: `grLoadPalette()` (`graphics.c:99`) converts the first 16 PAL colours (6-bit VGA)
  into RGBA by `<<2` (×4 to reach 8-bit) into `ttmPalette[16][4]`. Note BGR storage order:
  `[0]=b<<2,[1]=g<<2,[2]=r<<2,[3]=0`.
- **Layers / double-buffering**: each TTM thread renders to its own off-screen
  `SDL_CreateRGBSurface(640,480,32)` (`grNewLayer`, `graphics.c:218`) filled with magenta
  `0xA8,0x00,0xA8` as the color key (transparency). `grUpdateDisplay()` (`graphics.c:175`)
  composites in order: background surface → optional `grSavedZonesLayer` → each running thread's
  layer → holiday layer; then `eventsWaitTick(grUpdateDelay)`; then `SDL_UpdateWindowSurface`. This
  is the double-buffer/blit model.
- **Background**: `grLoadScreen()` (`graphics.c:505`) decodes a 4-bpp `.SCR` into a 32-bpp surface
  via the palette (2 px/byte, hi nibble then lo nibble). `grInitEmptyBackground()` = black
  background (used when no island).
- **Sprites**: `grLoadBmp()` (`graphics.c:568`) decodes each sub-image of a `.BMP` into a 32-bpp
  surface with magenta color key. `grDrawSprite()` blits at `(x+grDx, y+grDy)`. `grDrawSpriteFlip()`
  (`graphics.c:472`) mirrors by blitting column-by-column.
- **Primitives** (all honour `grDx/grDy` offset and write into the given layer):
  `grDrawPixel`, `grDrawLine` (Bresenham, `graphics.c:295`), `grDrawRect` (filled),
  `grDrawCircle` (Bresenham circle, only equal width/height and even diameters, `graphics.c:369`),
  `grSetClipZone` (SDL clip rect). `grPutPixel` (`graphics.c:67`) is the clipped low-level writer.
- **Zone save/restore** (approximate vs original): `grCopyZoneToBg` blits a region into the
  persistent `grSavedZonesLayer` (notes a +2 width fudge to fix a coordinate bug in GJIVS6.TTM,
  `graphics.c:255`). `grSaveImage1`/`grSaveZone` are near-stubs; `grRestoreZone` just frees the
  saved-zones layer (the original never overlaps saved zones, per comment `graphics.c:281`).
- **Fade-out**: `grFadeOut()` (`graphics.c:604`) cycles through 5 transition styles (expanding
  circle, expanding rect, R→L, L→R, middle-out) on each successive call.
- `grToggleFullScreen()` for the Alt+Return hot-key.

---

## 6. Sound — `sound.c` / `sound.h`

`NUM_OF_SOUNDS = 25`. `soundInit()` (`sound.c:67`) opens SDL audio and `SDL_LoadWAV("sound%d.wav")`
for i=0..24 (missing files tolerated → length 0). A single software mixer via `soundCallback`
(`sound.c:51`): one "current" sound at a time, streamed to the device; pads with `127` (=8-bit
silence) when exhausted. `soundPlay(nb)` (`sound.c:121`) sets the current pointer/length under
`SDL_LockAudio`. `soundDisabled` (set by `nosound` or init failure) short-circuits everything.
Sound 0 is the generic "scene transition/notify" cue played by the story engine before "special-day"
scenes (`story.c:247`, `story.c:269`). Other indices are triggered by TTM `PLAY_SAMPLE` opcodes.

---

## 7. Story system — `story.c` / `story.h` / `story_data.h`

This is the top-level director that, forever, picks scenes and stitches them together with walking,
honouring the current "day of the story", time of day, and holidays.

### Scene table — `storyScenes[NUM_SCENES]`, `NUM_SCENES = 63` (`story_data.h:24`)
Each row `TStoryScene` (`story_data.h:51`): `{ char adsName[13]; int adsTagNo; int spotStart; int
hdgStart; int spotEnd; int hdgEnd; int dayNo; int flags; }`.

**Flags** (`story_data.h:26`):
`FINAL 0x01`, `FIRST 0x02`, `ISLAND 0x04`, `LEFT_ISLAND 0x08`, `VARPOS_OK 0x10`, `LOWTIDE_OK 0x20`,
`NORAFT 0x40`, `HOLIDAY_NOK 0x80`.

**Spots** A–F = 0–5 (`SPOT_A..SPOT_F`). **Headings** (8-way, clockwise from south):
`HDG_S 0, HDG_SW 1, HDG_W 2, HDG_NW 3, HDG_N 4, HDG_NE 5, HDG_E 6, HDG_SE 7`.

`dayNo`: `0` = eligible on any day; `1..11` = only on that specific story-day (the scripted-story
beats). The full table (verbatim, `story_data.h:63`):

```
//           Name  Tag    Start   Start      End     End Day of  Flags
//                         spot     hdg     spot     hdg  story
  { "ACTIVITY.ADS",  1,  SPOT_E, HDG_SE,       0,      0,   0,  ISLAND | FINAL | VARPOS_OK                                },
  { "ACTIVITY.ADS", 12,  SPOT_D, HDG_SW,       0,      0,   0,  ISLAND | FINAL | VARPOS_OK | LOWTIDE_OK                   },
  { "ACTIVITY.ADS", 11,       0,      0,       0,      0,   0,  ISLAND | FINAL | FIRST | VARPOS_OK                        },
  { "ACTIVITY.ADS", 10,  SPOT_D, HDG_SW,       0,      0,   0,  ISLAND | FINAL | VARPOS_OK | LOWTIDE_OK                   },
  { "ACTIVITY.ADS",  4,  SPOT_E, HDG_SE,  SPOT_E, HDG_SE,   0,  ISLAND | VARPOS_OK | LOWTIDE_OK                           },
  { "ACTIVITY.ADS",  5,  SPOT_E, HDG_SW,       0,      0,   0,  ISLAND | FINAL | VARPOS_OK | LOWTIDE_OK                   },
  { "ACTIVITY.ADS",  6,  SPOT_D, HDG_SW,       0,      0,   0,  ISLAND | FINAL | VARPOS_OK                                },
  { "ACTIVITY.ADS",  7,  SPOT_D, HDG_SW,  SPOT_F, HDG_SW,   0,  ISLAND | VARPOS_OK | LOWTIDE_OK                           },
  { "ACTIVITY.ADS",  8,       0,      0,  SPOT_D, HDG_SE,   0,  ISLAND | FIRST | VARPOS_OK                                },
  { "ACTIVITY.ADS",  9,  SPOT_E, HDG_E ,       0,      0,   0,  ISLAND | FINAL | LOWTIDE_OK                               },

  { "BUILDING.ADS",  1,  SPOT_F, HDG_W ,  SPOT_A, HDG_W ,   0,  ISLAND | VARPOS_OK | LOWTIDE_OK                           },
  { "BUILDING.ADS",  4,  SPOT_A, HDG_E ,       0,      0,   0,  ISLAND | FINAL | VARPOS_OK                                },
  { "BUILDING.ADS",  3,  SPOT_A, HDG_E ,  SPOT_C, HDG_SE,   0,  ISLAND | VARPOS_OK | LOWTIDE_OK                           },
  { "BUILDING.ADS",  2,  SPOT_F, HDG_W ,       0,      0,   0,  ISLAND | FINAL | VARPOS_OK                                },
  { "BUILDING.ADS",  5,  SPOT_D, HDG_W ,  SPOT_D, HDG_E ,   0,  ISLAND | VARPOS_OK | LOWTIDE_OK                           },
//{ "BUILDING.ADS",  9, ...identical to tag 5 }
  { "BUILDING.ADS",  7,  SPOT_D, HDG_W ,  SPOT_D, HDG_E ,   0,  ISLAND | VARPOS_OK | LOWTIDE_OK                           },
//{ "BUILDING.ADS",  8, ...identical to tag 7 }
  { "BUILDING.ADS",  6,  SPOT_A, HDG_E ,       0,      0,   0,  ISLAND | FINAL | VARPOS_OK                                },

  { "FISHING.ADS" ,  1,  SPOT_D, HDG_W ,  SPOT_D, HDG_E ,   0,  ISLAND | VARPOS_OK | LOWTIDE_OK                           },
  { "FISHING.ADS" ,  2,  SPOT_D, HDG_W ,  SPOT_D, HDG_E ,   0,  ISLAND | VARPOS_OK | LOWTIDE_OK                           },
  { "FISHING.ADS" ,  3,  SPOT_D, HDG_W ,       0,      0,   0,  ISLAND | FINAL | VARPOS_OK | LOWTIDE_OK                   },
  { "FISHING.ADS" ,  4,  SPOT_E, HDG_E ,       0,      0,   0,  ISLAND | FINAL | LEFT_ISLAND | LOWTIDE_OK                 },
  { "FISHING.ADS" ,  5,  SPOT_E, HDG_E ,       0,      0,   0,  ISLAND | FINAL | VARPOS_OK                                },
  { "FISHING.ADS" ,  6,  SPOT_D, HDG_W ,       0,      0,   0,  ISLAND | FINAL | LOWTIDE_OK                               },
  { "FISHING.ADS" ,  7,  SPOT_E, HDG_E ,  SPOT_E, HDG_W ,   0,  ISLAND | LEFT_ISLAND | VARPOS_OK | LOWTIDE_OK             },
  { "FISHING.ADS" ,  8,  SPOT_E, HDG_E ,  SPOT_E, HDG_W ,   0,  ISLAND | LEFT_ISLAND | VARPOS_OK | LOWTIDE_OK             },

  { "JOHNNY.ADS"  ,  1,       0,      0,       0,      0,  11,  FINAL | FIRST                                             },
  { "JOHNNY.ADS"  ,  2,  SPOT_E, HDG_SW,  SPOT_F,      0,   2,  ISLAND | FINAL | VARPOS_OK                                },
  { "JOHNNY.ADS"  ,  3,  SPOT_E, HDG_SW,  SPOT_F, HDG_NE,   6,  ISLAND | VARPOS_OK | LOWTIDE_OK                           },
  { "JOHNNY.ADS"  ,  4,  SPOT_E, HDG_SW,  SPOT_F, HDG_NE,   0,  ISLAND | VARPOS_OK                                        },
  { "JOHNNY.ADS"  ,  5,  SPOT_E, HDG_SW,  SPOT_F, HDG_NE,   0,  ISLAND | VARPOS_OK                                        },
  { "JOHNNY.ADS"  ,  6,       0,      0,       0,      0,  10,  FINAL | FIRST                                             },

  { "MARY.ADS"    ,  1,  SPOT_E, HDG_SW,       0,      0,   5,  ISLAND | FINAL | VARPOS_OK | LOWTIDE_OK                   },
  { "MARY.ADS"    ,  3,  SPOT_F, HDG_SW,       0,      0,   4,  ISLAND | FINAL | FIRST | VARPOS_OK                        },
  { "MARY.ADS"    ,  2,  SPOT_E, HDG_E ,       0,      0,   1,  ISLAND | FINAL | VARPOS_OK                                },
  { "MARY.ADS"    ,  4,  SPOT_E, HDG_E ,       0,      0,   7,  ISLAND | FINAL | VARPOS_OK                                },
  { "MARY.ADS"    ,  5,  SPOT_E, HDG_NW,       0,      0,   8,  ISLAND | LEFT_ISLAND | FINAL | FIRST | NORAFT | VARPOS_OK },

  { "MISCGAG.ADS" ,  1,  SPOT_D, HDG_W ,       0,      0,   0,  ISLAND | FINAL | VARPOS_OK | LOWTIDE_OK                   },
  { "MISCGAG.ADS" ,  2,  SPOT_D, HDG_W ,       0,      0,   0,  ISLAND | FINAL | VARPOS_OK                                },

  { "STAND.ADS"   ,  1,  SPOT_A, HDG_SW,  SPOT_A, HDG_SW,   0,  ISLAND | VARPOS_OK | LOWTIDE_OK                           },
  { "STAND.ADS"   ,  2,  SPOT_A, HDG_W ,  SPOT_A, HDG_W ,   0,  ISLAND | VARPOS_OK | LOWTIDE_OK                           },
  { "STAND.ADS"   ,  3,  SPOT_A, HDG_NW,  SPOT_A, HDG_NW,   0,  ISLAND | VARPOS_OK | LOWTIDE_OK                           },
  { "STAND.ADS"   ,  4,  SPOT_B, HDG_SW,  SPOT_B, HDG_SW,   0,  ISLAND | VARPOS_OK | LOWTIDE_OK                           },
  { "STAND.ADS"   ,  5,  SPOT_B, HDG_S ,  SPOT_B, HDG_S ,   0,  ISLAND | VARPOS_OK | LOWTIDE_OK                           },
  { "STAND.ADS"   ,  6,  SPOT_B, HDG_SE,  SPOT_B, HDG_SE,   0,  ISLAND | VARPOS_OK | LOWTIDE_OK                           },
  { "STAND.ADS"   ,  7,  SPOT_C, HDG_NE,  SPOT_C, HDG_NE,   0,  ISLAND | VARPOS_OK | LOWTIDE_OK                           },
  { "STAND.ADS"   ,  8,  SPOT_C, HDG_E ,  SPOT_C, HDG_E ,   0,  ISLAND | VARPOS_OK | LOWTIDE_OK                           },
  { "STAND.ADS"   ,  9,  SPOT_D, HDG_NW,  SPOT_D, HDG_NW,   0,  ISLAND | VARPOS_OK | LOWTIDE_OK                           },
  { "STAND.ADS"   , 10,  SPOT_D, HDG_NE,  SPOT_D, HDG_NE,   0,  ISLAND | VARPOS_OK | LOWTIDE_OK                           },
  { "STAND.ADS"   , 11,  SPOT_E, HDG_NW,  SPOT_E, HDG_NW,   0,  ISLAND | VARPOS_OK | LOWTIDE_OK                           },
  { "STAND.ADS"   , 12,  SPOT_F, HDG_S ,  SPOT_F, HDG_S ,   0,  ISLAND | VARPOS_OK | LOWTIDE_OK                           },
  { "STAND.ADS"   , 15,  SPOT_A, HDG_S ,  SPOT_A, HDG_S ,   0,  ISLAND | VARPOS_OK | LOWTIDE_OK                           },
  { "STAND.ADS"   , 16,  SPOT_C, HDG_S ,  SPOT_C, HDG_S ,   0,  ISLAND | VARPOS_OK | LOWTIDE_OK                           },

  { "SUZY.ADS"    ,  1,       0,      0,       0,      0,   3,  FINAL | FIRST                                             },
  { "SUZY.ADS"    ,  2,       0,      0,       0,      0,   9,  FINAL | FIRST                                             },

  { "VISITOR.ADS" ,  1,  SPOT_A, HDG_S ,  SPOT_A, HDG_S ,   0,  ISLAND | LOWTIDE_OK                                       },
  { "VISITOR.ADS" ,  3,  SPOT_B, HDG_NW,  SPOT_D,      0,   0,  ISLAND | FINAL | HOLIDAY_NOK                              },
  { "VISITOR.ADS" ,  4,  SPOT_D, HDG_S ,  SPOT_D, HDG_W ,   0,  ISLAND | VARPOS_OK | LOWTIDE_OK                           },
  { "VISITOR.ADS" ,  6,  SPOT_D, HDG_S ,  SPOT_D, HDG_SW,   0,  ISLAND | VARPOS_OK | LOWTIDE_OK                           },
  { "VISITOR.ADS" ,  7,  SPOT_D, HDG_S ,  SPOT_D, HDG_SW,   0,  ISLAND | VARPOS_OK | LOWTIDE_OK                           },
  { "VISITOR.ADS" ,  5,  SPOT_E, HDG_SW,       0,      0,   0,  ISLAND | FINAL | LEFT_ISLAND | VARPOS_OK | LOWTIDE_OK     },

  { "WALKSTUF.ADS",  1,  SPOT_A, HDG_NE,       0,      0,   0,  ISLAND | FINAL | LOWTIDE_OK                               },
  { "WALKSTUF.ADS",  2,  SPOT_E, HDG_E ,  SPOT_D, HDG_SE,   0,  ISLAND | VARPOS_OK                                        },
  { "WALKSTUF.ADS",  3,  SPOT_D, HDG_W ,  SPOT_E, HDG_W ,   0,  ISLAND | VARPOS_OK | LOWTIDE_OK                           },
```

### The day-numbered (scripted) story beats
Scenes with non-zero `dayNo` are the "plot" of the 11-day cycle (the rest are ambient gags). Mapping
day → scripted scene(s):

| Day | Scene(s) | Note |
|-----|----------|------|
| 1 | MARY.ADS#2 | (also: raft progress = 1) |
| 2 | JOHNNY.ADS#2 | |
| 3 | SUZY.ADS#1 | |
| 4 | MARY.ADS#3 | |
| 5 | MARY.ADS#1 | |
| 6 | JOHNNY.ADS#3 | |
| 7 | MARY.ADS#4 | |
| 8 | MARY.ADS#5 (NORAFT, LEFT_ISLAND) | |
| 9 | SUZY.ADS#2 | |
| 10 | JOHNNY.ADS#6 | |
| 11 | JOHNNY.ADS#1 | last day of the cycle |

(Verified against `story_data.h:97-129`. JOHNNY.ADS#4/#5 and JOHNNY.ADS#2 are noted: #2 is day 2;
#4/#5 are `dayNo=0` ambient. Each row is matched by both ADS tag and dayNo.)

### Day advancement & persistence — `storyUpdateCurrentDay()` (`story.c:65`)
The story-day is **wall-clock driven and persisted** in a config file (`config.c`, file
`~/.jc_reborn`, keys `currentDay=` and `date=`). Logic: read config; `today = getDayOfYear()`
(`tm_yday`); **if the calendar day differs from the stored `date`, advance `currentDay += 1`** and
store the new date. Clamp `currentDay` to `1..11` (wraps to 1 beyond 11). So the multi-day story
advances **one beat per real-world day** the saver is run, looping every 11 days. `storyCurrentDay`
defaults to 1.

### Scene selection — `storyPickScene(wantedFlags, unwantedFlags)` (`story.c:42`)
Collects every scene whose flags include all `wantedFlags`, none of `unwantedFlags`, and whose
`dayNo` is 0 or equals `storyCurrentDay`; returns a uniformly random one.

### The director loop — `storyPlay()` (`story.c:194`)
`adsInit(); adsPlayIntro();` then forever:
1. `storyUpdateCurrentDay()`, `storyCalculateIslandFromDateAndTime()`.
2. Pick a **FINAL** scene (the climactic gag of this run) via `storyPickScene(FINAL,0)`.
3. If it is an `ISLAND` scene, compute island params (`storyCalculateIslandFromScene`) and
   `adsInitIsland()`; else `adsNoIsland()`.
4. Unless the final scene is also `FIRST`, play a chain of `6 + rand()%14` ambient scenes leading up
   to it. Each ambient scene must match the island's current low-tide / variable-position state
   (`wantedFlags`), excludes `FINAL`, and after the first excludes `FIRST`. Between consecutive
   scenes, Johnny **walks** from the previous scene's end spot/heading to the next scene's start
   spot/heading (`adsPlayWalk`). `ttmDx/ttmDy` is set to the island offset (+272 for `LEFT_ISLAND`).
   If the scene is a day-beat (`dayNo`), `soundPlay(0)` is cued.
5. Walk to the final scene, set offsets, optional `soundPlay(0)`, `adsPlay(finalScene)`,
   `grFadeOut()`, and release the island.

### Island state derived from a scene — `storyCalculateIslandFromScene()` (`story.c:123`)
- **Low tide**: if `LOWTIDE_OK` and `rand()%2`.
- **Island position**: if `VARPOS_OK`, randomly pick one of three offset ranges
  (`xPos = -222+rand%109 / -114+rand%134 / -114+rand%119`, similar y); else fixed (`LEFT_ISLAND` →
  `xPos=-272`, else `0`).
- **Raft progress** (how much of the escape raft is built): forced 0 if `NORAFT`; otherwise by day:
  days 0–2 → raft 1; days 3–5 → `day-1` (so 2,3,4); day ≥6 → 5. This is why the raft visibly grows
  across the story.
- `HOLIDAY_NOK` (only VISITOR.ADS#3, the cargo ship) forces `holiday=0` so holiday props aren't
  drawn over the ship's hull.

---

## 8. Events & special days (holidays) — `story.c` + `island.c` + `utils.c`

Holiday/night detection is in `storyCalculateIslandFromDateAndTime()` (`story.c:94`). Date string is
`getMonthAndDay()` → `strftime("%m%d")` (`utils.c:217`), hour via `getHour()` (`utils.c:206`).

**Night**: `int hour = getHour() % 8; islandState.night = (hour==0 || hour==7);` — i.e. roughly
nighttime in an 8-hour-wrapped clock (hours 0,7,8,15,16,23). When night, `island.c:41` loads
`NIGHT.SCR` instead of an `OCEANxx.SCR`.

**Holidays** (`islandState.holiday`, 0=none) — the literal date logic (`story.c:104`):
```
// Halloween : "1028" < date < "1101"          -> holiday = 1   (i.e. Oct 29–31)
// St Patrick: "0314" < date < "0318"          -> holiday = 2   (i.e. Mar 15–17)
// Christmas : "1222" < date < "1226"          -> holiday = 3   (i.e. Dec 23–25)
// New year  : "1228" < date  OR  date < "0102"-> holiday = 4   (i.e. Dec 29–Jan 1)
```
(String comparisons on `MMDD`; the New Year case spans the year boundary with an OR.)

Holiday rendering — `islandInitHoliday()` (`island.c:192`): if a holiday is active, create a layer,
load `HOLIDAY.BMP`, draw the matching prop and mark the holiday thread running (`isRunning=3`):
- 1 Halloween → sprite 0 at (410,298)
- 2 St Patrick → sprite 1 at (333,286)
- 3 Christmas → sprite 2 at (404,267)
- 4 New Year → sprite 3 at (361,155)

So the only "special-day" mechanics are: (a) these 4 fixed holiday windows that add a decorative
sprite, (b) day/night background swap by hour, and (c) the 11-day scripted story advanced by calendar
day. There is no separate `events.c`-style holiday scheduler — `events.c` is purely SDL input + the
tick wait. Random "events" are the weighted RANDOM blocks in ADS scripts plus the random scene
selection in `story.c`.

---

## 9. Walking & pathfinding — `walk.c`/`.h`, `walk_data.h`, `calcpath.c`/`.h`, `calcpath_data.h`

Johnny moves between 6 named **spots** A–F (the same nodes used by the story table). Movement = pick
a route through the spot graph, then play pre-baked walk-animation frames.

### Pathfinding — `calcpath.c` + `calcpath_data.h`
`NUM_OF_NODES = 6`, `UNDEF_NODE = 6`. `walkMatrix[7][6][6]` (`calcpath_data.h:28`) is a
**second-order adjacency** table: `walkMatrix[prevNode][curNode][nextNode]` = 1 if you may go
cur→next given you arrived from prev. Index `[6]` ("from any spot") is the fallback adjacency used
for the very first hop (prevNode = UNDEF_NODE). The per-prev rows encode turn restrictions (you
can't immediately backtrack certain ways), shaping natural-looking routes.

`calcPath(fromNode, toNode)` (`calcpath.c:81`): DFS (`calcPathRecurse`) enumerating ALL simple paths
from `from` to `to` (marking visited, up to `MAX_NUM_PATHS=50`, `MAX_PATH_LEN=7`), each terminated
by `UNDEF_NODE`; then returns a **random** one (`paths[rand()%numPaths]`). Comment admits it's not
the original algorithm, just a plausible fit. With `debug`, prints all candidate paths and the chosen
one as letters A–F.

### Walk animation — `walk.c` + `walk_data.h`
`walkData[][4]` (`walk_data.h:8`) is a big table of animation frames, each `{flip, x, y,
spriteNo}` (`flip`=1 → draw mirrored). It is segmented into per-route runs (A→E, A→F, A→C, A→B, A
turn, A wait, B→A, …) terminated by `{0,0,0,0}` sentinels. **Important**: this table is NOT in
`RESOURCE.001`; it was extracted from the original `SCRANTIC.SCR` executable (see
`extract_walk_data.c`, which seeks to offset `0x188ea` and reads triples until `0x019456`, packing
`flip = word0>>15`, `spriteNo = word0 & 0x7fff`).

Index tables (`walk_data.h:500`):
- `walkDataBookmarks[6][6]` — start index in `walkData[]` for a straight walk from spot row→col
  (`-1` if no direct segment).
- `walkDataBookmarksTurns[6] = {91,145,260,314,405,471}` — start index of the "turn" frames per
  spot (the 8 directional turn frames; `+9` offset selects the "hands in pockets / wait" variant).
- `walkDataStartHeadings[6][6]` and `walkDataEndHeadings[6][6]` — the heading Johnny faces at the
  start/end of each route segment.

`walkInit(fromSpot,fromHdg,toSpot,toHdg)` (`walk.c:47`): compute path via `calcPath`, set up the
first turn (compute `increment` = ±1 shortest rotation in the 8-way heading ring via
`(nextHdg-currentHdg)&0x07`).

`walkAnimate(ttmThread, ttmBgSlot)` (`walk.c:74`): a per-frame state machine returning the **delay**
(ticks) until the next frame (0 when arrived). It interleaves three phases: turning (step heading by
`increment`, pick turn frames at `walkDataBookmarksTurns[spot]+hdg`), walking forward (advance
through the route's frame run until the `{...,0,...}` sentinel marks spot reached), and arrival
(face `finalHdg`, switch to "hands in pockets" frames `+9`, `delay=80`; normal walk frame
`delay=6`). Special case `isBehindTree`: when walking between spots 3↔4 (D↔E) Johnny passes behind
the palm tree, so it redraws trunk (sprite 13 @442,148) and leaves (sprite 12 @365,122) over him
from the background slot (`walk.c:164`). Sprites come from `JOHNWALK.BMP` (loaded in
`adsPlayWalk`, `ads.c:915`).

`adsPlayWalk()` (`ads.c:912`) wires this into the scheduler: add a single thread, set
`grDx/grDy=island pos`, init walk, and loop animating background waves + the walk frames until
`walkAnimate` returns 0.

---

## 10. Island background — `island.c`/`.h`

`TIslandState islandState` (`island.h:24`): `{ lowTide, night, raft, holiday, xPos, yPos }`, the
single global describing the current scene's environment.

`islandInit()` (`island.c:35`): choose background (`NIGHT.SCR` if night, else
`OCEAN0{0,1,2}.SCR`), point the background thread's layer at `grBackgroundSfc`, then **paint the
static scene directly into the background surface** (so TTM/walk layers composite over it): raft
(0–5 stages from `MRAFT.BMP`, position shifts with low tide), clouds (`BACKGRND.BMP` sprites 15–17,
0–5 of them via a nested-`rand()` distribution, flipped per wind direction), the island (sprite 0
@288,279), palm trunk (13 @442,148), leaves (12 @365,122), shadow (14 @396,279), and low-tide shore
(1 @249,303) + rock (2 @150,328). Then 4 priming `islandAnimate()` calls and sets the waves thread
`delay=timer=8`.

`islandAnimate()` (`island.c:150`): cycles shore-wave sprites each call (two `static` counters):
high tide → 3 wave positions (sprites 3/6/9 +phase) ; low tide → 4 positions (sprites 30/33/36/39
+phase). 3-phase animation. This is the only continuously-running background animation; it is the
`ttmBackgroundThread` driven by the scheduler.

`islandInitHoliday()` — see §8.

`BACKGRND.BMP` sprite index legend (from code comments): 0 island, 1 low-tide shore, 2 rock,
3/6/9 high-tide waves (left/center/right, 3 phases each), 12 leaves, 13 trunk, 14 shadow,
15/16/17 clouds, 30/33/36 low-tide waves (left/center/right), 39 rock waves. `MRAFT.BMP` images
0–4 = raft build stages 1–5.

---

## 11. Config — `config.c`/`.h`; Dump — `dump.c`

**Config**: `~/.jc_reborn` (or CWD if no `$HOME`), plain text `currentDay=N` / `date=N`
(`config.c`). Only used by the story-day mechanism (§7). `TConfig {int currentDay; int date;}`.

**Dump** (`jc_reborn dump`): `dumpAllResources()` (`dump.c:495`) creates `./dump/{ADS,BMP,SCR,TTM}/`
and exports every resource: SCR/BMP → XPM images (`dumpScr`/`dumpBmp`, using `palResources[0]`),
ADS/TTM → human-readable disassembly text (`dumpAds`/`dumpTtm`). The dumpers contain the **most
complete opcode name tables** (they list opcodes the runtime ignores, e.g. SAVE_BACKGROUND `0x001F`,
FADE_IN/OUT `0x4110/0x4120`, the C0xx sample ops, DRAW_SPRITE1/3 `0xA510/0xA530`) — see the tables in
§3 and §4. `misc/adsbeautifier.awk` re-indents dumped ADS into braced blocks.

---

## 12. Key reusability notes for a modern port

- **The bytecode interpreters (TTM + ADS) are the crown jewels** and are fully transcribable: the
  opcode→semantics mapping above is the reverse-engineering result the whole project exists to
  capture. Port these faithfully and most behaviour follows.
- **Data-driven**: nearly everything (scenes, sprites, animations) lives in the original files; the
  C tables that are *not* in the data and must be carried over verbatim are: `story_data.h`
  (scene/day/flag table), `walk_data.h` (walk frames + bookmarks, extracted from `SCRANTIC.SCR`),
  `calcpath_data.h` (the second-order adjacency matrix). These encode designer intent that cannot be
  re-derived from `RESOURCE.001`.
- **Timing**: standardise on the 20 ms tick and the variable-timestep cooperative scheduler
  (`adsPlay` loop). The ADD_SCENE arg3 sign convention (negative=duration, positive=iterations) and
  `isRunning` tri-state must be preserved.
- **Approximations to improve**: walking path selection, the random scene scheduler, island
  placement/cloud distribution, and several zone-save/restore ops (`grSaveImage1`, `grSaveZone` are
  stubs) are the author's best-guess and acknowledged as imperfect; a disassembly of the original
  would refine them. `grUpdateDisplay` re-blits everything each frame (no dirty-rect) — fine for a
  modern GPU port.
- **Self-contained & small**: ~40 source files, one dependency (SDL2), GPLv3. Clean separation:
  resource/uncompress (I/O), ttm/ads (VM), graphics/sound (backend), story/events/island/walk
  (game logic). A modern port can keep this layering, swap SDL for any backend, and reuse the VMs +
  data tables wholesale.
- **License caveat**: GPLv3 — derived code inherits GPL obligations; reusing the *insights* (opcode
  meanings, data tables) vs. copying the *code* has different licensing implications.
