# 09 — Parity and easter eggs audit

> A direct answer to "**lose no resource**": it compares the [content
> bible](02-biblia-de-conteudo.md) (everything the original has) with what Wilson
> Reborn already does. Update when resources are (re)implemented.

> **Pivot 2026-06-15:** the **recreated pack was removed**. Wilson Reborn now uses
> **100% the original files**, so the `--data` path (total parity below) is the
> **only** path — and it is the complete experience. The mentions of the "recreated pack" / the
> **R** column below remain only as a historical record.

## Conclusion in 30 seconds

The content comes **100% from the original files** (`--data`): **TOTAL parity**. The engine
**interprets the original scripts** (`.ADS`/`.TTM` from `RESOURCE.001`), so **all the
63 scenes, gags, easter eggs, visitors and plot beats appear exactly as in the
original** — we do not reimplement each gag, we **execute the same bytecodes**.
Validated end to end (see [08](08-decisoes-e-status.md), the `real_data` test).

### Opcode coverage — 100% (audited 2026-06-15)

We audited **all** the opcodes the real data uses (41 TTM + 10 ADS) vs what the
engine handles:
- **ADS:** 100% covered.
- **TTM:** 100% covered. The "saved zone" opcodes **`COPY_ZONE_TO_BG` (0x4204)** and
  **`RESTORE_ZONE` (0xA064)** — used by the **giant cargo ship gag** — are now
  implemented (a saved-zone layer composed between background and threads, like
  jc_reborn's `grUpdateDisplay`). The other opcodes the engine treats as no-op
  (`LOAD_PALETTE` 0xF05F, `SET_PALETTE_SLOT`, `SAVE_IMAGE1`, `SAVE_ZONE`, `DRAW_SCREEN`,
  `SET_FRAME1`) **are also no-op in `jc_reborn`** ⇒ we match the reference.

That is: **there is no longer any opcode from the real data being silently ignored**.

> *(History)* There was an **embedded recreated pack** (procedural art) with complete logic
> but placeholder visuals; it was **removed** on 2026-06-15 because it did not reach the desired
> quality. The **R** column in the tables below reflected that pack.

> In short: **nothing from the original is lost** — it is all accessible via `--data`
> (or auto-detection, or the `embed-data` build). Since the focus became **100% the original
> data**, **there is nothing pending for "recreated art"**: the complete experience is the one of the
> original.

## Director logic — parity ✅ (with tests)

All of this is faithfully ported from `story.c`/`story_data.h` and **covered by tests**
(`crates/wilson-engine/src/story.rs`):

| Resource | State | Where |
|---|---|---|
| Table of **63 scenes** (10 `.ADS`) | ✅ | `STORY_SCENES` (test `table_has_63_scenes`) |
| **11-day** arc + advance by real date + restart | ✅ | `Director::advance_day` (test `advance_day_clamps_and_wraps`) |
| **Plot beats** of the 11 days (Mary/Suzy/Johnny) | ✅ | `day` field (test `day_beats_match_the_story`) |
| **4 holidays** with exact date ranges | ✅ | `holiday_for_date` (test `holidays`) |
| **Raft** (5 stages per day) | ✅ | `raft_for_day` (test `night_and_raft`) |
| **Tide** low/high + **night** | ✅ | `island_from_scene`, `is_night` |
| **24h day/night** (optional improvement) | ✅ | `DayNight` (test `night_24h_cycle`) |
| **Pathfinding** 2nd-order + **walk** between spots | ✅ | `path`/`walk` (own tests) |
| **Holiday props** drawn on the island | ✅ | `island.rs` (composed onto the scenery) |

### Holidays (ranges confirmed identical to the original)

| Holiday | Range | `Holiday` |
|---|---|---|
| New Year | 12/29 → 01/01 | `NewYear` |
| St. Patrick | 03/15 → 03/17 | `StPatrick` |
| Halloween | 10/29 → 10/31 | `Halloween` |
| Christmas | 12/23 → 12/25 | `Christmas` |

The bible notes the desire for an **extensible table** (e.g. July 4) — a possible future
improvement (it needs new `Holiday` + props; degrades with `--data`, whose `HOLIDAY.BMP` only
has 4 sprites).

## Gags, characters and easter eggs — status

Legend: **D** = appears with `--data` (original script) · **R** = recreated art in the
embedded pack.

| Resource (bible §3–§10) | D | R | Note |
|---|:--:|:--:|---|
| Fishing (common/rare catches, big octopus, shark-ski, ambidexterity) | ✅ | ❌ | original script runs; recreated art pending |
| Swimming/diving + animal jury | ✅ | ❌ | |
| Bath + thief seagull + shark scare | ✅ | ❌ | |
| Reading (book upside down, nap→coconut) | ✅ | ❌ | |
| Sleeping/snoring + being tied up by the pirates | ✅ | ❌ | |
| Fire/cooking (octopus on the face) | ✅ | ❌ | |
| Coconuts (bounces, cracking on the tree) | ✅ | ❌ | |
| **Raft** + **SOS in a bottle** (mini-Johnny, day 2) | ✅ | ⚠️ | the raft grows in the recreated pack; the bottle gag does not |
| Sandcastle → King Kong pirates | ✅ | ❌ | |
| Jogging / telescope | ✅ | ❌ | |
| Rain dance (drop → lightning) | ✅ | ❌ | |
| **Mary, the mermaid** (6 interactions + beats) | ✅ | ❌ | the director picks the days; visual pending |
| **Suzy** (resort/kiss/ear-tug) | ✅ | ❌ | |
| Seagull (5 gags) | ✅ | ❌ | |
| Pirates (King Kong + Gulliver, egg on the chest) | ✅ | ❌ | |
| Visitors (motorboat, biplane, helicopter, terminator, giant ship, x3 naked…) | ✅ | ❌ | `VISITOR.ADS` runs with `--data` |
| Rare easter eggs (ghost Johnny, silver balls, real clock, melting, "feeding the fishes", "THE END/Home Again") | ✅ | ❌ | |
| **Holidays** (props on the island) | ✅ | ✅ | pumpkin/pot/pine tree/fireworks recreated |
| **Sound** (`sound0..24`, `sound0` on the beats) | ✅ | ➖ | plays the original `.wav` with `--data`; the pack does not ship `.wav` (copyright) |

## "Charm" bugs as an optional easter egg (future)

Bible §12 lists original bugs; some became **beloved jokes** ("giant island",
"dozens of Johnnys", "twins"). Improvement idea: an **optional easter-egg mode** that
reproduces them on purpose. Not implemented (it is not a regression — they are bugs, not resources).

## *(History)* Visual roadmap of the recreated pack — canceled in the pivot

> These were the next steps **when** the goal was a standalone pack of recreated
> art. The 2026-06-15 pivot **removed the pack**; they remain here only as a record. Today the
> content comes **100% from the original data**, so **there is no visual parity pending**.

In order of impact (each one was a content increment of the pack):
1. **Recreated animations by category** (fishing, bathing, reading, sleeping, coconuts…), so that
   `STAND/ACTIVITY/FISHING/...` show distinct actions instead of a standing Johnny.
2. **Recreated characters** (Mary, Suzy) for the beats of days 1/3/4/5/7/8/9.
3. **Recreated visitors** (`VISITOR.ADS`) and **rare easter eggs**.
4. **SOS in a bottle** (day 2) and **rain dance**.

With the original data (`--data`/auto-detection/`embed-data`), **all of this already appears** —
they are the game's own bytecodes.
