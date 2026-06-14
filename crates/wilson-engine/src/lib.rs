// SPDX-License-Identifier: GPL-3.0-or-later
#![forbid(unsafe_code)]

//! `wilson-engine` — the SCRANTIC runtime for **Wilson Reborn**.
//!
//! This crate executes the decoded TTM/ADS scripts from [`wilson_dgds`] against an
//! abstract, indexed-color [`Surface`]. It is fully headless (no window or GPU), so
//! the animation logic can be unit-tested deterministically; a real rendering backend
//! later turns [`Surface`] pixels into on-screen frames via a palette.

pub mod ads_vm;
pub mod error;
pub mod surface;
pub mod ttm_exec;
pub mod ttm_vm;

pub use ads_vm::{AdsFrame, AdsVm, MAX_TTM_SLOTS, MAX_TTM_THREADS};
pub use error::{EngineError, Result};
pub use surface::{Rect, Surface, TRANSPARENT};
pub use ttm_exec::{TtmSlot, TtmThread, MAX_BMP_SLOTS};
pub use ttm_vm::{TtmStep, TtmVm};
