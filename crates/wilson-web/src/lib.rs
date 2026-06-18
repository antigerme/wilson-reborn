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

/// A running Wilson Reborn instance, driven from JavaScript.
#[wasm_bindgen]
pub struct Wilson {
    show: Show,
    archive: Archive,
    palette: Palette,
    delay_ticks: u16,
}

#[wasm_bindgen]
impl Wilson {
    /// Build the runtime from the user's `RESOURCE.MAP` and `RESOURCE.001` bytes.
    ///
    /// `seed` randomises the run (pass e.g. `Math.random() * 2**53`); `now_secs` is the wall
    /// clock as Unix seconds (`Date.now() / 1000`), used for the day/holiday logic.
    #[wasm_bindgen(constructor)]
    pub fn new(map: &[u8], data: &[u8], seed: f64, now_secs: f64) -> Result<Wilson, JsValue> {
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
        })
    }

    /// Advance one frame at wall-clock `now_secs` and return its pixels as RGBA bytes
    /// (`WIDTH * HEIGHT * 4`), ready to wrap in a `Uint8ClampedArray` / `ImageData`.
    pub fn frame(&mut self, now_secs: f64) -> Vec<u8> {
        self.show.set_clock(clock::from_unix(now_secs as u64));
        let frame = self.show.next_frame(&self.archive);
        self.delay_ticks = frame.delay_ticks;
        frame.surface.to_rgba(&self.palette)
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
