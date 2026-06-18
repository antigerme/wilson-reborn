# 05 — Engine Architecture (loop, director, walk, island, render, sound)

> How the pieces fit together at runtime. Based on `jc_reborn` (the most
> complete and faithful implementation). file:line point to `repos/jc_reborn/`.
> Complete notes: [`raw/jc_reborn-notes.md`](raw/jc_reborn-notes.md).

```
                    storyPlay()  [story.c]  ← the DIRECTOR (infinite loop)
                        │  picks scenes (day/tide/holiday), walks between spots
                        ▼
                    adsPlay()    [ads.c]    ← the SCHEDULER (interprets 1 ADS scene)
                        │  manages up to 10 TTM threads + background thread + holiday
                        ▼
                    ttmPlay()    [ttm.c]    ← the animation VM (1 frame of 1 thread)
                        │  draws onto the thread's layer (SDL surface)
                        ▼
                 grUpdateDisplay() [graphics.c] ← composes layers + waits a tick + presents
                        │
                 eventsWaitTick()  [events.c]   ← the heartbeat: 1 tick = 20 ms
```

---

## 1. Entry point and modes — `jc_reborn.c`

`main()` (`jc_reborn.c:152`): `parseArgs()`, always
`parseResourceFiles("RESOURCE.MAP")`, then dispatches:
- **default** → `graphicsInit(); soundInit(); storyPlay();` (the real screensaver, infinite
  loop);
- `dump` → extracts all resources to `./dump/`;
- `bench` → fps benchmark;
- `ttm <name>` → plays a TTM directly;
- `ads <name> <tag>` → plays an ADS scene.

Hot-keys (when enabled): `Esc`=quit, `Alt+Return`=fullscreen, `Space`=pause,
`Return`=advance 1 frame (paused), `M`=max/normal speed. **Without** hot-keys (screensaver
mode): **any key quits** (`exit(255)`).

---

## 2. Time model — `events.c` (the heartbeat)

Everything is measured in **ticks**; **1 tick = 20 ms** (`eventsWaitTick`: `delay *= 20`,
`events.c:108`). Nominal rate = **50 ticks/s**. A `delay` value of N in a TTM = N×20 ms.

`eventsWaitTick(delay)` does a busy-wait (`SDL_Delay(5)` granularity) until
`SDL_GetTicks() - lastTicks >= delay*20`, while polling SDL events. It respects
`paused`/`oneFrame` (step) and `maxSpeed` (the M key ignores the wait).

---

## 3. The scheduler — `adsPlay()` (`ads.c:658`)

It is the main loop **while a scene is playing**. A **cooperative, event-driven,
variable-timestep** model — it sleeps exactly until the next thread needs to be
serviced (it is not a fixed frame):

`while (numThreads)`:
1. If the **background** thread (waves) is due (`timer==0`) → `islandAnimate()`.
2. For each of the `MAX_TTM_THREADS` (**10**) TTM threads with `timer==0` → `ttmPlay()`.
3. `grUpdateDisplay(...)` (composes + waits).
4. `mini` = smallest pending `timer` among the threads (cap 300); decrements all the
   `timer`s by `mini`; `grUpdateDelay = mini`.
5. Per-thread post-processing: applies `nextGotoOffset`; decrements `sceneTimer`
   (negative ADD_SCENE); on finishing (`isRunning==2`) re-arms `sceneIterations` times
   (positive ADD_SCENE) or stops and fires `IF_LASTPLAYED`/`IF_NOT_RUNNING`
   (`adsPlayTriggeredChunks`).

**`isRunning` (tri-state+):** `0`=free slot; `1`=running; `2`=finished this step
(cleanup pending); `3`=background/holiday (drawn, not "stepped").

Constants: `MAX_TTM_SLOTS=10`, `MAX_TTM_THREADS=10`, `MAX_ADS_CHUNKS=100`,
`MAX_RANDOM_OPS=10`.

---

## 4. The story director — `story.c`

`storyPlay()` (`story.c:194`): `adsInit(); adsPlayIntro();` and then, forever:
1. `storyUpdateCurrentDay()` + `storyCalculateIslandFromDateAndTime()`.
2. Picks a **FINAL** scene (the round's climax gag) via `storyPickScene(FINAL,0)`.
3. If it is an `ISLAND` scene → computes island parameters and `adsInitIsland()`; otherwise
   `adsNoIsland()`.
4. Unless the final is also `FIRST`, it plays a chain of **`6 + rand()%14`** ambient
   scenes leading up to it. Between consecutive scenes, Johnny **walks** from the
   final spot/heading of the previous one to the initial one of the next (`adsPlayWalk`). Plot
   scenes (`dayNo≠0`) fire `soundPlay(0)`.
5. Walks to the final scene, plays it, `grFadeOut()`, frees the island.

### Scene selection — `storyPickScene(wanted, unwanted)` (`story.c:42`)
Collects every scene whose flags contain all the `wanted`, none of the `unwanted`, and whose
`dayNo` is 0 **or** equal to `storyCurrentDay`; returns a **uniformly random** one.

### Day advancement — `storyUpdateCurrentDay()` (`story.c:65`)
**Driven by the real clock and persisted** in `~/.jc_reborn` (`config.c`: `currentDay=`,
`date=`). If the calendar day (`getDayOfYear()`/`tm_yday`) differs from the stored one →
`currentDay += 1`; clamp/wrap to **1..11**. That is: the story advances **one beat per real
day** and repeats every 11 days.

> The complete table of 63 scenes (spots/headings/day/flags) and the day→scene map are in the
> [content bible](02-biblia-de-conteudo.md) §2/§13 and in
> [`raw/jc_reborn-notes.md`](raw/jc_reborn-notes.md) §7.

### Island state derived from the scene — `storyCalculateIslandFromScene()` (`story.c:123`)
- **Low tide:** if `LOWTIDE_OK` and `rand()%2`.
- **Island position:** if `VARPOS_OK`, draws 1 of 3 offset ranges; otherwise fixed
  (`LEFT_ISLAND` → `xPos=-272`, otherwise 0).
- **Raft progress:** 0 if `NORAFT`; otherwise by day (0–2→1; 3–5→day−1; ≥6→5).
- `HOLIDAY_NOK` (VISITOR.ADS#3 only, the cargo ship) forces `holiday=0`.

### Day/night and holidays — `storyCalculateIslandFromDateAndTime()` (`story.c:94`)
- **Night:** `hour = getHour()%8; night = (hour==0 || hour==7)` → loads `NIGHT.SCR`.
- **Holidays** (string comparison `MMDD`): Halloween 10/29–31 (1), St. Patrick
  03/15–17 (2), Christmas 12/23–25 (3), New Year 12/29–01/01 (4). Details and props in
  [bible §9](02-biblia-de-conteudo.md#9-anniversary-dates--holidays-annivers--storyc-logic).

---

## 5. Walk and pathfinding

Johnny moves between **6 named spots A–F** (the same nodes as the story table).
Movement = choosing a route in the spot graph, then playing pre-generated animation frames.

### Pathfinding — `calcpath.c` + `calcpath_data.h`
`NUM_OF_NODES=6`. `walkMatrix[7][6][6]` is a **second-order adjacency**:
`walkMatrix[prev][cur][next]` = 1 if you can go cur→next having come from prev (turn
constraints that avoid an abrupt reversal). The `[6]` index = "from any spot" (first hop).
`calcPath(from,to)` does a **DFS enumerating all simple paths** (up to
`MAX_NUM_PATHS=50`, `MAX_PATH_LEN=7`) and returns **a random one**. (The author admits it is
a plausible fit, not the original algorithm.)

### Animation — `walk.c` + `walk_data.h`
`walkData[][4]` = a frame table `{flip, x, y, spriteNo}`, segmented per route
(A→E, A→F, …) with `{0,0,0,0}` sentinels. **Extracted from the executable `SCRANTIC.SCR`**
(offset `0x188ea`), not from the resources. Index tables: `walkDataBookmarks[6][6]`
(start of each route), `walkDataBookmarksTurns[6]` (turn frames per spot),
`walkDataStart/EndHeadings[6][6]`.

`walkAnimate()` is a state machine (turn → walk → arrive) that returns the **delay**
until the next frame (0 on arrival). Detail: when walking between **D↔E**, Johnny passes
**behind the palm** — the engine redraws the trunk (sprite 13 @442,148) and leaves (12
@365,122) over him. Sprites come from `JOHNWALK.BMP`.

---

## 6. Island background — `island.c`

`TIslandState islandState` (`island.h:24`): `{lowTide, night, raft, holiday, xPos, yPos}`
— the global state of the current scenery.

`islandInit()` (`island.c:35`): chooses the background (`NIGHT.SCR` or `OCEAN0{0,1,2}.SCR`,
`rand%3`) and **paints the static scene directly onto the background surface** (for the TTM/walk
to compose over): raft (stages 0–5 of `MRAFT.BMP`), clouds (`BACKGRND.BMP`
sprites 15–17, 0–5 of them, mirrored according to the "wind"), island (sprite 0 @288,279),
trunk (13), leaves (12), shadow (14), and, at low tide, a strip of sand (1) + rock (2).

`islandAnimate()` (`island.c:150`): animates the beach waves (3 phases). High tide → 3
positions (sprites 3/6/9); low tide → 4 (30/33/36/39). **It is the only continuous
background animation**, driven by the scheduler's background thread (`delay=8`).

**Legend of `BACKGRND.BMP` sprites:** 0 island, 1 low-tide sand, 2 rock, 3/6/9 high-tide
waves (3 phases), 12 leaves, 13 trunk, 14 shadow, 15/16/17 clouds, 30/33/36 low-tide
waves, 39 waves on the rock. **`MRAFT.BMP`** images 0–4 = stages 1–5 of the raft.

Holiday props — `islandInitHoliday()` (`island.c:192`): loads `HOLIDAY.BMP` and
draws (onto an `isRunning=3` layer): Halloween→sprite 0 @(410,298); St. Patrick→1
@(333,286); Christmas→2 @(404,267); New Year→3 @(361,155).

---

## 7. Graphics — `graphics.c`

- **Fixed logical resolution 640×480**, 32-bpp SDL window. No scaling (origin
  `{0,0}`). *(This is the main thing to modernize — see [07](07-plano-do-port-moderno.md).)*
- **Palette:** the first 16 colors of the `.PAL` (6-bit VGA) converted to RGBA by
  `<<2`. **BGR** storage in jc_reborn.
- **Layers / double-buffer:** each TTM thread renders onto its own off-screen surface
  640×480, filled with **magenta `0xA8,0x00,0xA8`** as the transparency color
  key. `grUpdateDisplay()` composes in this order: **background → saved zones → each
  running thread → holiday layer**; waits for the tick; `SDL_UpdateWindowSurface`.
- **Primitives:** pixel, line (Bresenham), filled rectangle, circle, clip-zone.
  All respect the `grDx/grDy` offset (island-relative) and write onto the given layer.
- **Sprites:** `grLoadBmp()` decodes each sub-image into a 32-bpp surface with the magenta
  color key; `grDrawSprite` blits at `(x+grDx, y+grDy)`; `grDrawSpriteFlip` mirrors
  column by column.
- **Fade-out:** `grFadeOut()` alternates 5 transition styles (expanding circle, expanding
  rectangle, L→R, R→L, from the middle) on each call.

> **Known approximations:** `grSaveImage1`/`grSaveZone` are near-stubs;
> `grUpdateDisplay` re-blits everything every frame (no dirty-rects) — irrelevant on a
> modern GPU.

---

## 8. Sound — `sound.c`

`NUM_OF_SOUNDS=25`. `soundInit()` opens SDL audio and loads `sound%d.wav` (i=0..24;
missing ones tolerated). A **1-channel** software mixer (`soundCallback`): one "current" sound
at a time. `soundPlay(nb)` sets the pointer under `SDL_LockAudio`. **Sound 0** = generic
plot scene transition cue; the others come from `PLAY_SAMPLE` (`0xC051`) in the TTM.

---

## 9. Persistence — `config.c`

A text file `~/.jc_reborn` (or CWD if there is no `$HOME`): `currentDay=N` and `date=N`.
Used only by the story-day mechanism (§4). **In Wilson Reborn**, this equates to a
cross-platform config file (see [07](07-plano-do-port-moderno.md)).

---

## 10. What to port verbatim vs. what is data-driven

**Port verbatim (does not come from `RESOURCE.001`):**
- `story_data.h` — 63 scenes (ads/tag/spots/headings/day/flags);
- `walk_data.h` — walk frames + bookmarks (from `SCRANTIC.SCR`);
- `calcpath_data.h` — second-order adjacency matrix;
- the **opcode tables** ([04](04-engine-scripting-opcodes.md)) and the **holiday/day/night/tide/position
  logic** (§4).

**Data-driven (comes from the files):** all the animations, sprites, screens, palettes, sounds,
and the scene structure (via TTM/ADS bytecode).

**Clean layers (keep in the port):** I/O (`resource`/`uncompress`) · VM
(`ttm`/`ads`) · backend (`graphics`/`sound`) · game logic
(`story`/`events`/`island`/`walk`). Swapping SDL for any backend is straightforward.
