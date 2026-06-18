# 03 — Original Data and File Formats (DGDS / SCRANTIC)

> Byte-by-byte specification of the Johnny Castaway data format, for Wilson
> Reborn to **load the original files** (`RESOURCE.MAP` + `RESOURCE.001`).
>
> Sources: `repos/jc_reborn` (`resource.c`, `uncompress.c`) and `repos/castaway`
> (`docs/resindex.md`, `src/dgds/*`) — authoritative for **Johnny Castaway**; and the
> ScummVM DGDS engine (`repos/dgds`) — authoritative for the **DGDS family**
> (Rise of the Dragon, Heart of China). Where JC differs from the family, it is marked.
> Complete raw notes: [`raw/jc_reborn-notes.md`](raw/jc_reborn-notes.md),
> [`raw/dgds-scummvm-notes.md`](raw/dgds-scummvm-notes.md),
> [`raw/jcos-csharp-notes.md`](raw/jcos-csharp-notes.md).

---

## 1. Layered overview

```
RESOURCE.MAP  (index)   ─┐
                         ├─► list of entries { uncompressed_size, offset }
RESOURCE.001  (file)    ─┘     │
   └─ each entry = [name 13b][size u32][ data in CHUNKS ]
                                                   │
            each chunk = ["XXX:"][size u32] (+ if "packed": [method][unpackSize u32][body])
                                                   │
                  decompressed body (None / RLE / LZW)
                                                   │
       interpreted according to the resource TYPE (.ADS .TTM .SCR .BMP .PAL ...)
```

Only **two codecs** (RLE and LZW) and the **chunk container with an ASCII tag** are
needed to read everything Johnny needs.

---

## 2. Required original files

| File | Size (bytes) | MD5 | Role |
|---|---:|---|---|
| `RESOURCE.MAP` | 1,461 | `8bb6c99e9129806b5089a39d24228a36` | Resource index |
| `RESOURCE.001` | 1,175,645 | `374e6d05c5e0acd88fb5af748948c899` | File with all the resources |
| `SCRANTIC.SCR` | — | — | The screensaver **executable** (contains the engine + Johnny's embedded walk table — see §8). Not to be confused with internal `.SCR` resources. |
| `sound0.wav` … `sound24.wav` | various | (see `repos/jc_reborn/README.md`) | 24 sound effects (extracted by JCOS; see §7) |

> **Important:** the name of the data file (`RESOURCE.001`) **is read from within**
> `RESOURCE.MAP` (a 13-byte field), it is not hardcoded (`resource.c:358`). This
> allows multiple volumes (`RESOURCE.002`…) in the family format, although JC uses
> only one.

---

## 3. `RESOURCE.MAP` — the index

Layout read by `parseMapFile()` (`jc_reborn/resource.c:342`); confirmed in
`castaway/docs/resindex.md`:

```
offset  size  field
0       4    "salt" / unknown (4 bytes; in the DGDS family these are hash seeds)
4       2    numEntries (u16 LE)            ; number of resources
6       1    unknown
7       13   resFileName                    ; ASCIIZ, name of the data file ("RESOURCE.001")
20      ...  numEntries × { u32 length; u32 offset }
             length = UNCOMPRESSED SIZE of the entry
             offset = position of the entry inside RESOURCE.001
```

> The **compressed** size of each entry is obtained from the difference between
> consecutive offsets (`resindex.md`).

### ⚠️ JC × DGDS family divergence (crucial)
In **Rise of the Dragon / Heart of China** (ScummVM), the index (`VOLUME.RMF`/`.VGA`)
is structured as **multi-volume with hash**:
```
salt[4], u16 nvolumes, then per volume:
   name[13], u16 nfiles, nfiles × { int32 hash; u32 offset }
```
where `hash = dgdsHash(fileName, salt)` and the name is resolved by hash. **In Johnny
Castaway, the first `u32` of each entry is the _uncompressed length_, not a
hash** — and the resource name is read from within `RESOURCE.001` (§4). That is: for
Wilson Reborn, **use the `jc_reborn`/`castaway` parser** (length+offset), not the
ScummVM one. The `dgdsHash()` function (with deliberate `int16` overflow) is relevant only if one
day we want to support the other DGDS games.

---

## 4. `RESOURCE.001` — the resource file

For each index entry, `parseResourceFile()` (`resource.c:373`) does
`fseek(offset)` and reads:
```
13   resName (ASCIIZ)        ; the LAST 4 chars give the type: ".ADS" ".BMP" ".PAL" ".SCR" ".TTM"
4    resSize (u32 LE)
...  data in chunks (see §5)
```
The type determines the parser. Limits in jc_reborn: `MAX_ADS 100, BMP 200, PAL 1, SCR 20,
TTM 100`. There is exactly **1 global palette** (`palResources[0]`), used in everything.

`FILES.VIN` appears as a `.VIN` resource but is **ignored** (it is just a file
listing) — `resource.c:427`.

---

## 5. Chunk container

Each resource is a sequence of **chunks** with an ASCII tag. Header (read by
`DgdsChunk::readHeader`, `dgds.cpp:371`):
```
4    id      ; 3 letters + ':'  (the 4th byte MUST be ':' = 0x3A, otherwise invalid parse)
4    size (u32 LE)
```
- **Container bit:** if `size & 0x80000000`, the chunk is a **container** (nested) and
  has **no body of its own** — the following chunks are its children. Clear the bit:
  `size &= ~0x80000000`.
- **"Packed" (compressed) chunk:** some chunks (depending on the file type) have their body
  compressed, prefixed by:
  ```
  1    compressionMethod   ; 0=None, 1=RLE, 2=LZW  (3=RLE2 in some; see §6)
  4    unpackSize (u32 LE)  ; decompressed size
  ...  compressed body      ; size = size - 5
  ```

**Relevant chunk tags** (complete list in the ScummVM notes):
`VER:` version · `RES:` numbered list of `.TTM` names (in `.ADS`) · `SCR:` ADS
bytecode **or** screen image · `TT3:` TTM bytecode · `TAG:` id→string table · `PAG:`
page count (TTM) · `TTI:` instruction count (TTM) · `INF:` header
(image dimensions / indices) · `BIN:` pixel plane (low nibble) · `VGA:` pixel
plane (high nibble) **or** palette · `DIM:` dimensions (SCR) · `MA8:` 8bpp pixels
(256 colors, Heart of China) · `MTX:` tilemap · `SNG:` music · `FNT:` font.

---

## 6. Decompression

Dispatcher by `compressionMethod`:

### 6.1 Method 0 — None
Direct copy.

### 6.2 Method 1 — RLE (`uncompressRLE`, `uncompress.c:180`)
Driven by a control byte:
- byte `0x80` → **no-op** (writes nothing);
- high bit **set** (`& 0x80`) → **repetition**: `count = control & 0x7F`; reads 1 byte and
  repeats it `count` times;
- high bit **clear** → **literal**: copies the next `control` bytes verbatim.

### 6.3 Method 2 — LZW (`uncompressLZW`, `uncompress.c:77`)
Variable-width LZW, **bits in LSB-first order**:
- starts at **9 bits**, grows up to **12 bits** (max **4096** codes);
- `free_entry` starts at **257** (256 = `0x100` reserved);
- **code `0x100` = CLEAR/reset**: realigns the bit stream to the next group
  boundary, returns to 9 bits and `free_entry=256`;
- handles the classic "KwKwK" case (code not yet in the table).

> **Nuances that MUST be replicated exactly** (from ScummVM): the bit-alignment
> accounting (`_cacheBits`) at the moment of the CLEAR is a Dynamix-specific
> detail. ScummVM allocates a 16384-entry table, but the 12-bit width limits it to
> 4096 effective codes — the value that matters is **9→12 bits / 0x100=clear /
> 0x101=first free code / LSB-first**.

### 6.4 Method 3 — RLE2 (JCOS only)
JCOS (`Compression.cs`) mentions a method **3 = RLE2**. Neither jc_reborn nor
ScummVM implements it, and the JS ports throw an error on it — probably **not used** by
Johnny Castaway. Treat it as "unsupported / investigate if it shows up".

---

## 7. Formats per resource type

### 7.1 `.ADS` (sequence script) — `parseAdsResource` (`resource.c:54`)
```
VER: + size + version (5 bytes)
ADS: + 4 unknown bytes
RES: + size + u16 numRes + numRes×{ u16 id; char name[≤40] }   ; maps slot→.TTM file
SCR: + bytecode block (PACKED: method + unpackSize + body)      ; the ADS script
TAG: + size + u16 numTags + numTags×{ u16 id; char desc[≤40] }  ; sequence names
```
The ADS bytecode is in [04-opcodes](04-engine-scripting-opcodes.md).

### 7.2 `.TTM` (animation script) — `parseTtmResource` (`resource.c:269`)
```
VER: + version
PAG: + u32 numPages + 2 unknown
TT3: + bytecode block (PACKED)                   ; the animation
TTI: + 4 unknown
TAG: + numTags×{ u16 id; char desc[≤40] }        ; "scenes" (entry points) within the TTM
```

### 7.3 `.BMP` (sprite sheet) — `parseBmpResource` (`resource.c:134`)
```
BMP: + u16 width, height
INF: + size + u16 numImages + u16 widths[numImages] + u16 heights[numImages]
BIN: + compressed pixel block (4-bpp, 2 pixels/byte)
```
It is a **spritesheet**: a concatenated stream of pixels, sliced per image according to the
widths/heights. Pixels are **4 bits** (16 colors) — 2 pixels per byte (high nibble
first).

### 7.4 `.SCR` (full-screen image) — `parseScrResource` (`resource.c:222`)
```
SCR: + totalSize + flags
DIM: + size + u16 width, height
BIN: + full-screen 4-bpp image (compressed)
```

### 7.5 `.PAL` (palette) — `parsePalResource` (`resource.c:183`)
```
PAL: + size + 2 unknown
VGA: + 4 bytes
256 × { r, g, b }   ; 6-bit VGA values (0..63)
```
Only the **first 16 colors** are used (JC is 16 colors). Conversion to 8 bits:
`component << 2`. **Watch the storage order** in jc_reborn: it stores as
BGR (`[0]=b<<2, [1]=g<<2, [2]=r<<2`).

### 7.6 Images in 256-color games (DGDS family)
JC is 16 colors (only `BIN:`). In the larger games, color comes from **two 4-bit planes**:
`VGA:` (high nibble) + `BIN:` (low nibble), recombined 2 pixels per pair of bytes
(`convertBitmap`, `dgds.cpp:501`). `MA8:` = direct 8bpp (Heart of China). Useful to know if
Wilson Reborn ever supports other DGDS games in the future.

---

## 8. Data that is **not** in `RESOURCE.001`

Johnny's **walk animation table** (walk frames + bookmarks) **is not in
the resources** — it was extracted from the **executable `SCRANTIC.SCR`** by the utility
`extract_walk_data.c` (reads triplets from offset `0x188ea` to `0x019456`, with
`flip = word0>>15`, `spriteNo = word0 & 0x7fff`). This produced `walk_data.h`.

Likewise, `story_data.h` (scene/day table) and `calcpath_data.h` (pathfinding adjacency
matrix) **encode the designer's intent that cannot be
recovered from the data** — they were reconstructed by observation. See
[05-architecture](05-arquitetura-do-engine.md) §walk and §pathfinding.

> **Consequence for Wilson Reborn:** these 3 data sets (`story_data.h`,
> `walk_data.h`, `calcpath_data.h`) need to be **ported verbatim** (or
> re-extracted from `SCRANTIC.SCR`), since they do not come from `RESOURCE.001`.

---

## 9. Sounds

The 24 effects are loaded as **external `.wav`** files (`sound0.wav`…`sound24.wav`,
missing some indices like 11 and 13) by jc_reborn/JCOS — they were **extracted** by
Hans Milling (JCOS). In the original, the SCRANTIC effects were in the DGDS family's
**`.SX`** format (a container with `INF:`/`TAG:`/`DAT:` chunks, PCM marked by `0x00FE`). The
jc_reborn plays a 1-channel software mixer; `sound0` is the generic plot scene transition
cue. Details of MIDI music (`.SNG`) and PCM are in the ScummVM notes
(§9), but **JC practically uses only short PCM effects**.

---

## 10. Summary: the minimum to read everything from Johnny

1. Parse `RESOURCE.MAP` (length+offset, JC format — §3).
2. For each entry in `RESOURCE.001`: read name+size, then the chunks (§5).
3. Implement **RLE** and **LZW** (§6).
4. Parsers for `.ADS`, `.TTM`, `.BMP`, `.SCR`, `.PAL` (§7).
5. Port/extract `story_data.h`, `walk_data.h`, `calcpath_data.h` (§8).
6. Interpret the TTM/ADS bytecode ([04](04-engine-scripting-opcodes.md)) under the engine's
   architecture ([05](05-arquitetura-do-engine.md)).
