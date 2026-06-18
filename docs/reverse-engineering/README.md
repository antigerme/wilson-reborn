<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Reverse-engineering tools

These two small Python tools are how the findings in
[`../knowledge-base/10-engenharia-reversa-do-original.md`](../knowledge-base/10-engenharia-reversa-do-original.md)
were obtained — a from-scratch disassembly of the **original** `SCRANTIC.EXE` (the 1992
Johnny Castaway screensaver, an NE / Win16 executable). They exist so the reverse
engineering is **reproducible and auditable**: you can re-run them on your own copy of the
original and check every claim in the report against the bytes.

> These are our own code (GPL-3.0-or-later). They are **not** a reimplementation copied
> from `jc_reborn`/`JCOS`/ScummVM — those are reimplementations, not the original. The whole
> point here is to read the **original** binary directly.

## What they are

| File | What it does |
|---|---|
| [`ne.py`](ne.py) | A self-contained **NE (Win16) parser**: segment table, entry table, module/import tables, and **per-segment relocations** resolved to targets (internal `seg:off`, or `MODULE.Api` via the Wine ordinal maps). Run standalone for a summary; imported as a module by the disassembler. |
| [`disasm.py`](disasm.py) | A **recursive-descent disassembler** (capstone, 16-bit). Seeds from the entry-table exports + every `INTERNALREF` target, follows calls/jumps, and applies the relocations so each far call / data ref is labeled with its API name or internal target. Emits per-segment annotated `.asm`, a `funcs.txt` of each function's API signature, and a coverage summary. |
| [`extract_calcpath.py`](extract_calcpath.py) | Decodes the original's **pathfinding route streams** (the 6×6 weighted per-trip tables; KB10 §10.3) and emits them as the Rust `ROUTE_STREAMS` data table (`crates/wilson-engine/src/calcpath_data.rs`). `SCRANTIC_EXE=… python3 extract_calcpath.py` prints a route summary; add `--rust` to emit the data module. |

## Inputs (you supply these — none are in the repo)

The game files are **copyright Sierra/Dynamix** and are never committed here.

1. **`SCRANTIC.EXE`** — the original executable. It ships inside the screensaver; you can
   also get it from the preserved copy on the Internet Archive
   (<https://archive.org/details/johnny-castaway-screensaver>, in `scrantic-run.zip`).
2. **Wine `.spec` files** *(optional but recommended)* — to turn imported ordinals into API
   names (e.g. `MMSYSTEM.2` → `sndPlaySound`). Grab `gdi.spec`, `user.spec`, `kernel.spec`
   and `mmsystem.spec` from the Wine source tree
   (<https://gitlab.winehq.org/wine/wine>, under `dlls/<module>16/`) and drop them in one
   directory. Without them the tools still run; imports just show as `MODULE.<ordinal>`.

## Running

```sh
pip install capstone            # the only third-party dependency

export SCRANTIC_EXE=/path/to/SCRANTIC.EXE
export WINE_SPECS_DIR=/path/to/specs      # optional
export DISASM_OUT=/tmp/disasm             # optional (default: ./out)

python3 ne.py                   # prints segments, modules, imported APIs, entry table
python3 disasm.py               # writes $DISASM_OUT/seg*.asm + funcs.txt, prints coverage
```

The disassembly output (`seg*.asm`, `funcs.txt`) is a derivative of the copyright binary,
so — like the binary itself — **keep it local; do not commit it.** Only these tools and the
written-up findings (the knowledge-base report) live in the repo.

## See also

- The findings: [`../knowledge-base/10-engenharia-reversa-do-original.md`](../knowledge-base/10-engenharia-reversa-do-original.md)
- Why reimplementations aren't the original: [`../knowledge-base/06-projetos-de-referencia.md`](../knowledge-base/06-projetos-de-referencia.md)
