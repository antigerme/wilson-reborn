// SPDX-License-Identifier: GPL-3.0-or-later
//! Shared TTM execution core: a loaded TTM "slot", a per-thread execution state,
//! and [`run_frame`] which advances one thread by one frame.
//!
//! Used by both the single-thread [`crate::ttm_vm::TtmVm`] and the multi-thread ADS
//! scheduler ([`crate::ads_vm::AdsVm`]). Faithful port of `ttmPlay`
//! (`repos/jc_reborn/ttm.c`): the background (`LOAD_SCREEN`) is shared/global, while
//! sprite sheets (`LOAD_IMAGE`) live on the running slot and per-thread state holds
//! colours, clip, delay and the instruction pointer.

use std::collections::HashMap;

use wilson_dgds::{Archive, BmpImage, Palette, Ttm, TtmArgs};

use crate::error::{EngineError, Result};
use crate::surface::{Rect, Surface, TRANSPARENT};

/// Maximum number of bitmap (sprite-sheet) slots per TTM, as in the original.
pub const MAX_BMP_SLOTS: usize = 6;

/// RGB of the original magenta colour key (6-bit `(42,0,42)` scaled by `<<2`).
pub const MAGENTA: [u8; 3] = [168, 0, 168];

/// Find the palette index used as the transparent colour key, if present.
pub fn detect_transparent(palette: &Palette) -> Option<u8> {
    palette
        .colors
        .iter()
        .position(|c| *c == MAGENTA)
        .map(|i| i as u8)
}

/// A loaded TTM resource: decoded instructions, a tag→index map, and the sprite
/// sheets loaded by `LOAD_IMAGE` (shared by all threads running this slot).
#[derive(Debug, Clone)]
pub struct TtmSlot {
    /// Decoded TTM instructions.
    pub instructions: Vec<wilson_dgds::TtmInstruction>,
    /// Map from tag id to instruction index.
    pub tags: HashMap<u16, usize>,
    /// Sprite sheets per bitmap slot (each a list of frames).
    pub sprites: Vec<Vec<BmpImage>>,
}

impl TtmSlot {
    /// An empty, unused slot.
    pub fn empty() -> Self {
        TtmSlot {
            instructions: Vec::new(),
            tags: HashMap::new(),
            sprites: vec![Vec::new(); MAX_BMP_SLOTS],
        }
    }

    /// Load and decode a TTM resource into a slot.
    pub fn load(ttm: &Ttm) -> Result<Self> {
        let instructions = ttm.instructions()?;
        let mut tags = HashMap::new();
        for (idx, ins) in instructions.iter().enumerate() {
            if ins.opcode == 0x1111 || ins.opcode == 0x1101 {
                tags.insert(word(&ins.args, 0), idx);
            }
        }
        Ok(TtmSlot {
            instructions,
            tags,
            sprites: vec![Vec::new(); MAX_BMP_SLOTS],
        })
    }

    /// Instruction index for `tag` (0 if not found, mirroring the reference engine).
    pub fn find_tag(&self, tag: u16) -> usize {
        self.tags.get(&tag).copied().unwrap_or(0)
    }
}

/// Per-thread TTM execution state plus its own drawing layer.
#[derive(Debug, Clone)]
pub struct TtmThread {
    /// The thread's transparent drawing layer.
    pub layer: Surface,
    /// 0 = free, 1 = running, 2 = finished-this-pass (pending cleanup).
    pub running: u8,
    /// Index into the runtime's slot table.
    pub slot_no: usize,
    /// Scene identity (the ADS `(slot, tag)` that spawned this thread).
    pub scene_slot: u16,
    /// Scene tag.
    pub scene_tag: u16,
    /// Instruction pointer into the slot's instructions.
    pub ip: usize,
    /// Deferred jump target (applied between frames by the scheduler).
    pub next_goto: Option<usize>,
    /// Frame delay in ticks.
    pub delay: u16,
    /// Countdown to the next time this thread runs.
    pub timer: u16,
    /// Remaining "play for N ticks" budget (ADD_SCENE negative arg3).
    pub scene_timer: i32,
    /// Remaining replays (ADD_SCENE positive arg3).
    pub scene_iterations: u16,
    /// Currently selected bitmap slot for draws.
    pub selected_bmp_slot: usize,
    /// Foreground colour index.
    pub fg: u8,
    /// Background colour index.
    pub bg: u8,
    /// Active clip rectangle.
    pub clip: Option<Rect>,
}

impl TtmThread {
    /// A free thread with a transparent layer of the given size.
    pub fn new(width: u16, height: u16) -> Self {
        TtmThread {
            layer: Surface::new(width, height, TRANSPARENT),
            running: 0,
            slot_no: 0,
            scene_slot: 0,
            scene_tag: 0,
            ip: 0,
            next_goto: None,
            delay: 4,
            timer: 0,
            scene_timer: 0,
            scene_iterations: 0,
            selected_bmp_slot: 0,
            fg: 0x0F,
            bg: 0x0F,
            clip: None,
        }
    }
}

/// Outcome of running one frame of a TTM thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameOutcome {
    /// An `UPDATE` (or a looping `PURGE`) yielded a frame; `sounds` were triggered.
    Frame {
        /// Sound effect ids triggered during this frame.
        sounds: Vec<u16>,
    },
    /// The thread ended (`running` is set to 2).
    Finished,
}

/// Run a single thread from its current `ip` until the next `UPDATE` or its end.
///
/// `background` is the shared global background (set by `LOAD_SCREEN`); `dx`/`dy` are
/// the island offset added to all coordinates.
pub fn run_frame(
    thread: &mut TtmThread,
    slot: &mut TtmSlot,
    background: &mut Surface,
    transparent_src: Option<u8>,
    dx: i32,
    dy: i32,
    archive: &Archive,
) -> Result<FrameOutcome> {
    let mut sounds = Vec::new();

    loop {
        if thread.ip >= slot.instructions.len() {
            thread.running = 2;
            return Ok(FrameOutcome::Finished);
        }
        let idx = thread.ip;
        let ins = slot.instructions[idx].clone();
        thread.ip = idx + 1;
        let a = &ins.args;

        match ins.opcode {
            0x0FF0 => return Ok(FrameOutcome::Frame { sounds }),
            0x0110 => {
                // PURGE: loop the scene while a "play for N ticks" budget is active,
                // otherwise end the thread.
                if thread.scene_timer > 0 {
                    thread.next_goto = Some(find_previous_tag(slot, idx));
                    return Ok(FrameOutcome::Frame { sounds });
                }
                thread.running = 2;
                return Ok(FrameOutcome::Finished);
            }
            0x1021 => thread.delay = word(a, 0).max(4),
            0x2022 => {
                thread.delay = ((u32::from(word(a, 0)) + u32::from(word(a, 1))) / 2) as u16;
            }
            0x1051 => thread.selected_bmp_slot = word(a, 0) as usize,
            0x1201 => thread.next_goto = Some(slot.find_tag(word(a, 0))),
            0x2002 => {
                thread.fg = word(a, 0) as u8;
                thread.bg = word(a, 1) as u8;
            }
            0x4004 => {
                let x1 = signed(a, 0) + dx;
                let y1 = signed(a, 1) + dy;
                let x2 = signed(a, 2) + dx;
                let y2 = signed(a, 3) + dy;
                thread.clip = Some(Rect {
                    x: x1,
                    y: y1,
                    w: x2 - x1,
                    h: y2 - y1,
                });
            }
            0xA002 => thread
                .layer
                .put_pixel(signed(a, 0) + dx, signed(a, 1) + dy, thread.fg),
            0xA0A4 => thread.layer.draw_line(
                signed(a, 0) + dx,
                signed(a, 1) + dy,
                signed(a, 2) + dx,
                signed(a, 3) + dy,
                thread.fg,
            ),
            0xA104 => thread.layer.fill_rect(
                signed(a, 0) + dx,
                signed(a, 1) + dy,
                unsigned(a, 2),
                unsigned(a, 3),
                thread.fg,
                thread.clip,
            ),
            0xA404 => thread.layer.draw_circle(
                signed(a, 0) + dx,
                signed(a, 1) + dy,
                unsigned(a, 2),
                unsigned(a, 3),
                thread.fg,
                thread.bg,
            ),
            0xA504 | 0xA524 => {
                let x = signed(a, 0) + dx;
                let y = signed(a, 1) + dy;
                let frame = word(a, 2) as usize;
                let sheet = word(a, 3) as usize;
                let flip = ins.opcode == 0xA524;
                if let Some(img) = slot.sprites.get(sheet).and_then(|v| v.get(frame)) {
                    thread.layer.blit(
                        img.width,
                        img.height,
                        &img.pixels,
                        x,
                        y,
                        Some(TRANSPARENT),
                        flip,
                        thread.clip,
                    );
                }
            }
            0xA601 => thread.layer.fill(TRANSPARENT),
            0xC051 => sounds.push(word(a, 0)),
            0xF01F => {
                let name = string(a);
                let scr = archive
                    .scr(name)
                    .ok_or_else(|| EngineError::ResourceNotFound(name.to_string()))?;
                background.fill(0);
                for sy in 0..i32::from(scr.height) {
                    for sx in 0..i32::from(scr.width) {
                        let p = scr.pixels[(sy * i32::from(scr.width) + sx) as usize];
                        background.put_pixel(sx, sy, p);
                    }
                }
            }
            0xF02F => {
                let name = string(a);
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
                                if Some(px) == transparent_src {
                                    TRANSPARENT
                                } else {
                                    px
                                }
                            })
                            .collect(),
                    })
                    .collect();
                if let Some(sheet) = slot.sprites.get_mut(thread.selected_bmp_slot) {
                    *sheet = images;
                }
            }
            // Recognised no-ops and anything unknown.
            _ => {}
        }
    }
}

fn find_previous_tag(slot: &TtmSlot, before: usize) -> usize {
    for j in (0..before).rev() {
        let op = slot.instructions[j].opcode;
        if op == 0x1111 || op == 0x1101 {
            return j;
        }
    }
    0
}

pub(crate) fn word(args: &TtmArgs, i: usize) -> u16 {
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
