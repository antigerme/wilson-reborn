// SPDX-License-Identifier: GPL-3.0-or-later
//! Palette (`.PAL`) resources.
//!
//! Layout (per `repos/jc_reborn/resource.c`):
//! - `PAL:` tag, `u16` size, 2 unknown bytes
//! - `VGA:` tag, 4 bytes (size), then 256 RGB triples of 6-bit VGA values (0..=63)
//!
//! Johnny Castaway only uses the first 16 colours, but all 256 are parsed. 6-bit values
//! are scaled to 8-bit by shifting left by 2 (matching the reference engines).

use crate::error::Result;
use crate::reader::Reader;

/// Number of colours in a DGDS palette.
pub const PALETTE_LEN: usize = 256;

/// A 256-entry RGB palette (already scaled to 8 bits per channel).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Palette {
    /// RGB triples, 8 bits per channel.
    pub colors: [[u8; 3]; PALETTE_LEN],
}

impl Palette {
    /// Parse a `.PAL` resource body.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::new(bytes);
        r.expect_tag("PAL resource", b"PAL:")?;
        let _size = r.u16()?;
        let _unknown1 = r.u8()?;
        let _unknown2 = r.u8()?;
        r.expect_tag("PAL resource", b"VGA:")?;
        r.skip(4)?;

        let mut colors = [[0u8; 3]; PALETTE_LEN];
        for color in colors.iter_mut() {
            let red = r.u8()? & 0x3F;
            let green = r.u8()? & 0x3F;
            let blue = r.u8()? & 0x3F;
            *color = [red << 2, green << 2, blue << 2];
        }
        Ok(Palette { colors })
    }

    /// The RGB triple for palette index `i`.
    pub fn rgb(&self, i: usize) -> [u8; 3] {
        self.colors[i]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_pal() -> Vec<u8> {
        let mut p = Vec::new();
        p.extend_from_slice(b"PAL:");
        p.extend_from_slice(&0u16.to_le_bytes()); // size (u16)
        p.push(0); // unknown1
        p.push(0); // unknown2
        p.extend_from_slice(b"VGA:");
        p.extend_from_slice(&768u32.to_le_bytes()); // VGA size (4 bytes)
                                                    // colour 0 = (63,0,32); colour 1 = (0,63,0); rest zero.
        p.extend_from_slice(&[63, 0, 32]);
        p.extend_from_slice(&[0, 63, 0]);
        for _ in 2..PALETTE_LEN {
            p.extend_from_slice(&[0, 0, 0]);
        }
        p
    }

    #[test]
    fn parse_palette() {
        let pal = Palette::parse(&build_pal()).unwrap();
        assert_eq!(pal.rgb(0), [63 << 2, 0, 32 << 2]); // [252, 0, 128]
        assert_eq!(pal.rgb(1), [0, 252, 0]);
        assert_eq!(pal.rgb(255), [0, 0, 0]);
    }

    #[test]
    fn rejects_wrong_tag() {
        let mut bytes = b"XXX:".to_vec();
        bytes.extend_from_slice(&[0u8; 8]);
        assert!(Palette::parse(&bytes).is_err());
    }

    #[test]
    fn rejects_truncated() {
        let bytes = b"PAL:".to_vec();
        assert!(Palette::parse(&bytes).is_err());
    }
}
