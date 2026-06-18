# Knowledge Base — Wilson Reborn

Wilson Reborn is a **modern, portable clone** of the classic screensaver **Johnny
Castaway** (Sierra/Dynamix, 1992). This knowledge base gathers, completely and
non-superficially, **everything** you need to know to recreate the original and improve it: the
content (gags, story, easter eggs, holidays), the data format, the scripting engine
and the architecture — distilled from the canonical fan site and from **5 open-source
reimplementations** (not vendored in the repo; cloneable from their upstreams — see
[06](06-projetos-de-referencia.md)).

> **How this KB was assembled:** a full capture of the
> [johnny-castaway.com](https://johnny-castaway.com/) site (every page and sublink) +
> a deep read of **all** the files of the 5 reimplementations (jc_reborn, dgds/ScummVM, JCOS,
> castaway, dgds-viewer), with cross-validation between the implementations.

---

## Documents

| # | Document | Content |
|---|---|---|
| 01 | [History and Credits](01-historia-e-creditos.md) | Origin, creators, the *Screen Antics* brand, the DGDS engine, the community and the open-source lineage |
| 02 | [**Content Bible**](02-biblia-de-conteudo.md) | **ALL** behaviors, gags, characters, visitors, anniversary dates, easter eggs, the 11-day arc, bugs and the scene→behavior map. *(the "lose nothing" record)* |
| 03 | [Original Data and Formats](03-dados-originais-e-formatos.md) | `RESOURCE.MAP/.001`, DGDS chunk container, RLE/LZW, `.ADS/.TTM/.SCR/.BMP/.PAL` formats |
| 04 | [Scripting Engine: Opcodes](04-engine-scripting-opcodes.md) | Complete reference of **TTM** and **ADS** opcodes (consolidated from the 4 implementations) |
| 05 | [Engine Architecture](05-arquitetura-do-engine.md) | Main loop, 20 ms tick, story director, walk/pathfinding, layered rendering, island, sound |
| 06 | [Reference Projects](06-projetos-de-referencia.md) | Comparison of the 5 repos, what to reuse, licenses |
| 07 | [**Modern Port Plan**](07-plano-do-port-moderno.md) | Recommended stack, resolution independence, packaging, **roadmap of improvements**, phased plan, open decisions |
| 08 | [Decisions and Status](08-decisoes-e-status.md) | Consolidated state: firm decisions (ADR), processes and the phased roadmap |
| 09 | [**Parity and Easter Eggs Audit**](09-paridade-e-easter-eggs.md) | Bible × implementation comparison: with the original data (`--data`/auto-detection/`embed-data`) = **total parity** *(the "recreated pack" was removed in the 2026-06-15 pivot; kept only as history)* |
| 10 | [**Reverse Engineering of the Original**](10-engenharia-reversa-do-original.md) | **Byte-for-byte** verification of the tables vs `SCRANTIC.EXE`, opcode/resource coverage over the real data, and the **parity-gap report** |

**Raw technical notes** (detailed per-repository dumps, with complete opcode tables
and file:line references): [`raw/`](raw/).

---

## 1-minute summary

- **What it is:** "the first screensaver that tells a story" — Johnny, a castaway on an
  island with a coconut palm, lives through gags and an **11-day narrative** (the mermaid *Mary*
  and the city girl *Suzy*) that **advances according to the computer's real date**.
- **How it works:** the **DGDS/SCRANTIC** engine interprets two bytecodes — **TTM**
  (per-scene animation) and **ADS** (sequencing) — over data in `RESOURCE.001`. A
  **director** (`story`) draws scenes at random, has Johnny **walk** between 6 spots on the island, and
  applies **day/night, tide and holidays**.
- **Gold reference:** **`jc_reborn`** (C/SDL2) for gameplay; **ScummVM `dgds`** (C++)
  for the format; **JCOS** (C#) for the opcode dictionary; **castaway/dgds-viewer**
  (JS) for metadata and tooling.
- **For the port:** mirror the 4 layers (I/O → VMs → backend → logic), 20 ms tick, port
  verbatim the 3 tables that do not come from the data (`story/walk/calcpath`), and read
  `RESOURCE.*` with RLE/LZW + chunks.
- **Open decisions** (see [07 §10](07-plano-do-port-moderno.md#10-decisions-to-confirm-)):
  language/stack, asset strategy, MVP scope, license.

---

## Sources
- Canonical site: https://johnny-castaway.com/ (all pages).
- Reference reimplementations (**not vendored** in the repo — clone the upstreams):
  `jc_reborn` (jno6809) <https://github.com/jno6809/jc_reborn>;
  `Johnny-Castaway-Open-Source`/JCOS (nivs1978) <https://github.com/nivs1978/Johnny-Castaway-Open-Source>;
  `castaway` & `dgds-viewer` (xesf) <https://github.com/xesf/castaway> · <https://github.com/xesf/dgds-viewer>;
  `dgds` (ScummVM) <https://github.com/scummvm/scummvm>.
- Wikipedia, Computer Gaming World, Dynamix Wiki, Sierra Chest.
