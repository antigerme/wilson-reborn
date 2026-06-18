// SPDX-License-Identifier: GPL-3.0-or-later
//! A headless interpreter for a single TTM animation thread.
//!
//! A thin wrapper over [`crate::ttm_exec`] that runs one TTM scene against its own
//! background and layer — convenient for previewing a single `.TTM` (the engine's
//! `ttm <name>` mode). The multi-thread scene runtime is [`crate::ads_vm::AdsVm`].

use wilson_dgds::{Archive, Palette, Ttm};

use crate::error::Result;
use crate::surface::{Surface, TRANSPARENT};
use crate::ttm_exec::{detect_transparent, run_frame, FrameOutcome, TtmSlot, TtmThread};

/// The result of running [`TtmVm::step`] for one frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TtmStep {
    /// An `UPDATE` produced a frame; wait `delay_ticks` and play `sounds`.
    Frame {
        /// Frame duration in engine ticks (1 tick = [`crate::MS_PER_TICK`] = 16 ms).
        delay_ticks: u16,
        /// Sound effect ids triggered during this frame.
        sounds: Vec<u16>,
    },
    /// The script ended; no more frames.
    Finished,
}

/// A single-thread TTM interpreter.
#[derive(Debug, Clone)]
pub struct TtmVm {
    slot: TtmSlot,
    thread: TtmThread,
    background: Surface,
    saved_zones: Surface,
    transparent_src: Option<u8>,
    dx: i32,
    dy: i32,
}

impl TtmVm {
    /// Create a VM positioned at `start_tag` (falls back to the start if not found).
    pub fn new(
        ttm: &Ttm,
        start_tag: u16,
        palette: &Palette,
        width: u16,
        height: u16,
    ) -> Result<Self> {
        let slot = TtmSlot::load(ttm)?;
        let mut thread = TtmThread::new(width, height);
        thread.running = 1;
        thread.ip = slot.find_tag(start_tag);
        Ok(TtmVm {
            slot,
            thread,
            background: Surface::new(width, height, 0),
            saved_zones: Surface::new(width, height, TRANSPARENT),
            transparent_src: detect_transparent(palette),
            dx: 0,
            dy: 0,
        })
    }

    /// Set the island offset applied to all drawing coordinates.
    pub fn set_offset(&mut self, dx: i32, dy: i32) {
        self.dx = dx;
        self.dy = dy;
    }

    /// The TTM thread's drawing layer (with transparent holes).
    pub fn layer(&self) -> &Surface {
        &self.thread.layer
    }

    /// The current background (set by `LOAD_SCREEN`).
    pub fn background(&self) -> &Surface {
        &self.background
    }

    /// The composited current frame (background, then the saved-zones layer, then the
    /// thread's layer on top — the same order as the multi-thread runtime).
    pub fn frame(&self) -> Surface {
        self.background
            .compose_over(&self.saved_zones)
            .compose_over(&self.thread.layer)
    }

    /// Run opcodes until the next `UPDATE` (a frame) or the end of the script.
    pub fn step(&mut self, archive: &Archive) -> Result<TtmStep> {
        if self.thread.running == 2 {
            return Ok(TtmStep::Finished);
        }
        if let Some(target) = self.thread.next_goto.take() {
            self.thread.ip = target;
        }
        match run_frame(
            &mut self.thread,
            &mut self.slot,
            &mut self.background,
            &mut self.saved_zones,
            self.transparent_src,
            self.dx,
            self.dy,
            archive,
        )? {
            FrameOutcome::Frame { sounds } => Ok(TtmStep::Frame {
                delay_ticks: self.thread.delay,
                sounds,
            }),
            FrameOutcome::Finished => Ok(TtmStep::Finished),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface::TRANSPARENT;
    use wilson_dgds::{Bmp, BmpImage, Scr};

    fn palette_with_magenta() -> Palette {
        let mut colors = [[0u8; 3]; 256];
        colors[1] = [10, 20, 30];
        colors[2] = [40, 50, 60];
        colors[3] = [70, 80, 90];
        colors[5] = [168, 0, 168]; // transparent key at index 5
        Palette { colors }
    }

    fn archive_with_sprite_and_bg() -> Archive {
        let bmp = Bmp {
            width: 2,
            height: 2,
            images: vec![BmpImage {
                width: 2,
                height: 2,
                pixels: vec![1, 5, 2, 5],
            }],
        };
        let scr = Scr {
            width: 2,
            height: 2,
            pixels: vec![3, 3, 3, 3],
        };
        Archive {
            bitmaps: vec![("S.BMP".to_string(), bmp)],
            screens: vec![("BG.SCR".to_string(), scr)],
            ..Default::default()
        }
    }

    fn ttm(bytecode: Vec<u8>) -> Ttm {
        Ttm {
            version: "1.20".to_string(),
            num_pages: 1,
            bytecode,
            tags: Vec::new(),
        }
    }

    fn op(code: u16) -> [u8; 2] {
        code.to_le_bytes()
    }

    #[test]
    fn runs_load_draw_update() {
        let mut code = Vec::new();
        code.extend_from_slice(&op(0xF01F));
        code.extend_from_slice(b"BG.SCR\0\0");
        code.extend_from_slice(&op(0x1051));
        code.extend_from_slice(&0u16.to_le_bytes());
        code.extend_from_slice(&op(0xF02F));
        code.extend_from_slice(b"S.BMP\0");
        code.extend_from_slice(&op(0xA504));
        for v in [0u16, 0, 0, 0] {
            code.extend_from_slice(&v.to_le_bytes());
        }
        code.extend_from_slice(&op(0xC051));
        code.extend_from_slice(&3u16.to_le_bytes());
        code.extend_from_slice(&op(0x0FF0));

        let pal = palette_with_magenta();
        let archive = archive_with_sprite_and_bg();
        let mut vm = TtmVm::new(&ttm(code), 0, &pal, 2, 2).unwrap();

        let step = vm.step(&archive).unwrap();
        assert_eq!(
            step,
            TtmStep::Frame {
                delay_ticks: 4,
                sounds: vec![3]
            }
        );
        assert_eq!(vm.layer().pixels, vec![1, TRANSPARENT, 2, TRANSPARENT]);
        assert_eq!(vm.frame().pixels, vec![1, 3, 2, 3]);
        assert_eq!(vm.step(&archive).unwrap(), TtmStep::Finished);
    }

    #[test]
    fn set_colors_and_rect_and_delay() {
        let mut code = Vec::new();
        code.extend_from_slice(&op(0x1021));
        code.extend_from_slice(&9u16.to_le_bytes());
        code.extend_from_slice(&op(0x2002));
        code.extend_from_slice(&7u16.to_le_bytes());
        code.extend_from_slice(&0u16.to_le_bytes());
        code.extend_from_slice(&op(0xA104));
        for v in [0u16, 0, 2, 1] {
            code.extend_from_slice(&v.to_le_bytes());
        }
        code.extend_from_slice(&op(0x0FF0));

        let pal = palette_with_magenta();
        let archive = Archive::default();
        let mut vm = TtmVm::new(&ttm(code), 0, &pal, 2, 2).unwrap();

        let step = vm.step(&archive).unwrap();
        assert_eq!(
            step,
            TtmStep::Frame {
                delay_ticks: 9,
                sounds: vec![]
            }
        );
        assert_eq!(vm.layer().pixels, vec![7, 7, TRANSPARENT, TRANSPARENT]);
    }

    #[test]
    fn set_delay_and_timer_reset_the_timer() {
        // Regression: jc_reborn sets `timer = delay` on SET_DELAY (ttm.c:204) and TIMER
        // (ttm.c:253), so a mid-scene delay change lands on the current frame. Before the
        // fix we only set `delay`, leaving `timer` stale (this assertion would fail).
        let pal = palette_with_magenta();
        let archive = Archive::default();

        // SET_DELAY 10
        let mut code = Vec::new();
        code.extend_from_slice(&op(0x1021));
        code.extend_from_slice(&10u16.to_le_bytes());
        code.extend_from_slice(&op(0x0FF0));
        let mut vm = TtmVm::new(&ttm(code), 0, &pal, 2, 2).unwrap();
        vm.thread.timer = 4; // stale value that the opcode must overwrite
        vm.step(&archive).unwrap();
        assert_eq!(vm.thread.delay, 10);
        assert_eq!(vm.thread.timer, 10, "SET_DELAY must reset timer = delay");

        // TIMER 8 12 -> delay = (8+12)/2 = 10
        let mut code = Vec::new();
        code.extend_from_slice(&op(0x2022));
        code.extend_from_slice(&8u16.to_le_bytes());
        code.extend_from_slice(&12u16.to_le_bytes());
        code.extend_from_slice(&op(0x0FF0));
        let mut vm = TtmVm::new(&ttm(code), 0, &pal, 2, 2).unwrap();
        vm.thread.timer = 4;
        vm.step(&archive).unwrap();
        assert_eq!(vm.thread.delay, 10);
        assert_eq!(vm.thread.timer, 10, "TIMER must reset timer = delay");
    }

    #[test]
    fn missing_resource_errors() {
        let mut code = Vec::new();
        code.extend_from_slice(&op(0xF01F));
        code.extend_from_slice(b"NOPE.SCR\0\0");
        let pal = palette_with_magenta();
        let archive = Archive::default();
        let mut vm = TtmVm::new(&ttm(code), 0, &pal, 2, 2).unwrap();
        assert_eq!(
            vm.step(&archive),
            Err(crate::error::EngineError::ResourceNotFound(
                "NOPE.SCR".to_string()
            ))
        );
    }

    #[test]
    fn copy_zone_to_bg_persists_after_clear() {
        // Draw a pixel, copy its zone to the saved-zones layer, then clear the layer.
        // The pixel must survive in the composited frame (it lives in saved zones).
        let mut code = Vec::new();
        code.extend_from_slice(&op(0x2002)); // SET_COLORS fg=2 bg=0
        for v in [2u16, 0] {
            code.extend_from_slice(&v.to_le_bytes());
        }
        code.extend_from_slice(&op(0xA002)); // DRAW_PIXEL 5 5
        for v in [5u16, 5] {
            code.extend_from_slice(&v.to_le_bytes());
        }
        code.extend_from_slice(&op(0x4204)); // COPY_ZONE_TO_BG 4 4 4 4
        for v in [4u16, 4, 4, 4] {
            code.extend_from_slice(&v.to_le_bytes());
        }
        code.extend_from_slice(&op(0xA601)); // CLEAR_SCREEN (clears the layer)
        code.extend_from_slice(&0u16.to_le_bytes());
        code.extend_from_slice(&op(0x0FF0)); // UPDATE

        let pal = palette_with_magenta();
        let archive = Archive::default();
        let mut vm = TtmVm::new(&ttm(code), 0, &pal, 16, 16).unwrap();
        vm.step(&archive).unwrap();
        // The layer was cleared, but the saved-zones layer keeps the pixel on top of bg.
        assert_eq!(vm.layer().get(5, 5), Some(TRANSPARENT));
        assert_eq!(vm.frame().get(5, 5), Some(2));
    }

    #[test]
    fn restore_zone_releases_saved_zones() {
        // COPY_ZONE_TO_BG then RESTORE_ZONE: the saved zone is released, so after
        // clearing the layer the frame shows only the background again.
        let mut code = Vec::new();
        code.extend_from_slice(&op(0x2002));
        for v in [2u16, 0] {
            code.extend_from_slice(&v.to_le_bytes());
        }
        code.extend_from_slice(&op(0xA002));
        for v in [5u16, 5] {
            code.extend_from_slice(&v.to_le_bytes());
        }
        code.extend_from_slice(&op(0x4204));
        for v in [4u16, 4, 4, 4] {
            code.extend_from_slice(&v.to_le_bytes());
        }
        code.extend_from_slice(&op(0xA064)); // RESTORE_ZONE (releases saved zones)
        for v in [0u16, 0, 0, 0] {
            code.extend_from_slice(&v.to_le_bytes());
        }
        code.extend_from_slice(&op(0xA601));
        code.extend_from_slice(&0u16.to_le_bytes());
        code.extend_from_slice(&op(0x0FF0));

        let pal = palette_with_magenta();
        let archive = Archive::default();
        let mut vm = TtmVm::new(&ttm(code), 0, &pal, 16, 16).unwrap();
        vm.step(&archive).unwrap();
        assert_eq!(vm.frame().get(5, 5), Some(0)); // background only
    }
}
