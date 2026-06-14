// SPDX-License-Identifier: GPL-3.0-or-later
//! A headless interpreter for a single TTM animation thread.
//!
//! A thin wrapper over [`crate::ttm_exec`] that runs one TTM scene against its own
//! background and layer — convenient for previewing a single `.TTM` (the engine's
//! `ttm <name>` mode). The multi-thread scene runtime is [`crate::ads_vm::AdsVm`].

use wilson_dgds::{Archive, Palette, Ttm};

use crate::error::Result;
use crate::surface::Surface;
use crate::ttm_exec::{detect_transparent, run_frame, FrameOutcome, TtmSlot, TtmThread};

/// The result of running [`TtmVm::step`] for one frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TtmStep {
    /// An `UPDATE` produced a frame; wait `delay_ticks` and play `sounds`.
    Frame {
        /// Frame duration in engine ticks (1 tick = 20 ms).
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

    /// The composited current frame (background with the layer on top).
    pub fn frame(&self) -> Surface {
        self.background.compose_over(&self.thread.layer)
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
}
