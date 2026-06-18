// SPDX-License-Identifier: GPL-3.0-or-later
//! WASM/web frontend for **Wilson Reborn** — runs the headless engine in a browser.
//!
//! The game data is copyright (Sierra/Dynamix), so nothing is bundled (unless this is a
//! personal `embed-data` build): the page asks the user for their own `RESOURCE.*` (loose
//! files or a `scrantic-run.zip` / `scrantic-installer.zip`), hands the bytes to one of the
//! [`Wilson`] constructors, then calls [`Wilson::frame`] on a timer, drawing the returned
//! RGBA into a `<canvas>`. Runtime [`Options`] mirror the desktop CLI (speed, day, dissolve,
//! intro, day/night, story mode), so the page can drive them from URL params.
//!
//! The whole crate is `wasm32`-only (see the `#![cfg]` below); on any other target it is an
//! empty library, so it sits in the workspace without affecting the desktop build.
#![cfg(target_arch = "wasm32")]

use wasm_bindgen::prelude::*;
use wilson_dgds::{Archive, Palette};
use wilson_engine::{clock, Clock, DayNight, Director, Show};

/// The engine renders at the original's fixed 640×480.
const WIDTH: u16 = 640;
const HEIGHT: u16 = 480;
/// Sound-effect slots (ids `0..25`), matching the desktop player and `sounds_from_scrantic_exe`.
#[cfg(feature = "embed-data")]
const NUM_SOUNDS: usize = 25;
/// Playback-speed bounds (percent of the original timing), matching the desktop.
const SPEED_MIN: u32 = 25;
const SPEED_MAX: u32 = 400;
/// Story-mode default cadence: real seconds per story day (matches the desktop default).
const DEFAULT_STORY_DAY_SECS: u32 = 90;

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

/// Runtime options, mirroring the desktop CLI/config. JS builds one (`new Options()`), sets the
/// fields (e.g. from URL params), and passes it to a [`Wilson`] constructor.
#[wasm_bindgen]
#[derive(Clone)]
pub struct Options {
    /// Run seed (e.g. `Math.random() * 2**53`); `0` is allowed (just a fixed seed).
    pub seed: f64,
    /// Start the 11-day arc at this day (`1..=11`); `0` = day 1 (advancing with the real date).
    pub day: u8,
    /// Playback speed, percent of the original timing (`100` = original); `0` ⇒ `100`.
    pub speed: u32,
    /// Opt-in dissolve transition between story runs (the original's dormant tiled dissolve).
    pub dissolve: bool,
    /// Show the original's intro screen (`INTRO.SCR`) once at startup (on by default, like the
    /// original's `Introduction` option).
    pub intro: bool,
    /// Play the whole arc in order (day 1 → 11 → 1 …) on a fixed cadence, ignoring real days.
    pub story: bool,
    /// Story-mode cadence: real seconds per story day; `0` ⇒ [`DEFAULT_STORY_DAY_SECS`].
    pub story_secs: u32,
    /// Real 24-hour day/night (night 20:00–06:00) instead of the original's 8-hour cycle.
    pub real_daynight: bool,
}

#[wasm_bindgen]
impl Options {
    /// Defaults matching the desktop: speed 100%, intro on, everything else off / automatic.
    #[wasm_bindgen(constructor)]
    #[allow(clippy::new_without_default)] // wasm-bindgen exports this as the JS constructor
    pub fn new() -> Options {
        Options {
            seed: 0.0,
            day: 0,
            speed: 0,
            dissolve: false,
            intro: true,
            story: false,
            story_secs: 0,
            real_daynight: false,
        }
    }
}

impl Options {
    /// Speed clamped to the supported range (`0` ⇒ 100).
    fn speed_pct(&self) -> u32 {
        if self.speed == 0 {
            100
        } else {
            self.speed.clamp(SPEED_MIN, SPEED_MAX)
        }
    }
    /// Story-mode cadence (`0` ⇒ default).
    fn story_day_secs(&self) -> u32 {
        if self.story_secs == 0 {
            DEFAULT_STORY_DAY_SECS
        } else {
            self.story_secs
        }
    }
}

/// A running Wilson Reborn instance, driven from JavaScript.
#[wasm_bindgen]
pub struct Wilson {
    show: Show,
    archive: Archive,
    palette: Palette,
    delay_ticks: u16,
    /// Playback speed, percent (scales [`Wilson::delay_ms`]); `100` = original.
    speed: u32,
    /// Story mode: synthesize the clock from elapsed time (see [`story_clock`]).
    story: bool,
    story_secs: u32,
    /// Wall-clock seconds at construction (story mode measures elapsed time from here).
    start_secs: f64,
    /// WAV bytes per sound-effect id (the originals live in `SCRANTIC.EXE`, not `RESOURCE.*`):
    /// baked in for an `embed-data` build, or supplied via the zip / [`Wilson::set_sound_data`].
    /// Empty ⇒ silent. The page plays these via the Web Audio API.
    sounds: Vec<Option<Vec<u8>>>,
    /// Sound-effect ids the last [`Wilson::frame`] fired, drained by [`Wilson::take_sounds`].
    last_sounds: Vec<u16>,
}

#[wasm_bindgen]
impl Wilson {
    /// Build from the user's `RESOURCE.MAP` + `RESOURCE.001` bytes. `now_secs` is the wall clock
    /// as Unix seconds (`Date.now() / 1000`). Add sounds with [`Wilson::set_sound_data`].
    pub fn create(
        map: &[u8],
        data: &[u8],
        now_secs: f64,
        opts: &Options,
    ) -> Result<Wilson, JsValue> {
        Wilson::build(map, data, None, now_secs, opts)
    }

    /// Build from a `scrantic-run.zip` / `scrantic-installer.zip` (or any zip holding
    /// `RESOURCE.MAP` + `RESOURCE.001`/`RESOURCE.00$`, and optionally `SCRANTIC.EXE`/`.SCR`/`.SC$`
    /// for sound). The installer's DCL-compressed members are decompressed on the fly.
    pub fn from_zip(zip_bytes: &[u8], now_secs: f64, opts: &Options) -> Result<Wilson, JsValue> {
        let (map, data, exe) = resolve_zip(zip_bytes).map_err(|e| JsValue::from_str(&e))?;
        Wilson::build(&map, &data, exe.as_deref(), now_secs, opts)
    }

    /// Build from the data baked into the wasm at compile time (the `embed-data` feature) — a
    /// self-contained page, no file picker. Only present in an `embed-data` build (the page
    /// calls it when [`has_embedded_data`] is true).
    #[cfg(feature = "embed-data")]
    pub fn embedded(now_secs: f64, opts: &Options) -> Result<Wilson, JsValue> {
        let mut wilson = Wilson::build(embedded::MAP, embedded::DATA, None, now_secs, opts)?;
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
        let real = clock::from_unix(now_secs as u64);
        let cl = if self.story {
            let elapsed = (now_secs - self.start_secs).max(0.0) as u64;
            story_clock(real, elapsed, self.story_secs)
        } else {
            real
        };
        self.show.set_clock(cl);
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

    /// Whether any sound effect is loaded (so the page knows whether to show the mute toggle).
    pub fn has_sound(&self) -> bool {
        self.sounds.iter().any(Option::is_some)
    }

    /// Load the sound effects from a `SCRANTIC.EXE`/`.SCR` the user supplied (bring-your-own:
    /// the WAVs are embedded in that binary, not in `RESOURCE.*`). Returns how many loaded.
    pub fn set_sound_data(&mut self, exe: &[u8]) -> usize {
        self.sounds = wilson_dgds::sounds_from_scrantic_exe(exe);
        self.sounds.iter().filter(|o| o.is_some()).count()
    }

    /// How long to display the last [`Wilson::frame`], in milliseconds (for the JS timer),
    /// scaled by the playback speed (higher speed ⇒ shorter delay).
    pub fn delay_ms(&self) -> u32 {
        let base = u32::from(self.delay_ticks) * wilson_engine::MS_PER_TICK as u32;
        (base * 100 / self.speed).max(1)
    }

    /// Frame width in pixels (640).
    pub fn width(&self) -> u32 {
        u32::from(WIDTH)
    }

    /// Frame height in pixels (480).
    pub fn height(&self) -> u32 {
        u32::from(HEIGHT)
    }
}

// Private helpers (a separate, non-`#[wasm_bindgen]` impl so they are not exported to JS).
impl Wilson {
    /// Parse `map`+`data`, apply [`Options`], and start a [`Show`] — shared by all constructors.
    /// `exe` (if any) is the `SCRANTIC.EXE`/`.SCR` to extract sound effects from.
    fn build(
        map: &[u8],
        data: &[u8],
        exe: Option<&[u8]>,
        now_secs: f64,
        opts: &Options,
    ) -> Result<Wilson, JsValue> {
        let archive = Archive::parse(map, data)
            .map_err(|e| JsValue::from_str(&format!("failed to parse the game data: {e}")))?;
        let palette = archive
            .palette()
            .cloned()
            .ok_or_else(|| JsValue::from_str("the data has no palette (PAL) resource"))?;

        let real = clock::from_unix(now_secs as u64);
        let story_secs = opts.story_day_secs();
        // Story mode starts at day 1 / clock-day 0; otherwise start at the requested day
        // (or day 1) and let the real `yday` advance the arc — like the desktop.
        let (director, cl) = if opts.story {
            (Director::new(1, 0), story_clock(real, 0, story_secs))
        } else {
            let day = if opts.day == 0 {
                1
            } else {
                opts.day.clamp(1, 11)
            };
            (Director::new(day, real.yday), real)
        };
        let mode = if opts.real_daynight {
            DayNight::Real24h
        } else {
            DayNight::Original
        };
        let director = director.with_daynight(mode);

        let mut show = Show::new(
            &archive,
            &palette,
            WIDTH,
            HEIGHT,
            director,
            cl,
            opts.seed as u64,
        );
        if opts.dissolve {
            show.enable_dissolve();
        }
        if opts.intro {
            show.enable_intro(&archive);
        }

        let sounds = match exe {
            Some(bytes) => wilson_dgds::sounds_from_scrantic_exe(bytes),
            None => Vec::new(),
        };
        Ok(Wilson {
            show,
            archive,
            palette,
            delay_ticks: 1,
            speed: opts.speed_pct(),
            story: opts.story,
            story_secs,
            start_secs: now_secs,
            sounds,
            last_sounds: Vec::new(),
        })
    }
}

/// Synthesize the engine clock for **story mode** from elapsed runtime (a port of the desktop
/// `timectl::story_clock`): each story day lasts `day_secs` real seconds, the day index steps
/// 0,1,2,… (the director advances the arc at run boundaries), and the hour sweeps 0→23 within
/// a day so the day/night cycle is visible. Holidays keep the real month/day.
fn story_clock(real: Clock, elapsed_secs: u64, day_secs: u32) -> Clock {
    let day_secs = u64::from(day_secs.max(1));
    let day_index = (elapsed_secs / day_secs) as i32;
    let within = (elapsed_secs % day_secs) as f64 / day_secs as f64; // 0.0..1.0 through the day
    let hour = (within * 24.0) as u8; // 0..=23 (within < 1.0 ⇒ never 24)
    Clock {
        yday: day_index,
        hour: hour.min(23),
        month: real.month,
        day: real.day,
    }
}

/// `(RESOURCE.MAP, data file, optional SCRANTIC binary for sound)` pulled from a zip.
type ZipData = (Vec<u8>, Vec<u8>, Option<Vec<u8>>);

/// Pull `RESOURCE.MAP` + the data file (+ an optional `SCRANTIC` binary for sound) out of a zip,
/// in memory. Handles both the `scrantic-run.zip` (loose `RESOURCE.001`/`SCRANTIC.EXE`) and the
/// `scrantic-installer.zip` (DCL-compressed `RESOURCE.00$` / `SCRANTIC.SC$`). Returns
/// `(map, data, exe_opt)` or a human-readable error.
fn resolve_zip(bytes: &[u8]) -> Result<ZipData, String> {
    use std::io::Read;
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|e| format!("not a readable zip: {e}"))?;
    // Read every entry into memory, keyed by UPPERCASE basename (zip paths use '/').
    let mut files: std::collections::HashMap<String, Vec<u8>> = std::collections::HashMap::new();
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).map_err(|e| format!("zip entry {i}: {e}"))?;
        if !f.is_file() {
            continue;
        }
        let name = f
            .name()
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or("")
            .to_ascii_uppercase();
        if name.is_empty() {
            continue;
        }
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)
            .map_err(|e| format!("reading {name} from the zip: {e}"))?;
        files.insert(name, buf);
    }
    let get = |n: &str| files.get(n).cloned();

    let map = get("RESOURCE.MAP").ok_or("RESOURCE.MAP not found in the zip")?;
    let data = match get("RESOURCE.001") {
        Some(d) if d.len() > 1000 => d,
        _ => {
            let comp = get("RESOURCE.00$")
                .ok_or("the zip has no RESOURCE.001 and no RESOURCE.00$ (installer) file")?;
            wilson_dgds::decompress_installer(&comp)
                .ok_or("RESOURCE.00$ is present but could not be decompressed")?
        }
    };
    // Sound source (optional): SCRANTIC.EXE/.SCR, or the installer's compressed SCRANTIC.SC$.
    let exe = get("SCRANTIC.EXE")
        .or_else(|| get("SCRANTIC.SCR"))
        .or_else(|| get("SCRANTIC.SC$").and_then(|c| wilson_dgds::decompress_installer(&c)));
    Ok((map, data, exe))
}
