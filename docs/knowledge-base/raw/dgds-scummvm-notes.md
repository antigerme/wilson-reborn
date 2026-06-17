# DGDS / SCRANTIC File Format — Notes from the ScummVM "DGDS" Engine

Source under study: ScummVM "DGDS" engine `engines/dgds` (<https://github.com/scummvm/scummvm>),
formerly vendored under `repos/dgds/` (an early
WIP / reverse-engineering engine for the Dynamix Game Development System resource format).
Every `.cpp`/`.h` plus build files were read in full. This is the authoritative reference for
the Dynamix DGDS container used by *Rise of the Dragon*, *Heart of China*, and the SCRANTIC
engine of *Johnny Castaway*.

All references below are `file:line`. The engine is incomplete and exploratory (lots of
`debug()`/`hexdump()` and `// guesswork` comments), so treat field meanings as "best known"
rather than canon — but the byte-level layout is concrete and reliable.

> Big-picture caveat for Wilson Reborn: this engine targets *Rise of the Dragon* / *Heart of
> China*, which use a **RMF/volume index + volume files** packaging (`VOLUME.RMF` + `VOLUME.00x`).
> Johnny Castaway ships a `RESOURCE.MAP` + `RESOURCE.00x` set instead. The *index format itself*
> is not in `detection_tables.h` for JC, but the **chunk format, compression, TTM/ADS bytecode,
> font, palette and sound formats documented here are shared** and are exactly what we need.

---

## 1. Packaging layer: RMF/MAP index + volumes

Two readers implement the same on-disk index format: the verbose `explode()`
(`dgds.cpp:1339-1434`) and the lean `lookupVolume()`/`createReadStream()`
(`dgds.cpp:1453-1512`). The engine calls the index file the "RMF" (`_rmfName`,
`dgds.h:45`); for JC the equivalent is `RESOURCE.MAP`.

### 1.1 Index file layout (`*.RMF` / `RESOURCE.MAP`)
Read at `dgds.cpp:1351-1392` and `1453-1478`:

```
offset  size  field
0       4     salt[4]          ; 4 hash-seed bytes, used by dgdsHash()
4       2     nvolumes  (LE)   ; number of volume files
then, repeated nvolumes times:
        13    volumeName       ; ASCIIZ, fixed 13-byte field (DGDS_FILENAME_MAX=12 + NUL)
        2     nfiles    (LE)   ; number of files in that volume
        then, repeated nfiles times:
        4     hash      (LE, int32)  ; dgdsHash(filename, salt) of the contained file
        4     offset    (LE, uint32) ; byte offset of the file record inside the volume
```

`DGDS_FILENAME_MAX = 12`, `DGDS_TYPENAME_MAX = 4` (`dgds.cpp:100-101`).
Volume name field is read as `sizeof(name)` = 13 bytes and force-NUL-terminated at index 12.

### 1.2 Volume file record (inside `VOLUME.00x` / `RESOURCE.00x`)
At the `offset` from the index (`dgds.cpp:1386-1403`, `1496-1507`):

```
offset  size  field
0       13    fileName         ; ASCIIZ, fixed 13-byte field
13      4     fileSize  (LE, uint32)  ; 0xFFFFFFFF == deleted/empty slot, skip
17      …     fileData         ; the DGDS chunked file (see §2)
```

A `fileSize == 0xFFFFFFFF` record is skipped (`dgds.cpp:1394`, `1503`). The file payload is
exposed as a `SeekableSubReadStream` over the volume (`dgds.cpp:1403`, `1507`).

### 1.3 Filename hash — `dgdsHash()` (`dgds.cpp:440-463`)
Critical for locating files in the index. **Both intermediate types MUST be `int16`** (signed
16-bit overflow is part of the algorithm — the comment stresses this at `:450`):

```c
int32 dgdsHash(const char *s, byte *idx) {  // idx == the 4 salt bytes
    int16 isum = 0, ixor = 0;
    int i, c;
    for (i = 0; s[i]; i++) {            // i ends == strlen(s)
        c = toupper(s[i]);
        isum += c;
        ixor ^= c;
    }
    isum *= ixor;                       // int16 overflow intended
    c = 0;
    for (int16 j = 0; j < 4; j++) {     // build 4 bytes from salted positions
        c <<= 8;
        if (i > idx[j])                 // only if string is long enough
            c |= toupper(s[idx[j]]);
    }
    c += isum;                          // final 32-bit hash
    return c;
}
```

So the hash = (sum-of-uppercased-chars * xor-of-uppercased-chars), truncated to int16, plus a
32-bit value assembled from the characters at the 4 salt-selected string positions. The salt
bytes are indices into the filename. Uppercase-folded throughout.

---

## 2. DGDS container / chunk format

Each file inside a volume is a stream of **chunks**. Parser: `DgdsChunk::readHeader()`
(`dgds.cpp:371-396`) and `DgdsParser::parse()` (`dgds.cpp:164-192`).

### 2.1 Chunk header (8 bytes)
```
offset  size  field
0       4     id[4]    ; 4 ASCII bytes; the 4th byte MUST be ':' (0x3A) or parse aborts
4       4     size (LE, uint32)
```
- The 4th tag char being `':'` is the validity check (`dgds.cpp:381`): tags are written as
  3 letters + colon, e.g. `"VER:"`, `"BIN:"`, `"TT3:"`, `"DDS:"`.
- The 3-letter ID is packed to a 24-bit integer via `MKTAG24`:
  `MKTAG24(a0,a1,a2) = (a2 | (a1<<8) | (a0<<16))` (`dgds.cpp:130`). The colon is dropped from
  the numeric `_id`.
- **Container bit:** the top bit of `size` flags a container (nested) chunk:
  ```c
  _size = readUint32LE();
  if (_size & 0x80000000) { _size &= ~0x80000000; container = true; }
  else                      container = false;
  ```
  (`dgds.cpp:388-394`). A container chunk has **no payload of its own**; following chunks are
  its children until the parser logic changes `parent` (`dgds.cpp:724-727`, the loop sets
  `parent = chunk._id` and `continue`s).

### 2.2 Per-chunk payload: packed vs raw
`DgdsParser::parse()` decides per chunk (`dgds.cpp:182`):
```c
chunk._stream = chunk.isPacked(_ex) ? chunk.decodeStream(ctx) : chunk.readStream(ctx);
```
- `readStream()` (`dgds.cpp:421-431`): payload is a `SeekableSubReadStream` of `_size` bytes.
- `decodeStream()` (`dgds.cpp:398-419`): a **compressed** chunk. Layout of a packed chunk
  payload:
  ```
  offset  size  field
  0       1     compression  ; 0x00=None, 0x01=RLE, 0x02=LZW
  1       4     unpackSize (LE, uint32)  ; decompressed length
  5       …     compressed bytes  ; length = (_size - 5)
  ```
  `_size -= 5` before reading the body, then `decompress(compression, dest, unpackSize, file, _size)`.

### 2.3 The file "extension type" `_ex`
The 3-char file **extension** is turned into a `MKTAG24` value (`_ex`) and drives all parsing:
`parse()` uppercases it (`dgds.cpp:168-169`); `parseFile()` does **not** uppercase
(`dgds.cpp:563-564`). The `_ex` selects which chunk IDs are packed (`isPacked`) and how chunk
bodies are interpreted (the big `switch (_ex)` in `parseFile`, `dgds.cpp:753-1267`).

---

## 3. Chunk-ID tag constants (the 3-letter `XXX:` tags)

From `dgds.cpp:199-222`. These are *chunk* IDs found inside files:

| Constant   | Tag    | Meaning / used in |
|------------|--------|-------------------|
| `ID_BIN`   | `BIN:` | Bitmap plane data (low nibble planes); 4bpp image half |
| `ID_DAT`   | `DAT:` | Generic data; sound entry in Mac `.SX` |
| `ID_FNM`   | `FNM:` | Filename string table (Mac `.SX`) |
| `ID_FNT`   | `FNT:` | Font data |
| `ID_GAD`   | `GAD:` | Gadget (UI widget) strings, in `.REQ` requestors |
| `ID_INF`   | `INF:` | Info/header table (image dims, song index, sound index) |
| `ID_MTX`   | `MTX:` | Tile matrix (tilemap) for `.BMP` (e.g. SCROLL.BMP) |
| `ID_PAG`   | `PAG:` | Page count (TTM) |
| `ID_REQ`   | `REQ:` | Requestor (dialog/UI) definition |
| `ID_RES`   | `RES:` | Resource string list (ADS: numbered TTM resource names) |
| `ID_SCR`   | `SCR:` | Script bytecode (ADS) **or** screen image container (.SCR) |
| `ID_SDS`   | `SDS:` | Scene/Dialogue script (SDS/GDS) |
| `ID_SNG`   | `SNG:` | Song (MIDI-like) data (DOS `.SNG`) |
| `ID_TAG`   | `TAG:` | Tag string table (id→string) |
| `ID_TT3`   | `TT3:` | TTM bytecode (Text/Tableau? script v3) |
| `ID_TTI`   | `TTI:` | TTM instruction count marker |
| `ID_VER`   | `VER:` | Version string (4-char, e.g. "1.20") |
| `ID_VGA`   | `VGA:` | Bitmap plane data (high nibble planes) / VGA palette |
| `ID_VQT`   | `VQT:` | VQ-compressed image table (DOS; **not** decoded here) |

Heart of China additions (`dgds.cpp:220-222`):

| Constant | Tag    | Meaning |
|----------|--------|---------|
| `ID_MA8` | `MA8:` | 8bpp (256-color) bitmap data |
| `ID_DDS` | `DDS:` | "DDS" scene-data chunk (HoC) |
| `ID_THD` | `THD:` | Talk-head data (HoC character portrait header) |

Note also `OFF:` and `VQT:` are mentioned (`dgds.cpp:1176-1177`, `1201-1202`) as DOS
compressed-picture chunks that this engine deliberately does **not** decode.

---

## 4. File-extension tag constants (`_ex`)

From `dgds.cpp:224-247`. These classify *whole files* by extension:

| Constant | Ext    | Description (per engine) |
|----------|--------|--------------------------|
| `EX_ADH` | `.ADH` | ADS script (variant H) |
| `EX_ADL` | `.ADL` | ADS script (variant L) |
| `EX_ADS` | `.ADS` | ADS script — "scene"/animation-sequence bytecode |
| `EX_AMG` | `.AMG` | Amiga text list (newline-delimited) |
| `EX_BMP` | `.BMP` | Bitmap set (multi-image sheet) |
| `EX_GDS` | `.GDS` | Global Data Script (game logic; INF+SDS) |
| `EX_INS` | `.INS` | Instrument / AIFF sound sample (Amiga) |
| `EX_PAL` | `.PAL` | Palette (VGA: chunk, 256×RGB) |
| `EX_FNT` | `.FNT` | Font |
| `EX_REQ` | `.REQ` | Requestor (UI dialog) |
| `EX_RST` | `.RST` | "Restart"/state table (flat file) |
| `EX_SCR` | `.SCR` | Full-screen image (BIN/VGA/MA8 planes) |
| `EX_SDS` | `.SDS` | Scene/Dialogue Script |
| `EX_SNG` | `.SNG` | Song (music) |
| `EX_SX`  | `.SX`  | Mac sound bank (INF/TAG/FNM/DAT) — tag is `MKTAG24('S','X',0)` |
| `EX_TTM` | `.TTM` | TTM animation/tableau bytecode |
| `EX_VIN` | `.VIN` | Text list ("VIN", newline-delimited) |
| `EX_DAT` | `.DAT` | Generic data (flat) — HoC |
| `EX_DDS` | `.DDS` | Scene data — HoC |
| `EX_TDS` | `.TDS` | Talk-head/character data — HoC |
| `EX_OVL` | `.OVL` | Overlay (bundle of many driver sub-chunks) |

> Note `EX_SX = MKTAG24('S','X',0)` — only 2 letters, third byte 0. Relevant because JC/SCRANTIC
> sound effects use the `.SX` extension.

### 4.1 Flat (non-chunked) files — `isFlatfile()` (`dgds.cpp:257-287`)
Some files are **not** chunked and are read raw:
- Always flat: `.RST`, `.VIN`, `.DAT`.
- On Amiga only, also flat: `.BMP`, `.SCR`, `.INS`, `.AMG` (`.SNG` is commented out).
Everything else goes through the chunk parser.

---

## 5. Which chunks are compressed — `isPacked()` (`dgds.cpp:289-368`)

Compression is keyed on (file extension `_ex`, chunk id). Summary table:

| File ext (`_ex`)        | Packed chunk IDs |
|-------------------------|------------------|
| `.ADS`/`.ADL`/`.ADH`    | `SCR:` |
| `.BMP`                  | `BIN:`, `VGA:` |
| `.GDS`                  | `SDS:` |
| `.SCR`                  | `BIN:`, `VGA:`, `MA8:` |
| `.SDS`                  | `SDS:` |
| `.SNG`                  | `SNG:` |
| `.TTM`                  | `TT3:` |
| `.TDS`                  | `THD:` |
| `.DDS`                  | chunk literally `"DDS:"` |
| `.TDS`                  | chunk literally `"TDS:"` |
| `.OVL` (driver bundles) | `ADL:`,`ADS:`,`APA:`,`ASB:`,`GMD:`,`M32:`,`NLD:`,`PRO:`,`PS1:`,`SBL:`,`SBP:`,`STD:`,`TAN:`,`T3V:`,`001:`,`003:`,`004:`,`101:`,`VGA:` |

The `.OVL` list is essentially the per-sound-driver code overlays (M32=MT-32, GMD=General
MIDI, SBL=SoundBlaster, TAN=Tandy, PS1=PC speaker, etc.).

---

## 6. Decompression algorithms (`decompress.cpp`)

Dispatcher `decompress(compression, dest, uncompressedSize, input, size)` (`decompress.cpp:185-206`):
- `0x00` None → straight `input.read(data, size)`.
- `0x01` RLE → `RleDecompressor`.
- `0x02` LZW → `LzwDecompressor`.
- default → skip and `debug("unknown chunk compression")`.

`compressionDescr[] = {"None", "RLE", "LZW"}` (`decompress.cpp:183`).

### 6.1 RLE — `RleDecompressor::decompress()` (`decompress.cpp:31-59`)
Byte-oriented RLE driven by a control byte `lenR`:
```
read lenR (1 byte):
  if lenR == 128 (0x80):           -> emit nothing (lenW = 0)   [terminator/no-op]
  else if lenR <= 127:             -> LITERAL run:
        copy MIN(lenR, left) bytes verbatim from input to output;
        if the run was clipped to 'left', skip the remaining (lenR - lenW) input bytes.
  else (lenR > 128, i.e. high bit set):  -> REPEAT run:
        count = lenR & 0x7F;
        read 1 value byte; memset that value MIN(count,left) times.
loop until 'left' (remaining output) hits 0 or EOF.
```
So: top bit clear = literal copy of N bytes; top bit set = repeat next byte N times; the bare
value `0x80` means a zero-length op. `left` is decremented by `lenW` (the actually-written count).

### 6.2 LZW — `LzwDecompressor` (`decompress.cpp:61-180`, struct in `decompress.h:39-61`)
Variable-width LZW with these exact parameters:
- **Dictionary entry:** `{ byte str[256]; uint8 len; }`, table `_codeTable[0x4000]` (16384 entries)
  (`decompress.h:48-51`).
- **reset()** (`decompress.cpp:61-77`): entries 0..255 are single-byte strings (`len=1`).
  - `_tableSize = 0x101` (next free code = 257; 256=`0x100` reserved as CLEAR).
  - `_tableMax  = 0x200`.
  - `_codeSize  = 9` bits (starting code width).
  - `_codeLen   = 0`, `_cacheBits = 0`, `_tableFull = false`.
- **Special code `0x100` = CLEAR/reset dictionary** (`decompress.cpp:100-103`). On CLEAR it
  flushes the remaining bits of the current code group (`getCode(_codeSize*8 - _cacheBits)`)
  then `reset()`s.
- **Code-width growth:** width increases 9→10→11→12 as the table fills; when
  `_tableSize == _tableMax` and `_codeSize < 12`, `_codeSize++` and `_tableMax <<= 1`
  (`decompress.cpp:129-132`). At width 12 with table full, `_tableFull = true` and no more
  entries are added (`decompress.cpp:121-123`). Max width is **12 bits**.
- **The classic KwKwK case** (code not yet in table) is handled at `decompress.cpp:105-109`:
  emit `_codeCur` with its first byte appended.
- **Bit packing / `getCode()`** (`decompress.cpp:151-180`): **LSB-first**. Bits are pulled from
  the low end of each input byte (`_bitsData >>= useBits`) and OR'd into the result shifted by
  `(totalBits - numBits)`. Uses `bitMasks[9] = {0,1,3,7,0xF,0x1F,0x3F,0x7F,0xFF}`. A read past
  end-of-stream returns `0xFFFFFFFF` (treated as end).
- There is also an unusual `_cacheBits` accounting (`decompress.cpp:95-98`, `:126`) that tracks
  bit-group alignment; this is a Dynamix-specific quirk of when new dictionary entries are added
  vs. when bit padding happens. Replicate it exactly when writing a clean-room decoder.

> For a clone: implement RLE first (used by most BMP/SCR/SNG chunks); LZW (`0x02`) appears in
> packed fonts and some chunks. The combination of **LSB-first bit order, 9-bit start, 12-bit
> max, 0x100=clear, 0x101=first free code** is the precise spec.

---

## 7. Image / bitmap formats

### 7.1 4bpp planar pair: BIN + VGA  → 8bpp
DOS images store color as two 4-bit halves: `BIN:` carries the low nibble plane data, `VGA:`
the high nibble. `loadBitmap4()` (`dgds.cpp:483-490`) loads each as a CLUT8 surface of pitch
`tw>>1`. `convertBitmap()` (`dgds.cpp:501-514`) recombines two source bytes into 2 output
pixels per byte-pair:
```c
data[i+0]  = (vga[i>>1] & 0xF0)        | ((bin[i>>1] & 0xF0) >> 4);
data[i+1]  = ((vga[i>>1] & 0x0F) << 4) |  (bin[i>>1] & 0x0F);
```
i.e. each pixel = (high-nibble-from-VGA << 4) | (high-nibble-from-BIN) etc. — VGA supplies the
upper 4 bits of palette index, BIN the lower 4 bits, packed two pixels per source byte.

### 7.2 8bpp: MA8 (Heart of China)
`loadBitmap8()` (`dgds.cpp:492-499`) reads `tw` bytes/row directly as 256-color indices. The
overloaded `convertBitmap(... ma8)` (`dgds.cpp:515-525`) copies MA8 straight through if present,
else falls back to BIN/VGA recombination.

### 7.3 `.SCR` full-screen image (`dgds.cpp:1175-1199`)
A `.SCR` file is a chunk container with `BIN:`, `VGA:`, `MA8:`, `VQT:` (and `OFF:`) children.
Each is a 320×200 plane. `VQT:` (VQ-compressed) is **skipped** (`dgds.cpp:1185-1186`) — not
implemented. Screen size constant: `sw=320, sh=200` (`dgds.cpp:109`).

### 7.4 `.BMP` multi-image sheet (`dgds.cpp:1200-1264`)
- `INF:` chunk header (`dgds.cpp:1203-1221`):
  ```
  uint16 tcount               ; number of sub-images (tiles/frames)
  uint16 widths[tcount]  (LE) ; all widths, then…
  uint16 heights[tcount] (LE) ; all heights
  ; per-image byte offset = running sum of width*height (8bpp logical)
  ```
- `BIN:`/`VGA:` chunks hold the concatenated plane data; image *resource* index selects
  `tw[resource], th[resource], toffset[resource]`.
- `MTX:` chunk (`dgds.cpp:1222-1236`): a tilemap — `uint16 mw, mh` then `mw*mh` `uint16` tile
  indices. Used for scrolling backdrops (SCROLL.BMP / SCROLL2.BMP intro title).
- Amiga `.BMP` variant (flat, big-endian) parsed separately at `dgds.cpp:637-679`: counts and
  sizes are **big-endian**, offset stride `(tw+15)/16*th*5` (5 bitplanes).

### 7.5 `.PAL` palette (`dgds.cpp:1143-1160`)
Chunk `VGA:` = 256 RGB triples (768 bytes). Values are 6-bit VGA DAC values; the loader
**left-shifts each component by 2** to scale 0–63 → 0–252 (`dgds.cpp:1149-1153`). A
black/`blacks` palette is kept for fades.

---

## 8. Fonts (`font.cpp` / `font.h`)

Two font formats, distinguished by the first byte (magic) of the `FNT:` chunk
(`dgds.cpp:1163-1172`): `0xFF` ⇒ proportional `PFont`, otherwise raster `FFont`.

Common base `Font` (`font.h:39-54`): fields `_w, _h, _start, _count, _data`. `hasChar()` =
`chr >= _start && chr <= _start+_count` (`font.cpp:40-42`). Glyphs are 1-bit bitmaps; a pixel
is set via `isSet(set, bit) = set[bit>>3] & (1<<(bit&7))` (`font.cpp:44-46`); drawn MSB-first
per row with `idx+_w-1-j` (`font.cpp:48-69`).

### 8.1 `FFont` — fixed-width raster font (`FFont::load_Font`, `font.cpp:89-108`)
Header (no magic):
```
byte w, h, start, count
then h*count bytes of glyph bitmap data  (h bytes per glyph, one bit per pixel)
assert(4 + h*count == fileSize)
```
`getCharWidth` = `_w` (fixed). Glyph offset `pos = (chr-start)*h`, `bit = 8-w`.

### 8.2 `PFont` — proportional font (`PFont::load_PFont`, `font.cpp:128-168`)
Header:
```
byte  magic == 0xFF
byte  w, h, unknown, start, count
uint16 size (LE)
byte  compression
uint32 uncompressedSize (LE)   ; asserted == size
<compressed body>  -> decompress() into 'data'
```
Decompressed `data` layout: `_offsets = (uint16*)data` (per-char glyph offset), `_widths =
data + 2*count` (per-char width), `_data = data + 3*count` (bitmaps). `getCharWidth =
_widths[chr-start]` (`font.cpp:111-113`); glyph offset read LE from `_offsets[chr-start]`.

Known font files referenced: `DRAGON.FNT`, `CHINA.FNT`, `4X5.FNT`, `P6X6.FNT`
(`dgds.cpp:2418,2426,2460-2462`).

---

## 9. Sound & Music

### 9.1 Track container format (`sound.cpp`) — the SNG/driver layout
`availableSndTracks()` (`sound.cpp:57-109`) and `loadSndTrack()` (`sound.cpp:111-156`) parse a
SCI-like multi-driver sound resource:
- Optional 2-byte **SCI header**: if first `uint16 == 0x0084`, `sci_header = 2` and all
  part-offsets are biased by 2 (`sound.cpp:31-41`). Then an optional leading SysEx block
  (`0xF0`, length byte, then +6) is skipped.
- Body = list of **drivers**, each: `byte drv;` then a list of **parts** until `0xFF`; the
  driver list itself ends at `0xFF`.
- Part header (`readPartHeader`, `sound.cpp:43-50`): skip 2, `uint16 off (LE)`, `uint16 siz
  (LE)` — i.e. 6 bytes total; `off += sci_header`.
- Driver IDs (`sound.cpp:86-102`):
  | drv | device |
  |-----|--------|
  | 0   | SoundBlaster (digital PCM) **or** AdLib — disambiguated by part data |
  | 7   | General MIDI |
  | 9   | CMS |
  | 12  | MT-32 |
  | 18  | PC Speaker |
  | 19  | Tandy 1000 |
- A part is **digital PCM** if its first `uint16 == 0x00FE`; otherwise (for drv 0) it's AdLib
  (`sound.cpp:81-93`).
- Track-type bit flags (`sound.h:30-35`): `DIGITAL_PCM=1`, `TRACK_ADLIB=2`, `TRACK_GM=4`,
  `TRACK_MT32=8`. `loadSndTrack(track,…)` maps a requested type to a `drv` number and returns
  that driver's parts (`sound.cpp:111-156`).

### 9.2 Digital PCM playback (`DgdsEngine::playPCM`, `dgds.cpp:2200-2243`)
For each PCM part: `uint16 == 0x00FE` marker, then `rate, length, first, last` (all LE uint16),
8-byte header, skip `first` bytes, play `length` bytes as **unsigned raw** at `rate` Hz
(`Audio::makeRawStream(..., FLAG_UNSIGNED)`).

### 9.3 MIDI playback (`music.cpp`, `MidiParser_DGDS`)
- `loadMusic()` (`music.cpp:285-319`) loads the **MT-32** track parts, then mixes the parallel
  per-part streams into one standard MIDI track via `mixChannels()`.
- Per-part prefix (`music.cpp:293-303`): `byte number; byte voices(&0x0F)` then the 2 bytes are
  skipped.
- Delta-time encoding: a byte `0xF8` means **+240 ticks** (`music.cpp:83-87`, `:174`,
  `:222-236`); otherwise the byte is the delta. Running status supported.
- Special command bytes during mix: `0xFC` = end-of-channel (`music.cpp:250-252`), `0xF0…0xF7`
  = SysEx passthrough (`music.cpp:241-249`).
- `commandLengths[] = {2,2,2,2,1,1,2,0}` indexed by `(cmd>>4)-8` (`music.cpp:195`) — i.e.
  NoteOff/NoteOn/Aftertouch/CC = 2 data bytes, Program/ChannelPressure = 1, PitchBend = 2.
- NoteOn (`0x9x`) with velocity 0 is converted to NoteOff (`music.cpp:99-107`).
- Timing: `_ppqn = 1`, `setTempo(16667)` (≈ matches `setTimerRate`); player loops
  (`music.cpp:307-318`, `:354`).

### 9.4 `.INS` (Amiga) and `.SX`/`.DAT` (Mac) sounds
- `.INS` is read whole and decoded as **AIFF** (`Audio::makeAIFFStream`) — `dgds.cpp:680-685`,
  `2183-2191`. JC/Amiga uses `DYNAMIX.INS` (`dgds.cpp:1676`).
- Mac `.SX` bank: `INF:` (type + index table), `TAG:`/`FNM:` string tables, and `DAT:` entries
  each `{uint16 idx; uint16 type; byte compression; uint32 unpackSize}` + compressed body
  (`dgds.cpp:1094-1141`).
- DOS `.SNG`: `SNG:` = raw song bytes; `INF:` = `uint16[]` index table (`dgds.cpp:1062-1092`).

---

## 10. TTM bytecode (the animation/"tableau" scripts)

TTM lives in the `TT3:` chunk of a `.TTM` file (packed). Two readers exist: a disassembler
(`dgds.cpp:864-908`) and the actual **interpreter** `TTMInterpreter::run()`
(`dgds.cpp:1592-1908`). Other `.TTM` chunks: `VER:` (version string), `PAG:` (uint16 page
count), `TTI:` (instruction count), `TAG:` (id→string table) — `dgds.cpp:854-929`.

### 10.1 Instruction encoding (`dgds.cpp:1610-1644`)
```
uint16 code (LE)
count = code & 0x000F        ; number of int16 operands (0..14)
op    = code & 0xFFF0        ; opcode (low nibble cleared)
if count == 0x0F:            ; special: operand is a string
    read UTF-16-ish byte pairs until a 00 00 pair  (ASCIIZ, 2 bytes at a time)
else:
    read 'count' × int16 (LE) operands
```
So opcodes are 12-bit (top 12 bits), low nibble is the operand count; count `0xF` switches to a
string literal operand.

### 10.2 TTM opcode table (as implemented in `run()`)
Hex = the masked `op` (low nibble is the arg count, shown separately).

| Opcode | Args | Name / behaviour (engine comments) |
|--------|------|-------------------------------------|
| `0x0000` | 0 | **FINISH** — end of frame/no-op |
| `0x0020` | 0 | **SAVE BG / swap buffers** — `bottomBuffer.copyFrom(topBuffer)` (makes drawn bmp persist) |
| `0x0080` | 0 | DRAW BG (unimplemented; falls through to warning) |
| `0x0110` | 0 | PURGE BMPs? (unimplemented) |
| `0x0ff0` | 0 | **REFRESH / FLUSH** — composite bottom+top into `resData`, render text bubble, present |
| `0x1020` | 1 | **DELAY** — `script->delay += arg*10` (ms) |
| `0x1030` | 1 | **SET BMP** — select sub-image `id` of current BMP set into `_bmpData` (-1 = none) |
| `0x1050` | 1 | **SELECT BMP** — set active BMP-name slot `id` |
| `0x1060` | 1 | **SELECT SCR/PAL** — set active screen/palette slot `sid` |
| `0x1090` | 1 | **SELECT SONG** — (no-op here) |
| `0x10a0` | 1 | SET SCR/PAL id (unimplemented) |
| `0x1100` | 1 | ? arg [9] (unimplemented) |
| `0x1110` | 1 | **SET SCENE** — `script->scene = arg`; also triggers speech-bubble text in INTRO.TTM/BIGTV.TTM |
| `0x1300` | 1 | ? arg [72,98,99,100,107] (unimplemented) |
| `0x1310` | 1 | ? arg [107] (unimplemented) |
| `0x2000` | 2 | SET FRAME1? i,j [0..255] (unimplemented) |
| `0x4000` | 4 | **SET WINDOW** — `drawWin = Rect(x,y,w,h)` |
| `0x4110` | 4 | **FADE OUT** — delay, set palette to black, clear bottom buffer |
| `0x4120` | 4 | **FADE IN** — restore real palette |
| `0x4200` | 4 | **STORE AREA** — persist rect of composited image into bottomBuffer |
| `0xa050` | 4 | **GFX/blit** — composite bottom+top → topBuffer (z-aware blit; used in INTRO9) |
| `0xa100` | 4 | **SET (bmp) WINDOW** — `bmpWin = Rect(x,y,x+w,y+h)` |
| `0xa500` | 0 or 4 | **DRAW BMP** — blit current `_bmpData` at (x,y); 4-arg form (CHINA) also sets tile-id/bmp-id then draws |
| `0xa520` | 2 | **DRAW BMP at x,y** (one occurrence in INTRO.TTM); shares `0xa500` handler |
| `0xa530` | 4 | DRAW BMP4 — radial-symmetry draw (Dynamix logo star) (unimplemented) |
| `0xf010` | str | **LOAD SCR** — load screen image file `name.SCR` into bottomBuffer |
| `0xf020` | str | **LOAD BMP** — register BMP filename in slot `id` |
| `0xf050` | str | **LOAD PAL** — load palette file |
| `0xf060` | str | **LOAD SONG** — play music (`playMusic`); on Amiga plays `DYNAMIX.INS` SFX |

(`dgds.cpp:1651-1883`. Opcodes listed in the final `case`/fallthrough block 0x10a0, 0x2000,
0xa530, 0x0110, 0x0080, 0x1100, 0x1300, 0x1310 are recognized but currently `warning(
"Unimplemented TTM opcode")`.)

The standalone disassembler at `dgds.cpp:864-908` confirms the same `count = code&0xF`,
`op = code&0xFFF0`, `count==0xF ⇒ string` decoding (independent of the interpreter).

---

## 11. ADS bytecode (the scene/sequence scripts)

ADS lives in the `SCR:` chunk of a `.ADS`/`.ADL`/`.ADH` file (packed). The `.ADS` file also has:
- `VER:` version string (`dgds.cpp:967-971`, `2162-2166`).
- `RES:` a **numbered list of TTM resource filenames** the script uses (`dgds.cpp:972-979`,
  `2131-2154`): `uint16 count`, then per entry `uint16 idx` (asserted == i+1) + ASCIIZ name.
  These are the TTM files the ADS orchestrates.
- `TAG:` string table.

### 11.1 Instruction encoding (`dgds.cpp:989-1041` disassembler; `2069-2122` interpreter)
```
uint16 code (LE)
if (code & 0xFF00) == 0:   -> PUSH literal (code & 0xFF) : a TAG/resource id
else:                      -> opcode (a 16-bit value, NOT nibble-split like TTM)
```
ADS opcodes are full 16-bit words; operands (when present) are read as additional `uint16`s.

### 11.2 ADS opcode table
The only opcode with implemented behaviour is `0x2005`:

| Opcode | Args | Name / behaviour |
|--------|------|------------------|
| `0x2005` | 4×uint16 | **ADD_TTM / run sub-script** — `subIdx = args[0]; subMax = args[1]`; selects which loaded TTM to run and an upper scene bound (`dgds.cpp:2078-2089`). Disassembler labels it "? (res,rtag,?,?)". |
| `0x1350` | — | "? (res,rtag)" |
| `0x1330` | — | (blank desc) |
| `0x1510` | — | "? ()" |
| `0xFFFF` | 0 | **return** (end of a sub) — disassembler prints "return" and a separator (`dgds.cpp:1007-1010`) |
| `0xF010` `0xF200` `0xFDA8` `0xFE98` `0xFF88` `0xFF10` | 0 | "INT" — interrupt/builtin calls (disassembler: `INT 0xXXXX`) |
| `0x0190` `0x1070` `0x1340` `0x1360` `0x1370` `0x1420` `0x1430` `0x1500` `0x1520` `0x2000` `0x2010` `0x2020` `0x3010` `0x3020` `0x30FF` `0x4000` `0x4010` | ? | recognized opcodes, marked "?" — semantics unknown |

(Disassembler: `dgds.cpp:996-1039`. Interpreter: `dgds.cpp:2090-2120` lists the same set but
every one except `0x2005` falls to `warning("Unimplemented ADS opcode")`.)

`PUSH` (`(code&0xFF00)==0`): pushes a small integer = an ADS:TAG or TTM:TAG id
(`dgds.cpp:992-994`).

### 11.3 ADS execution model (`ADSInterpreter`, `dgds.cpp:1970-2173`)
- `load()` parses the ADS, reads the `RES:` name list, then **loads every referenced TTM** into
  its own `TTMData` via a nested `TTMInterpreter` (`dgds.cpp:2009-2017`).
- `init()` creates a parallel `TTMState` per referenced TTM (`dgds.cpp:2036-2052`).
- `run()` (`dgds.cpp:2054-2124`): if a sub-script is active (`subMax != 0`) it steps that TTM
  one frame and stops it when `state->scene >= subMax`; otherwise it reads the next ADS opcode.
  So ADS = a small driver that sequences TTM "movies".

---

## 12. Other file types (`.GDS`, `.SDS`, `.REQ`, `.RST`, `.TDS`/`.DDS`)

- **`.GDS`** (`dgds.cpp:931-962`): `INF:` = `uint32 mark` + 6-char version; `SDS:` = a packed
  bitfield stream (`uint16` tokens, terminator logic on bits `0x80`/`0xF0`). Game-global logic.
- **`.SDS`** (`dgds.cpp:797-853`): `SDS:` = `uint32 mark`, 6-char version, `uint16 idx`
  (→ `S<idx>.SDS`), then an opcode-ish stream from which **dialogue/speech-bubble strings** are
  extracted (the `0x4,0x3,0x0` pattern → count-prefixed string, `dgds.cpp:818-844`). JC's
  storyboard text would live in equivalent scene scripts.
- **`.REQ`** (`dgds.cpp:1049-1060`): UI requestors — nested `TAG:`→`REQ:`/`GAD:` string tables.
- **`.RST`** (`dgds.cpp:579-616`): flat state/restart table — `uint32 mark` then records of
  `uint16 idx + 7×uint16`, then `uint16 idx + 2×uint16` (described as "elaborate guesswork").
- **`.TDS`/`.DDS`** (HoC, `dgds.cpp:754-796`): `THD:`/`DDS:` carry `uint32 mark`, 7-char
  version, then ASCIIZ bmp/person/tag names (talk-head portraits).
- **`.VIN`/`.AMG`** (`dgds.cpp:690-704`): plain newline-delimited text lists.
- String tables everywhere use the same shape: `uint16 count`, then per entry `uint16 idx` +
  ASCIIZ string — `readStrings()` (`dgds.cpp:465-481`) and `loadTags()` (`dgds.cpp:530-557`).

---

## 13. Games / data files in `detection_tables.h`

`detection_tables.h` lists **only** *Rise of the Dragon* and *Heart of China* — **Johnny
Castaway / SCRANTIC is NOT in this engine's detection table.** Game ids: `"rise"`, `"china"`,
generic `"dgds"` (`detection.cpp:29-35`). Each entry keys on two files: the **index** (`volume.vga`
/ `volume.rmf`) and a **volume** (`volume.001`):

| Game | Platform | Index file | Index MD5 / size | Volume file | Volume MD5 / size |
|------|----------|-----------|------------------|-------------|-------------------|
| Rise of the Dragon (PC, GOG) | DOS | `volume.vga` | `2d08870dbfeff4f5e06061dd277d666d` / 8992 | `volume.001` | `5210b0a77f89bfa2544970d56b23f9e4` / 1153936 |
| Rise of the Dragon (PC) | DOS | `volume.vga` | `b0583c199614ed1c161a25398c5c7fba` / 7823 | `volume.001` | `3483f61b9bf0023c00a7fc1b568a54fa` / 769811 |
| Rise of the Dragon (Amiga) | Amiga | `volume.rmf` | `44cd1ffdfeb385dcfcd60563e1036167` / 8972 | `volume.001` | `71b0b4a623166dc4aeba9bd19d71697f` / 519385 |
| Rise of the Dragon (Mac) | Macintosh | `volume.rmf` | `fe8d0b0f68bb4068793f2ea438d28d97` / 7079 | `volume.001` | `90b30eb275d468e21d308ca836a3d3b8` / 1403672 |
| Heart of China (PC, GOG) | DOS | `volume.rmf` | `94402b65f07606a2fb5591f9dc514c19` / 10008 | `volume.001` | `26354d54b9f2e220620b0c1d31ed5a83` / 1096322 |
| Heart of China (PC) | DOS | `volume.rmf` | `677b91bc6961824f1997c187292f174e` / 9791 | `volume.001` | `3efe89a72940e85d2137162609b8b883` / 851843 |
| Heart of China (Mac) | Macintosh | `volume.rmf` | `6bc1730f371c7330333bed4c66fe7511` / 9918 | `volume.001` | `bca16136f0fd36d25b1b1ba1870aa97f` / 1240128 |

(`detection_tables.h:29-150`.)

**Index naming pattern:** PC RotD uses `VOLUME.VGA`; everything else uses `VOLUME.RMF`; data is
`VOLUME.001`. **Johnny Castaway** instead uses `RESOURCE.MAP` + `RESOURCE.001`/`RESOURCE.002`…
The same index/volume **format** (salt + nvolumes + per-volume file lists with hash+offset,
§1) applies — only the file names differ. So for Wilson Reborn we reuse §1's parser against
`RESOURCE.MAP`.

Engine-internal strings confirming the original copyright: `"Dynamix Game Development System
(C) Dynamix"` (`detection.cpp:56`).

---

## 14. Concrete Johnny Castaway / SCRANTIC-relevant filenames hard-coded here

These literal names appear in the engine and are the kind of files we expect in SCRANTIC too
(same engine family):
- `DRAGON.FNT`, `CHINA.FNT`, `4X5.FNT`, `P6X6.FNT` — fonts (`dgds.cpp:2418,2426,2460-2462`).
- `DRAGON.PAL` — palette (`dgds.cpp:2300`).
- `INTRO.ADS`, `TITLE.ADS` — top-level scene scripts (`dgds.cpp:2410,2422`).
- `INTRO.TTM`, `BIGTV.TTM`, `TITLE1.TTM`, `TITLE2.TTM` — animation scripts (`dgds.cpp:1822,1835,2408-2409`).
- `S55.SDS`, `S<idx>.SDS` — scene/dialogue scripts (`dgds.cpp:2419,810`).
- `DYNAMIX.SNG` — music (`dgds.cpp:2380`); `DYNAMIX.INS` — Amiga instrument bank (`dgds.cpp:1676`).
- Comment list of real BMP names (`dgds.cpp:1239-1240`): `DCORNERS.BMP, DICONS.BMP,
  HELICOP2.BMP, WALKAWAY.BMP, KARWALK.BMP, BLGREND.BMP, FLAMDEAD.BMP, W.BMP, ARCADE.BMP`;
  tilemap BMPs `SCROLL.BMP, SCROLL2.BMP`.

For Johnny Castaway specifically the engine analog would be `RESOURCE.MAP` + a set of
`.TTM`/`.ADS`/`.SCR`/`.BMP`/`.PAL`/`.SNG`/`.SX` resources — the SCRANTIC scripting that drives
the looping vignettes corresponds to ADS (sequence) + TTM (animation) here.

---

## 15. Things this engine does that a *minimal* clone can skip — and things it must NOT

**Can skip / not implemented here either:**
- `VQT:` / `OFF:` VQ-compressed DOS pictures are *not* decoded (`dgds.cpp:1176-1177`,
  `1185-1186`, `1261-1262`). If JC uses VQT we'd need a separate decoder.
- Most ADS opcodes and several TTM opcodes are stubs (`warning("Unimplemented …")`).
- `.RST`, parts of `.GDS`/`.SDS` are "guesswork" — field meanings are uncertain.
- Save/load, real game logic, mouse/UI, requestors are not wired up.

**Must replicate exactly (load-bearing details):**
1. Chunk header colon check + `0x80000000` container bit (`dgds.cpp:381,388-394`).
2. Packed-chunk prefix = `byte compression + uint32 unpackSize`, then body of `size-5`
   (`dgds.cpp:403-410`).
3. `dgdsHash` with **int16** intermediate overflow (`dgds.cpp:440-463`).
4. RLE control-byte semantics incl. `0x80`=no-op and high-bit=repeat (`decompress.cpp:31-59`).
5. LZW: **LSB-first**, 9→12 bit width, `0x100`=clear, `0x101`=first free code, 16384-entry
   table, the `_cacheBits` alignment quirk (`decompress.cpp:61-180`).
6. 4bpp BIN(low)+VGA(high) → 8bpp recombination, 2 pixels/byte (`dgds.cpp:501-514`).
7. Palette 6-bit→8-bit `<<2` scaling (`dgds.cpp:1149-1153`).
8. TTM instruction split `count=code&0xF`, `op=code&0xFFF0`, `count==0xF` ⇒ UTF-16-ish string
   (`dgds.cpp:1610-1644`).
9. ADS instruction model: `(code&0xFF00)==0` ⇒ PUSH small id; else 16-bit opcode; `0x2005`
   runs a referenced TTM (`dgds.cpp:2069-2122`).
10. Sound resource = SCI-style driver/part list; PCM marker `0x00FE`, MIDI delta `0xF8`=+240,
    end `0xFC` (`sound.cpp`, `music.cpp`).
