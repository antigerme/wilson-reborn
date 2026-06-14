// SPDX-License-Identifier: GPL-3.0-or-later
#![forbid(unsafe_code)]

//! `wilson-engine` — the SCRANTIC runtime for **Wilson Reborn**.
//!
//! This crate executes the decoded TTM/ADS scripts from [`wilson_dgds`] against an
//! abstract, indexed-color [`Surface`]. It is fully headless (no window or GPU), so
//! the animation logic can be unit-tested deterministically; a real rendering backend
//! later turns [`Surface`] pixels into on-screen frames via a palette.

pub mod error;
pub mod surface;
pub mod ttm_vm;

pub use error::{EngineError, Result};
pub use surface::{Rect, Surface, TRANSPARENT};
pub use ttm_vm::{TtmStep, TtmVm, MAX_BMP_SLOTS};
