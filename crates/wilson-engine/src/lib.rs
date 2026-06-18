// SPDX-License-Identifier: GPL-3.0-or-later
#![forbid(unsafe_code)]

//! `wilson-engine` — the SCRANTIC runtime for **Wilson Reborn**.
//!
//! This crate executes the decoded TTM/ADS scripts from [`wilson_dgds`] against an
//! abstract, indexed-color [`Surface`]. It is fully headless (no window or GPU), so
//! the animation logic can be unit-tested deterministically; a real rendering backend
//! later turns [`Surface`] pixels into on-screen frames via a palette.

/// Wall-clock duration of one engine tick, in milliseconds.
///
/// The original screensaver's animation clock fires every **16 ms** (~62.5 Hz): its
/// scheduler derives a 4 ms master unit (`1000 / (13 × 18)`, from the ~18.2 Hz PC timer
/// constant) and the animation callback runs every 4th one (`4 × 4 = 16 ms`), gated against
/// the real `GetCurrentTime`. Verified by disassembly of the original `SCRANTIC.EXE` (`seg9`);
/// see [`docs/knowledge-base/10`](https://github.com/antigerme/wilson-reborn/blob/main/docs/knowledge-base/10-engenharia-reversa-do-original.md).
/// A TTM `wait N` delay is therefore `N × 16 ms`. (jc_reborn approximates this as 20 ms; we
/// use the original's measured 16 ms.)
pub const MS_PER_TICK: u64 = 16;

pub mod ads_vm;
pub mod clock;
pub mod error;
pub mod island;
pub mod path;
pub mod rng;
pub mod show;
pub mod story;
pub mod surface;
pub mod ttm_exec;
pub mod ttm_vm;
pub mod upscale;
pub mod walk;
pub mod walk_data;
pub mod xbrz;

pub use ads_vm::{AdsFrame, AdsVm, MAX_TTM_SLOTS, MAX_TTM_THREADS};
pub use error::{EngineError, Result};
pub use island::Island;
pub use path::{calc_path, calc_paths, NUM_OF_NODES};
pub use rng::Rng;
pub use show::{Clock, DebugInfo, Frame, Show};
pub use story::{
    DayNight, Director, Holiday, IslandState, ScenePlay, StoryRun, StoryScene, STORY_SCENES,
};
pub use surface::{Rect, Surface, TRANSPARENT};
pub use ttm_exec::{TtmSlot, TtmThread, MAX_BMP_SLOTS};
pub use ttm_vm::{TtmStep, TtmVm};
pub use upscale::{dedither, xbr2x};
pub use walk::{WalkFrame, Walker};
pub use xbrz::xbrz2x;
