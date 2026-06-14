// SPDX-License-Identifier: GPL-3.0-or-later
//! 4-bpp pixel decoding shared by `.BMP` and `.SCR` resources.

/// Decode `num_pixels` 4-bpp palette indices from `packed`.
///
/// Each byte holds two pixels: the high nibble first, then the low nibble
/// (matching the reference engine's `grLoadBmp`/`grLoadScreen`). Decoding stops
/// once `num_pixels` indices have been produced or `packed` is exhausted.
pub fn decode_4bpp(packed: &[u8], num_pixels: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(num_pixels);
    for &byte in packed {
        if out.len() >= num_pixels {
            break;
        }
        out.push((byte & 0xF0) >> 4);
        if out.len() >= num_pixels {
            break;
        }
        out.push(byte & 0x0F);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_high_then_low_nibble() {
        // 0x1F -> [1, 15]; 0xA0 -> [10, 0]
        assert_eq!(decode_4bpp(&[0x1F, 0xA0], 4), vec![1, 15, 10, 0]);
    }

    #[test]
    fn stops_at_num_pixels() {
        assert_eq!(decode_4bpp(&[0x12, 0x34], 3), vec![1, 2, 3]);
    }

    #[test]
    fn tolerates_short_input() {
        assert_eq!(decode_4bpp(&[0x12], 4), vec![1, 2]);
    }
}
