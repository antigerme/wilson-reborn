// SPDX-License-Identifier: GPL-3.0-or-later
//! The integration runtime: ties the [`Director`], [`Island`], [`Walker`] and
//! [`AdsVm`] together into a stream of composited frames — "the life of Johnny".
//!
//! [`Show::next_frame`] drives the same high-level loop as `storyPlay`: plan a run
//! (island + ordered scenes), then for each scene walk Johnny into place and play the
//! ADS scene over the island background; when scenes run out, plan the next run. It is
//! headless (produces an indexed [`Surface`] per frame); a real backend turns those
//! into on-screen pixels via the palette. Missing resources skip a scene rather than
//! failing, so it degrades gracefully on partial data.

use wilson_dgds::{Archive, BmpImage, Palette};

use crate::ads_vm::AdsVm;
use crate::dissolve::Dissolve;
use crate::island::Island;
use crate::rng::Rng;
use crate::story::{Director, ScenePlay, StoryRun};
use crate::surface::{Surface, TRANSPARENT};
use crate::ttm_exec::detect_transparent;
use crate::walk::{WalkFrame, Walker};

/// How long the intro screen (`INTRO.SCR`) is held at startup, in engine ticks
/// (≈4 s at [`crate::MS_PER_TICK`] = 16 ms/tick), when the intro is enabled.
const INTRO_TICKS: u16 = 250;

/// The wall-clock inputs the director needs (injected so the runtime is testable).
#[derive(Debug, Clone, Copy)]
pub struct Clock {
    /// Day of the year (used to advance the story day).
    pub yday: i32,
    /// Hour of day 0–23 (used for the day/night cycle).
    pub hour: u8,
    /// Month 1–12 (used for holidays).
    pub month: u8,
    /// Day of month 1–31 (used for holidays).
    pub day: u8,
}

/// A composited frame to display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    /// The composited indexed-color image.
    pub surface: Surface,
    /// How long to display it, in engine ticks (1 tick = [`crate::MS_PER_TICK`] = 16 ms).
    pub delay_ticks: u16,
    /// Sound effect ids triggered this frame.
    pub sounds: Vec<u16>,
}

#[derive(Debug)]
enum Stage {
    Idle,
    Walk(Walker),
    Play(Box<AdsVm>),
}

/// A snapshot of the runtime's current state, for the `--debug` log/overlay.
#[derive(Debug, Clone)]
pub struct DebugInfo {
    /// Current story day (1–11).
    pub day: u8,
    /// Current scene `(ADS name, tag)`, if one is selected.
    pub scene: Option<(&'static str, u16)>,
    /// What the runtime is doing: `walk`, `play`, or `idle`.
    pub stage: &'static str,
    /// Island drift offset `(dx, dy)`, if on the island.
    pub offset: Option<(i32, i32)>,
    /// Whether the current run is on the island.
    pub on_island: bool,
    /// Whether it is night.
    pub night: bool,
    /// Whether the tide is low.
    pub low_tide: bool,
    /// Raft build stage (0–5).
    pub raft: u8,
    /// Active holiday, if any.
    pub holiday: crate::story::Holiday,
}

/// The end-to-end Johnny Castaway runtime, as a frame generator.
#[derive(Debug)]
pub struct Show {
    palette: Palette,
    width: u16,
    height: u16,
    director: Director,
    clock: Clock,
    rng: Rng,
    johnwalk: Vec<BmpImage>,
    run: StoryRun,
    scene_idx: usize,
    island: Option<Island>,
    stage: Stage,
    /// A sound to emit on the next produced frame (the day-beat cue, `sound 0`).
    pending_sound: Option<u16>,
    /// The original's intro screen (`INTRO.SCR`), shown once at startup if enabled
    /// (the original's `Introduction` option). Consumed by the first `next_frame`.
    intro: Option<Surface>,
    /// Opt-in: dissolve between story runs (the original's dormant LFSR tiled dissolve,
    /// KB10 §10.2) instead of a hard cut. Off by default = faithful hard cut.
    dissolve_on: bool,
    /// The last surface emitted, kept so a run-boundary dissolve has a "from" image.
    prev: Option<Surface>,
    /// An in-progress dissolve transition, if any (drives `next_frame` while active).
    dissolve: Option<Dissolve>,
    /// The destination frame deferred until the dissolve finishes (carries its sounds/delay).
    pending: Option<Frame>,
    /// Set when a new story run was just planned, so `next_frame` can start a transition.
    run_boundary: bool,
}

impl Show {
    /// Create a runtime. `johnwalk` (`JOHNWALK.BMP`) is loaded up front if present.
    pub fn new(
        archive: &Archive,
        palette: &Palette,
        width: u16,
        height: u16,
        director: Director,
        clock: Clock,
        seed: u64,
    ) -> Self {
        let transparent_src = detect_transparent(palette);
        let johnwalk = archive
            .bmp("JOHNWALK.BMP")
            .map(|bmp| remap(bmp, transparent_src))
            .unwrap_or_default();

        let mut show = Show {
            palette: palette.clone(),
            width,
            height,
            director,
            clock,
            rng: Rng::new(seed),
            johnwalk,
            run: StoryRun {
                on_island: false,
                island: crate::story::IslandState {
                    low_tide: false,
                    night: false,
                    raft: 0,
                    holiday: crate::story::Holiday::None,
                    x_pos: 0,
                    y_pos: 0,
                },
                scenes: Vec::new(),
            },
            scene_idx: 0,
            island: None,
            stage: Stage::Idle,
            pending_sound: None,
            intro: None,
            dissolve_on: false,
            prev: None,
            dissolve: None,
            pending: None,
            run_boundary: false,
        };
        show.plan_new_run(archive);
        show.run_boundary = false; // the very first run is not a transition
        show
    }

    /// Enable the opt-in dissolve transition between story runs (the original's dormant
    /// LFSR tiled dissolve — KB10 §10.2). Off by default = faithful hard cut.
    pub fn enable_dissolve(&mut self) {
        self.dissolve_on = true;
    }

    /// Update the wall-clock inputs (call before a new run picks up a new day).
    pub fn set_clock(&mut self, clock: Clock) {
        self.clock = clock;
    }

    /// Queue the original's intro screen (`INTRO.SCR`) to be shown once at startup —
    /// the original's `Introduction` option. No-op if the resource is missing, so it
    /// degrades gracefully. Call right after [`Show::new`].
    pub fn enable_intro(&mut self, archive: &Archive) {
        let Some(scr) = archive.scr("INTRO.SCR") else {
            return;
        };
        let mut surface = Surface::new(self.width, self.height, 0);
        let w = i32::from(scr.width).min(i32::from(self.width));
        let h = i32::from(scr.height).min(i32::from(self.height));
        for sy in 0..h {
            for sx in 0..w {
                let p = scr.pixels[(sy * i32::from(scr.width) + sx) as usize];
                surface.put_pixel(sx, sy, p);
            }
        }
        self.intro = Some(surface);
    }

    /// The director's persisted day state — `(current_day, stored_yday)` — so a host
    /// can save it and resume the 11-day arc across sessions.
    pub fn day_state(&self) -> (u8, i32) {
        (self.director.current_day, self.director.stored_yday)
    }

    /// The current run's island drift offset, if on the island (for tests).
    #[cfg(test)]
    fn island_offset(&self) -> Option<(i32, i32)> {
        self.island.as_ref().map(Island::offset)
    }

    /// A snapshot of the current runtime state, for the `--debug` overlay/log.
    pub fn debug_info(&self) -> DebugInfo {
        DebugInfo {
            day: self.director.current_day,
            scene: self
                .run
                .scenes
                .get(self.scene_idx)
                .map(|s| (s.ads_name, s.ads_tag)),
            stage: if self.intro.is_some() {
                "intro"
            } else {
                match self.stage {
                    Stage::Idle => "idle",
                    Stage::Walk(_) => "walk",
                    Stage::Play(_) => "play",
                }
            },
            offset: self.island.as_ref().map(Island::offset),
            on_island: self.run.on_island,
            night: self.run.island.night,
            low_tide: self.run.island.low_tide,
            raft: self.run.island.raft,
            holiday: self.run.island.holiday,
        }
    }

    /// Produce the next composited frame (the runtime never ends).
    ///
    /// With the opt-in dissolve enabled, a transition is woven in at story-run boundaries:
    /// the new run's first frame is revealed cell-by-cell over the previous one (the
    /// original's dormant LFSR tiled dissolve). Off by default ⇒ a hard cut, byte-identical
    /// to before.
    pub fn next_frame(&mut self, archive: &Archive) -> Frame {
        // Drive an in-progress dissolve to completion before advancing the scene.
        if self.dissolve.is_some() {
            return self.step_dissolve();
        }
        let frame = self.next_frame_inner(archive);
        // At a run boundary, start a dissolve from the previous frame to this one.
        if self.dissolve_on && self.run_boundary {
            if let Some(prev) = self.prev.take() {
                if prev.width == frame.surface.width
                    && prev.height == frame.surface.height
                    && prev.pixels != frame.surface.pixels
                {
                    self.run_boundary = false;
                    self.dissolve = Some(Dissolve::new(prev, frame.surface.clone()));
                    self.pending = Some(frame);
                    return self.step_dissolve();
                }
            }
        }
        self.run_boundary = false;
        // Only keep a "from" image when the transition is enabled (no per-frame clone
        // otherwise, so the default hard-cut path is exactly as cheap as before).
        if self.dissolve_on {
            self.prev = Some(frame.surface.clone());
        }
        frame
    }

    /// Advance an active dissolve by one step; on completion emit the (deferred) destination
    /// frame, carrying its sounds and delay.
    fn step_dissolve(&mut self) -> Frame {
        let done = {
            let d = self
                .dissolve
                .as_mut()
                .expect("called with an active dissolve");
            d.step();
            d.done()
        };
        if done {
            let surface = self.dissolve.take().unwrap().into_destination();
            self.prev = Some(surface.clone());
            return self.pending.take().unwrap_or(Frame {
                surface,
                delay_ticks: 1,
                sounds: Vec::new(),
            });
        }
        Frame {
            surface: self.dissolve.as_ref().unwrap().image().clone(),
            delay_ticks: 1,
            sounds: Vec::new(),
        }
    }

    fn next_frame_inner(&mut self, archive: &Archive) -> Frame {
        // The intro screen (`INTRO.SCR`) is shown once, before the first run, if enabled.
        if let Some(surface) = self.intro.take() {
            return Frame {
                surface,
                delay_ticks: INTRO_TICKS,
                sounds: Vec::new(),
            };
        }
        for _ in 0..20_000 {
            enum Action {
                Walk(WalkFrame),
                Play(crate::ads_vm::AdsFrame),
                WalkDone,
                Done,
            }
            let action = match &mut self.stage {
                Stage::Idle => Action::Done,
                Stage::Walk(w) => match w.next_frame() {
                    Some(wf) => Action::Walk(wf),
                    None => Action::WalkDone,
                },
                Stage::Play(vm) => match vm.next_frame(archive) {
                    Ok(Some(af)) => Action::Play(af),
                    _ => Action::Done,
                },
            };

            match action {
                Action::Walk(wf) => return self.frame_from_walk(wf),
                Action::Play(af) => {
                    let mut sounds = af.sounds;
                    if let Some(s) = self.pending_sound.take() {
                        sounds.insert(0, s);
                    }
                    return Frame {
                        surface: self.overlay_holiday(af.surface),
                        delay_ticks: af.delay_ticks,
                        sounds,
                    };
                }
                Action::WalkDone => {
                    let scene = self.run.scenes[self.scene_idx];
                    if !self.try_play(archive, &scene) {
                        self.go_next_scene(archive);
                    }
                }
                Action::Done => self.go_next_scene(archive),
            }
        }

        // Degenerate data (nothing renderable): yield a blank frame rather than hang.
        Frame {
            surface: Surface::new(self.width, self.height, 0),
            delay_ticks: 8,
            sounds: Vec::new(),
        }
    }

    fn frame_from_walk(&self, wf: WalkFrame) -> Frame {
        let (dx, dy) = self.island.as_ref().map_or((0, 0), Island::offset);
        let mut surface = match &self.island {
            Some(isl) => isl.background().clone(),
            None => Surface::new(self.width, self.height, 0),
        };
        if let Some(img) = self.johnwalk.get(wf.sprite as usize) {
            surface.blit(
                img.width,
                img.height,
                &img.pixels,
                wf.x + dx,
                wf.y + dy,
                Some(TRANSPARENT),
                wf.flip,
                None,
            );
        }
        if wf.behind_tree {
            if let Some(isl) = &self.island {
                isl.redraw_tree(&mut surface);
            }
        }
        Frame {
            surface: self.overlay_holiday(surface),
            delay_ticks: wf.delay,
            sounds: Vec::new(),
        }
    }

    /// Composite the holiday prop layer on top of `surface` (like `jc_reborn`'s
    /// `grUpdateDisplay`, the holiday is drawn last — above Johnny). No-op off-island.
    fn overlay_holiday(&self, surface: Surface) -> Surface {
        match self.island.as_ref().and_then(Island::holiday_layer) {
            Some(layer) => surface.compose_over(layer),
            None => surface,
        }
    }

    fn plan_new_run(&mut self, archive: &Archive) {
        self.run = self.director.plan_run(
            self.clock.yday,
            self.clock.hour,
            self.clock.month,
            self.clock.day,
            &mut self.rng,
        );
        self.scene_idx = 0;
        self.island = if self.run.on_island {
            Island::build(
                archive,
                &self.run.island,
                &self.palette,
                self.width,
                self.height,
                &mut self.rng,
            )
            .ok()
        } else {
            None
        };
        self.stage = self.stage_for_current(archive);
        self.run_boundary = true; // a fresh run ⇒ an opt-in dissolve may start
    }

    fn go_next_scene(&mut self, archive: &Archive) {
        self.scene_idx += 1;
        if self.scene_idx >= self.run.scenes.len() {
            self.plan_new_run(archive);
        } else {
            self.stage = self.stage_for_current(archive);
        }
    }

    fn stage_for_current(&mut self, archive: &Archive) -> Stage {
        let scene = self.run.scenes[self.scene_idx];
        if let (Some((fs, fh)), Some((ts, th))) = (scene.walk_from, scene.walk_to) {
            return Stage::Walk(Walker::new(fs, fh, ts, th, &mut self.rng));
        }
        match self.build_vm(archive, &scene) {
            Some(vm) => Stage::Play(Box::new(vm)),
            None => Stage::Idle,
        }
    }

    fn try_play(&mut self, archive: &Archive, scene: &ScenePlay) -> bool {
        match self.build_vm(archive, scene) {
            Some(vm) => {
                self.stage = Stage::Play(Box::new(vm));
                true
            }
            None => false,
        }
    }

    fn build_vm(&mut self, archive: &Archive, scene: &ScenePlay) -> Option<AdsVm> {
        let ads = archive.ads(scene.ads_name)?;
        let seed = u64::from(self.rng.next_u32());
        let mut vm = AdsVm::new(
            ads,
            scene.ads_tag,
            archive,
            &self.palette,
            self.width,
            self.height,
            seed,
        )
        .ok()?;
        if let Some(isl) = &self.island {
            vm.set_background(isl.background().clone());
            // Offset Johnny's animation to the island's drifted position, exactly like
            // walk frames. Without this, in-place gags (juggling, fishing, sitting…) draw
            // at the un-drifted origin — i.e. off the island, on the water.
            let (ox, oy) = ads_offset(isl.offset(), scene.left_island);
            vm.set_offset(ox, oy);
        }
        // Day-beat scenes play the transition cue (`sound 0`) as they begin, like
        // `jc_reborn` (`storyPlay` → `soundPlay(0)` for `dayNo` scenes).
        if scene.day_beat {
            self.pending_sound = Some(0);
        }
        Some(vm)
    }
}

/// The ADS sprite offset for an island scene: the island drift, plus the LEFT_ISLAND
/// shift. Mirrors jc_reborn `storyPlay`: `ttmDx = xPos + (LEFT_ISLAND ? 272 : 0)`,
/// `ttmDy = yPos`.
fn ads_offset((dx, dy): (i32, i32), left_island: bool) -> (i32, i32) {
    (dx + if left_island { 272 } else { 0 }, dy)
}

fn remap(bmp: &wilson_dgds::Bmp, transparent_src: Option<u8>) -> Vec<BmpImage> {
    bmp.images
        .iter()
        .map(|im| BmpImage {
            width: im.width,
            height: im.height,
            pixels: im
                .pixels
                .iter()
                .map(|&p| {
                    if Some(p) == transparent_src {
                        TRANSPARENT
                    } else {
                        p
                    }
                })
                .collect(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use wilson_dgds::{Ads, AdsRes, Bmp, Scr, Tag, Ttm};

    fn op(c: u16) -> [u8; 2] {
        c.to_le_bytes()
    }

    fn solid_bmp(count: usize, value: u8) -> Bmp {
        Bmp {
            width: 2,
            height: 2,
            images: (0..count)
                .map(|_| BmpImage {
                    width: 2,
                    height: 2,
                    pixels: vec![value; 4],
                })
                .collect(),
        }
    }

    /// A TTM that draws one pixel and yields a frame, then ends.
    fn one_frame_ttm() -> Ttm {
        let mut code = Vec::new();
        code.extend_from_slice(&op(0x1111)); // TAG 1
        code.extend_from_slice(&1u16.to_le_bytes());
        code.extend_from_slice(&op(0x1021)); // SET_DELAY 6
        code.extend_from_slice(&6u16.to_le_bytes());
        code.extend_from_slice(&op(0xA002)); // DRAW_PIXEL 5 5
        code.extend_from_slice(&5u16.to_le_bytes());
        code.extend_from_slice(&5u16.to_le_bytes());
        code.extend_from_slice(&op(0x0FF0)); // UPDATE
        Ttm {
            version: "1.20".into(),
            num_pages: 1,
            bytecode: code,
            tags: vec![Tag {
                id: 1,
                description: "s".into(),
            }],
        }
    }

    /// A minimal ADS that adds the one-frame TTM (slot 1) once.
    fn minimal_ads() -> Ads {
        let mut code = Vec::new();
        code.extend_from_slice(&op(0x0001)); // tag 1
        code.extend_from_slice(&op(0x2005)); // ADD_SCENE slot1 tag1 0 0
        for v in [1u16, 1, 0, 0] {
            code.extend_from_slice(&v.to_le_bytes());
        }
        code.extend_from_slice(&op(0x1510)); // PLAY_SCENE
        code.extend_from_slice(&op(0xFFFF)); // END
        Ads {
            version: "1.20".into(),
            resources: vec![AdsRes {
                id: 1,
                name: "J.TTM".into(),
            }],
            bytecode: code,
            tags: vec![Tag {
                id: 1,
                description: "s".into(),
            }],
        }
    }

    fn full_archive() -> Archive {
        let ads_names = [
            "ACTIVITY.ADS",
            "BUILDING.ADS",
            "FISHING.ADS",
            "JOHNNY.ADS",
            "MARY.ADS",
            "MISCGAG.ADS",
            "STAND.ADS",
            "SUZY.ADS",
            "VISITOR.ADS",
            "WALKSTUF.ADS",
        ];
        Archive {
            bitmaps: vec![
                ("BACKGRND.BMP".to_string(), solid_bmp(42, 1)),
                ("MRAFT.BMP".to_string(), solid_bmp(5, 9)),
                ("HOLIDAY.BMP".to_string(), solid_bmp(4, 5)),
                ("JOHNWALK.BMP".to_string(), solid_bmp(64, 8)),
            ],
            screens: (0..3)
                .map(|i| {
                    (
                        format!("OCEAN0{i}.SCR"),
                        Scr {
                            width: 640,
                            height: 480,
                            pixels: vec![3; 640 * 480],
                        },
                    )
                })
                .chain(std::iter::once((
                    "NIGHT.SCR".to_string(),
                    Scr {
                        width: 640,
                        height: 480,
                        pixels: vec![2; 640 * 480],
                    },
                )))
                .collect(),
            ttms: vec![("J.TTM".to_string(), one_frame_ttm())],
            ads: ads_names
                .iter()
                .map(|n| (n.to_string(), minimal_ads()))
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn produces_frames_across_runs() {
        let arch = full_archive();
        let pal = Palette {
            colors: [[1u8; 3]; 256],
        };
        let director = Director::new(5, 200);
        let clock = Clock {
            yday: 200,
            hour: 12,
            month: 6,
            day: 14,
        };
        let mut show = Show::new(&arch, &pal, 640, 480, director, clock, 42);

        // Drive many frames spanning walks, scenes and run transitions.
        for _ in 0..400 {
            let f = show.next_frame(&arch);
            assert_eq!(f.surface.width, 640);
            assert_eq!(f.surface.height, 480);
            assert!(f.delay_ticks > 0);
        }
    }

    #[test]
    fn dissolve_transition_is_woven_in_at_run_boundaries() {
        let arch = full_archive();
        let pal = Palette {
            colors: [[1u8; 3]; 256],
        };
        let clock = Clock {
            yday: 200,
            hour: 12,
            month: 6,
            day: 14,
        };
        // Drive the same deterministic sequence with the dissolve off vs on, counting the
        // short (1-tick) frames. The opt-in dissolve injects a burst of short intermediate
        // frames at each run boundary, so enabling it strictly increases their count — and
        // every frame stays a valid 640×480 (it never panics or stalls).
        let count_short = |dissolve: bool| {
            let mut show = Show::new(&arch, &pal, 640, 480, Director::new(5, 200), clock, 42);
            if dissolve {
                show.enable_dissolve();
            }
            let mut short = 0u32;
            for _ in 0..1500 {
                let f = show.next_frame(&arch);
                assert_eq!((f.surface.width, f.surface.height), (640, 480));
                assert!(f.delay_ticks > 0);
                if f.delay_ticks == 1 {
                    short += 1;
                }
            }
            short
        };
        let off = count_short(false);
        let on = count_short(true);
        assert!(
            on > off,
            "dissolve should add short transition frames (on={on}, off={off})"
        );
    }

    #[test]
    fn intro_is_shown_once_when_enabled() {
        let mut arch = full_archive();
        arch.screens.push((
            "INTRO.SCR".to_string(),
            Scr {
                width: 640,
                height: 480,
                pixels: vec![7; 640 * 480],
            },
        ));
        let pal = Palette {
            colors: [[1u8; 3]; 256],
        };
        let clock = Clock {
            yday: 200,
            hour: 12,
            month: 6,
            day: 14,
        };
        let mut show = Show::new(&arch, &pal, 640, 480, Director::new(5, 200), clock, 42);
        show.enable_intro(&arch);
        assert!(show.intro.is_some());
        // The first frame is the intro screen (held for INTRO_TICKS); the upper-left is the
        // INTRO.SCR fill value (7), and it is consumed so it never shows again.
        let f0 = show.next_frame(&arch);
        assert_eq!(f0.delay_ticks, INTRO_TICKS);
        assert_eq!(f0.surface.get(10, 10), Some(7));
        assert!(show.intro.is_none(), "intro is shown only once");
    }

    #[test]
    fn intro_missing_is_graceful() {
        let arch = full_archive(); // no INTRO.SCR
        let pal = Palette {
            colors: [[1u8; 3]; 256],
        };
        let clock = Clock {
            yday: 200,
            hour: 12,
            month: 6,
            day: 14,
        };
        let mut show = Show::new(&arch, &pal, 640, 480, Director::new(5, 200), clock, 42);
        show.enable_intro(&arch); // INTRO.SCR absent -> no-op
        assert!(show.intro.is_none());
    }

    #[test]
    fn holiday_prop_is_composited_on_top() {
        // Like `jc_reborn` (`grUpdateDisplay`), the holiday layer is drawn on top of
        // Johnny. HOLIDAY.BMP uses value 5 in the fixture; a Christmas date must add
        // those pixels vs the same scenes on a non-holiday date (same seed → same runs).
        let arch = full_archive();
        let pal = Palette {
            colors: [[1u8; 3]; 256],
        };
        let count_holiday_pixels = |month: u8, day: u8| {
            let director = Director::new(5, 100);
            let clock = Clock {
                yday: 100,
                hour: 12,
                month,
                day,
            };
            let mut show = Show::new(&arch, &pal, 640, 480, director, clock, 7);
            let mut total = 0usize;
            for _ in 0..800 {
                let f = show.next_frame(&arch);
                total += f.surface.pixels.iter().filter(|&&p| p == 5).count();
            }
            total
        };
        let xmas = count_holiday_pixels(12, 24); // Christmas → prop on top
        let plain = count_holiday_pixels(6, 14); // no holiday
        assert!(
            xmas > plain,
            "holiday prop should appear on top (xmas={xmas}, plain={plain})"
        );
    }

    #[test]
    fn day_beat_emits_transition_sound() {
        // Day-beat scenes play `sound 0` as they begin (jc_reborn `storyPlay`). The
        // fixture's TTM emits no sounds, so seeing `0` means the day-beat cue fired.
        let arch = full_archive();
        let pal = Palette {
            colors: [[1u8; 3]; 256],
        };
        // Day 5's beat is MARY.ADS#1 (a FINAL day scene); over many runs it is chosen.
        // Seed-robust (sweep seeds) so it doesn't hinge on one RNG sequence.
        let clock = Clock {
            yday: 100,
            hour: 12,
            month: 6,
            day: 14,
        };
        let heard = (0..64u64).any(|seed| {
            let director = Director::new(5, 100);
            let mut show = Show::new(&arch, &pal, 640, 480, director, clock, seed);
            (0..4000).any(|_| show.next_frame(&arch).sounds.contains(&0))
        });
        assert!(heard, "expected the day-beat transition sound (0)");
    }

    #[test]
    fn day_advances_and_is_observable_for_persistence() {
        // The host reads `day_state()` to persist the 11-day arc. At construction the
        // day matches the director; when the calendar day changes, the next run picks
        // it up and `day_state()` reflects the advance.
        let arch = full_archive();
        let pal = Palette {
            colors: [[1u8; 3]; 256],
        };
        let director = Director::new(5, 200);
        let clock = Clock {
            yday: 200,
            hour: 12,
            month: 6,
            day: 14,
        };
        let mut show = Show::new(&arch, &pal, 640, 480, director, clock, 42);
        assert_eq!(show.day_state(), (5, 200));

        // Simulate the next calendar day; drive frames until a new run picks it up.
        show.set_clock(Clock {
            yday: 201,
            hour: 12,
            month: 6,
            day: 15,
        });
        for _ in 0..2000 {
            show.next_frame(&arch);
            if show.day_state() == (6, 201) {
                break;
            }
        }
        assert_eq!(show.day_state(), (6, 201));
    }

    #[test]
    fn missing_ads_degrades_to_blank_without_hang() {
        // Only island/walk resources, no ADS scenes: every scene is skipped, so the
        // runtime must still return (blank) frames rather than hang.
        let arch = Archive {
            screens: (0..3)
                .map(|i| {
                    (
                        format!("OCEAN0{i}.SCR"),
                        Scr {
                            width: 64,
                            height: 64,
                            pixels: vec![3; 64 * 64],
                        },
                    )
                })
                .collect(),
            bitmaps: vec![
                ("BACKGRND.BMP".to_string(), solid_bmp(42, 1)),
                ("MRAFT.BMP".to_string(), solid_bmp(5, 9)),
            ],
            ..Default::default()
        };
        let pal = Palette {
            colors: [[0u8; 3]; 256],
        };
        let director = Director::new(1, 0);
        let clock = Clock {
            yday: 0,
            hour: 0,
            month: 1,
            day: 1,
        };
        let mut show = Show::new(&arch, &pal, 64, 64, director, clock, 7);
        for _ in 0..10 {
            let f = show.next_frame(&arch);
            assert_eq!(f.surface.width, 64);
        }
    }

    /// A TTM that draws a single distinctive pixel at `(x, y)`, then yields a frame.
    fn marker_ttm(x: u16, y: u16, color: u8) -> Ttm {
        let mut code = Vec::new();
        code.extend_from_slice(&op(0x1111)); // TAG 1
        code.extend_from_slice(&1u16.to_le_bytes());
        code.extend_from_slice(&op(0x1021)); // SET_DELAY 6
        code.extend_from_slice(&6u16.to_le_bytes());
        code.extend_from_slice(&op(0x2002)); // SET_COLOR fg=color bg=color
        code.extend_from_slice(&u16::from(color).to_le_bytes());
        code.extend_from_slice(&u16::from(color).to_le_bytes());
        code.extend_from_slice(&op(0xA002)); // DRAW_PIXEL x y
        code.extend_from_slice(&x.to_le_bytes());
        code.extend_from_slice(&y.to_le_bytes());
        code.extend_from_slice(&op(0x0FF0)); // UPDATE
        Ttm {
            version: "1.20".into(),
            num_pages: 1,
            bytecode: code,
            tags: vec![Tag {
                id: 1,
                description: "s".into(),
            }],
        }
    }

    #[test]
    fn ads_animation_follows_island_drift() {
        // Regression for the "Johnny off the island" bug: ADS/TTM scenes must be drawn at
        // the island's drifted offset (like walk frames and jc_reborn `storyPlay`'s
        // ttmDx/ttmDy), not at the un-drifted origin. We mark Johnny's TTM with a unique
        // pixel at (MX,MY) and require it to land at (MX+dx, MY+dy) on a drifted run.
        const MX: u16 = 320;
        const MY: u16 = 240;
        const MARK: u8 = 0x2A; // distinct from every fixture background/sprite value
        let mut arch = full_archive();
        arch.ttms = vec![("J.TTM".to_string(), marker_ttm(MX, MY, MARK))];
        let pal = Palette {
            colors: [[1u8; 3]; 256],
        };

        // Deterministically search seeds for a drifted island play frame that drew the
        // mark; assert it is at the offset position (and never at the un-offset origin).
        let mut found_drifted = false;
        'seeds: for seed in 0..128u64 {
            let director = Director::new(5, 100);
            let clock = Clock {
                yday: 100,
                hour: 12,
                month: 6, // no holiday layer to overdraw the mark
                day: 14,
            };
            let mut show = Show::new(&arch, &pal, 640, 480, director, clock, seed);
            for _ in 0..400 {
                let f = show.next_frame(&arch);
                let Some((dx, dy)) = show.island_offset() else {
                    continue;
                };
                let at_offset = f.surface.get(MX as i32 + dx, MY as i32 + dy) == Some(MARK);
                if dx != 0 || dy != 0 {
                    // On a drifted run the mark must never sit at the un-offset origin.
                    if f.surface.get(MX as i32, MY as i32) == Some(MARK) {
                        panic!("ADS sprite drawn at un-offset origin (island offset ignored)");
                    }
                    if at_offset {
                        found_drifted = true;
                        break 'seeds;
                    }
                }
            }
        }
        assert!(
            found_drifted,
            "expected a drifted island scene whose ADS sprite is drawn at the offset"
        );
    }

    #[test]
    fn ads_offset_applies_left_island_shift() {
        // Non-LEFT_ISLAND: just the island drift.
        assert_eq!(ads_offset((-100, -30), false), (-100, -30));
        assert_eq!(ads_offset((0, 0), false), (0, 0));
        // LEFT_ISLAND scenes shift the sprite +272 in x (jc_reborn storyPlay), y unchanged.
        assert_eq!(ads_offset((-272, 0), true), (0, 0));
        assert_eq!(ads_offset((-100, -30), true), (172, -30));
    }

    #[test]
    fn real_data_timing_is_paced_like_the_original() {
        // Gated: only runs when WILSON_DATA_DIR points at the original data.
        //
        // The original waits `delay_ticks × 16 ms` between display updates (its animation
        // clock fires every 16 ms — verified by disassembly, see `crate::MS_PER_TICK` and
        // KB10). Our engine mirrors this: `AdsFrame::delay_ticks == mini` (the shortest
        // pending thread delay), and the host waits `ticks × MS_PER_TICK`
        // (`config::frame_delay_ms`). This asserts the *engine* emits a human-visible pace —
        // so once the host honours `delay_ticks` (the winit loop must not redraw faster than
        // the timer) the playback speed matches the original.
        let Some(dir) = std::env::var_os("WILSON_DATA_DIR") else {
            return;
        };
        let dir = std::path::PathBuf::from(dir);
        let map = std::fs::read(wilson_dgds::find_ci(&dir, "RESOURCE.MAP").expect("RESOURCE.MAP"))
            .expect("read map");
        let rm = wilson_dgds::ResourceMap::parse(&map).expect("parse map");
        let data =
            std::fs::read(wilson_dgds::find_ci(&dir, &rm.data_file_name).expect("data file"))
                .expect("read data");
        let arch = Archive::parse(&map, &data).expect("parse archive");
        let pal = arch.palette().cloned().expect("palette");

        let director = Director::new(1, 0);
        let clock = Clock {
            yday: 0,
            hour: 12,
            month: 6,
            day: 14,
        };
        let mut show = Show::new(&arch, &pal, 640, 480, director, clock, 7);

        const N: u32 = 3000;
        let mut total_ticks = 0u64;
        let mut max_ticks = 0u16;
        for _ in 0..N {
            let f = show.next_frame(&arch);
            total_ticks += u64::from(f.delay_ticks);
            max_ticks = max_ticks.max(f.delay_ticks);
        }
        let total_ms = total_ticks * crate::MS_PER_TICK;
        let avg_ms = total_ms as f64 / f64::from(N);
        eprintln!(
            "real-data pace: {N} frames over {:.1}s, avg {avg_ms:.0} ms/frame, max {} ms",
            total_ms as f64 / 1000.0,
            u32::from(max_ticks) * crate::MS_PER_TICK as u32
        );
        // A real, contemplative pace — not the uncapped spin of the old winit loop.
        assert!(avg_ms >= 50.0, "engine pace too fast: {avg_ms:.0} ms/frame");
    }

    #[test]
    fn tick_is_16ms_the_original_rate() {
        // Regression guard for the disassembly finding (KB10 §9.1): the original's animation
        // clock fires every 16 ms — its scheduler derives a 4 ms master unit (1000/(13×18))
        // and the animation callback runs every 4th one (4×4 = 16 ms), gated on real time.
        // This is the original's rate, NOT jc_reborn's 20 ms approximation; don't revert it.
        assert_eq!(crate::MS_PER_TICK, 16);
    }

    /// FNV-1a hash of a surface's pixels — a cheap frame fingerprint for liveness
    /// checks (are consecutive frames actually changing, or is the run frozen?).
    fn frame_hash(s: &Surface) -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        for &p in &s.pixels {
            h ^= u64::from(p);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        h
    }

    #[test]
    fn engine_run_stays_live_and_paced() {
        // CI-friendly long run on fixtures (no original data needed). The screensaver
        // must, over thousands of frames: never panic, always emit a 640x480 frame, keep
        // animating (no frozen stretch), and stay at a human pace. This is the automatic
        // safety net for the bug classes we hit (runaway speed, freezes, bad frames); the
        // gated test below asserts the same — plus opacity and day-cycling — on real data.
        let arch = full_archive();
        let pal = Palette {
            colors: [[1u8; 3]; 256],
        };
        let mut show = Show::new(
            &arch,
            &pal,
            640,
            480,
            Director::new(1, 0),
            Clock {
                yday: 0,
                hour: 12,
                month: 6,
                day: 14,
            },
            9,
        );

        const N: u32 = 4000;
        let mut total_ticks = 0u64;
        let mut all = std::collections::HashSet::new();
        let mut window = std::collections::HashSet::new();
        for i in 0..N {
            let f = show.next_frame(&arch);
            assert_eq!(
                (f.surface.width, f.surface.height),
                (640, 480),
                "frame {i} has wrong dimensions"
            );
            total_ticks += u64::from(f.delay_ticks);
            let h = frame_hash(&f.surface);
            all.insert(h);
            window.insert(h);
            if i % 500 == 499 {
                assert!(window.len() >= 2, "run appears frozen near frame {i}");
                window.clear();
            }
        }
        let avg_ms = total_ticks as f64 * crate::MS_PER_TICK as f64 / f64::from(N);
        assert!(
            avg_ms >= 40.0,
            "fixture pace too fast: {avg_ms:.0} ms/frame"
        );
        assert!(
            all.len() >= 20,
            "run not animating (only {} distinct frames)",
            all.len()
        );
    }

    #[test]
    fn real_data_long_run_invariants() {
        // Gated (WILSON_DATA_DIR): the "render a long run and check it" validation, but
        // as machine-checkable invariants instead of an unwatchable video. Over ~1000 s
        // of playback, advancing the calendar so the 11-day arc cycles, assert every
        // frame is well-formed: right size, fully opaque (no leftover TRANSPARENT — the
        // "magenta water" class), the run keeps animating, the pace is human, several
        // distinct story days are reached, and nothing ever panics.
        let Some(dir) = std::env::var_os("WILSON_DATA_DIR") else {
            return;
        };
        let dir = std::path::PathBuf::from(dir);
        let map = std::fs::read(wilson_dgds::find_ci(&dir, "RESOURCE.MAP").expect("RESOURCE.MAP"))
            .expect("read map");
        let rm = wilson_dgds::ResourceMap::parse(&map).expect("parse map");
        let data =
            std::fs::read(wilson_dgds::find_ci(&dir, &rm.data_file_name).expect("data file"))
                .expect("read data");
        let arch = Archive::parse(&map, &data).expect("parse archive");
        let pal = arch.palette().cloned().expect("palette");

        let mut show = Show::new(
            &arch,
            &pal,
            640,
            480,
            Director::new(1, 0),
            Clock {
                yday: 0,
                hour: 12,
                month: 6,
                day: 14,
            },
            1,
        );

        const N: u32 = 8000;
        let mut total_ticks = 0u64;
        let mut all = std::collections::HashSet::new();
        let mut days = std::collections::HashSet::new();
        for i in 0..N {
            // Advance the calendar periodically so the director rolls the story day as
            // runs complete (each run spans many frames), exercising day progression.
            if i % 600 == 0 {
                show.set_clock(Clock {
                    yday: (i / 600) as i32,
                    hour: 12,
                    month: 6,
                    day: 14,
                });
            }
            let f = show.next_frame(&arch);
            assert_eq!((f.surface.width, f.surface.height), (640, 480));
            assert!(
                !f.surface.pixels.contains(&TRANSPARENT),
                "frame {i} has unresolved transparent pixels (background not opaque)"
            );
            total_ticks += u64::from(f.delay_ticks);
            all.insert(frame_hash(&f.surface));
            days.insert(show.day_state().0);
        }
        let avg_ms = total_ticks as f64 * crate::MS_PER_TICK as f64 / f64::from(N);
        eprintln!(
            "long-run: {N} frames, {:.0}s playback, avg {avg_ms:.0} ms/frame, \
             {} distinct frames, days seen {:?}",
            total_ticks as f64 * crate::MS_PER_TICK as f64 / 1000.0,
            all.len(),
            {
                let mut d: Vec<u8> = days.iter().copied().collect();
                d.sort_unstable();
                d
            }
        );
        assert!(avg_ms >= 50.0, "pace too fast: {avg_ms:.0} ms/frame");
        assert!(all.len() >= 200, "run not animating enough");
        assert!(days.len() >= 3, "story day did not advance (saw {days:?})");
    }
}
