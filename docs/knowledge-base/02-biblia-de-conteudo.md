# 02 — Content Bible (ALL behaviors, gags, events and story)

> **Purpose of this document:** to record *exhaustively* everything Johnny does
> in the original, so that **Wilson Reborn loses no resource**. It is the "parity
> contract" with the original: events, narrative sequences, gags, easter eggs,
> anniversary dates, behaviors and antics.
>
> Sources: the canonical catalog at https://johnny-castaway.com/ + the scene data
> decoded in `repos/jc_reborn/story_data.h`, `repos/jc_reborn/story.c` and
> `repos/castaway/src/scrantic/metadata/scenes.mjs`.
>
> **How to read:** when known, each behavior points to the **`.ADS` file +
> tag** that implements it (see §13). This links the "what" (this doc) to the "how"
> ([04-opcodes](04-engine-scripting-opcodes.md) and [05-architecture](05-arquitetura-do-engine.md)).

---

## 1. The setting (Johnny's "world")

- **Tiny island** with **one coconut palm** (palm tree). It is the fixed stage.
- **Sea** all around, with animated waves; **clouds** drawn in random positions.
- **Tide**: there is **low tide**, which exposes more sand and enables
  certain scenes (fish in moonlight, etc.). Random when the scene allows (`LOWTIDE_OK`).
- **Day/night cycle**: in the original it is an **8-hour cycle** (not 24h). It is "night"
  at the edges of each 8h block. There is **moonlight fishing/diving at low tide**.
- **Island position**: redrawn in a **random position** on the screen every sequence
  (when the scene allows `VARPOS_OK`), with specific coordinate ranges.
- **The raft** grows over the course of the story (see §2 and §3). It is a scenery element
  *and* a plot element.
- **Holiday items** (Christmas tree, pumpkin, clovers, New Year banner)
  appear drawn on the island on the right dates (see §9).

---

## 2. The 11-day narrative arc (the "story")

The screensaver advances **one "story day" each time the system's real date
changes** (once per real day). The cycle has **11 days** and then restarts
(`story.c`: `currentDay` goes from 1 to 11, then resets). Central characters of the
narrative: **Mary, the mermaid** and **Suzy, the city girl**.

| Day | Narrative event | Scene (ADS#tag) |
|---:|---|---|
| **1** | The **mermaid** watches Johnny while he fishes (establishes her presence). | `MARY.ADS#2` |
| **2** | Johnny writes an **SOS and throws it in a bottle**; in the thought bubble a **mini-Johnny** appears standing on the island. | `JOHNNY.ADS#2` |
| **3** | The bottle reaches **Suzy**, in the city, who imagines being visited by an idealized Johnny. The **raft grows**. A **shark** tries to attack him during his bath. | `SUZY.ADS#1` |
| **4** | Johnny meets the **mermaid** and invites her on a date, giving her a **seashell necklace**. The raft is almost ready. | `MARY.ADS#3` |
| **5** | At night, Johnny and the mermaid **dine and dance**. He wears a **gala outfit (tails and top hat)** — but remains barefoot. | `MARY.ADS#1` |
| **6** | Johnny **draws himself embracing Suzy** and records the fantasy in another **bottle**. | `JOHNNY.ADS#3` |
| **7** | The mermaid reappears; Johnny invites her to return to the mainland with him. She **leaves crying**, refusing. | `MARY.ADS#4` |
| **8** | Johnny **bids farewell to the shark and the mermaid** and **departs on the complete raft** (rows away). | `MARY.ADS#5` *(LEFT_ISLAND, NORAFT)* |
| **9** | A **clock (frog/"frog clock")** appears; the raft floats; he reaches the beach and **reunites with Suzy**. | `SUZY.ADS#2` |
| **10** | Johnny **sleeps at an office desk**, dreaming of the island (and of the mermaid). | `JOHNNY.ADS#6` |
| **11** | Johnny **returns to the island by plane** (parachute), restarting the cycle. | `JOHNNY.ADS#1` |

> **Research note:** Wikipedia mentions an arc of "~120 days"; the open-source
> engines implement **11 days** (data from `story_data.h`). Probably a difference
> between the public's perception and the actual structure of the data. **Wilson Reborn should
> follow the 11 days of the data**, but this is configurable.

### Raft growth (logic from `story.c`)
| Story day | Raft stage |
|---:|---:|
| 0–2 | 1 (start) |
| 3 | 2 |
| 4 | 3 |
| 5 | 4 |
| 6+ | 5 (complete) |

Scenes with the `NORAFT` flag force the raft to **not** appear (e.g. day 8, when he
has already departed on it).

---

## 3. Day-to-day activities and gags ("common" behaviors)

These are the "mundane" scenes and the gags that run randomly between the plot
events. The scheduler plays **6 to 20 intermediate scenes** walking between
island "spots", and then a "final" scene (`story.c`).

### 3.1 Fishing (`FISHING.ADS`, and gags in `ACTIVITY/MISCGAG`)
The main food source; there is **moonlight fishing at low tide**.
- **Common catches:** an old boot (sometimes stashed behind the tree); a crab
  (bites Johnny's nose, who throws it back); a starfish (discarded).
- **Less common catches** (stashed behind the tree): a buoy marked **"SS Titanic"**
  (causes a graphical *glitch* above the head when he fishes from the left); a green
  fish; a wooden plank; a small octopus.
- **Rare events:**
  - **Shark:** Johnny hooks a shark and ends up **water-skiing** behind it.
  - **Big octopus:** he catches several fish and then a **huge octopus** that chases him to the
    tree, **steals all the fish** and dives back, leaving him furious.
  - **Green fish:** occasionally it **squirts water** at him, who throws it back in
    disgust.
- **Animation detail:** ambidexterity — fishing from the **right** of the island he uses a
  right-handed reel; from the **left**, a left-handed reel.

### 3.2 Swimming, diving and bathing (`swimming` / `ACTIVITY.ADS`)
- **Palm-tree dive:** he climbs the palm and dives into the sea (sometimes in moonlight, at
  low tide). **Scored dives:** a **starfish, a crab, a fish
  and a seagull** hold cards giving scores. The **crab is crazy**: it gives **−0.5**
  for a good dive and **10!** for a bad belly flop. *(scenes "MUNDANE DIVE",
  "GAG DIVES")*
- **Sea bath:** Johnny washes himself sitting in the sea; when he realizes he is being
  watched, he **covers himself as best he can**, goes to get dressed behind the tree and then **shakes
  his fist** (angry). The **seagull sometimes steals the swim trunks** (see §6.3). *(scene "JOHN BATH")*
- **Shark attack (gag):** Johnny comes to the edge with a **towel and a scrubbing
  brush**, tests the water with his big toe — the **shark jumps and bites him**; he falls
  bloodied against the palm, drops the towel/brush. When he inspects himself, he discovers
  his leg is **intact** (he was just sitting on it) and is relieved.

### 3.3 Reading (`reading` / `ACTIVITY.ADS`: "GULL READING", "JOHN READ")
- **Confused reading:** the most frequent gag — he holds the book **upside
  down**, and even "the right way up" he does not understand it; he **turns the pages from left to
  right** (fans joked the book must be "in Hebrew or Arabic").
- **Nap + coconut:** he reads, gradually **dozing off and waking up** with head jolts;
  the jolts **shake the coconut palm** until a **coconut falls on his head**.
- **Book-thief seagull:** the seagull dives and **steals the book** (see §6).

### 3.4 Sleeping (`common#sleeping`)
- Johnny **takes naps** frequently (starts **snoring** almost immediately).
- **Pirates tie him up while he sleeps** (see §7) — and, in that specific case, he does **not
  snore**.

### 3.5 Making fire and cooking (`common`)
- He tries to start a fire **rubbing sticks together**: he succeeds after **2–4 attempts** or
  gives up — and then the fire lights "**spontaneously**".
- With the fire, he **cooks his catch** (often the green fish, or the **old boot**
  when he is very hungry). When eating a **small octopus**, it **sticks to his face**
  before being eaten.

### 3.6 Eating coconuts (`common#coco`)
- Coconuts fall from the tree with **varied bounce patterns** (hard to the right; or
  two soft bounces to the left).
- In one variation, Johnny's **head spins completely around** (a fan joked he must be on
  "Devil's Island").
- When he gets the coconut, he **bangs it against the tree** to crack the shell, sits down and eats.

### 3.7 Building the raft (`common#raft` / `BUILDING.ADS`)
- He builds the **raft** (a scenery and plot piece). In long sessions, the raft
  sometimes **shrinks back to its original size** (there is also an "over-built raft" bug
  — see §12).

### 3.8 SOS in a bottle (`common#bottle`)
- He writes messages, puts them in **bottles** and throws them into the sea. They usually **wash back
  ashore**, sometimes **reach somewhere else** (Suzy — see §2).
- He thinks **"SOS"**; sometimes he thinks of a **pretty girl**. On **day 2**, the bubble shows
  a **mini-Johnny** standing on the island.

### 3.9 Sandcastle (`common#castle` / `BUILDING.ADS`)
- Johnny **builds a sandcastle** — this **triggers the "King Kong" pirates
  scene** (see §7.1).

### 3.10 Jogging / running (`common#jogging`)
- Johnny **jogs** (runs) around the island — one of the routine activities.

### 3.11 Telescope / spyglass (`visitors#notseen`)
- He uses a **telescope/spyglass** to scan the horizon. The recurring gag: while
  he looks one way, **something passes behind his back** without him seeing it (see §8.1).

### 3.12 Rain dance / "native" (`ACTIVITY.ADS`: "NATIVE 1/3")
- When it is hot, Johnny **dresses as a shaman/witch doctor** and does a **rain
  dance**. A cloud releases **a single drop** and then he **gets struck by lightning**. (It also appears in
  scenes with tourists — see §8.2.)

---

## 4. Mary, the mermaid (`MARY.ADS`)
The mermaid is named **Mary**. Interactions (some are the plot beats of days
1/4/5/7/8 — see §2):
- **Hears but does not see:** Johnny fishes, the mermaid approaches from behind; he hears the
  splash, repositions himself and hooks something heavy — **false teeth** or **an old
  boot**.
- **The invitation:** while reading, he sees her swimming nearby; she gives him a **seashell necklace**, he
  offers her the **Titanic buoy**; they think of dinner (she imagines a **green traffic
  light**), then they part.
- **The dinner:** he changes clothes behind the tree (**top hat and tails**), sets up a whole
  **dinner table**, they eat; then he brings a **gramophone** and they **dance**
  until she returns to the sea.
- **The plea:** Johnny stands on the raft **begging** her to come aboard; she leaves
  and he is **devastated**. (Variation: she asks what the raft is for,
  he shows her visions of the city, and upon discovering he wants to leave, **she cries**.)
- **The departure:** he tries to convince her to come on the raft; she stays with the **shark**
  (they are **laughing**) and Johnny departs alone.
- **Reverie:** Johnny sleeps at the office desk and **dreams** that he dines with the mermaid
  on the island (day 10).

---

## 5. Suzy, the city girl (`SUZY.ADS`) and the escapes (`leaving`)
- **The bottle and Suzy:** **Suzy** (city girl), sunbathing in a **pink bikini** at a
  resort, finds Johnny's bottle and **imagines him as an attractive man** sweeping her
  off her feet (in her reverie, she appears younger and slimmer). There is the **reverse**: Johnny
  finds Suzy's message and daydreams of her looking at her watch.
- **Departure on the raft:** "Johnny has just climbed onto the raft and rowed away",
  taking an **oar and a sack**. A **dolphin** (later identified as a **shark**) and the
  **mermaid** accompany him before he returns to normal activities.
- **Encounter at the resort:** Johnny rows past Suzy near a resort with
  skyscrapers; she **grabs and kisses** him passionately.
- **Ear-tug:** the mood sours when Suzy discovers **gum on her cleavage** after
  the kiss; furious, she fights with him and **tugs his ear**.
- **Intimate scene (unconfirmed):** several reports of "**naughty things**" between Johnny
  and a woman, with confusion over whether it was indoors or just a smaller screen.

---

## 6. The seagull (`seagull`)
The seagull almost always comes out on top; "Johnny usually comes off worse".
- **Book thief:** it takes the book to the top of the palm and "**reads**" it, turning pages with
  its beak.
- **Sits on his head:** it steals the book and lands on Johnny's head; he tries to remove it with
  a **club**, but **hits himself** (raises a bump) — the seagull keeps hovering.
- **Clothes thief:** while he bathes in the sea, it dives and **steals the swim trunks**
  (two image variations).
- **Nest in the hat:** it lands on his head, **steals the hat**, takes it to the top of the tree and
  **makes a nest** in it.
- **Nest on the chest:** after the pirates tie Johnny up, the seagull makes a **nest
  on his chest, lays an egg** and leaves (see §7.2).

---

## 7. Pirates (`pirates`)
### 7.1 "King Kong" scene (triggered by building the sandcastle)
When Johnny builds a **sandcastle**, a **miniature pirate galleon**
arrives. Little pirates row to the beach, **occupy the castle**, hoist a **flag** and
**fire cannons** at Johnny. He takes refuge in the palm while **several tiny biplanes**
take off from the castle to attack him. The sequence ends with Johnny
**falling into the water** — a parody of **King Kong (1933)** atop the Empire State.

### 7.2 "Gulliver's Travels" scene (while sleeping)
Pirates approach the **sleeping** Johnny and **tie him up with ropes** — a reference to
Jonathan Swift's *Gulliver* (the site suggests they would be a "Lilliputian Navy"). It is
**nocturnal**; in it Johnny does **not snore** and the seagull may **not appear**; when it
appears, it makes a **nest on his chest** and **lays an egg**. There is a **bug** at the end of the scene
(a strange rectangle in the sea — see §12).

---

## 8. Visitors and rescue attempts (`visitors`)
### 8.1 They pass behind him (he almost never sees — `notseen`)
- A **motorboat** with a **woman and a dog**.
- A **biplane** passes while he uses the telescope.
- A **helicopter** (on 1998-12-28 a fan suggested it was an **autogyro**, not a helicopter).
- A **plane** flying low over the island.

### 8.2 Visitors he does see
- **Party boat:** a boat arrives with **revelers** who take him aboard; he **swims
  back** to the island and the boat leaves — just as Johnny realizes what he did. A woman
  **water-skis** behind a motorboat and **knocks Johnny over**.
- **Naked Johnny (3 variations):** (1) a couple arrives and he begs to be taken,
  taking off **all his clothes** to convince the woman; (2) he dances in **tribal
  garb**, tourists photograph him, and he **rips off his clothes** and waves them in the air; (3) during the
  **rain dance**, tourists mistake him for a native and, to prove he is not, he
  **takes off his clothes** — which annoys the man.
- **Johnny "Terminator":** this time he **actually spots** the plane; he throws a **coconut**
  to get the pilot's attention, but **hits the plane**, which **crashes into the sea**. The pilot
  bails out by **parachute** before impact.
- **Johnny vandal (unconfirmed):** he throws a coconut at a ship trying to sink it.
- **Giant ship (cargo):** Johnny spots a ship far away and **jumps to get
  attention**; the ship turns out to be **enormous** and almost **cuts the island in half** — Johnny
  runs to save himself. *(scene `VISITOR.ADS#3`, marked `HOLIDAY_NOK`: it never shows
  holiday items, otherwise they would be drawn over the hull that fills the screen.)*
- **Mermaid:** see §4.

---

## 9. Anniversary dates / holidays (`annivers` + `story.c` logic)
Special items are drawn on the island within date ranges (string comparison
`"MMDD"` in `story.c`). **They can be forced by adjusting the system clock.**

| Holiday | Date range (engine) | What appears / happens | `holiday=` |
|---|---|---|---:|
| **New Year** | **12/29 → 01/01** | **"Happy New-Year"** banner on the palm. | 4 |
| **St. Patrick's Day** | **03/15 → 03/17** | Island covered in **four-leaf clovers** (the intent was clovers/shamrocks). | 2 |
| **Halloween** | **10/29 → 10/31** | A large **pumpkin (jack-o'-lantern)** in front of the island. | 1 |
| **Christmas** | **12/23 → 12/25** | **Christmas tree** on the island. Variation: when fishing the **big octopus**, it **steals Christmas baubles** from the tree before diving. | 3 |

> **Independence Day (July 4):** Wikipedia cites July 4 among the holidays,
> but it is **not** in the site's `annivers` nor implemented in `jc_reborn`. **Open
> item** — investigate in the original data; there may be unscheduled art/scene.
> Wilson Reborn should keep the holiday table **extensible** (castaway's roadmap
> suggests "extend festive days").

---

## 10. Rare events and easter eggs (`unusual`)
- **Fight (ghost Johnny):** a **transparent** Johnny comes out of the water while the normal
  Johnny chases a coconut; they **fight** and Johnny #1 knocks the ghost back into the
  sea. Then a plane comes in, he throws the coconut, downs the plane and the pilot bails out.
- **Silver balls:** **two silver balls/bowls**, one on each side of the palm,
  before the scene ends abruptly.
- **Real-time clock:** a **clock in the thought bubble** shows the **real time
  of the computer** (reports from 1997 and 2008).
- **Rain dance:** see §3.12 (cloud → one drop → lightning).
- **"Feeding the Fishes":** a **shark jumps onto the island**, **swallows Johnny**, swims
  around, makes a face and **spits him back out**.
- **Melting Johnny:** he uses a **yellow fan**, his knees soften and he
  **melts into a blob** (also happens in moonlight).
- **Office reverie:** after a clock appears, Johnny shows up in an
  **office dreaming** of the island and the mermaid (day 10 — see §2).
- **"Home Again?" / THE END:** a small **silhouetted screen** shows a plane over the island,
  a man **parachuting** down, jumping for joy, and the text **"THE END"**.
- **Wandering pattern:** Johnny wanders ~5–6 min and then "does the special thing"
  before the scene changes.

---

## 11. Sounds
The original has **24 sound effects** (`sound0.wav`…`sound24.wav`, with gaps at 11 and
13). `sound0` is played on plot scene transitions (`story.c`: `soundPlay(0)`
when the scene has a `dayNo`). Exact MD5/sizes in
[01-history](01-historia-e-creditos.md) / `repos/jc_reborn/README.md`.

---

## 12. Original bugs (cataloged in `bugs`)
It is important to decide, in Wilson Reborn, **which are "charm" to preserve** and **which to
fix**. The site's list:

**Installation:** "Can't find data files" on Win2000/NT (looks in `windows`, installs in
`winnt`).

**Visual glitches:** freezing on the title screen (Win95); **vanished Johnny** (low
memory — only the rod and the sound); **black smudge** semicircle when reeling in fishing from the
left; **rectangle** in the sea after the pirates; **rod disappears** when turning toward the palm
with the boot; **over-built raft**; **transparent palm**; **cloud with
lines**; **flying Johnny** (a duplicate suspended after the dive); **ghost
dancers** in the clouds after the lightning; **Johnny in a box** (appears in the square that
should scramble after the pirate ship); **giant island** / **multiple islands** /
**dozens of Johnnys** (after a long run); **black boxes** (sometimes freeze); **dark
cloud**; **red sea**; **simultaneous day+night scene**; **twins** (duplicated Johnny,
e.g. in the "terminator" scene); **"tidy your room"** (the table/gramophone stay in the
background after the dinner with the mermaid); **screen color split**; **freeze climbing
the tree**; **no Johnny**; **teleporting Johnny**; **hidden coconut**.

**Audio:** **sound only** without video (may persist on the desktop); **"muttering mode"**
(the sound card freezes muttering after a crash).

> Many of these bugs are artifacts of Windows 3.1/16-bit and **disappear
> naturally** in a modern engine. Some are **accidental gags beloved** by the
> community (e.g. "giant island", "dozens of Johnnys") — they could become an
> **optional mode/easter egg** in Wilson Reborn.

---

## 13. Scene→Behavior map (the 10 `.ADS` files)

The `story_data.h` of `jc_reborn` defines **63 scenes** distributed across **10 `.ADS`
files** (each file groups numbered "tags"; each tag is a scene). General mapping,
with the descriptive names confirmed in `castaway/.../scenes.mjs` when
available:

| `.ADS` file | Content (category) | Notable scenes/tags |
|---|---|---|
| **ACTIVITY.ADS** | Various activities/gags | #1 *GAG DIVES*, #4 *MUNDANE DIVE*, #6 *GAG JOHN READ*, #7 *MUNDANE JOHN READ*, #8 *JOHN BATH*, #10 *GULL 1 READING*, #11 *GULL 2 BATHING*, #12 *GULL 3 STILL READING*, #5 *NATIVE 1*, #9 *NATIVE 3* |
| **BUILDING.ADS** | Building (raft / sandcastle) | tags 1–7 |
| **FISHING.ADS** | Fishing (catches, left/right sides) | tags 1–8 (#4,#7,#8 marked `LEFT_ISLAND`) |
| **JOHNNY.ADS** | Johnny's plot beats | #1 → day 11 (returns by plane), #2 → day 2 (SOS), #3 → day 6 (drawing of Suzy), #6 → day 10 (office), #4/#5 free |
| **MARY.ADS** | The mermaid Mary | #2 → day 1, #3 → day 4, #1 → day 5, #4 → day 7, #5 → day 8 (departs on the raft) |
| **MISCGAG.ADS** | Miscellaneous gags | tags 1–2 |
| **STAND.ADS** | Poses/idle at each island spot | tags 1–16 (transitions/idle) |
| **SUZY.ADS** | The city girl Suzy | #1 → day 3, #2 → day 9 |
| **VISITOR.ADS** | Visitors/rescues | #1, #3 (giant cargo, `HOLIDAY_NOK`), #4, #5 (`LEFT_ISLAND`), #6, #7 |
| **WALKSTUF.ADS** | "Stuff" related to walking | tags 1–3 |

> The "spots" (A–F) and "headings" (S, SW, W, NW, N, NE, E, SE) define **where** on the island a
> scene starts/ends and **where** Johnny looks, allowing the engine to **walk**
> transitionally between scenes. Details of the model in
> [05-engine-architecture](05-arquitetura-do-engine.md).

---

## 14. Parity checklist ("lose nothing" summary)

- [ ] **11-day** arc (Mary + Suzy), advancing by real date and restarting.
- [ ] **Fishing** (all common/rare catches + ambidexterity + big octopus + shark-ski).
- [ ] **Swimming/diving** with the **animal jury** and the crab with inverted scores.
- [ ] **Bath** + swim-trunks-thief seagull + shark scare ("intact" leg).
- [ ] **Reading** (book upside down, nap→coconut, reading seagull).
- [ ] **Sleeping/snoring** + being tied up by the pirates.
- [ ] **Fire/cooking** (2–4 attempts, octopus on the face).
- [ ] **Coconuts** (bounces, spinning head, cracking on the tree).
- [ ] **Raft** (5 stages) and **SOS in a bottle** (mini-Johnny on day 2).
- [ ] **Sandcastle** → **King Kong pirates**.
- [ ] **Jogging** and **telescope** (something passing behind).
- [ ] **Rain dance** (drop → lightning).
- [ ] **Mary, the mermaid** (all 6 interactions).
- [ ] **Suzy** + **escape/resort/kiss/ear-tug** scenes.
- [ ] **Seagull** (5 gags).
- [ ] **Pirates** (King Kong + Gulliver, with nest/egg on the chest).
- [ ] **Visitors** (motorboat+woman+dog, biplane, helicopter/autogyro, low plane,
      party boat, water-skier, tourists, terminator, giant ship, naked x3).
- [ ] **4 holidays** (New Year, St. Patrick, Halloween, Christmas) + extensible table.
- [ ] **Rare easter eggs** (ghost Johnny, silver balls, real clock, melting,
      "feeding the fishes", "THE END/Home Again").
- [ ] **24 sounds**.
- [ ] **Low tide**, **day/night cycle**, **random island position**, **clouds**.

---

### Sources
johnny-castaway.com (common, fishing, swimming, reading, mermaid, pirates, seagull,
visitors, leaving, annivers, story, unusual, bugs); `repos/jc_reborn/story_data.h` and
`story.c`; `repos/castaway/src/scrantic/metadata/scenes.mjs` and `types.mjs`.
