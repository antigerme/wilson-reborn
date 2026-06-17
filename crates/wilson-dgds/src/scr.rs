// SPDX-License-Identifier: GPL-3.0-or-later
//! Full-screen image (`.SCR`) resources.
//!
//! Layout (per `repos/jc_reborn/resource.c`):
//! - `SCR:` tag, `u16` total size, `u16` flags
//! - `DIM:` tag, `u32` size, `u16` width, `u16` height
//! - `BIN:` tag, packed block of 4-bpp pixels (`width * height` indices)

use crate::chunk::read_packed_block;
use crate::error::Result;
use crate::pixels::decode_4bpp;
use crate::reader::Reader;

/// A decoded full-screen image: `width * height` palette indices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scr {
    /// Image width in pixels.
    pub width: u16,
    /// Image height in pixels.
    pub height: u16,
    /// Palette indices (row-major, one byte per pixel, values `0..=15`).
    pub pixels: Vec<u8>,
}

impl Scr {
    /// Parse a `.SCR` resource body.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::new(bytes);
        r.expect_tag("SCR resource", b"SCR:")?;
        let _total_size = r.u16()?;
        let _flags = r.u16()?;

        r.expect_tag("SCR resource", b"DIM:")?;
        let _dim_size = r.u32()?;
        let width = r.u16()?;
        let height = r.u16()?;

        r.expect_tag("SCR resource", b"BIN:")?;
        let data = read_packed_block(&mut r)?;

        let num_pixels = width as usize * height as usize;
        let pixels = decode_4bpp(&data, num_pixels);
        Ok(Scr {
            width,
            height,
            pixels,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packed_none(body: &[u8]) -> Vec<u8> {
        // total = body + 5 (method + unpackSize); method 0 = stored.
        let mut out = Vec::new();
        out.extend_from_slice(&((body.len() + 5) as u32).to_le_bytes());
        out.push(0);
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.extend_from_slice(body);
        out
    }

    #[test]
    fn parse_scr() {
        // 2x2 image: bytes 0x12, 0x34 -> pixels [1,2,3,4]
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"SCR:");
        bytes.extend_from_slice(&0u16.to_le_bytes()); // total size
        bytes.extend_from_slice(&0u16.to_le_bytes()); // flags
        bytes.extend_from_slice(b"DIM:");
        bytes.extend_from_slice(&4u32.to_le_bytes()); // dim size
        bytes.extend_from_slice(&2u16.to_le_bytes()); // width
        bytes.extend_from_slice(&2u16.to_le_bytes()); // height
        bytes.extend_from_slice(b"BIN:");
        bytes.extend_from_slice(&packed_none(&[0x12, 0x34]));

        let scr = Scr::parse(&bytes).unwrap();
        assert_eq!((scr.width, scr.height), (2, 2));
        assert_eq!(scr.pixels, vec![1, 2, 3, 4]);
    }

    #[test]
    fn rejects_wrong_tag() {
        let mut bytes = b"XXX:".to_vec();
        bytes.extend_from_slice(&[0u8; 8]);
        assert!(Scr::parse(&bytes).is_err());
    }
}
