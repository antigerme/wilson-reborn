# 01 — History, Credits and Open-Source Lineage

> Part of the **Wilson Reborn** Knowledge Base — a modern, portable clone of the
> *Johnny Castaway* screensaver. This document establishes the historical context,
> the credits of the original and the "family tree" of the open-source projects that
> serve as reference.

---

## 1. What Johnny Castaway is

*Johnny Castaway* (full commercial name: **"Screen Antics™: Johnny Castaway"**)
was released in **November 1992** for **Windows 3.1**. It was advertised as
**"the world's first story-telling screen saver"**.

Unlike the screensavers of the era (pipes, stars, flying toasters), Johnny
was not a random *loop*: it stages the life of a castaway on a tiny island
with **a single coconut palm**, and a **background narrative slowly reveals
itself over the real days** — the screensaver reads the system clock/calendar
and advances the story as real-world time passes.

The humor consists of **sight gags** (visual jokes) in the style of the comics by
**Johnny Hart (B.C.)** and with the comic castaway premise à la *Gilligan's Island*:
every rescue attempt fails in absurd ways.

- **Original platform:** Windows 3.1 (16-bit), required a 386SX. Distributed on a
  3½" floppy disk.
- **Stable version:** 1.02 (1993).
- **Business model:** a product that was cheap to produce and very profitable — cited by
  Ken Williams in the same category as *The Incredible Machine* and *Hoyle Card Games*.

### Reception
*Computer Gaming World* called the launch "a great launch" for the Screen Antics
brand and concluded: *"Fans of Johnny Hart-style comics and sight gag lovers
everywhere should love it"*. The consensus is of a well-received novelty that reinforced
Sierra's reputation in casual software.

---

## 2. Credits of the original

| Role | Person | Note |
|---|---|---|
| Producer / mastermind | **Jeff Tunnell** | Founder of Dynamix; created **Jeff Tunnell Productions (JTP)**, a division of Dynamix |
| Character design | **Shawn Bird** | Created Johnny's "weathered but likable" look |
| Lead designer | **Chris Cole** | |
| Art director / gags | **Brian Hahn** | Responsible for the visual jokes |
| Animator | **Sherry Wheeler** | Created the animations |

**Production chain:** developed by **Jeff Tunnell Productions** (a division of
**Dynamix**), published by **Sierra On-Line** under the **Screen Antics** brand. It was
one of three projects begun in January 1992 by JTP, alongside
*The Incredible Machine* and *Turbo Learning: Mega Math*.

---

## 3. The engine: DGDS / "SCRANTIC"

Internally, the engine is the **DGDS — Dynamix Game Development System**, based on
pre-existing Sierra technology. The screensaver's "personality" is called
internally **SCRANTIC** (from *Screen Antics*) — hence the screensaver program
`SCRANTIC.SCR` (a renamed `.exe`) and the icon `SCRANTIC.ICO`.

The **same DGDS engine** runs other Dynamix games — an important fact because the
documentation/decoding of the format applies to all of them:

- *Johnny Castaway Screen Saver*
- *Rise of the Dragon*
- *Heart of China*
- *The Adventures of Willy Beamish*
- *Quarky & Quaysoo's Turbo Science*

> That is why the `dgds-viewer` repository ships screenshots `dragon.png`, `willy.png`,
> `hoc.png`, `turbosci.png` and `dynamix.png`: it is a generic DGDS resource viewer,
> not just for Johnny.

### Original data files (required to run any clone)
- **`RESOURCE.MAP`** (1,461 bytes) — resource index.
- **`RESOURCE.001`** (1,175,645 bytes) — all the compressed resources
  (animations, bitmaps, palettes, scripts).
- **`SCRANTIC.SCR`** — the **screensaver executable** (`.scr` = renamed `.exe`; embeds the
  engine, the *walk* table and the 23 sound effects). The background images are `.SCR` resources
  **inside** `RESOURCE.001`, not this file.
- Optional sounds: `sound0.wav` … `sound24.wav` (24 effects; see
  [02-content-bible](02-biblia-de-conteudo.md#11-sounds)).

> Exact MD5s and sizes are recorded in `repos/jc_reborn/README.md` and are
> reproduced in [03-data-formats](03-dados-originais-e-formatos.md).

### ⚖️ Legal note (important for Wilson Reborn)
The `RESOURCE.*` files and Johnny's bitmaps/animations are **intellectual property
of Sierra/Dynamix** (today under the holders of Sierra's rights). All the
open-source clones (JCOS, jc_reborn, castaway) **do not distribute** this data —
they require the user to provide the original files. **Recommended strategy for
Wilson Reborn:** the engine is free, but (a) by default it loads the original
`RESOURCE.*` files the user owns, and/or (b) offers a set of assets recreated from
scratch (new art) as an optional package for a 100% redistributable version. See
[07-modern-port-plan](07-plano-do-port-moderno.md).

---

## 4. The community and the canonical source of behaviors

The site **https://johnny-castaway.com/** is "the online source of all Johnny Castaway
since 1996". Originally maintained by **Maria Bare**; administration was
transferred to a new curator on **October 4, 2018**. It is the **canonical
catalog** of everything Johnny does — each page documents a category of
behavior, often with dated fan reports (1996–2008).

Pages (all captured in the [content bible](02-biblia-de-conteudo.md)):
`index/list` (A-Z index), `common`, `fishing`, `swimming`, `reading`, `mermaid`,
`pirates`, `seagull`, `visitors`, `leaving` (escape/departure), `annivers` (anniversary
dates), `story`, `unusual` (rare/easter eggs), `bugs`.

---

## 5. Open-source lineage (the reference projects)

Five independent reimplementations of the **same** engine — together they form the complete
technical reference. Historical precedence matters: each project deciphered one more
piece of the format.

| Project (folder) | Author | Language / Stack | Role as reference |
|---|---|---|---|
| **JCOS** — `Johnny-Castaway-Open-Source` | Hans Milling (*nivs1978*), 2015 | C# / WinForms (.NET) | **Pioneer.** First to decode all the data files and to understand many TTM/ADS instructions. Also publishes the extracted `sound*.wav`. |
| **jc_reborn** | Jérémie Guillaume (*jno6809*), 2019 | C / SDL2 | **Most complete as a playable engine.** Understands almost every TTM/ADS instruction, implements walking between scenes, the random scene scheduler, island/cloud drawing, the 11-day cycle and holidays. Runs on Linux and Windows (MinGW), 32/64-bit. **Best gameplay blueprint.** |
| **castaway** | Alexandre Fontoura (*xesf*) | JavaScript (ES Modules), web (canvas) | Web port; documents the format (`docs/resindex.md`), scene metadata with **descriptive names** and an *improvements roadmap* very aligned with your goals. |
| **dgds-viewer** | Alexandre Fontoura (*xesf*) | JS + React + Electron | Generic DGDS resource viewer (all 5 games). More elaborate script interpreter (`process.js`). Good for asset inspection/debugging. |
| **dgds** (ScummVM) | Vasco Costa (*vcosta*) and contributors | C++ (ScummVM engine) | **DGDS format authority.** `detection_tables.h` lists files/MD5 of the games; decompression (RLE/LZW), fonts, music/MIDI and sound handled with engineering rigor. |

**Indirect credit:** the **xBaK** project (Guido) was the basis for understanding the
TTM and ADS commands — cited by both JCOS and jc_reborn. The *Johnny Castaway* section
of the **Sierra Chest** site (screenshots and video captures) also helped validate
behaviors.

### How the projects relate (knowledge flow)
```
xBaK (TTM/ADS) ─┐
                ├─► JCOS (C#, 2015) ──► castaway (JS) ──► dgds-viewer (JS/Electron)
ScummVM DGDS ───┘        │
 (format)                └─────────► jc_reborn (C/SDL2, 2019)  ◄── most faithful engine
```

---

## 6. Implications for Wilson Reborn

1. **Do not start reverse engineering from scratch.** `jc_reborn` (gameplay) + the `dgds`
   from ScummVM (format) cover ~95% of what is needed. The known gaps are
   documented by the authors themselves (see jc_reborn's README: "every scene works
   with only some inaccuracies").
2. **The content (gags, story, holidays) is already in the original data** — the engine
   just needs to interpret it correctly. "Lose no resource" = faithfully interpret
   `RESOURCE.001` + replicate the scheduling/holiday logic of
   `story.c`.
3. **The desired improvements** (higher resolutions, real day/night cycle, etc.) are
   feasible because the engine is simple and the assets are vectorizable/scalable. See
   [07-modern-port-plan](07-plano-do-port-moderno.md).
4. **Licenses:** jc_reborn and JCOS are **GPLv3**; castaway/dgds-viewer have their own
   license (see each upstream's `LICENSE`); the ScummVM DGDS engine is **GPLv2+ (ScummVM)**.
   Reusing code from these projects requires GPL compatibility. Reimplementing from
   the *documentation* (this knowledge base) avoids license "contagion", if
   different licensing is desired.

---

### Sources
- Canonical fan site: https://johnny-castaway.com/ (pages index, common, fishing,
  swimming, reading, mermaid, pirates, seagull, visitors, leaving, annivers, story,
  unusual, bugs, johnew).
- `repos/jc_reborn/README.md` (md5/sizes, acknowledgments, status).
- `repos/castaway/README.md` and `repos/castaway/docs/resindex.md`.
- Wikipedia *Johnny Castaway*; Computer Gaming World (via research summary);
  Dynamix Wiki (Fandom); Sierra Chest.
