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
use crate::island::Island;
use crate::rng::Rng;
use crate::story::{Director, ScenePlay, StoryRun};
use crate::surface::{Surface, TRANSPARENT};
use crate::ttm_exec::detect_transparent;
use crate::walk::{WalkFrame, Walker};

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
    /// How long to display it, in engine ticks (1 tick = 20 ms).
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
        };
        show.plan_new_run(archive);
        show
    }

    /// Update the wall-clock inputs (call before a new run picks up a new day).
    pub fn set_clock(&mut self, clock: Clock) {
        self.clock = clock;
    }

    /// The director's persisted day state — `(current_day, stored_yday)` — so a host
    /// can save it and resume the 11-day arc across sessions.
    pub fn day_state(&self) -> (u8, i32) {
        (self.director.current_day, self.director.stored_yday)
    }

    /// Produce the next composited frame (the runtime never ends).
    pub fn next_frame(&mut self, archive: &Archive) -> Frame {
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
        }
        // Day-beat scenes play the transition cue (`sound 0`) as they begin, like
        // `jc_reborn` (`storyPlay` → `soundPlay(0)` for `dayNo` scenes).
        if scene.day_beat {
            self.pending_sound = Some(0);
        }
        Some(vm)
    }
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
        let director = Director::new(5, 100);
        let clock = Clock {
            yday: 100,
            hour: 12,
            month: 6,
            day: 14,
        };
        let mut show = Show::new(&arch, &pal, 640, 480, director, clock, 42);
        let mut heard = false;
        for _ in 0..6000 {
            if show.next_frame(&arch).sounds.contains(&0) {
                heard = true;
                break;
            }
        }
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
}
