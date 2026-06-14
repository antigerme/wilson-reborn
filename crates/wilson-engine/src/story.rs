// SPDX-License-Identifier: GPL-3.0-or-later
//! The story director: the 63-scene table, scene selection, the 11-day cycle and the
//! per-scene island state (tide, night, raft, holiday, position).
//!
//! Faithful port of `story.c` + `story_data.h` (`repos/jc_reborn`). This is pure
//! decision logic — it produces a [`StoryRun`] (which `.ADS` scenes to play, in order,
//! with the island state) that a higher layer feeds to the ADS scheduler. Date, time
//! and randomness are injected, so it is fully deterministic and testable.

use crate::rng::Rng;

// --- Scene flags (story_data.h) ---------------------------------------------

/// The climactic scene of a run.
pub const FINAL: u8 = 0x01;
/// A scene with no walk-in lead-up.
pub const FIRST: u8 = 0x02;
/// A scene that takes place on the island (vs. a cutaway).
pub const ISLAND: u8 = 0x04;
/// The island is drawn shifted fully to the left.
pub const LEFT_ISLAND: u8 = 0x08;
/// The island position may be randomised.
pub const VARPOS_OK: u8 = 0x10;
/// Low tide is allowed for this scene.
pub const LOWTIDE_OK: u8 = 0x20;
/// The raft must not be shown.
pub const NORAFT: u8 = 0x40;
/// Holiday props must not be drawn (e.g. the cargo ship fills the screen).
pub const HOLIDAY_NOK: u8 = 0x80;

// --- Spots (A–F) and 8-way headings ----------------------------------------

/// Island spot A.
pub const SPOT_A: u8 = 0;
/// Island spot B.
pub const SPOT_B: u8 = 1;
/// Island spot C.
pub const SPOT_C: u8 = 2;
/// Island spot D.
pub const SPOT_D: u8 = 3;
/// Island spot E.
pub const SPOT_E: u8 = 4;
/// Island spot F.
pub const SPOT_F: u8 = 5;

/// Heading: south.
pub const HDG_S: u8 = 0;
/// Heading: south-west.
pub const HDG_SW: u8 = 1;
/// Heading: west.
pub const HDG_W: u8 = 2;
/// Heading: north-west.
pub const HDG_NW: u8 = 3;
/// Heading: north.
pub const HDG_N: u8 = 4;
/// Heading: north-east.
pub const HDG_NE: u8 = 5;
/// Heading: east.
pub const HDG_E: u8 = 6;
/// Heading: south-east.
pub const HDG_SE: u8 = 7;

/// One entry of the story scene table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoryScene {
    /// The `.ADS` resource that plays this scene.
    pub ads_name: &'static str,
    /// The sequence tag within the `.ADS`.
    pub ads_tag: u16,
    /// Spot Johnny starts at (for walk-in).
    pub spot_start: u8,
    /// Heading Johnny faces at the start.
    pub hdg_start: u8,
    /// Spot Johnny ends at.
    pub spot_end: u8,
    /// Heading Johnny faces at the end.
    pub hdg_end: u8,
    /// Story day this scene is tied to (0 = any day).
    pub day: u8,
    /// Behaviour flags (see the `FINAL`/`ISLAND`/… constants).
    pub flags: u8,
}

#[allow(clippy::too_many_arguments)]
const fn sc(
    ads_name: &'static str,
    ads_tag: u16,
    spot_start: u8,
    hdg_start: u8,
    spot_end: u8,
    hdg_end: u8,
    day: u8,
    flags: u8,
) -> StoryScene {
    StoryScene {
        ads_name,
        ads_tag,
        spot_start,
        hdg_start,
        spot_end,
        hdg_end,
        day,
        flags,
    }
}

/// The full 63-scene table (verbatim from `story_data.h`).
pub static STORY_SCENES: &[StoryScene] = &[
    // ACTIVITY.ADS
    sc(
        "ACTIVITY.ADS",
        1,
        SPOT_E,
        HDG_SE,
        SPOT_A,
        HDG_S,
        0,
        ISLAND | FINAL | VARPOS_OK,
    ),
    sc(
        "ACTIVITY.ADS",
        12,
        SPOT_D,
        HDG_SW,
        SPOT_A,
        HDG_S,
        0,
        ISLAND | FINAL | VARPOS_OK | LOWTIDE_OK,
    ),
    sc(
        "ACTIVITY.ADS",
        11,
        SPOT_A,
        HDG_S,
        SPOT_A,
        HDG_S,
        0,
        ISLAND | FINAL | FIRST | VARPOS_OK,
    ),
    sc(
        "ACTIVITY.ADS",
        10,
        SPOT_D,
        HDG_SW,
        SPOT_A,
        HDG_S,
        0,
        ISLAND | FINAL | VARPOS_OK | LOWTIDE_OK,
    ),
    sc(
        "ACTIVITY.ADS",
        4,
        SPOT_E,
        HDG_SE,
        SPOT_E,
        HDG_SE,
        0,
        ISLAND | VARPOS_OK | LOWTIDE_OK,
    ),
    sc(
        "ACTIVITY.ADS",
        5,
        SPOT_E,
        HDG_SW,
        SPOT_A,
        HDG_S,
        0,
        ISLAND | FINAL | VARPOS_OK | LOWTIDE_OK,
    ),
    sc(
        "ACTIVITY.ADS",
        6,
        SPOT_D,
        HDG_SW,
        SPOT_A,
        HDG_S,
        0,
        ISLAND | FINAL | VARPOS_OK,
    ),
    sc(
        "ACTIVITY.ADS",
        7,
        SPOT_D,
        HDG_SW,
        SPOT_F,
        HDG_SW,
        0,
        ISLAND | VARPOS_OK | LOWTIDE_OK,
    ),
    sc(
        "ACTIVITY.ADS",
        8,
        SPOT_A,
        HDG_S,
        SPOT_D,
        HDG_SE,
        0,
        ISLAND | FIRST | VARPOS_OK,
    ),
    sc(
        "ACTIVITY.ADS",
        9,
        SPOT_E,
        HDG_E,
        SPOT_A,
        HDG_S,
        0,
        ISLAND | FINAL | LOWTIDE_OK,
    ),
    // BUILDING.ADS
    sc(
        "BUILDING.ADS",
        1,
        SPOT_F,
        HDG_W,
        SPOT_A,
        HDG_W,
        0,
        ISLAND | VARPOS_OK | LOWTIDE_OK,
    ),
    sc(
        "BUILDING.ADS",
        4,
        SPOT_A,
        HDG_E,
        SPOT_A,
        HDG_S,
        0,
        ISLAND | FINAL | VARPOS_OK,
    ),
    sc(
        "BUILDING.ADS",
        3,
        SPOT_A,
        HDG_E,
        SPOT_C,
        HDG_SE,
        0,
        ISLAND | VARPOS_OK | LOWTIDE_OK,
    ),
    sc(
        "BUILDING.ADS",
        2,
        SPOT_F,
        HDG_W,
        SPOT_A,
        HDG_S,
        0,
        ISLAND | FINAL | VARPOS_OK,
    ),
    sc(
        "BUILDING.ADS",
        5,
        SPOT_D,
        HDG_W,
        SPOT_D,
        HDG_E,
        0,
        ISLAND | VARPOS_OK | LOWTIDE_OK,
    ),
    sc(
        "BUILDING.ADS",
        7,
        SPOT_D,
        HDG_W,
        SPOT_D,
        HDG_E,
        0,
        ISLAND | VARPOS_OK | LOWTIDE_OK,
    ),
    sc(
        "BUILDING.ADS",
        6,
        SPOT_A,
        HDG_E,
        SPOT_A,
        HDG_S,
        0,
        ISLAND | FINAL | VARPOS_OK,
    ),
    // FISHING.ADS
    sc(
        "FISHING.ADS",
        1,
        SPOT_D,
        HDG_W,
        SPOT_D,
        HDG_E,
        0,
        ISLAND | VARPOS_OK | LOWTIDE_OK,
    ),
    sc(
        "FISHING.ADS",
        2,
        SPOT_D,
        HDG_W,
        SPOT_D,
        HDG_E,
        0,
        ISLAND | VARPOS_OK | LOWTIDE_OK,
    ),
    sc(
        "FISHING.ADS",
        3,
        SPOT_D,
        HDG_W,
        SPOT_A,
        HDG_S,
        0,
        ISLAND | FINAL | VARPOS_OK | LOWTIDE_OK,
    ),
    sc(
        "FISHING.ADS",
        4,
        SPOT_E,
        HDG_E,
        SPOT_A,
        HDG_S,
        0,
        ISLAND | FINAL | LEFT_ISLAND | LOWTIDE_OK,
    ),
    sc(
        "FISHING.ADS",
        5,
        SPOT_E,
        HDG_E,
        SPOT_A,
        HDG_S,
        0,
        ISLAND | FINAL | VARPOS_OK,
    ),
    sc(
        "FISHING.ADS",
        6,
        SPOT_D,
        HDG_W,
        SPOT_A,
        HDG_S,
        0,
        ISLAND | FINAL | LOWTIDE_OK,
    ),
    sc(
        "FISHING.ADS",
        7,
        SPOT_E,
        HDG_E,
        SPOT_E,
        HDG_W,
        0,
        ISLAND | LEFT_ISLAND | VARPOS_OK | LOWTIDE_OK,
    ),
    sc(
        "FISHING.ADS",
        8,
        SPOT_E,
        HDG_E,
        SPOT_E,
        HDG_W,
        0,
        ISLAND | LEFT_ISLAND | VARPOS_OK | LOWTIDE_OK,
    ),
    // JOHNNY.ADS
    sc(
        "JOHNNY.ADS",
        1,
        SPOT_A,
        HDG_S,
        SPOT_A,
        HDG_S,
        11,
        FINAL | FIRST,
    ),
    sc(
        "JOHNNY.ADS",
        2,
        SPOT_E,
        HDG_SW,
        SPOT_F,
        HDG_S,
        2,
        ISLAND | FINAL | VARPOS_OK,
    ),
    sc(
        "JOHNNY.ADS",
        3,
        SPOT_E,
        HDG_SW,
        SPOT_F,
        HDG_NE,
        6,
        ISLAND | VARPOS_OK | LOWTIDE_OK,
    ),
    sc(
        "JOHNNY.ADS",
        4,
        SPOT_E,
        HDG_SW,
        SPOT_F,
        HDG_NE,
        0,
        ISLAND | VARPOS_OK,
    ),
    sc(
        "JOHNNY.ADS",
        5,
        SPOT_E,
        HDG_SW,
        SPOT_F,
        HDG_NE,
        0,
        ISLAND | VARPOS_OK,
    ),
    sc(
        "JOHNNY.ADS",
        6,
        SPOT_A,
        HDG_S,
        SPOT_A,
        HDG_S,
        10,
        FINAL | FIRST,
    ),
    // MARY.ADS
    sc(
        "MARY.ADS",
        1,
        SPOT_E,
        HDG_SW,
        SPOT_A,
        HDG_S,
        5,
        ISLAND | FINAL | VARPOS_OK | LOWTIDE_OK,
    ),
    sc(
        "MARY.ADS",
        3,
        SPOT_F,
        HDG_SW,
        SPOT_A,
        HDG_S,
        4,
        ISLAND | FINAL | FIRST | VARPOS_OK,
    ),
    sc(
        "MARY.ADS",
        2,
        SPOT_E,
        HDG_E,
        SPOT_A,
        HDG_S,
        1,
        ISLAND | FINAL | VARPOS_OK,
    ),
    sc(
        "MARY.ADS",
        4,
        SPOT_E,
        HDG_E,
        SPOT_A,
        HDG_S,
        7,
        ISLAND | FINAL | VARPOS_OK,
    ),
    sc(
        "MARY.ADS",
        5,
        SPOT_E,
        HDG_NW,
        SPOT_A,
        HDG_S,
        8,
        ISLAND | LEFT_ISLAND | FINAL | FIRST | NORAFT | VARPOS_OK,
    ),
    // MISCGAG.ADS
    sc(
        "MISCGAG.ADS",
        1,
        SPOT_D,
        HDG_W,
        SPOT_A,
        HDG_S,
        0,
        ISLAND | FINAL | VARPOS_OK | LOWTIDE_OK,
    ),
    sc(
        "MISCGAG.ADS",
        2,
        SPOT_D,
        HDG_W,
        SPOT_A,
        HDG_S,
        0,
        ISLAND | FINAL | VARPOS_OK,
    ),
    // STAND.ADS
    sc(
        "STAND.ADS",
        1,
        SPOT_A,
        HDG_SW,
        SPOT_A,
        HDG_SW,
        0,
        ISLAND | VARPOS_OK | LOWTIDE_OK,
    ),
    sc(
        "STAND.ADS",
        2,
        SPOT_A,
        HDG_W,
        SPOT_A,
        HDG_W,
        0,
        ISLAND | VARPOS_OK | LOWTIDE_OK,
    ),
    sc(
        "STAND.ADS",
        3,
        SPOT_A,
        HDG_NW,
        SPOT_A,
        HDG_NW,
        0,
        ISLAND | VARPOS_OK | LOWTIDE_OK,
    ),
    sc(
        "STAND.ADS",
        4,
        SPOT_B,
        HDG_SW,
        SPOT_B,
        HDG_SW,
        0,
        ISLAND | VARPOS_OK | LOWTIDE_OK,
    ),
    sc(
        "STAND.ADS",
        5,
        SPOT_B,
        HDG_S,
        SPOT_B,
        HDG_S,
        0,
        ISLAND | VARPOS_OK | LOWTIDE_OK,
    ),
    sc(
        "STAND.ADS",
        6,
        SPOT_B,
        HDG_SE,
        SPOT_B,
        HDG_SE,
        0,
        ISLAND | VARPOS_OK | LOWTIDE_OK,
    ),
    sc(
        "STAND.ADS",
        7,
        SPOT_C,
        HDG_NE,
        SPOT_C,
        HDG_NE,
        0,
        ISLAND | VARPOS_OK | LOWTIDE_OK,
    ),
    sc(
        "STAND.ADS",
        8,
        SPOT_C,
        HDG_E,
        SPOT_C,
        HDG_E,
        0,
        ISLAND | VARPOS_OK | LOWTIDE_OK,
    ),
    sc(
        "STAND.ADS",
        9,
        SPOT_D,
        HDG_NW,
        SPOT_D,
        HDG_NW,
        0,
        ISLAND | VARPOS_OK | LOWTIDE_OK,
    ),
    sc(
        "STAND.ADS",
        10,
        SPOT_D,
        HDG_NE,
        SPOT_D,
        HDG_NE,
        0,
        ISLAND | VARPOS_OK | LOWTIDE_OK,
    ),
    sc(
        "STAND.ADS",
        11,
        SPOT_E,
        HDG_NW,
        SPOT_E,
        HDG_NW,
        0,
        ISLAND | VARPOS_OK | LOWTIDE_OK,
    ),
    sc(
        "STAND.ADS",
        12,
        SPOT_F,
        HDG_S,
        SPOT_F,
        HDG_S,
        0,
        ISLAND | VARPOS_OK | LOWTIDE_OK,
    ),
    sc(
        "STAND.ADS",
        15,
        SPOT_A,
        HDG_S,
        SPOT_A,
        HDG_S,
        0,
        ISLAND | VARPOS_OK | LOWTIDE_OK,
    ),
    sc(
        "STAND.ADS",
        16,
        SPOT_C,
        HDG_S,
        SPOT_C,
        HDG_S,
        0,
        ISLAND | VARPOS_OK | LOWTIDE_OK,
    ),
    // SUZY.ADS
    sc(
        "SUZY.ADS",
        1,
        SPOT_A,
        HDG_S,
        SPOT_A,
        HDG_S,
        3,
        FINAL | FIRST,
    ),
    sc(
        "SUZY.ADS",
        2,
        SPOT_A,
        HDG_S,
        SPOT_A,
        HDG_S,
        9,
        FINAL | FIRST,
    ),
    // VISITOR.ADS
    sc(
        "VISITOR.ADS",
        1,
        SPOT_A,
        HDG_S,
        SPOT_A,
        HDG_S,
        0,
        ISLAND | LOWTIDE_OK,
    ),
    sc(
        "VISITOR.ADS",
        3,
        SPOT_B,
        HDG_NW,
        SPOT_D,
        HDG_S,
        0,
        ISLAND | FINAL | HOLIDAY_NOK,
    ),
    sc(
        "VISITOR.ADS",
        4,
        SPOT_D,
        HDG_S,
        SPOT_D,
        HDG_W,
        0,
        ISLAND | VARPOS_OK | LOWTIDE_OK,
    ),
    sc(
        "VISITOR.ADS",
        6,
        SPOT_D,
        HDG_S,
        SPOT_D,
        HDG_SW,
        0,
        ISLAND | VARPOS_OK | LOWTIDE_OK,
    ),
    sc(
        "VISITOR.ADS",
        7,
        SPOT_D,
        HDG_S,
        SPOT_D,
        HDG_SW,
        0,
        ISLAND | VARPOS_OK | LOWTIDE_OK,
    ),
    sc(
        "VISITOR.ADS",
        5,
        SPOT_E,
        HDG_SW,
        SPOT_A,
        HDG_S,
        0,
        ISLAND | FINAL | LEFT_ISLAND | VARPOS_OK | LOWTIDE_OK,
    ),
    // WALKSTUF.ADS
    sc(
        "WALKSTUF.ADS",
        1,
        SPOT_A,
        HDG_NE,
        SPOT_A,
        HDG_S,
        0,
        ISLAND | FINAL | LOWTIDE_OK,
    ),
    sc(
        "WALKSTUF.ADS",
        2,
        SPOT_E,
        HDG_E,
        SPOT_D,
        HDG_SE,
        0,
        ISLAND | VARPOS_OK,
    ),
    sc(
        "WALKSTUF.ADS",
        3,
        SPOT_D,
        HDG_W,
        SPOT_E,
        HDG_W,
        0,
        ISLAND | VARPOS_OK | LOWTIDE_OK,
    ),
];

/// A commemorative day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Holiday {
    /// No holiday.
    None,
    /// Halloween (Oct 29–31).
    Halloween,
    /// St Patrick's Day (Mar 15–17).
    StPatrick,
    /// Christmas (Dec 23–25).
    Christmas,
    /// New Year (Dec 29–Jan 1).
    NewYear,
}

/// The environment state for a run's island.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IslandState {
    /// Whether the tide is low.
    pub low_tide: bool,
    /// Whether it is night.
    pub night: bool,
    /// Raft build stage (0–5).
    pub raft: u8,
    /// Active holiday, if any.
    pub holiday: Holiday,
    /// Island X offset.
    pub x_pos: i32,
    /// Island Y offset.
    pub y_pos: i32,
}

/// One scene to play within a [`StoryRun`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScenePlay {
    /// The `.ADS` resource name.
    pub ads_name: &'static str,
    /// The sequence tag.
    pub ads_tag: u16,
    /// `(spot, heading)` Johnny walks from (None for the first scene of the run).
    pub walk_from: Option<(u8, u8)>,
    /// `(spot, heading)` Johnny walks to (None for cutaway scenes off the island).
    pub walk_to: Option<(u8, u8)>,
    /// Whether this scene is a scripted day beat (plays the transition sound).
    pub day_beat: bool,
    /// Whether this scene uses the left-shifted island.
    pub left_island: bool,
}

/// A planned story run: the island state and the ordered scenes to play.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoryRun {
    /// Whether the climactic scene takes place on the island.
    pub on_island: bool,
    /// The island environment for the run.
    pub island: IslandState,
    /// The scenes to play, in order (the climactic scene is last).
    pub scenes: Vec<ScenePlay>,
}

/// Whether the given hour is "night" (the original uses an 8-hour cycle).
pub fn is_night(hour: u8) -> bool {
    let h = hour % 8;
    h == 0 || h == 7
}

/// The raft build stage for a story day (`story.c`).
pub fn raft_for_day(day: u8) -> u8 {
    match day {
        0..=2 => 1,
        3 => 2,
        4 => 3,
        5 => 4,
        _ => 5,
    }
}

/// The holiday active on `month`/`day` (MMDD ranges from `story.c`).
pub fn holiday_for_date(month: u8, day: u8) -> Holiday {
    let mmdd = i32::from(month) * 100 + i32::from(day);
    if 1028 < mmdd && mmdd < 1101 {
        Holiday::Halloween
    } else if 314 < mmdd && mmdd < 318 {
        Holiday::StPatrick
    } else if 1222 < mmdd && mmdd < 1226 {
        Holiday::Christmas
    } else if !(102..=1228).contains(&mmdd) {
        Holiday::NewYear
    } else {
        Holiday::None
    }
}

/// Pick a uniformly-random scene matching the flag/day constraints.
pub fn pick_scene(day: u8, wanted: u8, unwanted: u8, rng: &mut Rng) -> Option<&'static StoryScene> {
    let matches: Vec<&'static StoryScene> = STORY_SCENES
        .iter()
        .filter(|s| {
            (s.flags & wanted) == wanted
                && (s.flags & unwanted) == 0
                && (s.day == 0 || s.day == day)
        })
        .collect();
    if matches.is_empty() {
        None
    } else {
        Some(matches[rng.below(matches.len() as u32) as usize])
    }
}

/// Compute the island state for a chosen scene (`storyCalculateIslandFromScene`).
pub fn island_from_scene(
    scene: &StoryScene,
    day: u8,
    holiday: Holiday,
    night: bool,
    rng: &mut Rng,
) -> IslandState {
    let low_tide = scene.flags & LOWTIDE_OK != 0 && rng.below(2) == 1;

    let (x_pos, y_pos) = if scene.flags & VARPOS_OK != 0 {
        if rng.below(2) == 1 {
            (-222 + rng.below(109) as i32, -44 + rng.below(128) as i32)
        } else if rng.below(2) == 1 {
            (-114 + rng.below(134) as i32, -14 + rng.below(99) as i32)
        } else {
            (-114 + rng.below(119) as i32, -73 + rng.below(60) as i32)
        }
    } else if scene.flags & LEFT_ISLAND != 0 {
        (-272, 0)
    } else {
        (0, 0)
    };

    let raft = if scene.flags & NORAFT != 0 {
        0
    } else {
        raft_for_day(day)
    };

    let holiday = if scene.flags & HOLIDAY_NOK != 0 {
        Holiday::None
    } else {
        holiday
    };

    IslandState {
        low_tide,
        night,
        raft,
        holiday,
        x_pos,
        y_pos,
    }
}

/// The story director: tracks the current day and plans runs (`storyPlay`).
#[derive(Debug, Clone)]
pub struct Director {
    /// Current story day (1–11).
    pub current_day: u8,
    /// Persisted day-of-year used to detect calendar changes.
    pub stored_yday: i32,
}

impl Director {
    /// Create a director at `current_day` with a stored day-of-year.
    pub fn new(current_day: u8, stored_yday: i32) -> Self {
        Director {
            current_day,
            stored_yday,
        }
    }

    /// Advance the story day if the calendar day changed; clamp to 1–11.
    /// Returns whether anything changed (`storyUpdateCurrentDay`).
    pub fn advance_day(&mut self, today_yday: i32) -> bool {
        let mut changed = false;
        if today_yday != self.stored_yday {
            self.stored_yday = today_yday;
            self.current_day += 1;
            changed = true;
        }
        if self.current_day < 1 || self.current_day > 11 {
            self.current_day = 1;
            changed = true;
        }
        changed
    }

    /// Plan one run: pick the climactic scene, the ambient lead-up chain (with
    /// walking between spots), and the island state (`storyPlay`).
    pub fn plan_run(
        &mut self,
        today_yday: i32,
        hour: u8,
        month: u8,
        day: u8,
        rng: &mut Rng,
    ) -> StoryRun {
        self.advance_day(today_yday);
        let night = is_night(hour);
        let holiday = holiday_for_date(month, day);

        let final_scene = *pick_scene(self.current_day, FINAL, 0, rng)
            .expect("there is always at least one FINAL scene");
        let on_island = final_scene.flags & ISLAND != 0;

        let island = if on_island {
            island_from_scene(&final_scene, self.current_day, holiday, night, rng)
        } else {
            IslandState {
                low_tide: false,
                night,
                raft: 0,
                holiday: Holiday::None,
                x_pos: 0,
                y_pos: 0,
            }
        };

        let mut scenes = Vec::new();
        let mut prev: Option<(u8, u8)> = None;

        if final_scene.flags & FIRST == 0 {
            let count = 6 + rng.below(14);
            let mut wanted = 0u8;
            if island.low_tide {
                wanted |= LOWTIDE_OK;
            }
            if island.x_pos != 0 || island.y_pos != 0 {
                wanted |= VARPOS_OK;
            }
            let mut unwanted = FINAL;
            for _ in 0..count {
                let Some(scene) = pick_scene(self.current_day, wanted, unwanted, rng) else {
                    break;
                };
                scenes.push(ScenePlay {
                    ads_name: scene.ads_name,
                    ads_tag: scene.ads_tag,
                    walk_from: prev,
                    walk_to: Some((scene.spot_start, scene.hdg_start)),
                    day_beat: scene.day != 0,
                    left_island: scene.flags & LEFT_ISLAND != 0,
                });
                unwanted |= FIRST;
                prev = Some((scene.spot_end, scene.hdg_end));
            }
        }

        scenes.push(ScenePlay {
            ads_name: final_scene.ads_name,
            ads_tag: final_scene.ads_tag,
            walk_from: prev,
            walk_to: if on_island {
                Some((final_scene.spot_start, final_scene.hdg_start))
            } else {
                None
            },
            day_beat: final_scene.day != 0,
            left_island: final_scene.flags & LEFT_ISLAND != 0,
        });

        StoryRun {
            on_island,
            island,
            scenes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_has_63_scenes() {
        assert_eq!(STORY_SCENES.len(), 63);
    }

    #[test]
    fn day_beats_match_the_story() {
        // The single FINAL scene tied to each scripted day.
        let beat = |day: u8| -> Option<(&'static str, u16)> {
            STORY_SCENES
                .iter()
                .find(|s| s.day == day && s.flags & FINAL != 0)
                .map(|s| (s.ads_name, s.ads_tag))
        };
        assert_eq!(beat(1), Some(("MARY.ADS", 2)));
        assert_eq!(beat(2), Some(("JOHNNY.ADS", 2)));
        assert_eq!(beat(3), Some(("SUZY.ADS", 1)));
        assert_eq!(beat(4), Some(("MARY.ADS", 3)));
        assert_eq!(beat(5), Some(("MARY.ADS", 1)));
        // Day 6's beat (JOHNNY.ADS#3) is an ambient, non-FINAL scene.
        assert_eq!(beat(6), None);
        assert!(STORY_SCENES
            .iter()
            .any(|s| s.day == 6 && s.ads_name == "JOHNNY.ADS" && s.ads_tag == 3));
        assert_eq!(beat(7), Some(("MARY.ADS", 4)));
        assert_eq!(beat(8), Some(("MARY.ADS", 5)));
        assert_eq!(beat(9), Some(("SUZY.ADS", 2)));
        assert_eq!(beat(10), Some(("JOHNNY.ADS", 6)));
        assert_eq!(beat(11), Some(("JOHNNY.ADS", 1)));
    }

    #[test]
    fn holidays() {
        assert_eq!(holiday_for_date(10, 30), Holiday::Halloween);
        assert_eq!(holiday_for_date(10, 28), Holiday::None);
        assert_eq!(holiday_for_date(3, 16), Holiday::StPatrick);
        assert_eq!(holiday_for_date(12, 24), Holiday::Christmas);
        assert_eq!(holiday_for_date(12, 31), Holiday::NewYear);
        assert_eq!(holiday_for_date(1, 1), Holiday::NewYear);
        assert_eq!(holiday_for_date(6, 14), Holiday::None);
    }

    #[test]
    fn night_and_raft() {
        for h in [0u8, 7, 8, 15, 16, 23] {
            assert!(is_night(h), "expected night at {h}");
        }
        for h in [1u8, 3, 6, 12] {
            assert!(!is_night(h), "expected day at {h}");
        }
        assert_eq!(raft_for_day(1), 1);
        assert_eq!(raft_for_day(2), 1);
        assert_eq!(raft_for_day(3), 2);
        assert_eq!(raft_for_day(4), 3);
        assert_eq!(raft_for_day(5), 4);
        assert_eq!(raft_for_day(6), 5);
        assert_eq!(raft_for_day(11), 5);
    }

    #[test]
    fn advance_day_clamps_and_wraps() {
        let mut d = Director::new(5, 100);
        assert!(!d.advance_day(100)); // same day, no change
        assert_eq!(d.current_day, 5);
        assert!(d.advance_day(101)); // new day
        assert_eq!(d.current_day, 6);
        d.current_day = 11;
        assert!(d.advance_day(102)); // 11 -> 12 -> wraps to 1
        assert_eq!(d.current_day, 1);
    }

    #[test]
    fn plan_run_invariants() {
        let mut d = Director::new(5, 200);
        let mut rng = Rng::new(7);
        let run = d.plan_run(200, 12, 6, 14, &mut rng); // same yday, day 6/14 (no holiday), noon
        assert!(!run.scenes.is_empty());
        assert_eq!(run.island.holiday, Holiday::None);
        assert!(!run.island.night);
        // Last scene is FINAL; ambient scenes are never FINAL.
        let last = run.scenes.last().unwrap();
        assert!(STORY_SCENES.iter().any(|s| s.ads_name == last.ads_name
            && s.ads_tag == last.ads_tag
            && s.flags & FINAL != 0));
        if run.scenes.len() > 1 {
            assert!((7..=20).contains(&run.scenes.len())); // 1 final + 6..=19 ambient
            for amb in &run.scenes[..run.scenes.len() - 1] {
                let scene = STORY_SCENES
                    .iter()
                    .find(|s| s.ads_name == amb.ads_name && s.ads_tag == amb.ads_tag)
                    .unwrap();
                assert_eq!(scene.flags & FINAL, 0, "ambient scene must not be FINAL");
            }
        }
    }

    #[test]
    fn raft_reflects_day_in_plan() {
        let mut d = Director::new(5, 300);
        let mut rng = Rng::new(1);
        let run = d.plan_run(300, 12, 6, 14, &mut rng);
        if run.on_island {
            assert_eq!(run.island.raft, raft_for_day(5));
        }
    }
}
