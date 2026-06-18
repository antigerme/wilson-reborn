// SPDX-License-Identifier: GPL-3.0-or-later
//! The ADS scene scheduler: runs an `.ADS` script that orchestrates up to
//! [`MAX_TTM_THREADS`] concurrent TTM animations, compositing their layers each frame.
//!
//! Faithful port of `adsLoad`/`adsPlayChunk`/`adsPlay` (`repos/jc_reborn/ads.c`):
//! a cooperative, variable-timestep scheduler. [`AdsVm::next_frame`] performs one
//! scheduler iteration and returns the composited frame (or `None` when the scene
//! ends). Island background/holiday threads and walking are added in a later phase.

use std::collections::HashMap;

use wilson_dgds::{decode_ads, Ads, AdsInstruction, Archive, Palette};

use crate::error::{EngineError, Result};
use crate::rng::Rng;
use crate::surface::{Surface, TRANSPARENT};
use crate::ttm_exec::{detect_transparent, run_frame, FrameOutcome, TtmSlot, TtmThread};

/// Maximum number of concurrent TTM threads (as in the original engine).
pub const MAX_TTM_THREADS: usize = 10;
/// Maximum number of loaded TTM slots.
pub const MAX_TTM_SLOTS: usize = 10;

/// A composited frame produced by the scheduler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdsFrame {
    /// The composited indexed-color image for this frame.
    pub surface: Surface,
    /// How long to display the frame, in engine ticks (1 tick = [`crate::MS_PER_TICK`] = 16 ms).
    pub delay_ticks: u16,
    /// Sound effect ids triggered during this frame.
    pub sounds: Vec<u16>,
}

#[derive(Debug, Clone, Copy)]
struct Chunk {
    slot: u16,
    tag: u16,
    start: usize,
}

#[derive(Debug, Clone, Copy)]
struct RandOp {
    kind: u8, // 0 = add scene, 1 = stop scene, 2 = nop
    slot: u16,
    tag: u16,
    num_plays: u16,
    weight: u16,
}

/// The ADS scene runtime.
#[derive(Debug, Clone)]
pub struct AdsVm {
    width: u16,
    height: u16,
    palette: Palette,
    transparent_src: Option<u8>,
    background: Surface,
    saved_zones: Surface,
    slots: Vec<TtmSlot>,
    threads: Vec<TtmThread>,
    ads: Vec<AdsInstruction>,
    tags: HashMap<u16, usize>,
    chunks: Vec<Chunk>,
    local_chunks: Vec<Chunk>,
    rand_ops: Vec<RandOp>,
    num_threads: usize,
    stop_requested: bool,
    rng: Rng,
    dx: i32,
    dy: i32,
}

impl AdsVm {
    /// Load an `.ADS` scene starting at `start_tag`, resolving its TTMs from `archive`.
    pub fn new(
        ads: &Ads,
        start_tag: u16,
        archive: &Archive,
        palette: &Palette,
        width: u16,
        height: u16,
        seed: u64,
    ) -> Result<Self> {
        let instructions = decode_ads(&ads.bytecode)?;
        let (tags, chunks, start) = ads_load(&instructions, start_tag);

        let mut slots = vec![TtmSlot::empty(); MAX_TTM_SLOTS];
        for res in &ads.resources {
            let ttm = archive
                .ttm(&res.name)
                .ok_or_else(|| EngineError::ResourceNotFound(res.name.clone()))?;
            let id = res.id as usize;
            if id < slots.len() {
                slots[id] = TtmSlot::load(ttm)?;
            }
        }

        let threads = (0..MAX_TTM_THREADS)
            .map(|_| TtmThread::new(width, height))
            .collect();

        let mut vm = AdsVm {
            width,
            height,
            palette: palette.clone(),
            transparent_src: detect_transparent(palette),
            background: Surface::new(width, height, 0),
            saved_zones: Surface::new(width, height, TRANSPARENT),
            slots,
            threads,
            ads: instructions,
            tags,
            chunks,
            local_chunks: Vec::new(),
            rand_ops: Vec::new(),
            num_threads: 0,
            stop_requested: false,
            rng: Rng::new(seed),
            dx: 0,
            dy: 0,
        };
        vm.play_chunk(start);
        Ok(vm)
    }

    /// The palette used to render frames.
    pub fn palette(&self) -> &Palette {
        &self.palette
    }

    /// Number of currently active TTM threads.
    pub fn active_threads(&self) -> usize {
        self.num_threads
    }

    /// Set the island offset applied to all drawing coordinates.
    pub fn set_offset(&mut self, dx: i32, dy: i32) {
        self.dx = dx;
        self.dy = dy;
    }

    /// Replace the shared background that thread layers composite over (e.g. the
    /// island scenery). Island activity scenes draw over this rather than black.
    pub fn set_background(&mut self, background: Surface) {
        self.background = background;
    }

    /// Run one scheduler iteration; returns the composited frame, or `None` when the
    /// scene has ended (no more threads).
    pub fn next_frame(&mut self, archive: &Archive) -> Result<Option<AdsFrame>> {
        if self.num_threads == 0 {
            return Ok(None);
        }

        let mut sounds = Vec::new();

        // 1. Step every thread whose timer has elapsed.
        {
            let Self {
                threads,
                slots,
                background,
                saved_zones,
                transparent_src,
                dx,
                dy,
                ..
            } = self;
            // Indexed loop: we need disjoint &mut borrows of threads[i] and slots[..].
            #[allow(clippy::needless_range_loop)]
            for i in 0..threads.len() {
                if threads[i].running == 1 && threads[i].timer == 0 {
                    threads[i].timer = threads[i].delay;
                    let slot_idx = threads[i].slot_no;
                    let thread = &mut threads[i];
                    let slot = &mut slots[slot_idx];
                    match run_frame(
                        thread,
                        slot,
                        background,
                        saved_zones,
                        *transparent_src,
                        *dx,
                        *dy,
                        archive,
                    )? {
                        FrameOutcome::Frame { sounds: mut s } => sounds.append(&mut s),
                        FrameOutcome::Finished => {}
                    }
                }
            }
        }

        // 2. Composite background → saved-zones layer → every active thread layer
        //    (same order as jc_reborn's grUpdateDisplay).
        let mut surface = self.background.compose_over(&self.saved_zones);
        for t in &self.threads {
            if t.running != 0 {
                surface = surface.compose_over(&t.layer);
            }
        }

        // 3. Shortest pending delay across active threads.
        let mut mini = 300u16;
        for t in &self.threads {
            if t.running != 0 {
                mini = mini.min(t.delay).min(t.timer);
            }
        }

        // 4. Advance all timers by that amount.
        for t in &mut self.threads {
            if t.running != 0 {
                t.timer = t.timer.saturating_sub(mini);
            }
        }

        // 5. Post-process threads whose timer reached 0.
        for i in 0..self.threads.len() {
            if self.threads[i].running == 0 || self.threads[i].timer != 0 {
                continue;
            }
            if let Some(g) = self.threads[i].next_goto.take() {
                self.threads[i].ip = g;
            }
            if self.threads[i].scene_timer > 0 {
                self.threads[i].scene_timer -= i32::from(self.threads[i].delay);
                if self.threads[i].scene_timer <= 0 {
                    self.threads[i].running = 2;
                }
            }
            if self.threads[i].running == 2 {
                if self.threads[i].scene_iterations > 0 {
                    self.threads[i].scene_iterations -= 1;
                    self.threads[i].running = 1;
                    let slot_idx = self.threads[i].slot_no;
                    let tag = self.threads[i].scene_tag;
                    self.threads[i].ip = self.slots[slot_idx].find_tag(tag);
                } else {
                    let slot = self.threads[i].scene_slot;
                    let tag = self.threads[i].scene_tag;
                    self.stop_scene(i);
                    if !self.stop_requested {
                        self.play_triggered_chunks(slot, tag);
                    }
                }
            }
        }

        Ok(Some(AdsFrame {
            surface,
            delay_ticks: mini,
            sounds,
        }))
    }

    fn find_tag(&self, tag: u16) -> usize {
        self.tags.get(&tag).copied().unwrap_or(0)
    }

    fn is_running(&self, slot: u16, tag: u16) -> bool {
        self.threads
            .iter()
            .any(|t| t.running == 1 && t.scene_slot == slot && t.scene_tag == tag)
    }

    fn add_scene(&mut self, slot_no: u16, tag: u16, arg3: u16) {
        if self
            .threads
            .iter()
            .any(|t| t.running == 1 && t.scene_slot == slot_no && t.scene_tag == tag)
        {
            return;
        }
        let Some(i) = self.threads.iter().position(|t| t.running == 0) else {
            return;
        };

        let slot_idx = slot_no as usize;
        let ip = if slot_no != 0 {
            self.slots.get(slot_idx).map_or(0, |s| s.find_tag(tag))
        } else {
            0
        };

        let mut thread = TtmThread::new(self.width, self.height);
        thread.running = 1;
        thread.slot_no = slot_idx;
        thread.scene_slot = slot_no;
        thread.scene_tag = tag;
        thread.ip = ip;
        let signed = arg3 as i16;
        if signed < 0 {
            thread.scene_timer = -i32::from(signed);
        } else if signed > 0 {
            thread.scene_iterations = arg3 - 1;
        }
        self.threads[i] = thread;
        self.num_threads += 1;
    }

    fn stop_scene(&mut self, i: usize) {
        if self.threads[i].running != 0 {
            self.threads[i].running = 0;
            self.num_threads -= 1;
        }
    }

    fn stop_by_tag(&mut self, slot: u16, tag: u16) {
        for i in 0..self.threads.len() {
            if self.threads[i].running != 0
                && self.threads[i].scene_slot == slot
                && self.threads[i].scene_tag == tag
            {
                self.stop_scene(i);
            }
        }
    }

    fn random_end(&mut self) {
        if self.rand_ops.is_empty() {
            return;
        }
        let total: u32 = self.rand_ops.iter().map(|o| u32::from(o.weight)).sum();
        if total == 0 {
            return;
        }
        let pick = self.rng.below(total);
        let mut acc = 0u32;
        let mut chosen = self.rand_ops[0];
        for op in &self.rand_ops {
            acc += u32::from(op.weight);
            if pick < acc {
                chosen = *op;
                break;
            }
        }
        match chosen.kind {
            0 => self.add_scene(chosen.slot, chosen.tag, chosen.num_plays),
            1 => self.stop_by_tag(chosen.slot, chosen.tag),
            _ => {}
        }
    }

    fn play_triggered_chunks(&mut self, slot: u16, tag: u16) {
        if !self.local_chunks.is_empty() {
            let starts: Vec<usize> = self
                .local_chunks
                .iter()
                .filter(|c| c.slot == slot && c.tag == tag)
                .map(|c| c.start)
                .collect();
            self.local_chunks
                .retain(|c| !(c.slot == slot && c.tag == tag));
            for s in starts {
                self.play_chunk(s);
            }
        } else {
            let starts: Vec<usize> = self
                .chunks
                .iter()
                .filter(|c| c.slot == slot && c.tag == tag)
                .map(|c| c.start)
                .collect();
            for s in starts {
                self.play_chunk(s);
            }
        }
    }

    fn play_chunk(&mut self, start: usize) {
        let mut i = start;
        let mut in_rand = false;
        let mut in_or = false;
        let mut in_skip = false;
        let mut in_if_local = false;

        while i < self.ads.len() {
            let ins = self.ads[i].clone();
            i += 1;
            let (a0, a1, a2, a3) = (arg(&ins, 0), arg(&ins, 1), arg(&ins, 2), arg(&ins, 3));

            match ins.opcode {
                0x1070 => {
                    in_if_local = true;
                    self.local_chunks.push(Chunk {
                        slot: a0,
                        tag: a1,
                        start: i,
                    });
                }
                0x1330 => {}
                0x1350 => {
                    if !in_or {
                        break;
                    }
                    in_or = false;
                }
                0x1360 => {
                    if self.is_running(a0, a1) {
                        in_skip = true;
                    }
                }
                0x1370 => in_skip = !self.is_running(a0, a1),
                0x1420 => {}
                0x1430 => in_or = true,
                0x1510 => {
                    if in_skip {
                        in_skip = false;
                    } else {
                        break;
                    }
                }
                0x1520 => {
                    if in_if_local {
                        in_if_local = false;
                    } else {
                        self.add_scene(a1, a2, a3);
                    }
                }
                0x2005 => {
                    if !in_skip {
                        if in_rand {
                            self.rand_ops.push(RandOp {
                                kind: 0,
                                slot: a0,
                                tag: a1,
                                num_plays: a2,
                                weight: a3,
                            });
                        } else {
                            self.add_scene(a0, a1, a2);
                        }
                    }
                }
                0x2010 => {
                    if !in_skip {
                        if in_rand {
                            self.rand_ops.push(RandOp {
                                kind: 1,
                                slot: a0,
                                tag: a1,
                                num_plays: 0,
                                weight: a2,
                            });
                        } else {
                            self.stop_by_tag(a0, a1);
                        }
                    }
                }
                0x3010 => {
                    self.rand_ops.clear();
                    in_rand = true;
                }
                0x3020 => {
                    if in_rand {
                        self.rand_ops.push(RandOp {
                            kind: 2,
                            slot: 0,
                            tag: 0,
                            num_plays: 0,
                            weight: a0,
                        });
                    }
                }
                0x30FF => {
                    self.random_end();
                    in_rand = false;
                }
                0x4000 => {}
                0xF010 => {}
                0xF200 => {
                    let target = self.find_tag(a0);
                    self.play_chunk(target);
                }
                0xFFFF => {
                    if in_skip {
                        in_skip = false;
                    } else {
                        self.stop_requested = true;
                    }
                }
                0xFFF0 => {}
                _ => {}
            }
        }
    }
}

fn arg(ins: &AdsInstruction, i: usize) -> u16 {
    ins.args.get(i).copied().unwrap_or(0)
}

fn ads_load(
    instructions: &[AdsInstruction],
    start_tag: u16,
) -> (HashMap<u16, usize>, Vec<Chunk>, usize) {
    let mut tags = HashMap::new();
    let mut chunks = Vec::new();
    let mut start = 0usize;
    let mut bookmarking = false;
    let mut bookmarking_inr = false;

    for (i, ins) in instructions.iter().enumerate() {
        match ins.opcode {
            0x1350 => {
                if bookmarking {
                    bookmarking_inr = false;
                    chunks.push(Chunk {
                        slot: arg(ins, 0),
                        tag: arg(ins, 1),
                        start: i + 1,
                    });
                }
            }
            0x1360 => {
                if bookmarking && bookmarking_inr {
                    chunks.push(Chunk {
                        slot: arg(ins, 0),
                        tag: arg(ins, 1),
                        start: i + 1,
                    });
                }
            }
            0x1370 => bookmarking_inr = false,
            _ if ins.is_tag() => {
                tags.insert(ins.opcode, i + 1);
                if ins.opcode == start_tag {
                    start = i + 1;
                    bookmarking = true;
                    bookmarking_inr = true;
                } else {
                    bookmarking = false;
                    bookmarking_inr = false;
                }
            }
            _ => {}
        }
    }

    (tags, chunks, start)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wilson_dgds::{Bmp, BmpImage, Scr, Ttm};

    fn palette() -> Palette {
        let mut colors = [[0u8; 3]; 256];
        colors[5] = [168, 0, 168]; // transparent key
        Palette { colors }
    }

    fn op(code: u16) -> [u8; 2] {
        code.to_le_bytes()
    }

    fn anim_ttm() -> Ttm {
        // TAG 1; LOAD_SCREEN BG.SCR; SET_BMP_SLOT 0; LOAD_IMAGE S.BMP;
        // DRAW_SPRITE 0 0 0 0; UPDATE
        let mut code = Vec::new();
        code.extend_from_slice(&op(0x1111));
        code.extend_from_slice(&1u16.to_le_bytes());
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
        code.extend_from_slice(&op(0x0FF0));
        Ttm {
            version: "1.20".to_string(),
            num_pages: 1,
            bytecode: code,
            tags: Vec::new(),
        }
    }

    fn archive() -> Archive {
        Archive {
            bitmaps: vec![(
                "S.BMP".to_string(),
                Bmp {
                    width: 2,
                    height: 2,
                    images: vec![BmpImage {
                        width: 2,
                        height: 2,
                        pixels: vec![1, 5, 2, 5],
                    }],
                },
            )],
            screens: vec![(
                "BG.SCR".to_string(),
                Scr {
                    width: 2,
                    height: 2,
                    pixels: vec![3, 3, 3, 3],
                },
            )],
            ttms: vec![("A.TTM".to_string(), anim_ttm())],
            ..Default::default()
        }
    }

    fn ads(bytecode: Vec<u8>) -> Ads {
        Ads {
            version: "1.20".to_string(),
            resources: vec![wilson_dgds::AdsRes {
                id: 1,
                name: "A.TTM".to_string(),
            }],
            bytecode,
            tags: Vec::new(),
        }
    }

    #[test]
    fn plays_single_scene() {
        // TAG 1; ADD_SCENE slot=1 tag=1 arg3=0; PLAY_SCENE; END
        let mut code = Vec::new();
        code.extend_from_slice(&op(0x0001)); // tag id 1
        code.extend_from_slice(&op(0x2005));
        for v in [1u16, 1, 0, 0] {
            code.extend_from_slice(&v.to_le_bytes());
        }
        code.extend_from_slice(&op(0x1510)); // PLAY_SCENE
        code.extend_from_slice(&op(0xFFFF)); // END

        let arch = archive();
        let pal = palette();
        let mut vm = AdsVm::new(&ads(code), 1, &arch, &pal, 2, 2, 12345).unwrap();
        assert_eq!(vm.active_threads(), 1);

        let frame = vm.next_frame(&arch).unwrap().expect("a frame");
        assert_eq!(frame.surface.pixels, vec![1, 3, 2, 3]);
        assert_eq!(frame.delay_ticks, 4);

        // Drain to completion.
        let mut guard = 0;
        while vm.next_frame(&arch).unwrap().is_some() {
            guard += 1;
            assert!(guard < 100, "scene did not terminate");
        }
        assert_eq!(vm.active_threads(), 0);
    }

    #[test]
    fn random_block_picks_one_scene() {
        // TAG 1; RANDOM_START; ADD_SCENE 1 1 0 w=1; ADD_SCENE 1 2 0 w=1; RANDOM_END; PLAY_SCENE
        let mut code = Vec::new();
        code.extend_from_slice(&op(0x0001));
        code.extend_from_slice(&op(0x3010)); // RANDOM_START
        code.extend_from_slice(&op(0x2005));
        for v in [1u16, 1, 0, 1] {
            code.extend_from_slice(&v.to_le_bytes());
        }
        code.extend_from_slice(&op(0x2005));
        for v in [1u16, 2, 0, 1] {
            code.extend_from_slice(&v.to_le_bytes());
        }
        code.extend_from_slice(&op(0x30FF)); // RANDOM_END
        code.extend_from_slice(&op(0x1510)); // PLAY_SCENE

        let arch = archive();
        let pal = palette();
        let vm = AdsVm::new(&ads(code), 1, &arch, &pal, 2, 2, 999).unwrap();
        // Exactly one of the two candidate scenes was launched.
        assert_eq!(vm.active_threads(), 1);
    }
}
