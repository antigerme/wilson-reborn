# JCOS — Johnny Castaway Open Source (C# / WinForms) — Deep Technical Notes

Source: `/home/user/wilson-reborn/repos/Johnny-Castaway-Open-Source`
Author: Hans Milling (nivs1978@gmail.com), GPLv3, dated 2015. AssemblyVersion `0.0.0.2`.
Root namespace / assembly name: **`SCRANTIC`** (the internal name of the original Dynamix engine).
Target: .NET Framework **v2.0** (`<TargetFrameworkVersion>v2.0</TargetFrameworkVersion>`), `OutputType=WinExe`, x86. Built to `scrantic.exe`, installed/renamed to `SCRANTIC.SCR` (a Windows `.scr` screensaver).

This is an **incomplete work-in-progress**: only a handful of scenes play, the TTM player is partly stubbed, sound is partly wired, no palette animation, hard-coded debug rectangles drawn over everything. Its great value as a reference is that it is the **most heavily documented/annotated port for the binary resource formats and the TTM/ADS opcode tables** — many opcodes carry English names and human-readable `ToString()` descriptions, the DGDS chunk grammar is explicit, and there are 4 distinct decompression methods spelled out. Credits (website index.html) name **xBaK** (Guido) as the source of TTM/ADS command understanding, and Jaap/Kevin & Liam Ryan/Grégori for resource-format and Lempel-Ziv help.

---

## 1. Overall architecture & control flow

### 1.1 Entry point — `Program.cs`
- `static string INIFILE = "ScrAntic.ini"` (line 25).
- `Main(string[] args)` implements the standard Windows screensaver argument protocol (lines 30-84):
  - Parses `args[0]`, lowercased/trimmed. Handles both colon-joined forms (`/c:1234567`, `/P:1234567`) and space-separated forms. If `arg.Length > 2`, it takes `handler = arg.Substring(3)` and `arg = arg.Substring(0,2)`; else `handler = args[1]`.
  - `/c` → **Configuration mode**: `Application.Run(new SettingsForm())`.
  - `/p` → **Preview mode**: parses `handler` to an `IntPtr previewWndHandle` and runs `new JohnnyCastawayForm(previewWndHandle)`. (Note bug: the null-check tests `arg == null` instead of `handler == null`.)
  - `/s` → **Full screensaver mode**: `ShowScreenSaver(); Application.Run();`.
  - No args → Configuration mode (SettingsForm).
- `ShowScreenSaver()` (lines 86-103): iterates `Screen.AllScreens`; for the primary screen creates `JohnnyCastawayForm(screen.Bounds)` and `.Show()`; for every other screen creates a borderless black `Form` covering its bounds (so multi-monitor → blank secondary displays).
- Top-level try/catch shows the exception message + stack trace in a MessageBox.

### 1.2 Main display form — `JohnnyCastawayForm.cs` (+ `.Designer.cs`)
- A borderless 640×480 black form; the only control is a `PictureBox pbScreen` that displays each rendered `Bitmap` frame. (Designer: `ClientSize 640×480`, `BackColor Black`, `FormBorderStyle None`.)
- Three constructors: default; `(Rectangle Bounds)` for full-screen on primary; `(IntPtr PreviewWndHandle)` for preview, which uses P/Invoke `SetParent` / `SetWindowLong(GWL_STYLE=-16, WS_CHILD=0x40000000)` / `GetClientRect` to embed itself in the control-panel preview pane and sets `previewMode = true`.
- **The scene table is HARD-CODED in the form** (lines 39-44). A comment (line 34) states the intent: *"For now the working scripts are hardcoded. In the future they will be read from the ADS files pointed by the FILES.VIN resource."* The list:
  ```csharp
  List<KeyValuePair<string, List<UInt16>>> scenes = {
    { "FISHING.ADS",  { 0x01 } },                        // others 2..8 commented out
    { "WALKSTUF.ADS", { 0x03 } },
    { "MISCGAG.ADS",  { 0x01 } },
    { "ACTIVITY.ADS", { 0x01, 0x0C, 0x06, 0x08, 0x09 } }
  };
  ```
- **Intro animation** (`bwintro_DoWork`, lines 128-151): loads `INTRO.SCR`, then for `i = 0..29` draws the intro image clipped to an expanding circle centred at (320,240) with radius `i*10` (the original screensaver's iris-open intro). `ClipToCircle()` (219-232) uses a `GraphicsPath.AddEllipse` region clip. In preview mode it scales the circle into the small window. After the iris completes it sleeps 1s, then `bw_RunWorkerCompleted` → `playRandomADS()`.
- **`playRandomADS()`** (174-196) — the scene scheduler:
  1. Clears the screen, sleeps 1s.
  2. Picks `random.Next(scenes.Count)` to choose an ADS file, then `random.Next(...Value.Count)` to choose a script number within that file.
  3. **Pre-loads ALL ten ADS files** into the ResourceManager cache regardless of which was chosen: `{ "ACTIVITY.ADS","BUILDING.ADS","FISHING.ADS","JOHNNY.ADS","MARY.ADS","MISCGAG.ADS","STAND.ADS","SUZY.ADS","VISITOR.ADS","WALKSTUF.ADS" }` (line 185). This is the canonical list of the 10 original ADS "story" scripts.
  4. Builds an `ADSPlayer(ads)`, wires `UpdateEvent → updateImage` (blits the new frame to `pbScreen.Image`) and `CompleteEvent → complete` (which loops back into `playRandomADS()`), then `player.runADS(scriptno)`.
- **Exit handling**: `doExit()` (234-245) cancels the intro worker and `Application.Exit()` — but only when `!previewMode`. Triggered by `KeyPress`, `MouseClick`, and `MouseMove` (with a 5-pixel dead-zone, lines 257-266). Cursor hidden on load.

### 1.3 Settings — `SettingsForm.cs` (+ `.Designer.cs`), `INI.cs`
- `SettingsForm` loads/saves an INI at `Path.Combine(CommonApplicationData, "JCOS", "ScrAntic.ini")` (line 31 + Program.INIFILE). `Load` reads it; `btnOK` calls `ini.save()`; `btnCancel` just closes. **The form never actually reads from or writes any keys to the INI** — the checkboxes/labels are not bound. So settings are non-functional placeholders.
- UI controls (Designer) mirror the original Sierra config dialog: `lblStartOfDay` ("9:00 am") with `btnAddHour`/`btnSubHour` (Webdings up/down arrows, no click handlers wired), `ckhLoadBackground` ("Load Background"), `chkPassword` ("Password"), `chkSounds` ("Sounds"), the SCRANTIC logo (`Resources.LOGO` from `LOGO.BMP`), OK/Cancel. Title "Screen Antics". None of these options are implemented in logic.
- **`INI.cs`** is a minimal hand-rolled INI: `load`/`save` (line-based, ASCII), `set(key,value)` (case-insensitive key match, append if not found), `get(key)` (returns value or `null`). No section `[...]` support — flat `key=value` lines only. `save()` creates the directory first.

### 1.4 Logging — `Log.cs`
- `static Log.write(text)` appends to `%TEMP%/JCOS.log` (thread-safe via a lock) and also `Debug.WriteLine`. Used for "Playing sound N", "Running ADS script N", "Playing <ads> sequence N".

---

## 2. Resource system (the DGDS container & per-type parsers)

The original game stores everything in a Dynamix **DGDS**-style archive: a `RESOURCE.MAP` index file pointing into one big `RESOURCE.001` data file, both under `C:\SIERRA\SCRANTIC\`. Each logical resource is a named blob built from FOURCC-style **chunk headers** (`"VER:"`, `"ADS:"`, `"RES:"`, `"SCR:"`, `"TAG:"`, `"BMP:"`, `"INF:"`, `"BIN:"`, `"PAG:"`, `"TT3:"`, `"TTI:"`, `"DIM:"`, `"VGA:"`, `"PAL:"`), each followed by a size and payload, with payloads usually compressed.

### 2.1 `ResourceManager.cs` (static cache + loader)
- Hard-coded path: `private static string resourcemap = "C:\\SIERRA\\SCRANTIC\\RESOURCE.MAP";` (line 27). **This is the only place the original install location is encoded.** README confirms files must be in `c:\SIERRA`.
- `static Resource get(string name)` (lines 31-87):
  - Caches in `Dictionary<string, Resource> resources` keyed case-insensitively.
  - Lazily parses the map: `map = new Map(); map.parse(resourcemap)`.
  - Opens `map.ResourceFile` (the .001 data file), looks up `getResourceIndex(name)` (returns -1 if missing → returns null), seeks to `getOffset(index)`.
  - Reads a **12-byte fixed filename block** (`getStringBlock(12)`), a **DWORD size**, then `size` bytes of payload.
  - Dispatches on file extension to construct + `parse()` the right `Resource` subclass: `ads → ADS`, `bmp → BMP`, `pal → PAL`, `scr → SCR`, `ttm → TTM`, `vin → (no-op / break)`.
- Note the caching bug: lookup uses `resources.ContainsKey(name.ToLower())` but stores/returns with mixed keys (`resources[name]` then `resources.Add(name.ToLower(), …)`). The dictionary is constructed with `StringComparer.CurrentCultureIgnoreCase`, which papers over it.

### 2.2 `Map.cs` — RESOURCE.MAP parser
- `parse(mapfile)` (lines 74-93):
  - Reads a **6-byte header** (unused, stored in `header`).
  - Reads a **12-byte string block** = the name of the data file; resolves it relative to the map's directory → `ResourceFile` (e.g. `C:\SIERRA\SCRANTIC\RESOURCE.001`).
  - Reads a **WORD `Resources`** count.
  - For each entry: **DWORD `bytes`** (uncompressed/declared size) + **DWORD `offset`** (into the data file). It then seeks the *data file* to `offset` and reads the 12-byte resource name there, building `Entry(bytes, offset, name)`.
- `getResourceIndex(name)` linear case-insensitive search; `getOffset(no)` returns the stored offset. So the MAP is essentially `{header(6)}{datafilename(12)}{count:WORD}{ (bytes:DWORD, offset:DWORD) * count }`, and the names live inline in the data file.

### 2.3 `FileParser.cs` / `DataParser.cs` — little-endian byte readers
- `DataParser` wraps a `byte[]` with an offset cursor. Primitives (all **little-endian**):
  - `getByte`, `peekByte`, `getBytes(count)`, `skip(i)`, `atEnd()`, `bytesLeft()` (= `Length-1-offset`).
  - `getWord()` = `byte + byte*256` (LE u16). `getDWord()` = `getWord() + getWord()*65536` (LE u32).
  - `getString()` — NUL-terminated C string.
  - `getStringBlock(length)` — reads a NUL-terminated string but always consumes exactly `length` bytes total (fixed-width padded field).
  - `getStringFixed(length)` — reads exactly `length` bytes, returns chars up to the first NUL (used for the 4-char FOURCC tags like `"VER:"`).
- `FileParser` is just a thin wrapper that loads a whole file via `File.ReadAllBytes` and delegates to a `DataParser`; shows a MessageBox + throws if the file is missing.

### 2.4 `Resource/Resource.cs` — base class
- `abstract class Resource : IDisposable` with `string FileName`, `abstract void parse(byte[] data)`, `abstract void Dispose()`. All concrete types subclass this.

### 2.5 `Resource/ADS.cs` — ADS script container
ADS = "after-dark script"-style **sequencer** that references TTM scripts. Header grammar parsed in `parse()` (lines 50-101):
```
"VER:" {SizeVersionString:DWORD} {Version: fixed string}
"ADS:" {Unknown2:WORD}{Unknown3:WORD}
"RES:" {RESSize:DWORD}{Resources:WORD}
        Resources × { id:WORD, name:Cstring }     // id→TTM filename map (resourceproperties)
"SCR:" {SCRSize:DWORD}{SCRCompressionMethod:byte}{SCRUncompressedSize:DWORD}
        {SCRData: (SCRSize-5) bytes}               // compressed script bytecode
"TAG:" {TAGSize:DWORD}{TagCount:WORD}
        TagCount × { id:WORD, description:Cstring } // human labels for sequences
```
- After reading, `script = Compression.decompress(SCRData, SCRCompressionMethod)` and the decompressed bytecode is disassembled into `Dictionary<int,List<Instruction>> sequences` keyed by sequence id. See §5 for ADS opcode handling. The `id→name` `resourceproperties` are how `data[0]` of `ADD_TTM` indexes a TTM file.
- Side effect: `parse()` writes a human-readable disassembly to `this.FileName` in the working directory (`File.WriteAllText/AppendAllText`) — a debugging artifact left enabled.
- `dump(filename)` exports a multi-sheet Excel-XML workbook (via `Excel.cs`) listing resource properties, TTM tag names, and per-sequence opcode/operand rows — an analysis tool, not used at runtime.

### 2.6 `Resource/TTM.cs` — TTM ("the movie") container
TTM = per-actor **animation script** (drawing/timing bytecode). Header grammar (lines 46-81):
```
"VER:" {SizeVersionString:DWORD}{Version: fixed string}
"PAG:" {Unknown2:DWORD}{Unknown3:WORD}
"TT3:" {TTMSize:DWORD (minus 5 stored)}{TTMCompressionMethod:byte}{TTMUncompressedSize:DWORD}
        {TTMData: TTMSize bytes}                    // compressed movie bytecode
"TTI:" {Unknown4:WORD}{Unknown5:WORD}
"TAG:" {TAGSize:DWORD}{TagCount:WORD}
        TagCount × { id:WORD, description:Cstring } // scene/tag labels
```
- `script = comp.decompress(TTMData, TTMCompressionMethod)` (comment: *"Usually RLE/Huffmann"*).
- **Bytecode disassembly** (lines 90-148) — the key format detail:
  - Each instruction word: `code = getWord()`. The **low nibble is the operand count**: `size = code & 0x000f`; the opcode is `code &= 0xfff0`.
  - Special case `code==0x1110 && size==1` (SET_SCENE): read one WORD `id`, and if it matches a TAG, attach `tags[id]` as the instruction's `name`.
  - Special case `size==15` (0xF): the operand is an **inline string** — `getString().ToUpper()` (a resource filename, e.g. for LOAD_IMAGE/LOAD_SCREEN); skip a trailing pad NUL if present.
  - Otherwise read `size` WORDs into `data`.
  - **Scene grouping**: a `0x1110` (SET_SCENE) sets the current `scene = data[0]`; instructions are bucketed into `Dictionary<int,List<Instruction>> scripts` keyed by scene id. Scene `0` is special — when first encountered it adds `tags[0]="init"` and sets `NeedsInit = true` (an implicit initialization sub-script run before the requested one).

### 2.7 `Resource/BMP.cs` — multi-image sprite sheet (16-color / 4-bit)
Represents **a set of bitmaps** (an actor's frames). Grammar (lines 32-58):
```
"BMP:" {Width:WORD}{Height:WORD}
"INF:" {DataSize:DWORD}{Images:WORD}
        Images × {Width:WORD}  then  Images × {Height:WORD}   // two parallel arrays
"BIN:" {BMPSize:DWORD (-5 stored)}{BMPCompressionMethod:byte}{BMPUncompressedSize:DWORD}
        {BMPData: BMPSize bytes}
```
- `data = decompress(BMPData, BMPCompressionMethod)`; then `images = Tools.getBitmaps(data, Widths, Heights)`. Declares but does not populate VGA* fields (those belong to higher-color variants not used here — only the 4-bit "BIN" plane is decoded).
- `ToString()` prints a stats line (sizes, compression ratio, per-image dims) — used for analysis dumps.

### 2.8 `Resource/SCR.cs` — full-screen background image
Single-image background. Grammar (lines 31-54):
```
"SCR:" {TotalSize:WORD}{Flags:WORD}
"DIM:" {DIMSize:DWORD}{Width:WORD}{Height:WORD}
"BIN:" {BMPDataSize:DWORD (-5)}{BMPCompressionMethod:byte}{BMPUncompressedSize:DWORD}
        {BMPData}
```
- Decompresses then `Tools.getBitmaps(data, {Width}, {Height})[0]` → `image`. Used for `INTRO.SCR` and any `LOAD_SCREEN` target. Like BMP, it ignores the VGA plane.

### 2.9 `Resource/PAL.cs` — VGA palette
Grammar (lines 14-30):
```
"PAL:" {size:WORD}{unknown1:byte}{unknown2:byte}
"VGA:" 256 × { r:byte, g:byte, b:byte }   // each component ×4 (6-bit VGA → 8-bit)
```
- `Color = new Color[256]`, each `Color.FromArgb(255, r*4, g*4, b*4)`. **The PAL resource is parsed but never actually applied in the renderer** — the player uses a fixed 16-color palette (`Palette.cs`) instead. Palette slot / fade opcodes are stubbed.

### 2.10 `Resource/VIN.cs` — stub
Empty class (`class VIN {}`). The real VIN ("FILES.VIN") would be the index of which ADS files form the story; here it is **not implemented** and the scene list is hard-coded instead (see §1.2). This is the single biggest piece of "story-selection" logic the C# port is missing.

---

## 3. Decompression — `Compression.cs`

`byte[] decompress(byte[] dta, byte compressionmethod)` (lines 82-97) dispatches on the method byte stored in each chunk header:

| Method | Algorithm | Routine |
|---|---|---|
| **0** | None (stored) | returns `data` unchanged |
| **1** | RLE | `decompressRLE()` |
| **2** | LZW | `decompressLZW()` |
| **3** | RLE variant 2 | `decompressRLE2()` |

### 3.1 RLE (method 1) — `decompressRLE()` (178-199)
Control byte per packet:
- If high bit set (`control & 0x80`): `length = control & 0x7F`; read **one** byte and emit it `length` times (a **run**).
- Else: `control` is a literal count — copy the next `control` bytes verbatim (a **literal copy**).

### 3.2 RLE2 (method 3) — `decompressRLE2()` (201-220)
Same control-bit convention but the run encodes the value in the control byte and the length in the next byte:
- If high bit set: `length = readByte()`; emit `(control & 0x7F)` exactly `length` times.
- Else: copy the next `control` literal bytes.

### 3.3 LZW (method 2) — `decompressLZW()` (99-176)
A classic **variable-width LZW** (compress-style), 9→12 bits, with a dictionary-reset code:
- Bit reader: LSB-first via `getBits(n)` reading `current` byte bit `nextbit`, advancing to the next byte at bit 8.
- Constants/params:
  - `CodeTableEntry { ushort prefix; byte append; }`, table size **4096**; `decodestack[4096]`.
  - `n_bits = 9` initial code width; grows up to **12** (`while (free_entry >= (1<<n_bits) && n_bits < 12) n_bits++`).
  - `free_entry = 257` initial next-free code; **code 256 = CLEAR/reset** marker.
  - First code is read with `getBits(9)` and emitted directly; `oldcode`/`lastbyte` seed the decoder.
- On a **clear code (256)**: it skips the remaining bits of the current code group — `nskip = (n_bits<<3 - ((bitpos-1) % (n_bits<<3))) - 1` — then resets `n_bits=9`, `free_entry=256`, `bitpos=0`. (This byte-group alignment skip is a quirk of the original LZWCOM-style stream.)
- Standard KwKwK handling: if `code >= free_entry`, push `lastbyte` and reuse `oldcode`; unwind the prefix chain onto `decodestack` until `code < 256`; flush stack to output; then add `{prefix=oldcode, append=lastbyte}` at `free_entry++`.
- Wrapped in `try/catch {}` returning whatever was decoded so far (defensive against malformed streams). This is the algorithm the website credits "Grégori" with helping reverse.

---

## 4. The TTM player & opcode table

### 4.1 `Resource/Instruction.cs` — the **complete opcode constant table**
This single file is the authoritative opcode dictionary for both TTM and ADS, plus a `ToString()` disassembler that documents operand layouts and even names the sound effects. **Note opcodes are stored with the low nibble masked off** (`& 0xfff0`); the low nibble was the operand count.

**TTM opcodes (drawing/animation):**

| Hex | Constant | Meaning / operands (from `ToString`) |
|---|---|---|
| `0x0020` | `SAVE_BACKGROUND` | save background |
| `0x0080` | `DRAW_BACKGROUND` | draw background |
| `0x0110` | `PURGE` | purge saved images |
| `0x0FF0` | `UPDATE` | update / present frame (also drives wave animation + delay) |
| `0x1020` | `DELAY` | delay (operand × 20 → ms) |
| `0x1050` | `SLOT_IMAGE` | select image slot (image) |
| `0x1060` | `SLOT_PALETTE` | select palette slot (palette) |
| `0x1110` | `SET_SCENE` | set scene (id) — scene/tag boundary |
| `0x2000` | `SET_FRAME0` | set frame 0 (?, frame) |
| `0x2010` | `SET_FRAME1` | set frame 1 (?, frame) |
| `0x4000` | `SET_WINDOW1` | set window (x1,y1,x2,y2) — clip rect via corners |
| `0x4110` | `FADE_OUT` | fade out (first, n, steps, delay) |
| `0x4120` | `FADE_IN` | fade in (first, n, steps, delay) |
| `0x4200` | `SAVE_IMAGE0` | save image region 0 (x, y, w, h) |
| `0x4210` | `SAVE_IMAGE1` | save image region 1 (x, y, w, h) |
| `0xA0A0` | `DRAW_WHITE_LINE` | draw white line (x1,y1,x2,y2) |
| `0xA100` | `SET_WINDOW0` | set window (x,y,w,h) — clip rect via size |
| `0xA400` | `DRAW_BUBBLE` | draw circle/ellipse (x,y,w,h) — speech bubble |
| `0xA500` | `DRAW_SPRITE0` | draw sprite (x, y, frame, imageSlot) |
| `0xA510` | `DRAW_SPRITE1` | draw sprite 1 (x,y,frame,image) — *not implemented (throws)* |
| `0xA520` | `DRAW_SPRITE2` | draw sprite 2 (x,y,frame,image) — **horizontally flipped** |
| `0xA530` | `DRAW_SPRITE3` | draw sprite 3 (x,y,frame,image) — *not implemented (throws)* |
| `0xA600` | `UNKNOWN2` | undefined / no-op |
| `0xB600` | `DRAW_SCREEN` | draw screen slot (x, y, w, h, ?, ?) |
| `0xC020` | `LOAD_SOUND` | load sound resource |
| `0xC030` | `SELECT_SOUND` | select sound (sound) |
| `0xC040` | `DESELECT_SOUND` | deselect sound (sound) |
| `0xC050` | `PLAY_SOUND` | play sound (index → named SFX) |
| `0xC060` | `STOP_SOUND` | stop sound (sound) |
| `0xF010` | `LOAD_SCREEN` | load screen resource (name) — inline string operand |
| `0xF020` | `LOAD_IMAGE` | load image resource (name) — inline string operand |
| `0xF050` | `LOAD_PALETTE` | load palette resource |

Additional opcodes that appear only in the `ToString()` disassembler as `"undefined"`/observed-but-unhandled (low-nibble already stripped): `0x00C0, 0x0400, 0x0500, 0x0510, 0x1070, 0x1100, 0x1120, 0x1200, 0x1370(see ADS), 0x2300, 0x2310, 0x2320, 0x2400, 0xA010, 0xA030, 0xA090, 0xA0B0, 0xA5A0, 0xF040`.

**Sound-effect name table** (`Instruction.cs` line 254, the `PLAY_SOUND` index → label):
```
0:(none) 1:Splash 2:horn! 3:cursing 4:stretching 5:puzzled 6:humming
7:grumble 8:hurt 9:breath 10:thunder 11:seagull attack 12:exert 13:swipe
14:plunge 15:short humm 16:woman scholding 17:spring 18:mermaid 19:seagull
20:woosh 21:flump 22:sigh 23:chimes
```
(These map to the bundled `sound0.wav … sound24.wav`.)

### 4.2 `TTMPlayer.cs` — how frames are played & drawn
- Constructed with a target TTM and a shared 640×480 `Bitmap screen`; obtains `Graphics g = Graphics.FromImage(screen)`. On construction it immediately draws a **red debug rectangle** around the whole screen (line 86) — leftover debug.
- Loads `BACKGRND.BMP` into `backgrnd` (the static island/sea background sprite sheet used for the animated waves).
- **Image/palette slots**: `MAX_IMAGE_SLOTS = 10`, `MAX_PALETTE_SLOTS = 10`. `imageSlot[10]` holds `Resource.BMP` objects; the current target slot is `currImage` (set by `SLOT_IMAGE`).
- Runs on a `BackgroundWorker bw` (`DoWork += play`); `bw.ReportProgress(0, new Bitmap(screen))` is how each finished frame is pushed up to the UI (via ADSPlayer's `ProgressChanged → UpdateEvent`).
- `play()` (578-590): on first run, if `ttm.NeedsInit`, it first executes `playscript(0)` (the scene-0 "init" sub-script) before the requested script.
- **`playscript(no)`** (119-575) iterates `ttm.scripts[no]` and switches on `mc.code`. Notable behaviours:
  - `SAVE_BACKGROUND (0x0020)`: copies the current `screen` into `backgroundImage` (also draws a red rect).
  - `DRAW_BACKGROUND (0x0080)` / handled lazily in `UPDATE`: blits `backgroundImage` back (with a green debug rect).
  - `UPDATE (0x0FF0)`: the **frame-present** opcode. Re-blits background + the two saved-image regions if not already drawn; then **procedurally animates the three sea-wave sprites** from `BACKGRND.BMP` images `3+wave1`@(270,306), `6+wave2`@(364,319), `9+wave3`@(520,303), where each `waveN` cycles 0..2 on a `wavedelay = 2_500_000` tick clock, phase-shifted by thirds to desync them (lines 208-217). Then `Thread.Sleep(currDelay)` and `bw.ReportProgress(0, new Bitmap(screen))`. This is the C# port's own embellishment to keep the ocean alive between scripted frames.
  - `DELAY (0x1020)`: `currDelay = data[0] * 20` (ms).
  - `SLOT_IMAGE (0x1050)`: `currImage = data[0]`.
  - `SAVE_IMAGE0/1 (0x4200/0x4210)`: capture a sub-rectangle (x,y,w,h) of the screen into `savedImage0/1` for later restore (used to erase a moving sprite by repainting what was underneath).
  - `DRAW_WHITE_LINE (0xA0A0)`: `g.DrawLine(White, x1,y1,x2,y2)`.
  - `SET_WINDOW0 (0xA100)`: set `g.Clip` to (x,y,w,h) (+yellow debug rect). `SET_WINDOW1 (0x4000)`: clip from corner pair `(x1,y1)-(x2,y2)`.
  - `DRAW_BUBBLE (0xA400)`: white filled ellipse with black outline (speech bubble).
  - `DRAW_SPRITE0 (0xA500)`: blits `imageSlot[data[3]].images[data[2]]` at `(data[0],data[1])` (after restoring bg/saved layers). `DRAW_SPRITE2 (0xA520)`: same but **mirrored horizontally** (`DrawImage(..., x+w, y, -w, h)`). `DRAW_SPRITE1/3` throw `"Not implemented"`.
  - `DRAW_SCREEN (0xB600)`: blits the `screenSlot` background at `(data[0],data[1])`.
  - `LOAD_SCREEN (0xF010)`: `screenSlot = ((Resource.SCR)ResourceManager.get(mc.name)).image`, draws it, then **fills a blue debug rectangle** over (0,350,640,130) (placeholder for the lower sea region).
  - `LOAD_IMAGE (0xF020)`: `imageSlot[currImage] = ResourceManager.get(mc.name) as Resource.BMP`.
  - `PLAY_SOUND (0xC050)`: constructs a `System.Media.SoundPlayer`, sets `.Stream` to the embedded `Properties.Resources.soundN`, and `.Play()`. A big `switch` maps indices 0-23 to embedded WAVs — **note indices 11 and 13 are missing from the switch** (those WAVs are referenced in the .csproj but not bundled in the Resources folder, which only contains sound0-10,12,14-24), and 24 exists as a file but isn't in this switch either.
  - `SET_SCENE, SLOT_PALETTE, FADE_IN/FADE_OUT, LOAD_SOUND, SELECT_SOUND, DESELECT_SOUND, STOP_SOUND, LOAD_PALETTE, PURGE` are **stubs or near-stubs** (FADE_OUT just `g.Clear(Black)`; palette ops do nothing). `SKIP_NEXT_IF/SET_FRAME0/SET_FRAME1` only `Debug.WriteLine` (frame logic not used because sprites are drawn directly with explicit frame indices).
  - Cancellation: each iteration checks `bw.CancellationPending`.
- 4-bit images are converted to 24-bit `Bitmap`s by `Tools.getBitmaps` (see §6) using the fixed 16-color palette.

---

## 5. The ADS player & opcode table

### 5.1 ADS opcodes — constants in `Instruction.cs` (lines 44-53) + parser cases in `ADS.cs`

| Hex | Constant | Meaning / operands |
|---|---|---|
| `0x1350` | `SKIP_NEXT_IF` | skip next instruction unless the referenced (ttm,seq) was the last played (2 operands) |
| `0x1360` | `SKIP_NEXT_IF2` | same family (2 operands) |
| `0x1430` | `OR` | logical OR chaining for SKIP_NEXT_IF conditions (no operands) |
| `0x1510` | `PLAY_SEQUENCES` | execute the accumulated/random TTM calls (no operands) |
| `0x1515` | `PLAY_SEQUENCES2` | variant (no operands) |
| `0x2005` | `ADD_TTM` | queue a TTM call: (ttmId, seqId, repeat, weight/count) |
| `0x3010` | `RANDOM_START` | begin a random-choice block |
| `0x3020` | `RANDOM_UNKNOWN1` | (1 operand) inside random block |
| `0x30FF` | `RANDOM_END` | end random block |

Extra opcodes the ADS disassembler reads operands for but the player ignores (from `ADS.parse` switch, lines 116-189): `0x1070`(2w), `0x1330`(2w), `0x1370`(2w), `0x1420`(0), `0x1520`(5w), `0x2010`(3w), `0x2014`(0), `0x4000`(0), `0xF010`(0), `0xF200`(1w), `0xFFFF`(end-marker). Any `code < 0x100` is treated as a **sequence label** that opens a new `sequences[code]` bucket (this is how scripts are numbered, e.g. `0x01`, `0x03`, `0x06`…). Opcodes `>= 0x100` that aren't recognized throw `"Unsupported opcode"`.

### 5.2 `ADSPlayer.cs` — how it sequences TTM scripts
- Constructor (`ADSPlayer(Resource.ADS ads)`, 92-118): creates the shared 640×480 `screen`, and for **each `ResourceProperty (id,name)`** in the ADS, resolves the referenced TTM via `ResourceManager.get(name)` and builds a `TTMPlayer(ttm, screen)`, stored in `Dictionary<UInt16,TTMPlayer> ttms` keyed by the property `id`. Each TTMPlayer's `bw.ProgressChanged` is forwarded to `bw_ProgressChanged → UpdateEvent` so frames bubble up to the form.
- `runADS(no)` kicks `bwads.RunWorkerAsync(no)`; `DoWork = play`.
- **`play()`** (160-257) interprets `ads.sequences[sequenceno]` linearly with these mechanics:
  - **`ADD_TTM (0x2005)`** → appends the instruction to a `playback` list (deferred; executed by the next `PLAY_SEQUENCES`).
  - **`RANDOM_START (0x3010)`** → scans forward to `RANDOM_END (0x30FF)`, and for each `ADD_TTM` inside, pushes it `data[3]` times into a `randoms` list (so `data[3]` acts as a **selection weight**).
  - **`PLAY_SEQUENCES (0x1510)`** → if `randoms` is non-empty, pick one at random (`rand.Next(randoms.Count)`) and play it; otherwise play every queued `playback` instruction in order. Then clears both lists. Then consults `bookmark` to possibly **loop back**: if any just-`lastplayed` (ttm,seq) tuple has a recorded bookmark earlier in the script, `inst` jumps there (`inst = idx - 1`).
  - **`SKIP_NEXT_IF / SKIP_NEXT_IF2 (0x1350/0x1360)`** → conditional branch on history: records a `bookmark[tuple]=inst`; computes `runnext = lastplayed.Contains(tuple)`; supports `OR (0x1430)`-chained alternatives (advancing by 2 each `OR`). If the condition fails, it **skips forward until the next `PLAY_SEQUENCES`**. If it succeeds, it `lastplayed.Clear()`s. This is how the original creates branching narrative ("if Johnny just did X, now do Y").
- **`playTTMInstruction(instruction)`** (130-147): looks up `ttms[data[0]]` (the TTM), and `playAndWait(ttmplayer, data[1])` (the sequence). `repeat = data[2] & 0xff`; if >0, replays the sequence `repeat` times total. Records the `(ttm,seq)` tuple via `getTouple(a,b)=a*65536+b` into `lastplayed` (the history used by SKIP_NEXT_IF and loop bookmarks).
- **`playAndWait`** (120-128) busy-waits (20 ms polls) for the TTMPlayer's worker to be free, fires `RunWorkerAsync(seq)`, then waits until `Playing` goes false — i.e. ADS playback is **synchronous/serialized over the TTM workers**.
- On completion `bw_RunWorkerCompleted` fires `CompleteEvent` → the form loops to the next random scene.

---

## 6. Graphics, palette, sound

### 6.1 `Tools.getBitmaps` (Tools.cs 39-76) — 4-bit decoder
- Given the decompressed pixel stream and parallel `widths[]`/`heights[]`, builds a `List<Bitmap>` (one per sub-image). Pixels are **4 bits each, packed two-per-byte, high nibble first** (`color = byte >> 4` on even pixel index, `color = byte & 0x0F` and advance on odd). Each index maps through `Palette.color[16]`. Pixels are painted one-by-one with `g.FillRectangle(SolidBrush, x, y, 1, 1)` — correct but extremely slow; wrapped in `try/catch{}` to tolerate truncated data. Pixel packing is continuous **across image boundaries** (shared `bitidx`/`idx`), matching the original sheet layout.

### 6.2 `Palette.cs` — fixed 16-color (EGA/VGA default) palette
Hard-coded `Color[16]` (used for ALL 4-bit sprites). Index 0 = transparent magenta (`ARGB 0,168,0,168` — alpha 0). 1-7 are the dim/standard colors, 8-15 the bright set:
```
0 transparent(168,0,168 a=0)  1 (0,0,168)    2 (0,168,0)    3 (0,168,168)
4 (168,0,0)   5 (0,0,0)        6 (168,168,0)  7 (212,212,212)
8 (128,128,128) 9 (0,0,255)   10 (0,255,0)   11 (0,255,255)
12 (255,0,0) 13 (255,0,255)   14 (255,255,0) 15 (255,255,255)
```
Index 0's zero alpha gives sprite transparency (magenta key). The PAL resource class exists but its 256-color data is **never wired into rendering** — this fixed 16-color table is authoritative in practice.

### 6.3 `Map.cs` / `Palette.cs` / `BMP.cs` summary
- `Map.cs` = archive index (covered §2.2). `BMP.cs`/`SCR.cs` = image containers (§2.7/2.8). The actual blitting all happens in TTMPlayer with `System.Drawing.Graphics`.

### 6.4 Sound
- Sound is **embedded** (not read from the SIERRA folder): `Properties/Resources.resx` references `..\Resources\soundN.wav` and `Resources.Designer.cs` exposes them as `UnmanagedMemoryStream sound0…sound24` (sounds 0-10, 12, 14-24; **11 and 13 absent**). Playback is `System.Media.SoundPlayer.Stream = …; Play()` in `TTMPlayer.PLAY_SOUND`. `LOAD_SOUND/SELECT_SOUND/DESELECT_SOUND/STOP_SOUND` are stubbed, so there's no looping/streaming sound — just fire-and-forget WAV playback on `0xC050`. The "Sounds" settings checkbox does nothing.

### 6.5 `Excel.cs` — analysis export (not runtime)
- `DataTablesToExcelXml(List<DataTable>, autofilter)` serializes one or more `DataTable`s into **SpreadsheetML** (`<?mso-application progid="Excel.Sheet"?>` workbook XML) with styled header cells, typed cells (Number/Boolean/DateTime/String), optional AutoFilter. References `Microsoft.VisualBasic`. Consumed by `ADS.dump()` to export opcode/operand tables and TTM tag maps for reverse-engineering. Pure tooling; never called by the screensaver path.

---

## 7. Story / scene / day-cycle logic

- **No day-cycle, no special-days, no weather/seasonal/holiday logic exists in this port.** The original screensaver had a 9-am "start of day" notion (the `lblStartOfDay`/`btnAddHour`/`btnSubHour` UI is preserved in `SettingsForm` purely cosmetically) and special calendar events; none of that is implemented here.
- Scene selection is the simple two-level uniform random pick in `JohnnyCastawayForm.playRandomADS()` over the **hard-coded 4-entry `scenes` table** (FISHING/WALKSTUF/MISCGAG/ACTIVITY), looping forever via the ADS `CompleteEvent`. The full set of 10 ADS story files is known (and preloaded) but most are commented out / unused because the TTM player can't yet render them.
- The intended-but-unimplemented mechanism is documented in code: the **`FILES.VIN` resource** would list the ADS files comprising the day's story, and the ADS branching opcodes (`SKIP_NEXT_IF`/`OR`/bookmark-loops + `RANDOM_*` weighted picks) provide the actual narrative state machine. The `VIN` class is an empty stub.
- Within an ADS, "scene chosen" = which `sequences[label]` runs (script number passed to `runADS`); within a TTM, "scene" = the `0x1110 SET_SCENE` buckets, with scene 0 = implicit init.

---

## 8. What this C# version documents/does that other ports may not

1. **Most complete annotated opcode dictionary**: `Instruction.cs` gives English constant names AND a `ToString()` that spells out operand tuples for ~60 opcodes (TTM + ADS), including many `undefined`/observed-but-unhandled codes (`0x00C0,0x0400,0x05x0,0x107x,0x11xx,0x12xx,0x23xx,0x24xx,0xA0xx,0xA5A0,0xF040`) — a superset useful for cross-checking other implementations.
2. **Named sound-effect table** (`0:none,1:Splash,2:horn!,…,23:chimes`) tying SFX indices to semantic labels.
3. **Four explicit decompression methods** (0 store / 1 RLE / 2 LZW 9-12bit / 3 RLE2) with the exact bit/byte conventions, including the LZW clear-code group-alignment skip.
4. **Explicit DGDS chunk grammars** for every resource type (ADS/TTM/BMP/SCR/PAL) with the FOURCC tags, size/compression-method/uncompressed-size triplets, and the `-5` size adjustments.
5. **The TTM low-nibble = operand-count encoding**, the `size==15` inline-string operand convention, and the scene-0 implicit-init rule — concrete details a reimplementation needs.
6. **The ADS history/bookmark/weighted-random state machine** spelled out in `ADSPlayer.play()` (lastplayed tuples, OR-chained SKIP_NEXT_IF, loop-back bookmarks, `data[3]` as random weight, `data[2]&0xff` as repeat count).
7. **`ADS.dump()` + `Excel.cs`** provide a built-in disassembler-to-spreadsheet exporter — an analysis affordance unique to this port.
8. A documented intent for the **`FILES.VIN`-driven** story selection (even though unimplemented).

### Caveats / known incompleteness (so we don't trust it blindly)
- Hard-coded Windows path `C:\SIERRA\SCRANTIC\RESOURCE.MAP`; Windows-only (WinForms + P/Invoke + `System.Media`).
- Renderer leaves **debug rectangles** (red/green/yellow/blue) drawn over real output; `LOAD_SCREEN` paints a blue placeholder over the sea region.
- `DRAW_SPRITE1/3` throw; palette resources, fades, palette slots, sound select/stop, and `SET_FRAME*`/`SKIP_NEXT_IF` (TTM side) are stubs.
- Per-pixel `FillRectangle` decode is very slow.
- Settings UI is non-functional (no INI keys bound).
- Sound switch omits indices 11/13 (and 24); those WAVs are inconsistently bundled vs. referenced in the csproj.
- Per-scene `File.WriteAllText(this.FileName, …)` disassembly side-effect runs at parse time.

---

## Appendix A — Original data files referenced by name in the C# code
- Archive: `C:\SIERRA\SCRANTIC\RESOURCE.MAP` (+ the `RESOURCE.001`-style data file named inside the MAP).
- Backgrounds/images: `BACKGRND.BMP` (waves/island sheet, loaded by every TTMPlayer), `INTRO.SCR` (iris-in intro), plus whatever `LOAD_SCREEN`/`LOAD_IMAGE` opcodes name inline at runtime.
- ADS story scripts (the 10 preloaded in `playRandomADS`): `ACTIVITY.ADS, BUILDING.ADS, FISHING.ADS, JOHNNY.ADS, MARY.ADS, MISCGAG.ADS, STAND.ADS, SUZY.ADS, VISITOR.ADS, WALKSTUF.ADS`.
- Currently-played subset (hard-coded `scenes`): `FISHING.ADS#1, WALKSTUF.ADS#3, MISCGAG.ADS#1, ACTIVITY.ADS#{1,12,6,8,9}`.
- Intended index (unimplemented stub): `FILES.VIN`.
- Logo: `Resources/LOGO.BMP` (settings dialog). Bundled SFX: `Resources/sound0..sound24.wav` (missing 11, 13).

## Appendix B — Resource→class dispatch (ResourceManager.get)
`ads→Resource.ADS`, `bmp→Resource.BMP`, `pal→Resource.PAL`, `scr→Resource.SCR`, `ttm→Resource.TTM`, `vin→(unhandled/no-op)`. All cached by lowercased name.
