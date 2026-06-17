// SPDX-License-Identifier: GPL-3.0-or-later
//! Sprite-sheet (`.BMP`) resources: a set of 4-bpp images.
//!
//! Layout (per `repos/jc_reborn/resource.c`):
//! - `BMP:` tag, `u16` width, `u16` height (sheet bounds)
//! - `INF:` tag, `u32` size, `u16` image count, then that many `u16` widths and
//!   `u16` heights
//! - `BIN:` tag, packed block holding every image's 4-bpp pixels concatenated
//!
//! Images are decoded sequentially from the single pixel stream; each consumes
//! `width * height` indices (`width` is always even in the original data).

use crate::chunk::read_packed_block;
use crate::error::{DgdsError, Result};
use crate::pixels::decode_4bpp;
use crate::reader::Reader;

/// One decoded image (sprite frame) from a `.BMP` sheet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BmpImage {
    /// Image width in pixels.
    pub width: u16,
    /// Image height in pixels.
    pub height: u16,
    /// Palette indices (row-major, one byte per pixel, values `0..=15`).
    pub pixels: Vec<u8>,
}

/// A parsed `.BMP` sprite sheet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bmp {
    /// Nominal sheet width.
    pub width: u16,
    /// Nominal sheet height.
    pub height: u16,
    /// The individual images, in order.
    pub images: Vec<BmpImage>,
}

impl Bmp {
    /// Parse a `.BMP` resource body.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::new(bytes);
        r.expect_tag("BMP resource", b"BMP:")?;
        let width = r.u16()?;
        let height = r.u16()?;

        r.expect_tag("BMP resource", b"INF:")?;
        let _data_size = r.u32()?;
        let num_images = r.u16()? as usize;
        let mut widths = Vec::with_capacity(num_images);
        for _ in 0..num_images {
            widths.push(r.u16()?);
        }
        let mut heights = Vec::with_capacity(num_images);
        for _ in 0..num_images {
            heights.push(r.u16()?);
        }

        r.expect_tag("BMP resource", b"BIN:")?;
        let data = read_packed_block(&mut r)?;

        let mut images = Vec::with_capacity(num_images);
        let mut offset = 0usize;
        for i in 0..num_images {
            let w = widths[i] as usize;
            let h = heights[i] as usize;
            let num_pixels = w * h;
            let byte_len = num_pixels / 2; // 2 pixels per byte (width is even)
            let slice = data.get(offset..offset + byte_len).ok_or_else(|| {
                DgdsError::eof("bmp: image pixels", offset + byte_len, data.len())
            })?;
            images.push(BmpImage {
                width: widths[i],
                height: heights[i],
                pixels: decode_4bpp(slice, num_pixels),
            });
            offset += byte_len;
        }

        Ok(Bmp {
            width,
            height,
            images,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packed_none(body: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&((body.len() + 5) as u32).to_le_bytes());
        out.push(0);
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.extend_from_slice(body);
        out
    }

    #[test]
    fn parse_two_images() {
        // image 0: 2x2 (2 bytes) -> [1,2,3,4]; image 1: 2x1 (1 byte) -> [10,11]
        let pixels = [0x12u8, 0x34, 0xAB];
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"BMP:");
        bytes.extend_from_slice(&2u16.to_le_bytes()); // sheet width
        bytes.extend_from_slice(&3u16.to_le_bytes()); // sheet height
        bytes.extend_from_slice(b"INF:");
        bytes.extend_from_slice(&0u32.to_le_bytes()); // INF size
        bytes.extend_from_slice(&2u16.to_le_bytes()); // num images
        bytes.extend_from_slice(&2u16.to_le_bytes()); // widths[0]
        bytes.extend_from_slice(&2u16.to_le_bytes()); // widths[1]
        bytes.extend_from_slice(&2u16.to_le_bytes()); // heights[0]
        bytes.extend_from_slice(&1u16.to_le_bytes()); // heights[1]
        bytes.extend_from_slice(b"BIN:");
        bytes.extend_from_slice(&packed_none(&pixels));

        let bmp = Bmp::parse(&bytes).unwrap();
        assert_eq!(bmp.images.len(), 2);
        assert_eq!(bmp.images[0].pixels, vec![1, 2, 3, 4]);
        assert_eq!((bmp.images[1].width, bmp.images[1].height), (2, 1));
        assert_eq!(bmp.images[1].pixels, vec![10, 11]);
    }

    #[test]
    fn rejects_truncated_pixels() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"BMP:");
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(b"INF:");
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes()); // 1 image
        bytes.extend_from_slice(&4u16.to_le_bytes()); // width 4
        bytes.extend_from_slice(&4u16.to_le_bytes()); // height 4 -> needs 8 bytes
        bytes.extend_from_slice(b"BIN:");
        bytes.extend_from_slice(&packed_none(&[0x12, 0x34])); // only 2 bytes
        assert!(Bmp::parse(&bytes).is_err());
    }
}
