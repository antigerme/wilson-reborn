// SPDX-License-Identifier: GPL-3.0-or-later
#![forbid(unsafe_code)]

//! `wilson-dgds` — reader and decoder for the Dynamix **DGDS / SCRANTIC** resource
//! format used by the 1992 *Johnny Castaway* screensaver.
//!
//! This crate is the data layer (Phase 0) of **Wilson Reborn**. It parses the original
//! `RESOURCE.MAP` index and `RESOURCE.001` archive, decompresses packed chunks
//! (RLE / LZW), and exposes typed resources (palette, …).
//!
//! The algorithms are faithful ports validated against the open-source reference
//! implementations (notably `repos/jc_reborn`). See `docs/knowledge-base/` for the
//! full format specification.
//!
//! No original game data is bundled: callers supply their own `RESOURCE.*` files.

pub mod ads;
pub mod archive;
pub mod bmp;
pub mod chunk;
pub mod decompress;
pub mod error;
pub mod pal;
pub mod pixels;
pub mod reader;
pub mod resource;
pub mod scr;
pub mod ttm;

pub use ads::{ads_opcode_info, decode_ads, Ads, AdsInstruction, AdsRes};
pub use archive::Archive;
pub use bmp::{Bmp, BmpImage};
pub use error::{DgdsError, Result};
pub use pal::Palette;
pub use pixels::decode_4bpp;
pub use reader::Reader;
pub use resource::{
    read_entry_header, resource_extension, ResourceEntry, ResourceMap, ResourceMapEntry, Tag,
};
pub use scr::Scr;
pub use ttm::{decode_ttm, ttm_opcode_name, Ttm, TtmArgs, TtmInstruction};
