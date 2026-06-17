# Castaway & DGDS-Viewer — Source Code Knowledge Base

Exhaustive notes from reading the source of two JavaScript projects by Alexandre Fontoura (aka **xesf**, xesfnet@gmail.com) that parse and run the Dynamix DGDS / SCRANTIC resource format (the 1992 Sierra/Dynamix "Johnny Castaway" screensaver) in the browser:

- **castaway** — upstream <https://github.com/xesf/castaway> (formerly `repos/castaway`) — a focused re-implementation of *Johnny Castaway* as native browser **ES modules** (`.mjs`), no build step. Served with `http-server`. Aim: "complete re-implementation of the Johnny Castaway Screen Saver ... using Javascript modules."
- **dgds-viewer** — upstream <https://github.com/xesf/dgds-viewer> (formerly `repos/dgds-viewer`) — a more general **React/Electron** resource browser/player for *multiple* Dynamix DGDS games. Built with webpack/babel, Electron shell, Express SSR dev server.

The viewer was extracted from castaway during development (castaway README, line 86-88: "I've create a DGDS Resource Viewer while I was building the initial version of castaway. I've then split it into its own project").

Credits cite three prior efforts that informed the format reverse-engineering:
- Jérémie Guillaume (jno6809) — `jc_reborn` (C)
- Hans Milling (nivs1978) — Johnny-Castaway-Open-Source (C#)
- Vasco Costa (vcosta) — DGDS engine in ScummVM

---

## 1. High-level architecture & file inventory

### castaway (`src/`)
```
index.mjs                      -> calls run() from scrantic/main.mjs
manifest.json                  -> PWA manifest ("Johnny Castaway Viewer")
dump.mjs                       -> NodeJS shell dumper (#!/usr/bin/env node)
scrantic/
  main.mjs                     -> entry: fetch RESOURCE.MAP + RESOURCE.001, show INTRO.SCR, start Story
  story.mjs                    -> Story class: day counter (localStorage), getRandomScene(), play()
  palette.mjs                  -> hardcoded 16-color EGA-ish PALETTE
  metadata/scenes.mjs          -> StoryScenes[] : 10 ACTIVITY.ADS scene definitions
  metadata/types.mjs           -> FlagType / PointType / HeadingType enums
dgds/
  resource.mjs                 -> loadResources() index parser; loadResourceEntry() dispatcher
  graphics.mjs                 -> canvas drawing (drawImage, drawScreen, drawPalette, ...)
  audio.mjs                    -> WebAudio; slices samples out of SCRANTIC.SCR at sampleOffsets[]
  compression.mjs              -> dispatch: None/RLE/LZW/RLE2
  compression/lzw.mjs          -> 9..12-bit LZW decoder
  compression/rle.mjs          -> RLE decoder
  compression/rle2.mjs         -> NOT IMPLEMENTED (throws)
  data/scripting.mjs           -> TTMCommandType[] + ADSCommandType[] opcode tables (parse-time)
  resources/ads.mjs            -> ADS (animation sequence) parser
  resources/ttm.mjs            -> TTM (scripting macro) parser
  resources/bmp.mjs            -> BMP (multi-image sprite sheet) parser
  resources/scr.mjs            -> SCR (full-screen background) parser
  resources/pal.mjs            -> PAL (256-color VGA palette) parser
  scripting/process.mjs        -> THE INTERPRETER (~915 lines): runtime opcode execution + main loop
  utils/string.mjs             -> getString() null-terminated reader
  utils/dump.mjs               -> dump helpers (samples/images/scripts/index)
```

### dgds-viewer (`src/`)
```
index.js                       -> ReactDOM.render(<App/>)
constants.js                   -> INDEX_STRING_SIZE=12, PALETTE (same 16 colors as castaway)
global.js                      -> RESOURCES = { PALETTE }  (mutable global; palette swapped at runtime)
compression/{index,lzw,rle,rle2}.js   -> identical algorithms to castaway
resources/
  index.js                     -> ResourceType[] (+ many NOP types), loadResourceMap, loadResourcebyName, loadResources
  {ads,ttm,bmp,scr,pal}.js     -> parsers (functionally identical to castaway, palette via RESOURCES.PALETTE)
  data/scripting.js            -> TTMCommandType[] + ADSCommandType[] (parse-time tables)
scripting/process.js           -> THE INTERPRETER (~1017 lines): runtime opcode execution + main loop
graphics/index.js              -> canvas drawing; drawPalette() also sets RESOURCES.PALETTE
audio/index.js                 -> WebAudio; loads PRE-CONVERTED data/castaway/samples/sampleN.aac
utils/string.js                -> getString()
utils/preload.js               -> preloadFileAsync() XHR arraybuffer loader (+progress events)
dump/index.js                  -> NodeJS dumper (babel-node), also dumps .wav not used here
electron/index.js              -> Electron main process (BrowserWindow 920x715)
server/index.js                -> Express + webpack-dev-middleware dev server (port 8585)
server/index-ssr.jsx           -> static HTML shell (loads Semantic-UI CDN, bundle.js)
ui/App.jsx, Viewer.jsx, ViewerApp.jsx
ui/components/ResourceList.jsx, ResourceItems.jsx, ResourceContent.jsx, ResourceView.jsx, ScriptCode.jsx
ui/params.js                   -> URL hash param parsing (mobile/editor)
```

### Build / runtime targets
- **castaway**: NO package.json, NO build step. Pure native ES modules in the browser. Run: `cd src; http-server -c-1; open localhost:8080`. Data files placed in `src/data/`: `SCRANTIC.SCR`, `RESOURCE.MAP`, `RESOURCE.001`. `.prettierrc.js`: tabWidth 4, semi, singleQuote, printWidth 120.
- **dgds-viewer**: `package.json` main = `src/electron/index.js`. Toolchain: React 16.13, webpack 4, babel (`@babel/preset-env`, `@babel/preset-react`, `babel-preset-minify` with `mangle:false`), Electron 8, Express + `webpack-dev-middleware`, lodash, async, semantic-ui-react. Scripts: `start` (concurrently server + electron), `server` (babel-node + nodemon), `dump`, `build:electron` (electron-builder). `webpack.config.js` entry `@babel/polyfill + ./src/index.js`, output `dist/bundle.js`, `optimization.minimize:false`. browserslist production `>0.2%, not dead, not op_mini all`.

---

## 2. DGDS resource format (as decoded by the parsers)

### 2.1 Resource index (RESOURCE.MAP / VOLUME.*) — `docs/resindex.md` (IDENTICAL in both repos)

> Both repos ship the *same* `docs/resindex.md` (rev 0.0.1, 2018-11-09, author "Alexandre Fontoura aka xesf"). Full content reproduced:

```
# Resource Index File Format [rev 0.0.1]
Resource Map is a file containing index information about resources.
It allow us to identify which resource files need to be imported and the details
of each entry of that resource file.

# Engine
Dynamix Game Development System (DGDS) - an engine originally created by Dynamix
based on Sierra pre-existing engine.

## Games
- Johny Castaway Screen Saver
- Quarky & Quaysoo's Turbo Science,
- Heart of China,
- The Adventures of Willy Beamish,
- Rise of the Dragon

# Format
It is composed by:
- Header
- Resource List
    - Resource Entries

## Header  (static, 6 bytes)
- u8 unknow0
- u8 unknow1
- u8 unknow2
- u8 unknow3
- u8 numResources   (number of resources files available in this index)
- u8 unknow5

## Resource List  (for each numResources)
- *u8 name          (resource file name; static 13 chars; last byte 0 terminator)
- u16 numEntries    (number of entries the resource file has)

## Resource Entry Header  (for each numEntries)
- u32 length        (DECOMPRESSED entry size)
- u32 offset        (offset of the entry inside the Resource file)
Note: compressed size = next_offset - current_offset.

# Document History
- v0.0.1 2018-11-09: First document draft

# Author
Alexandre Fontoura aka xesf  (xesfnet@gmail.com, github.com/xesf)

## References
- Hans Milling aka nivs1978 (github) .../Johnny-Castaway-Open-Source/blob/master/JCOS/Map.cs
- Vasco Costa aka vcosta (github) .../scummvm/wiki
```

**Important:** the resindex.md is a *format spec only*. It does NOT contain a listing of the actual data files or the scenes — there is no enumeration of original game files or scene-to-file mapping in either repo's docs. The concrete data-file names live in the code (Section 5) and in the SCRANTIC metadata (Section 4).

#### Actual index parsing (code differs slightly from the doc)
`castaway/src/dgds/resource.mjs:25-94` `loadResources(buffer, resbuffer)`:
- `INDEX_HEADER_SIZE = 6`, `INDEX_STRING_SIZE = 12` (note: doc says 13-char name; code reads 12 then `+13` for numEntries offset, i.e. name field is effectively 13 bytes incl. terminator).
- Header fields: `unk0,unk1,unk2,unk3,numResources,unk5` (`resource.mjs:30-37`).
- Per resource file: name (12), `numEntries = getUint16(innerOffset+13)`, then `innerOffset += 15`.
- Per entry, the MAP gives `entrySize = getUint16(innerOffset)` (uncompressed size) and `entryOffset = getUint32(innerOffset+4)`; `innerOffset += 8` (so each MAP entry record is 8 bytes: 2-byte size + (pad) + 4-byte offset).
- The actual NAME, COMPRESSED size and data live in RESOURCE.001 at `entryOffset`: name(12), `compressedSize = getUint32(entryOffset+13)`, payload starts at `entryOffset+17`.
- Each parsed entry = `{ name, type (ext after '.'), size (uncompressed), offset, compressedSize, buffer (sliced ArrayBuffer), data (DataView) }`.
- Returns `{ header, resources, getResource(name) }`. Each `res` also gets `getEntry(name)` and `loadEntry(name)` helpers.

`dgds-viewer/src/resources/index.js` is the superset:
- `loadResourceMap(buffer)` — reads only the MAP (no RESOURCE file) into `{header, resources:[{name,numEntries,entries:[{type,size,offset}]}]}` (note: has a bug — `entry.type = name.split('.')[1]` references the outer `name` global, line 78, because the entry name isn't known until the .RMF/.001 is read).
- `loadResourcebyName(resource, resbuffer)` — second pass: resolves each entry's `name/type/compressedSize/buffer/data` from the resource volume. (Has a duplicate-push bug: iterates `res.entries[e]` while also pushing to `res.entries`.)
- `loadResources(buffer, resbuffer)` — single-pass combined version, essentially identical to castaway's.

### 2.2 ResourceType dispatch
castaway `resource.mjs:12-18` — 5 handled types:
```
ADS -> loadADSResourceEntry  (Animation sequences)
BMP -> loadBMPResourceEntry  (Various raw images / sprite sheets)
PAL -> loadPALResourceEntry  (VGA palette)
SCR -> loadSCRResourceEntry  (Background raw images / full screens)
TTM -> loadTTMResourceEntry  (Scripting macros)
```
dgds-viewer `resources/index.js:12-35` adds many **NOP** placeholder types (recognized but not parsed), reflecting the broader DGDS games it browses:
```
VIN, SDS, FNT, DAT, DDS, TDS, REQ, WLD, SNG, ADL, ADH, RST, OVL, GDS, SX, RES  -> NOP
```

### 2.3 Per-resource block layouts (from the parsers)

**TTM** (`ttm.mjs` / `ttm.js`, identical logic):
- Header chunk `VER` + u32 versionSize + version string ("4.09").
- `PAG` chunk: u32 numPages, u16 pagUnknown02.
- `TT3` chunk: u32 blockSize, u8 compressionType, u32 uncompressedSize, then `blockSize-5` bytes of compressed bytecode → `decompress()` → re-wrapped as `DataView(new Int8Array(data).buffer)`.
- `TTI` chunk: 2x u16 unknown.
- `TAG` chunk: u32 size, u16 numTags, then numTags × {u16 id, null-terminated description}.
- The decompressed TT3 stream is the TTM bytecode (parsed in Section 3.4).

**ADS** (`ads.mjs` / `ads.js`):
- `VER` + version.
- `ADS` chunk: 2x u16 unknown.
- `RES` chunk: u32 size, u16 numResources, then numResources × {u16 id, name}. (This is the list of TTM files this ADS sequences — e.g. an ACTIVITY.ADS references ACTIVITY.TTM-style resources.)
- `SCR` chunk: u32 blockSize, u8 compressionType, u32 uncompressedSize, compressed ADS bytecode → decompress.
- `TAG` chunk: like TTM (id + description per scene/sequence).

**BMP** (`bmp.mjs` / `bmp.js`): header `BMP` + u16 width,height (noted "weird values, not used"); `INF` chunk → u16 numImages; then numImages u16 widths, then numImages u16 heights; `BIN` chunk → u32 blockSize, u8 compression, u32 uncompressed, compressed data. Pixels are **4-bit packed** (two pixels per byte: high nibble first, then low nibble) indexing the 16-color PALETTE. Each pixel stored as `{index, a, r, g, b}` and a parallel `buffer[]` of raw indices. dgds-viewer guards `if (block === 'BIN')` (graceful) vs castaway's throw.

**SCR** (`scr.mjs` / `scr.js`): header `SCR` + u16 totalSize + u16 flags; `DIM` chunk → u32 size, u16 width, u16 height; `BIN` chunk → compression + data. Single image, same 4-bit packed → PALETTE decode. Always 640×480-ish full screens. dgds-viewer guards `if (block === 'DIM')` and returns null otherwise.

**PAL** (`pal.mjs` / `pal.js`): header `PAL`, skip 8, `VGA` chunk, then **256** RGB triples (u8 each), scaled `*4` (6-bit VGA → 8-bit), alpha 255. Returns `{palette:[{index,a,r,g,b}]}`. In dgds-viewer, `drawPalette()` ALSO assigns `RESOURCES.PALETTE = data.palette` (so subsequent BMP/SCR use the game's real palette). castaway never uses PAL at runtime — it hardcodes the 16-color screensaver palette.

### 2.4 Compression — `compression.mjs` / `compression/index.js`
```
index 0 None  -> returns data as-is (passthrough)
index 1 RLE   -> decompressRLE
index 2 LZW   -> decompressLZW
index 3 RLE2  -> decompressRLE2  (NOT IMPLEMENTED — throws 'Decompress Type RLE not implemented')
```
- **RLE** (`rle.mjs`): control byte; if high bit (0x80) set → run of `(control & 0x7F)` copies of next byte; else literal copy of `control` bytes. Straightforward.
- **LZW** (`lzw.mjs`): variable-width LZW, **9→12 bits**, code 256 = reset/clear (with bit-alignment skip via `numBits<<3`), freeEntry starts at 257, code table cap 4096, decode stack. Reads bits LSB-first via `getBits()`. Robust against running off the end (`current=0`). This is the heavy lifter for sprites/screens/scripts.
- LZW and RLE source is **byte-identical** between the two repos (only `export const fn=` vs `export function fn`).

---

## 3. THE SCRIPTING SYSTEM (TTM + ADS)

DGDS has a two-level scripting model:
- **TTM** = "macro"/movie scripts: low-level per-frame drawing & timing opcodes that animate one *scene* (sprite blits, clips, delays, sound). Comparable to a display list / animation program.
- **ADS** = "animation sequence" / director scripts: high-level scheduler that decides WHICH TTM scenes to play, with conditionals (IF_PLAYED / IF_RUNNING), randomized selection, add/stop scene, etc.

Two distinct opcode roles exist in the code:
1. **Parse-time tables** in `data/scripting.mjs` (`TTMCommandType[]`, `ADSCommandType[]`) — map opcode → human name (and for ADS, `paramSize` + `indent`). Used by the parsers to disassemble bytecode into `{opcode, params, line, ...}`.
2. **Runtime handlers** in `scripting/process.mjs` (`CommandType[]` array in castaway, `CommandType2{}` object in viewer) — map opcode → JS function that actually mutates the canvas/state.

### 3.1 TTM opcode table (parse-time, `data/scripting.mjs`)

`opcode & 0x000f` is the **param count** (low nibble), `opcode & 0xfff0` is the actual opcode (see `ttm.mjs:80-81`). Params are signed int16. Below is the merged table with both repos' names (castaway name / viewer name where they differ) and the runtime meaning:

| Opcode | Name (castaway data) | Name (viewer data) | Runtime meaning (process.mjs) |
|--------|----------------------|--------------------|-------------------------------|
| 0x0020 | SAVE_BACKGROUND | SAVE_BACKGROUND | NOP (unused) |
| 0x0080 | DRAW_BACKGROUND | DRAW_BACKGROUND | redraw island/ocean background |
| 0x0110 | PURGE | PURGE | NOP (commented `state.purge=true`) |
| 0x0FF0 | UPDATE | UPDATE | **frame commit + delay/timer gate** (see 3.3) |
| 0x1020 | SET_DELAY | SET_DELAY | `state.delay = (delay||1)*20` ms |
| 0x1050 | SLOT_IMAGE | SLOT_IMAGE | `state.slot = slot` (active image slot) |
| 0x1060 | SLOT_PALETTE | SLOT_PALETTE | NOP |
| 0x1100 | UNKNOWN_0 (Scene?) | UNKNOWN_0 | NOP |
| 0x1110 | SET_SCENE | SET_SCENE | NOP at runtime; **at parse-time delimits scenes by tag** |
| 0x1120 | SET_BACKGROUND | SET_BACKGROUND | `state.saveIndex = index` |
| 0x1200 | GOTO | GOTO | `state.reentry = 0` (loop restart; "TODO check other scenes") |
| 0x2000 | SET_COLORS | SET_COLORS | set fg/bg from PALETTE if `<16` |
| 0x2010 | SET_FRAME1 | SET_FRAME | NOP |
| 0x2020 | UNKNOWN_3 (SET_FRAME2?) | SET_TIMER | **set timer** (formula differs, see 3.7) |
| 0x4000 | SET_CLIP_REGION | SET_CLIP_REGION | `state.clip = {x,y,w=x2-x1,h=y2-y1}` |
| 0x4110 | FADE_OUT | FADE_OUT | NOP |
| 0x4120 | FADE_IN | FADE_IN | NOP |
| 0x4200 | SAVE_IMAGE0 | DRAW_BACKGROUND_REGION | copy region of context → saveBkg[0] (canDraw) |
| 0x4210 | SAVE_IMAGE1 | SAVE_IMAGE_REGION | NOP (commented out) |
| 0xA000 | UNKNOWN_4 (Draw Line?) | UNKNOWN_4 | NOP (debug rect, commented) |
| 0xA050 | UNKNOWN_5 (Draw Line?) | SAVE_REGION | NOP (commented) |
| 0xA060 | UNKNOWN_6 (Draw Line?) | RESTORE_REGION | clear saveBkg[0] region |
| 0xA0A0 | DRAW_LINE | DRAW_LINE | stroke a white line x1,y1→x2,y2 |
| 0xA100 | DRAW_RECT | DRAW_RECT | fillRect in foregroundColor |
| 0xA400 | DRAW_BUBBLE | DRAW_BUBBLE | filled white circle (speech bubble) |
| 0xA500 | DRAW_SPRITE | DRAW_SPRITE | blit `res[slot].images[index]` at x,y, clipped |
| 0xA510 | DRAW_SPRITE1 | DRAW_SPRITE1 | NOP (unused) |
| 0xA520 | DRAW_SPRITE_FLIP | DRAW_SPRITE_FLIP | horizontally flipped sprite blit |
| 0xA530 | DRAW_SPRITE3 | DRAW_SPRITE3 | NOP (unused) |
| 0xA600 | CLEAR_SCREEN | CLEAR_SCREEN | clear context + tmp + drawContext |
| 0xB600 | DRAW_SCREEN | DRAW_SCREEN | NOP |
| 0xC020 | LOAD_SAMPLE | LOAD_SAMPLE | NOP |
| 0xC030 | SELECT_SAMPLE | SELECT_SAMPLE | NOP |
| 0xC040 | DESELECT_SAMPLE | DESELECT_SAMPLE | NOP |
| 0xC050 | PLAY_SAMPLE | PLAY_SAMPLE | play sound fx index via WebAudio |
| 0xC060 | STOP_SAMPLE | STOP_SAMPLE | NOP |
| 0xF010 | LOAD_SCREEN | LOAD_SCREEN | load SCR background by name; set island type |
| 0xF020 | LOAD_IMAGE | LOAD_IMAGE | load BMP into `res[slot]` (FLAME/FLURRY→FIRE1) |
| 0xF050 | LOAD_PALETTE | LOAD_PALETTE | NOP |

> The 0x1110 (`SET_SCENE`) param low-nibble == 1 carries a tag id; the parser uses it to split the TTM into per-tag scene scripts. (`ttm.mjs:91-105`.)

### 3.2 ADS opcode table (parse-time, `data/scripting.mjs`) — IDENTICAL in both repos

ADS opcodes are read as raw u16 (NOT nibble-masked); each has fixed `paramSize` (count of int16 params) and an `indent` delta for pretty-printing nested IF blocks. Opcodes `<= 0x100` are treated as scene **tag ids** (start of a new sequence), not commands (`ads.mjs:103,140-153`).

| Opcode | Command | paramSize | indent | Runtime meaning (process.mjs) |
|--------|---------|-----------|--------|-------------------------------|
| 0x1070 | UNKNOWN_0 | 2 | null | NOP |
| 0x1330 | IF_NOT_PLAYED | 2 | 1 | NOP (logic commented out) |
| 0x1350 | IF_PLAYED *(// SKIP_NEXT_IF)* | 2 | 1 | conditional: continue if scene(sceneIdx,tagId) already played; queue removal if its timer hit 0 |
| 0x1360 | IF_NOT_RUNNING | 2 | 1 | NOP |
| 0x1370 | IF_RUNNING | 2 | 1 | NOP |
| 0x1420 | AND | 0 | null | NOP |
| 0x1430 | OR | 0 | null | NOP |
| 0x1510 | PLAY_SCENE | 0 | 0 | **commit add/remove scene lists; decide continue** (see 3.6) |
| 0x1520 | PLAY_SCENE_2 | 5 | 0 | NOP |
| 0x2005 | ADD_SCENE | 4 | null | queue scene {sceneIdx,tagId,retriesDelay,unk} (or into random pool) |
| 0x2010 | STOP_SCENE | 3 | null | queue removal of {sceneIdx,tagId} |
| 0x3010 | RANDOM_START | 0 | 1 | begin random-selection block (`state.randomize=true`) |
| 0x3020 | RANDOM_UNKNOWN_0 | 1 | null | NOP |
| 0x30ff | RANDOM_END | 0 | -1 | pick ONE random queued scene → ADD_SCENE it |
| 0x4000 | UNKNOWN_6 | 3 | null | NOP (note: 0x4000 also = TTM SET_CLIP_REGION; the ADS runtime maps it to ADS_UNKNOWN_6) |
| 0xf010 | FADE_OUT | 0 | 0 | NOP (ADS_FADE_OUT) |
| 0xf200 | RUN_SCRIPT | 1 | 0 | NOP |
| 0xffff | END | 0 | (none) | end-of-sequence handling (toggles continue; clears scenes if played) |
| 0xfff0 | END_IF | 0 | (none) | **synthetic** — injected by parser to close IF/RANDOM indent blocks |

> Note the opcode-space collision between TTM and ADS: 0x2010 is TTM `SET_FRAME1`/`SET_TIMER` but ADS `STOP_SCENE`; 0x4000 is TTM `SET_CLIP_REGION` but ADS `UNKNOWN_6`; 0xf010 is TTM `LOAD_SCREEN` but ADS `FADE_OUT`. The runtime resolves this purely by `state.type` ('TTM' vs 'ADS') choosing which script stream to run — the SAME `CommandType` table holds both, with the later (ADS) definitions overwriting in object-lookup form. In castaway's `CommandType` array, `find()` returns the FIRST match, so for an opcode present in both, the TTM handler wins; in viewer's `CommandType2` object the LAST key wins (ADS handler). This is a real behavioral divergence (see 3.7).

### 3.3 TTM bytecode disassembly (`ttm.mjs` / `ttm.js`)
Loop over decompressed TT3 bytes (`ttm.mjs:78-140`):
- read u16, `size = opcode & 0x000f`, `opcode &= 0xfff0`.
- If `opcode==0x1110 && size==1`: read u16 tagId, attach matching tag description, **push previous sceneScripts as a scene** keyed by `prevTagId`, reset, set `prevTagId=tagId`. (This is how a TTM file is sliced into named scenes.)
- Else if `size==15` (0xF): read a null-terminated **string** param (used by LOAD_SCREEN/LOAD_IMAGE etc.), consume up to two trailing zero bytes.
- Else: read `size` signed int16 params.
- Build `command.line` text via TTMCommandType lookup; push to both `scripts[]` (flat) and `sceneScripts[]` (current scene).
- Returns `{name,type,numPages,...,tags,buffer,scripts,scenes}` where `scenes=[{tagId,script[]}]`.

### 3.4 ADS bytecode disassembly (`ads.mjs` / `ads.js`)
Loop (`ads.mjs:91-157`):
- read u16 opcode.
- If found in ADSCommandType AND `opcode>0x100`: read `paramSize` int16 params; manage `indent` (push synthetic `0xfff0 END_IF` lines to close blocks when an indent-0 command like PLAY_SCENE/END appears); push to `sceneScripts`.
- Else (opcode ≤ 0x100): treat as a **scene tag** — close previous scene, start new `sceneScripts`, `prevTagId = command.tag`.
- Returns `{name,type,...,resources,tags,buffer,scripts,scenes}`.

### 3.5 Interpreter step model (`process.mjs` / `process.js`)

Shared global module state (a single in-flight `state`, plus module-level `scenes`, `addScenes`, `removeScenes`, `scenesRandom`, `scenesRes`, `currentScene`, and background caches `bkgScreen/bkgRes/bkgOcean/bkgRaft` + cloud position).

`startProcess(initialState)`:
- builds `state` with `context`, `tmpContext` (offscreen 640×480), `mainContext`, three `save[]` canvases + one `saveBkg[]` canvas, `audioManager`, `clip={0,0,640,480}`, fg/bg = PALETTE[0], `type` ('ADS'|'TTM').
- If `type==='ADS'`: pre-load each `data.resources[]` (the TTM files referenced) into `scenesRes[]` via `loadResourceEntry`.
- starts `mainloop()` (rAF, throttled to 60 fps via `fps = 1000/60`).

`mainloop()` (rAF): each frame calls `runScripts()`; if it returns true (TTM finished), `cancelAnimationFrame`.

`runScripts()`:
- **ADS path**: clear context; draw island background to mainContext; blit saved bkg region; run the *current top-level* scene script `data.scenes[currentScene]` via `runScript(state, scene.script, true)`. When `!state.continue`, also iterate active `scenes[]`, run each sub-scene's TTM `runScript(s.state, s.script)` and composite each onto `state.context`.
- **TTM path**: draw island bg; `runScript(state, state.data.scripts)` (flat program).

`runScript(state, script, main)` — the core stepper (`process.mjs:706-744`):
- iterate from `state.reentry` to end; look up handler by opcode; if `main` log the line; mark `lastCommand` on the final instruction; call `callback(state, ...c.params)`; set `state.reentry=i`; **break if `!state.continue`** (this is how UPDATE/IF/PLAY_SCENE pause execution mid-frame and resume next frame).
- When reaching the last command: set `lastCommand`, reset `reentry=0`, `runs++`, force `continue=true`, `played=true`; if `main` increment `currentScene`; if `type==='TTM'` return true (movie done).

**Stepping / timing / delays:**
- `SET_DELAY` (0x1020): `state.delay = (delay||1) * 20` ms.
- `UPDATE` (0x0FF0): the frame-pacing gate. If `continue` and a delay is set, set `continue=false` and `state.elapsed = delay + Date.now()` (parks the script). Once `Date.now() > state.elapsed`, set `continue=true` and (if lastCommand) mark `played`. So an animation frame is "held" on screen for `delay` ms before the script advances.
- `SET_TIMER` (0x2020): scene lifetime timer (see 3.7 for the formula divergence).

**Branching / loops:**
- `GOTO` (0x1200): `state.reentry = 0` — restarts the script from the top (loop). (Comment notes it doesn't yet handle jumping to arbitrary tags.)
- IF_PLAYED/IF_NOT_PLAYED/IF_RUNNING etc. set `state.continue` to gate whether the following block executes; `END`/`END_IF` adjust it back.

**Scene selection / scheduling (ADS director):**
- `ADD_SCENE` (0x2005): pushes {sceneIdx, tagId, retriesDelay, unk} to `addScenes` (or to `scenesRandom` when inside a RANDOM block).
- `STOP_SCENE` (0x2010): pushes to `removeScenes`.
- `RANDOM_START`/`RANDOM_END` (0x3010/0x30ff): collect candidate scenes, then `RANDOM_END` picks ONE at random and ADD_SCENEs it — this is the core "pick a random Johnny activity" mechanism.
- `PLAY_SCENE` (0x1510): the commit point — applies queued removals (lodash `remove` in viewer / `splice` in castaway), then queued additions via `getSceneState()`, then computes `canContinue` (true if any active scene has `runs>0`, or no scenes).
- `getSceneState(state, sceneIdx, tagId, retriesDelay, unk)`: looks up the TTM `scenesRes[sceneIdx-1]`, finds the scene with matching `tagId`, creates a fresh offscreen canvas + a per-scene `state` (cloned from initialState), and for the FIRST scene prepends `ttm.scenes[0].script` (the common setup/prologue) to the scene script. Each active scene runs its own TTM program on its own canvas; they are composited together each frame.
- `END` (0xffff): toggles continue; if `lastCommand` and a scene has played, clears `scenes` and continues (restart the director).

### 3.6 Rendering targets (HTML Canvas)
- All drawing is **2D Canvas**. Story/main use two stacked `<canvas>` 640×480 (`canvas` for sprites zIndex 1, `mainCanvas` for background zIndex 0 — see `ResourceView.jsx:85-86` and `story.mjs`).
- `graphics.drawImage(image, ctx, x, y)`: builds an `ImageData` from each pixel's RGBA and `putImageData`. `drawScreen` paints a full 640×480 SCR. `drawAllImages` lays a BMP's sub-images side-by-side (for the viewer's BMP preview). `drawPalette` paints color swatches (and in viewer, captures the palette globally).
- Sprite blits go: decode pixels → `tmpContext` via `putImageData` → `context.drawImage(tmpCanvas, sx,sy,sw,sh, dx,dy,dw,dh)` with a `clip()` rect. `DRAW_SPRITE_FLIP` uses `translate+scale(-1,1)` for mirroring.
- Background composition (`drawBackground`) hand-places island pieces from `BACKGRND.BMP` (isle img[0], palm imgs 12/13/14, shore imgs 3/6/10), a raft from `MRAFT.BMP` (img[3]), and an animated cloud (random index 15–17, drifting left). `state.island` (1 or 2) selects island X position (288 vs 16). Ocean frames OCEAN00/01/02 + NIGHT chosen randomly.

### 3.7 castaway vs dgds-viewer — interpreter differences (concrete)
The two `process.*` files are ~95% identical (same handler bodies, same state machine). Differences:
1. **Dispatch structure**: castaway uses `const CommandType = [ {opcode, callback}, ... ]` and `CommandType.find(ct => ct.opcode === c.opcode)` (`process.mjs:642-704,712`). dgds-viewer keeps that array commented out and instead uses `const CommandType2 = { '0x0020': fn, ... }` with `CommandType2['0x' + opcode.toString(16).padStart(4,'0').toUpperCase()]` (`process.js:742-804,813`). The object form is O(1) and is the meaningful perf/clarity improvement; but it changes collision resolution (last-key-wins → ADS handler wins for 0x2010/0x4000/0xf010, whereas castaway's find() returns first → TTM handler wins).
2. **SET_TIMER (0x2020) formula DIVERGES**:
   - castaway `process.mjs:169-172`: `state.timer = timer*20 + (delay||1)*20;` (does NOT set state.delay).
   - viewer `process.js:173-176`: `state.delay = (delay||1)*20; state.timer = timer*20;` (sets BOTH).
3. **Palette source**: castaway imports a fixed `PALETTE` from `scrantic/palette.mjs`; viewer reads `RESOURCES.PALETTE` (mutable global, swapped when a PAL is viewed). For Johnny Castaway both are the same 16 colors.
4. **Scene removal impl**: viewer uses lodash `remove(scenes, ...)` (correct); castaway uses a broken `scenes.indexOf(predicate)`+`splice` in PLAY_SCENE (`process.mjs:483-485`) — i.e. castaway's removal logic is buggier/less complete.
5. **IF_PLAYED**: castaway has an extra fallback branch (`process.mjs:462-466`) that sets `continue=true` when the scene isn't found at all; viewer lacks it.
6. **Logging / dev hooks**: viewer logs more (`console.log('elapsed'...)`, before/after remove dumps) and accepts a `callback: updateScriptLine` to highlight the current line in the on-screen `ScriptCode` table; castaway has no script-view UI.
7. **Imports**: viewer imports `remove` from lodash and `RESOURCES` from `../global`; castaway imports `PALETTE` directly. Otherwise the handler bodies are line-for-line identical.

The parse-time `data/scripting.*` tables differ only in **labels/comments**, not opcodes/params: castaway marks 0x2020 as `UNKNOWN_3 (SET_FRAME2?)`, 0x4200/0x4210 as `SAVE_IMAGE0/1`, 0xA050/0xA060 as `UNKNOWN_5/6`, 0x2010 as `SET_FRAME1`; dgds-viewer gives the cleaner names `SET_TIMER`, `DRAW_BACKGROUND_REGION`, `SAVE_IMAGE_REGION`, `SAVE_REGION`, `RESTORE_REGION`, `SET_FRAME`. dgds-viewer is the more *documented* table.

---

## 4. SCRANTIC metadata & story/day logic (castaway only)

> dgds-viewer has NO scrantic metadata — it is a generic resource browser. All scene/story scheduling metadata lives in **castaway/src/scrantic/**.

### 4.1 Palette — `scrantic/palette.mjs` (full data)
16-color palette (note index 0 is transparent: a:0). Identical array also in `dgds-viewer/src/constants.js`.
```
 0  {a:0,   r:168,g:0,  b:168}   (transparent magenta key)
 1  {a:255, r:0,  g:0,  b:168}   blue
 2  {a:255, r:0,  g:168,b:0}     green
 3  {a:255, r:0,  g:168,b:168}   cyan
 4  {a:255, r:168,g:0,  b:0}     red
 5  {a:255, r:0,  g:0,  b:0}     black
 6  {a:255, r:168,g:168,b:0}     yellow/olive
 7  {a:255, r:212,g:212,b:212}   light gray
 8  {a:255, r:128,g:128,b:128}   dark gray
 9  {a:255, r:0,  g:0,  b:255}   bright blue
10  {a:255, r:0,  g:255,b:0}     bright green
11  {a:255, r:0,  g:255,b:255}   bright cyan
12  {a:255, r:255,g:0,  b:0}     bright red
13  {a:255, r:255,g:0,  b:255}   bright magenta
14  {a:255, r:255,g:255,b:0}     bright yellow
15  {a:255, r:255,g:255,b:255}   white
```

### 4.2 Enum types — `scrantic/metadata/types.mjs` (full data)
```
FlagType   = { NONE:0x00, STORY_SCENE:0x01, SPECIAL_SCENE:0x02, LOW_TIDE:0x04,
               LEFT_ISLAND:0x08, NO_SEQUENCE:0x10, NO_ISLAND:0x20, NO_RAFT:0x40 }  // bit flags
PointType  = { A:0, B:1, C:2, D:3, E:4, F:5 }                  // island location points
HeadingType= { S:0, SW:1, W:2, NW:3, N:4, NE:5, E:6, SE:7 }    // 8-way facing
```

### 4.3 Scene enumeration — `scrantic/metadata/scenes.mjs` (full data)
`defaultScene = { scene:'', storyDay:0, tag:0, description:'', startPoint:A, startHeading:S, endPoint:A, endHeading:S, flags:NONE }`.

`StoryScenes[]` — **10 entries, all `name:'ACTIVITY.ADS'`**, "order as per ADS scene tags". (This is the only scene metadata defined; the broader story-day scheduling is NOT data-driven yet — `Story.getRandomScene()` just picks uniformly at random from these 10.)

| # | name | tag | description | startPoint | startHeading | endPoint/Heading | flags |
|---|------|-----|-------------|-----------|--------------|------------------|-------|
| 1 | ACTIVITY.ADS | 1  | GAG DIVES          | E | SE | (A,S default) | SPECIAL_SCENE |
| 2 | ACTIVITY.ADS | 12 | GULL 3 STILL READING | D | SW | (default) | SPECIAL_SCENE \| LOW_TIDE |
| 3 | ACTIVITY.ADS | 11 | GULL 2 BATHING     | A | S  | (default) | SPECIAL_SCENE \| NO_SEQUENCE |
| 4 | ACTIVITY.ADS | 10 | GULL 1 READING     | D | SW | (default) | SPECIAL_SCENE \| LOW_TIDE |
| 5 | ACTIVITY.ADS | 4  | MUNDANE DIVE       | E | SE | E, SE | LOW_TIDE |
| 6 | ACTIVITY.ADS | 5  | NATIVE 1           | E | SW | (default) | SPECIAL_SCENE \| LOW_TIDE |
| 7 | ACTIVITY.ADS | 6  | GAG JOHN READ      | D | SW | (default) | SPECIAL_SCENE |
| 8 | ACTIVITY.ADS | 7  | MUNDANE JOHN READ  | D | SW | F, SW | LOW_TIDE |
| 9 | ACTIVITY.ADS | 8  | JOHN BATH          | A | S  | D, SE | NO_SEQUENCE |
|10 | ACTIVITY.ADS | 9  | NATIVE 3           | E | E  | (default) | SPECIAL_SCENE \| LOW_TIDE |

(The `tag` here corresponds to the ADS scene tag id inside ACTIVITY.ADS. Note the list is not in numeric tag order; it's "ADS scene tag order" as authored. The flags drive intended behavior — e.g. LOW_TIDE scenes need the tide out, NO_SEQUENCE scenes aren't part of a story chain, SPECIAL_SCENE marks gag/special animations.)

### 4.4 Story / day logic — `scrantic/story.mjs` (full behavior)
```js
class Story {
  constructor(resource) {
    this.currentDay = localStorage.getItem('currentDay') || 1;
    this.startDate  = localStorage.getItem('startDate') || (new Date()).toLocaleDateString();
    this.resource = resource;
  }
  getRandomScene() { return StoryScenes[ floor(random()*StoryScenes.length) ]; }
  async play() {
    if (this.startDate !== today) this.currentDay += 1;       // advance "story day" once per calendar day
    localStorage.setItem('currentDay', this.currentDay);
    localStorage.setItem('startDate', this.startDate);
    // clear canvas + mainCanvas
    const scene = this.getRandomScene();                       // pick a random ACTIVITY scene
    const data  = this.resource.loadEntry(scene.name);         // load ACTIVITY.ADS
    startProcess({ type:'ADS', context, mainContext, data, entries: this.resource.entries });
  }
}
```
- "Day" is tracked in `localStorage` and incremented when the calendar date changes (the README roadmap wants a richer 24h day/night and real story-sequence playback; currently it just bumps a counter).
- `getRandomScene()` chooses uniformly among the 10 StoryScenes — there is **no** implemented day→scene schedule or full ordered story sequence yet (that's on the roadmap: "Play Full Story Sequence / Choose single activities").

### 4.5 Entry flow — `scrantic/main.mjs`
`run()`: fetch `data/RESOURCE.MAP` + `data/RESOURCE.001` → `loadResources()` → `getResource('RESOURCE.001')` → load & `drawScreen('INTRO.SCR')` on mainCanvas → wait 1s (localhost) or 3s → `new Story(resource).play()`.

---

## 5. Concrete data-file / resource names referenced in code

These are the actual original Johnny Castaway data files and resource entries the runtime expects (gathered from `process.mjs`, `main.mjs`, `audio.mjs`):

- **Container files** (placed in `src/data/`): `RESOURCE.MAP` (index), `RESOURCE.001` (resource volume), `SCRANTIC.SCR` (audio sample bank).
- **Screens (SCR)** — `SCREEN_TYPE` map (`process.mjs:341-348`):
  - `ISLETEMP.SCR` → island type 1
  - `ISLAND2.SCR` → island type 2
  - `SUZBEACH.SCR`, `JOFFICE.SCR`, `THEEND.SCR`, `INTRO.SCR` → type 0 (non-island)
  - Ocean/night backgrounds: `OCEAN00.SCR`, `OCEAN01.SCR`, `OCEAN02.SCR`, `NIGHT.SCR` (chosen at random; index 4 reserved for night).
- **Bitmaps (BMP)**: `BACKGRND.BMP` (island scenery sprite sheet), `MRAFT.BMP` (raft), `FIRE1.BMP` (and aliases `FLAME.BMP`, `FLURRY.BMP` are remapped to FIRE1).
- **ADS director**: `ACTIVITY.ADS` (the activity sequencer referenced by all StoryScenes).
- dgds-viewer additionally knows other games' index files (`ViewerApp.jsx:12-18`): castaway/turbosci → `RESOURCE.MAP`; Heart of China / Willy Beamish → `VOLUME.RMF`; Rise of the Dragon → `VOLUME.VGA`.

---

## 6. Audio handling

### castaway — `dgds/audio.mjs`
- WebAudio (`AudioContext` / webkit fallback). One reusable `sfxSource` with `gainNode` + `BiquadFilter` ('allpass').
- **Samples are sliced directly out of `data/SCRANTIC.SCR`** at hardcoded byte offsets `sampleOffsets[]` (index 0 and 11 are -1 = none):
```
[-1, 0x1DC00, 0x20800, 0x20E00, 0x22C00, 0x24000, 0x24C00, 0x28A00, 0x2C600,
 0x2D000, 0x2DE00, -1, 0x34400, 0x32E00, 0x39C00, 0x43400, 0x37200, 0x37E00,
 0x45A00, 0x3AE00, 0x3E600, 0x3F400, 0x41200, 0x42600, 0x42C00, 0x43400]
```
- `source.load(index, cb)`: skips if index ≤ -1 / already playing / offset == -1; reads size = `getInt32(off+4)+8` (RIFF chunk size + 8 header), slices `[off, off+size]`, `decodeAudioData`, caches per index, then plays. Volume ramped via `gain.setValueAtTime(volume, currentTime+1)`.
- `PLAY_SAMPLE` (0xC050) handler calls `audioManager.getSoundFxSource().load(index, () => source.play())`.
- The Node dumper (`utils/dump.mjs:dumpSamples`) writes each slice as `sampleN.wav` (so the offsets point at embedded RIFF/WAV blobs in SCRANTIC.SCR).

### dgds-viewer — `audio/index.js`
- Same WebAudio source structure, BUT loads **pre-converted external files**: `data/castaway/samples/sample${index}.aac` via `XMLHttpRequest`+`decodeAudioData` (no SCRANTIC.SCR slicing at runtime). This is the more browser-friendly approach (no need to ship/parse the raw sample bank).

---

## 7. dgds-viewer UI / shell (no castaway analog)

- **App → Viewer → ViewerApp** React tree. `Viewer.jsx` is a top menu to switch game (castaway / turbosci / hoc / willy / dragon). `ViewerApp.jsx` loads the chosen game's index (`data/{game}/{RESOURCE.MAP|VOLUME.RMF|VOLUME.VGA}`) via `loadResourceMap`, renders `ResourceList` (left) + `ResourceContent` (right).
- `ResourceList` → expandable per-resource-file; `ResourceItems` lazily loads each volume (`loadResourcebyName`) and lists entries as `#entry=game,resfile,entryname` hash links.
- `ResourceContent` listens to `hashchange`, parses `game,resource,name`, `loadResourceEntry(entry)`, renders `ResourceView`.
- `ResourceView` dispatches by type: BMP→drawAllImages, PAL→drawPalette, SCR→drawScreen, ADS/TTM→`nop` preview + `startProcess()` (animated) and shows `ScriptCode` (the disassembled, indented, line-numbered, tag-highlighted bytecode with current-line highlight). VIN→nop.
- `ScriptCode.jsx` renders the `scripts[]` as an HTML table; tags shown bold/red, indent × 20px, current line highlighted blue (`rgb(0,136,253)`).
- **Electron** (`electron/index.js`): BrowserWindow 920×715 loading `http://localhost:8585` (dev) or `build/index.html` (prod). **Express SSR dev server** (`server/index.js`) on 8585 with webpack-dev-middleware, serves `/assets` and `/data` statically; `index-ssr.jsx` is the HTML shell pulling Semantic-UI from CDN.

---

## 8. Reusability assessment for a modern web port

**Directly reusable (high value):**
- **The DGDS format parsers** (`resource`, `resources/{ads,ttm,bmp,scr,pal}`, `compression/{rle,lzw}`, `utils/string`) are clean, dependency-free, DataView-based, and work in any modern JS runtime (browser/Node/Deno/Bun). They cleanly separate "decode bytes → structured `{tags, scripts, scenes, images, palette}`". This is the most valuable, portable layer.
- **The two opcode tables** (`data/scripting`) and the disassembly logic are a ready-made spec of the TTM/ADS bytecode — invaluable as ground truth for a reimplementation regardless of language.
- **The SCRANTIC metadata** (palette, FlagType/PointType/HeadingType, the 10-scene StoryScenes table with start/end points, headings, and behavior flags) is hand-curated reverse-engineering data worth preserving verbatim.
- The 16-color palette, the sample offset table, and the island/background sprite-index placement constants are concrete magic numbers a port must reproduce.

**Reusable with caveats:**
- **The interpreter (`process.*`)** is the weakest part: a single big mutable module-level `state` plus several module-level arrays (`scenes`, `addScenes`, `removeScenes`, ...) — not re-entrant, hard to test, littered with commented-out experiments, `console.log`s, and TODO/FIXME ("this state needs a deep clean up", "Improve this code repetition"). Many opcodes are NOPs (FADE, PURGE, SAVE/RESTORE region, most sample ops). The ADS scheduler (IF_PLAYED/STOP_SCENE/PLAY_SCENE) is admittedly buggy (castaway's `indexOf(predicate)` removal is wrong). A modern port should treat `process.*` as a *reference for opcode semantics and the frame/delay model* and rewrite the orchestration with an explicit per-scene VM (instruction pointer, delay timer, scene list) rather than copy it.
- Rendering is plain 2D Canvas (`putImageData` per sprite then `drawImage` blit). Correct but not GPU-efficient; fine to reuse for a faithful port, or swap for WebGL/WebGPU/a texture atlas later. Decoding pixels to per-pixel `{index,a,r,g,b}` objects is memory-heavy and should be replaced with typed arrays / ImageBitmap for performance.
- Audio: prefer dgds-viewer's pre-converted-`.aac` approach (or decode the SCRANTIC.SCR RIFF slices once and cache as AudioBuffers) over re-slicing each play.

**Which project is more complete?**
- **dgds-viewer is the more complete & polished codebase overall**: object-based O(1) opcode dispatch, cleaner opcode names, defensive parsing (guards instead of throws), lodash-based correct scene removal, runtime palette loading, a full React/Electron UI with a live bytecode disassembler/highlighter, multi-game support, a Node dumper, and an SSR dev server. It is the better *engine + tooling*.
- **castaway is more complete as an actual *Johnny Castaway product***: it is the ONLY one with the SCRANTIC story layer (`scenes.mjs`, `story.mjs`, `types.mjs`, `main.mjs`), the day counter, the INTRO→Story boot flow, the island/ocean/cloud/raft background compositor wired to `state.island`, and runtime audio straight from SCRANTIC.SCR. dgds-viewer can *play/preview* TTM/ADS resources but has no game/story orchestration.
- They share ~90% of the parsing + interpreter code (often byte-identical). For a Wilson Reborn port: take **dgds-viewer's parsers + opcode tables + dispatch shape**, plus **castaway's SCRANTIC metadata + story/day/background/audio logic**, and rewrite the interpreter core cleanly.

**Notable gaps a port must finish:**
- RLE2 decompression is unimplemented in both (throws). Needs implementing if any Johnny Castaway resource uses compression type 3.
- The full story SCHEDULE (which scenes on which day, ordered sequences, festive/holiday days) is NOT data-driven yet — only a 10-entry random pool exists. The original's day-by-day storyline must be reconstructed.
- Many TTM opcodes are NOPs (fades, region save/restore, sample select/stop) — visual/audio fidelity gaps.
- The ADS conditional engine (IF_PLAYED / IF_RUNNING / AND / OR / timers) is only partially wired; proper "play each activity once, advance the story" logic needs a correct implementation.
