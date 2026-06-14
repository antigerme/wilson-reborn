// SPDX-License-Identifier: GPL-3.0-or-later
//! A headless interpreter for a single TTM animation thread.
//!
//! Faithful port of `ttmPlay` (`repos/jc_reborn/ttm.c`) for one thread drawing onto
//! a [`Surface`] layer over a background. Each [`TtmVm::step`] runs opcodes until an
//! `UPDATE` yields a frame (or the script ends). The multi-thread ADS scheduler,
//! "saved zones" and island/walk logic come in later phases — the corresponding
//! opcodes are treated as no-ops here, exactly as the reference engine does for the
//! ones it stubs.

use std::collections::HashMap;

use wilson_dgds::{Archive, BmpImage, Palette, Ttm, TtmArgs, TtmInstruction};

use crate::error::{EngineError, Result};
use crate::surface::{Rect, Surface, TRANSPARENT};

/// Maximum number of bitmap (sprite-sheet) slots, as in the original engine.
pub const MAX_BMP_SLOTS: usize = 6;

/// RGB of the original magenta colour key (6-bit `(42,0,42)` scaled by `<<2`).
const MAGENTA: [u8; 3] = [168, 0, 168];

/// The result of running [`TtmVm::step`] for one frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TtmStep {
    /// An `UPDATE` produced a frame; wait `delay_ticks` and play `sounds`.
    Frame {
        /// Frame duration in engine ticks (1 tick = 20 ms).
        delay_ticks: u16,
        /// Sound effect ids triggered during this frame (from `PLAY_SAMPLE`).
        sounds: Vec<u16>,
    },
    /// The script ended (or was purged); no more frames.
    Finished,
}

/// A single-thread TTM interpreter.
#[derive(Debug, Clone)]
pub struct TtmVm {
    instructions: Vec<TtmInstruction>,
    tags: HashMap<u16, usize>,
    ip: usize,
    pending_goto: Option<usize>,
    finished: bool,

    width: u16,
    height: u16,
    background: Surface,
    layer: Surface,
    slots: Vec<Vec<BmpImage>>,
    selected_slot: usize,

    fg: u8,
    bg: u8,
    clip: Option<Rect>,
    delay: u16,
    dx: i32,
    dy: i32,
    /// Palette index whose colour is the transparent key, if any.
    transparent_src: Option<u8>,
}

impl TtmVm {
    /// Create a VM positioned at `start_tag` (falls back to the start if not found).
    ///
    /// `palette` is used to detect the transparent colour key when loading sprites;
    /// `width`/`height` size the layers (normally 640×480).
    pub fn new(
        ttm: &Ttm,
        start_tag: u16,
        palette: &Palette,
        width: u16,
        height: u16,
    ) -> Result<Self> {
        let instructions = ttm.instructions()?;

        let mut tags = HashMap::new();
        for (idx, ins) in instructions.iter().enumerate() {
            if ins.opcode == 0x1111 || ins.opcode == 0x1101 {
                tags.insert(word(&ins.args, 0), idx);
            }
        }

        let transparent_src = palette
            .colors
            .iter()
            .position(|c| *c == MAGENTA)
            .map(|i| i as u8);

        let ip = tags.get(&start_tag).copied().unwrap_or(0);

        Ok(TtmVm {
            instructions,
            tags,
            ip,
            pending_goto: None,
            finished: false,
            width,
            height,
            background: Surface::new(width, height, 0),
            layer: Surface::new(width, height, TRANSPARENT),
            slots: vec![Vec::new(); MAX_BMP_SLOTS],
            selected_slot: 0,
            fg: 0x0F,
            bg: 0x0F,
            clip: None,
            delay: 4,
            dx: 0,
            dy: 0,
            transparent_src,
        })
    }

    /// The TTM thread's drawing layer (with [`TRANSPARENT`] holes).
    pub fn layer(&self) -> &Surface {
        &self.layer
    }

    /// The current background (set by `LOAD_SCREEN`).
    pub fn background(&self) -> &Surface {
        &self.background
    }

    /// The composited current frame (background with the layer on top).
    pub fn frame(&self) -> Surface {
        self.background.compose_over(&self.layer)
    }

    /// Set the island offset applied to all drawing coordinates (`ttmDx`/`ttmDy`).
    pub fn set_offset(&mut self, dx: i32, dy: i32) {
        self.dx = dx;
        self.dy = dy;
    }

    /// Run opcodes until the next `UPDATE` (a frame) or the end of the script.
    pub fn step(&mut self, archive: &Archive) -> Result<TtmStep> {
        if self.finished {
            return Ok(TtmStep::Finished);
        }
        if let Some(target) = self.pending_goto.take() {
            self.ip = target;
        }

        let mut sounds = Vec::new();

        loop {
            if self.ip >= self.instructions.len() {
                self.finished = true;
                return Ok(TtmStep::Finished);
            }
            let ins = self.instructions[self.ip].clone();
            self.ip += 1;
            let a = &ins.args;

            match ins.opcode {
                0x0FF0 => {
                    // UPDATE: yield the frame.
                    return Ok(TtmStep::Frame {
                        delay_ticks: self.delay,
                        sounds,
                    });
                }
                0x0110 => {
                    // PURGE: end of scene (looping via sceneTimer is an ADS concern).
                    self.finished = true;
                    return Ok(TtmStep::Finished);
                }
                0x1021 => self.delay = word(a, 0).max(4),
                0x2022 => {
                    self.delay = ((u32::from(word(a, 0)) + u32::from(word(a, 1))) / 2) as u16;
                }
                0x1051 => self.selected_slot = word(a, 0) as usize,
                0x1201 => self.pending_goto = self.tags.get(&word(a, 0)).copied(),
                0x2002 => {
                    self.fg = word(a, 0) as u8;
                    self.bg = word(a, 1) as u8;
                }
                0x4004 => {
                    let x1 = signed(a, 0) + self.dx;
                    let y1 = signed(a, 1) + self.dy;
                    let x2 = signed(a, 2) + self.dx;
                    let y2 = signed(a, 3) + self.dy;
                    self.clip = Some(Rect {
                        x: x1,
                        y: y1,
                        w: x2 - x1,
                        h: y2 - y1,
                    });
                }
                0xA002 => {
                    self.layer
                        .put_pixel(signed(a, 0) + self.dx, signed(a, 1) + self.dy, self.fg);
                }
                0xA0A4 => self.layer.draw_line(
                    signed(a, 0) + self.dx,
                    signed(a, 1) + self.dy,
                    signed(a, 2) + self.dx,
                    signed(a, 3) + self.dy,
                    self.fg,
                ),
                0xA104 => self.layer.fill_rect(
                    signed(a, 0) + self.dx,
                    signed(a, 1) + self.dy,
                    unsigned(a, 2),
                    unsigned(a, 3),
                    self.fg,
                    self.clip,
                ),
                0xA404 => self.layer.draw_circle(
                    signed(a, 0) + self.dx,
                    signed(a, 1) + self.dy,
                    unsigned(a, 2),
                    unsigned(a, 3),
                    self.fg,
                    self.bg,
                ),
                0xA504 | 0xA524 => self.draw_sprite(a, ins.opcode == 0xA524),
                0xA601 => self.layer.fill(TRANSPARENT),
                0xC051 => sounds.push(word(a, 0)),
                0xF01F => self.load_screen(archive, string(a))?,
                0xF02F => self.load_image(archive, string(a))?,
                // Recognised no-ops (and anything unknown): nothing to do here.
                _ => {}
            }
        }
    }

    fn draw_sprite(&mut self, args: &TtmArgs, flip: bool) {
        let x = signed(args, 0) + self.dx;
        let y = signed(args, 1) + self.dy;
        let frame = word(args, 2) as usize;
        let slot = word(args, 3) as usize;
        if let Some(img) = self.slots.get(slot).and_then(|imgs| imgs.get(frame)) {
            self.layer.blit(
                img.width,
                img.height,
                &img.pixels,
                x,
                y,
                Some(TRANSPARENT),
                flip,
                self.clip,
            );
        }
    }

    fn load_screen(&mut self, archive: &Archive, name: &str) -> Result<()> {
        let scr = archive
            .scr(name)
            .ok_or_else(|| EngineError::ResourceNotFound(name.to_string()))?;
        let mut bg = Surface::new(self.width, self.height, 0);
        for sy in 0..i32::from(scr.height) {
            for sx in 0..i32::from(scr.width) {
                let p = scr.pixels[(sy * i32::from(scr.width) + sx) as usize];
                bg.put_pixel(sx, sy, p);
            }
        }
        self.background = bg;
        Ok(())
    }

    fn load_image(&mut self, archive: &Archive, name: &str) -> Result<()> {
        let bmp = archive
            .bmp(name)
            .ok_or_else(|| EngineError::ResourceNotFound(name.to_string()))?;
        let images: Vec<BmpImage> = bmp
            .images
            .iter()
            .map(|im| BmpImage {
                width: im.width,
                height: im.height,
                pixels: im
                    .pixels
                    .iter()
                    .map(|&px| {
                        if Some(px) == self.transparent_src {
                            TRANSPARENT
                        } else {
                            px
                        }
                    })
                    .collect(),
            })
            .collect();
        if let Some(slot) = self.slots.get_mut(self.selected_slot) {
            *slot = images;
        }
        Ok(())
    }
}

fn word(args: &TtmArgs, i: usize) -> u16 {
    match args {
        TtmArgs::Words(v) => v.get(i).copied().unwrap_or(0),
        TtmArgs::Str(_) => 0,
    }
}

fn signed(args: &TtmArgs, i: usize) -> i32 {
    i32::from(word(args, i) as i16)
}

fn unsigned(args: &TtmArgs, i: usize) -> i32 {
    i32::from(word(args, i))
}

fn string(args: &TtmArgs) -> &str {
    match args {
        TtmArgs::Str(s) => s,
        TtmArgs::Words(_) => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wilson_dgds::{Bmp, Scr};

    fn palette_with_magenta() -> Palette {
        let mut colors = [[0u8; 3]; 256];
        colors[1] = [10, 20, 30];
        colors[2] = [40, 50, 60];
        colors[3] = [70, 80, 90];
        colors[5] = MAGENTA; // transparent key at index 5
        Palette { colors }
    }

    fn archive_with_sprite_and_bg() -> Archive {
        let bmp = Bmp {
            width: 2,
            height: 2,
            images: vec![BmpImage {
                width: 2,
                height: 2,
                pixels: vec![1, 5, 2, 5], // index 5 = magenta -> transparent after load
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
        code.extend_from_slice(&op(0xF01F)); // LOAD_SCREEN
        code.extend_from_slice(b"BG.SCR\0\0"); // 7 bytes + 1 pad
        code.extend_from_slice(&op(0x1051)); // SET_BMP_SLOT
        code.extend_from_slice(&0u16.to_le_bytes());
        code.extend_from_slice(&op(0xF02F)); // LOAD_IMAGE
        code.extend_from_slice(b"S.BMP\0"); // 6 bytes (even)
        code.extend_from_slice(&op(0xA504)); // DRAW_SPRITE x y frame slot
        for v in [0u16, 0, 0, 0] {
            code.extend_from_slice(&v.to_le_bytes());
        }
        code.extend_from_slice(&op(0xC051)); // PLAY_SAMPLE
        code.extend_from_slice(&3u16.to_le_bytes());
        code.extend_from_slice(&op(0x0FF0)); // UPDATE

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
        // Layer: sprite blitted, magenta holes transparent.
        assert_eq!(vm.layer().pixels, vec![1, TRANSPARENT, 2, TRANSPARENT]);
        // Frame: background (3) shows through the holes.
        assert_eq!(vm.frame().pixels, vec![1, 3, 2, 3]);

        // No more frames.
        assert_eq!(vm.step(&archive).unwrap(), TtmStep::Finished);
    }

    #[test]
    fn set_colors_and_rect_and_delay() {
        let mut code = Vec::new();
        code.extend_from_slice(&op(0x1021)); // SET_DELAY 9
        code.extend_from_slice(&9u16.to_le_bytes());
        code.extend_from_slice(&op(0x2002)); // SET_COLORS fg=7 bg=0
        code.extend_from_slice(&7u16.to_le_bytes());
        code.extend_from_slice(&0u16.to_le_bytes());
        code.extend_from_slice(&op(0xA104)); // DRAW_RECT 0 0 2 1
        for v in [0u16, 0, 2, 1] {
            code.extend_from_slice(&v.to_le_bytes());
        }
        code.extend_from_slice(&op(0x0FF0)); // UPDATE

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
        // Top row filled with fg=7, bottom row untouched (transparent).
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
            Err(EngineError::ResourceNotFound("NOPE.SCR".to_string()))
        );
    }
}
