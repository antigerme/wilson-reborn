// SPDX-License-Identifier: GPL-3.0-or-later
//! WASM/web frontend for **Wilson Reborn** — runs the headless engine in a browser.
//!
//! The game data is copyright (Sierra/Dynamix), so nothing is bundled: the page asks the
//! user to pick their own `RESOURCE.MAP` + `RESOURCE.001`, hands the bytes to [`Wilson::new`],
//! and then calls [`Wilson::frame`] on a timer, drawing the returned RGBA into a `<canvas>`.
//!
//! The whole crate is `wasm32`-only (see the `#![cfg]` below); on any other target it is an
//! empty library, so it sits in the workspace without affecting the desktop build.
#![cfg(target_arch = "wasm32")]

use wasm_bindgen::prelude::*;
use wilson_dgds::{Archive, Palette};
use wilson_engine::{clock, Director, Show};

/// The engine renders at the original's fixed 640×480.
const WIDTH: u16 = 640;
const HEIGHT: u16 = 480;
/// Sound-effect slots (ids `0..25`), matching the desktop player and `sounds_from_scrantic_exe`.
#[cfg(feature = "embed-data")]
const NUM_SOUNDS: usize = 25;

/// Original data baked into the wasm by `build.rs` (only in an `embed-data` build).
#[cfg(feature = "embed-data")]
mod embedded {
    include!(concat!(env!("OUT_DIR"), "/embedded_data.rs"));
}

/// Whether this build has the original data baked in (the `embed-data` feature). The page
/// reads it to decide: auto-start ([`Wilson::embedded`]) vs. ask for the user's `RESOURCE.*`.
#[wasm_bindgen]
pub fn has_embedded_data() -> bool {
    cfg!(feature = "embed-data")
}

/// A running Wilson Reborn instance, driven from JavaScript.
#[wasm_bindgen]
pub struct Wilson {
    show: Show,
    archive: Archive,
    palette: Palette,
    delay_ticks: u16,
    /// WAV bytes per sound-effect id (the originals live in `SCRANTIC.EXE`, not `RESOURCE.*`):
    /// baked in for an `embed-data` build, or supplied at runtime via [`Wilson::set_sound_data`].
    /// Empty ⇒ silent. The page plays these via the Web Audio API.
    sounds: Vec<Option<Vec<u8>>>,
    /// Sound-effect ids the last [`Wilson::frame`] fired, drained by [`Wilson::take_sounds`].
    last_sounds: Vec<u16>,
}

#[wasm_bindgen]
impl Wilson {
    /// Build the runtime from the user's `RESOURCE.MAP` and `RESOURCE.001` bytes.
    ///
    /// `seed` randomises the run (pass e.g. `Math.random() * 2**53`); `now_secs` is the wall
    /// clock as Unix seconds (`Date.now() / 1000`), used for the day/holiday logic.
    #[wasm_bindgen(constructor)]
    pub fn new(map: &[u8], data: &[u8], seed: f64, now_secs: f64) -> Result<Wilson, JsValue> {
        Wilson::build(map, data, seed, now_secs)
    }

    /// Build from the data baked into the wasm at compile time (the `embed-data` feature) — a
    /// self-contained page, no file picker. Only present in an `embed-data` build (the page
    /// calls it when [`has_embedded_data`] is true).
    #[cfg(feature = "embed-data")]
    pub fn embedded(seed: f64, now_secs: f64) -> Result<Wilson, JsValue> {
        let mut wilson = Wilson::build(embedded::MAP, embedded::DATA, seed, now_secs)?;
        // Bake-in the sound effects too (extracted from SCRANTIC.EXE at build time).
        let mut sounds = vec![None; NUM_SOUNDS];
        for &(id, bytes) in embedded::SOUNDS {
            if let Some(slot) = sounds.get_mut(id as usize) {
                *slot = Some(bytes.to_vec());
            }
        }
        wilson.sounds = sounds;
        Ok(wilson)
    }

    /// Advance one frame at wall-clock `now_secs` and return its pixels as RGBA bytes
    /// (`WIDTH * HEIGHT * 4`), ready to wrap in a `Uint8ClampedArray` / `ImageData`. The
    /// sound effects fired this frame are stashed for [`Wilson::take_sounds`].
    pub fn frame(&mut self, now_secs: f64) -> Vec<u8> {
        self.show.set_clock(clock::from_unix(now_secs as u64));
        let frame = self.show.next_frame(&self.archive);
        self.delay_ticks = frame.delay_ticks;
        self.last_sounds = frame.sounds; // partial move; frame.surface stays usable
        frame.surface.to_rgba(&self.palette)
    }

    /// Drain the sound-effect ids the last [`Wilson::frame`] fired (incl. the day-transition
    /// cue, which the engine folds in). The page maps each id through [`Wilson::sound_wav`].
    pub fn take_sounds(&mut self) -> Vec<u32> {
        std::mem::take(&mut self.last_sounds)
            .into_iter()
            .map(u32::from)
            .collect()
    }

    /// The WAV bytes for sound-effect `id`, or an empty array if none is loaded for it.
    pub fn sound_wav(&self, id: u32) -> Vec<u8> {
        self.sounds
            .get(id as usize)
            .and_then(|o| o.clone())
            .unwrap_or_default()
    }

    /// Whether any sound effect is loaded (so the page can hint when audio is unavailable).
    pub fn has_sound(&self) -> bool {
        self.sounds.iter().any(Option::is_some)
    }

    /// Load the sound effects from a `SCRANTIC.EXE`/`.SCR` the user supplied (bring-your-own:
    /// the WAVs are embedded in that binary, not in `RESOURCE.*`). Returns how many loaded.
    pub fn set_sound_data(&mut self, exe: &[u8]) -> usize {
        self.sounds = wilson_dgds::sounds_from_scrantic_exe(exe);
        self.sounds.iter().filter(|o| o.is_some()).count()
    }

    /// How long to display the last [`Wilson::frame`], in milliseconds (for the JS timer).
    pub fn delay_ms(&self) -> u32 {
        u32::from(self.delay_ticks) * wilson_engine::MS_PER_TICK as u32
    }

    /// Frame width in pixels (640).
    pub fn width(&self) -> u32 {
        u32::from(WIDTH)
    }

    /// Frame height in pixels (480).
    pub fn height(&self) -> u32 {
        u32::from(HEIGHT)
    }

    /// Enable the opt-in dissolve transition between story runs (the original's dormant
    /// LFSR tiled dissolve). Off by default = faithful hard cut.
    pub fn enable_dissolve(&mut self) {
        self.show.enable_dissolve();
    }
}

// Private helpers (a separate, non-`#[wasm_bindgen]` impl so they are not exported to JS).
impl Wilson {
    /// Parse `map`+`data`, take the palette, and start a [`Show`] — shared by `new`
    /// (JS-provided bytes) and `embedded` (compile-time bytes).
    fn build(map: &[u8], data: &[u8], seed: f64, now_secs: f64) -> Result<Wilson, JsValue> {
        let archive = Archive::parse(map, data)
            .map_err(|e| JsValue::from_str(&format!("failed to parse the game data: {e}")))?;
        let palette = archive
            .palette()
            .cloned()
            .ok_or_else(|| JsValue::from_str("the data has no palette (PAL) resource"))?;
        let cl = clock::from_unix(now_secs as u64);
        let director = Director::new(1, cl.yday);
        let show = Show::new(&archive, &palette, WIDTH, HEIGHT, director, cl, seed as u64);
        Ok(Wilson {
            show,
            archive,
            palette,
            delay_ticks: 1,
            sounds: Vec::new(), // populated by `embedded` or `set_sound_data`
            last_sounds: Vec::new(),
        })
    }
}
