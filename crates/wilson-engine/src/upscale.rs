// SPDX-License-Identifier: GPL-3.0-or-later
//! xBR-style **edge-directed 2× upscaler** for the indexed art once it's been turned
//! into RGBA.
//!
//! It is the Scale2x/EPX family with two additions that make it smooth *and* sharp:
//! pixels are compared by **colour distance with a tolerance** (so it works on the
//! game's near-colour gradients, not only exact matches), and detected diagonal edges
//! are **blended** (anti-aliased) instead of hard-stepped. Flat regions and dithered
//! fields (where opposite neighbours are similar) are left untouched, so the original
//! look is preserved while jagged sprite/edge staircases are rounded off.
//!
//! This is the runtime cousin of ffmpeg's `xbr` filter (which we use only to eyeball the
//! result offline); it is a faithful *style*, not a byte-exact port of Hyllian's xBR.

/// Squared, luma-weighted RGB distance between two pixels (slices starting at an RGBA
/// pixel; alpha is ignored). Green is weighted highest, matching human luma sensitivity.
#[inline]
fn dist(a: &[u8], b: &[u8]) -> i32 {
    let dr = i32::from(a[0]) - i32::from(b[0]);
    let dg = i32::from(a[1]) - i32::from(b[1]);
    let db = i32::from(a[2]) - i32::from(b[2]);
    2 * dr * dr + 4 * dg * dg + 3 * db * db
}

/// Two pixels count as "the same colour" when within this squared-distance tolerance.
/// ~`2·dr²+4·dg²+3·db²`; chosen so close shades merge but distinct colours (sprite vs
/// background) stay separate. Tuned by eye on the original Johnny Castaway frames.
const TOLERANCE: i32 = 12_000;

/// Upscale an RGBA image by 2× using the edge-directed rule. Returns the `2w × 2h` RGBA
/// buffer. `src` must be `w*h*4` bytes.
pub fn xbr2x(src: &[u8], w: usize, h: usize) -> Vec<u8> {
    let (dw, dh) = (w * 2, h * 2);
    let mut out = vec![0u8; dw * dh * 4];
    if w == 0 || h == 0 {
        return out;
    }
    let idx = |x: i32, y: i32| -> usize {
        let xc = x.clamp(0, w as i32 - 1) as usize;
        let yc = y.clamp(0, h as i32 - 1) as usize;
        (yc * w + xc) * 4
    };

    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let e = idx(x, y);
            let b = idx(x, y - 1); // up
            let d = idx(x - 1, y); // left
            let f = idx(x + 1, y); // right
            let hh = idx(x, y + 1); // down

            // The four 2×2 output texels for this source pixel.
            let o_tl = ((2 * y as usize) * dw + 2 * x as usize) * 4;
            let o_tr = o_tl + 4;
            let o_bl = o_tl + dw * 4;
            let o_br = o_bl + 4;

            let same = |i: usize, j: usize| dist(&src[i..], &src[j..]) <= TOLERANCE;

            // Only round corners at a genuine edge: opposite neighbours must differ
            // (otherwise we're inside a flat area or an even dither field — leave it).
            let is_edge = !same(b, hh) && !same(d, f);
            let corners = [
                (o_tl, is_edge && same(d, b), d),  // top-left  ← left/up diagonal
                (o_tr, is_edge && same(b, f), f),  // top-right ← up/right diagonal
                (o_bl, is_edge && same(d, hh), d), // bottom-left  ← left/down diagonal
                (o_br, is_edge && same(hh, f), f), // bottom-right ← down/right diagonal
            ];
            for (o, blend, n) in corners {
                if blend {
                    // 50/50 blend of the centre with the matching edge colour: softens
                    // the staircase into an anti-aliased diagonal.
                    out[o] = ((u16::from(src[e]) + u16::from(src[n])) / 2) as u8;
                    out[o + 1] = ((u16::from(src[e + 1]) + u16::from(src[n + 1])) / 2) as u8;
                    out[o + 2] = ((u16::from(src[e + 2]) + u16::from(src[n + 2])) / 2) as u8;
                    out[o + 3] = 255;
                } else {
                    out[o..o + 4].copy_from_slice(&src[e..e + 4]);
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doubles_the_dimensions() {
        let src = vec![0u8; 3 * 2 * 4];
        let out = xbr2x(&src, 3, 2);
        assert_eq!(out.len(), 6 * 4 * 4);
    }

    #[test]
    fn flat_image_is_unchanged() {
        // A uniform field has no edges, so every output texel is the source colour.
        let mut src = Vec::new();
        for _ in 0..(4 * 4) {
            src.extend_from_slice(&[40, 80, 120, 255]);
        }
        let out = xbr2x(&src, 4, 4);
        assert!(out.chunks_exact(4).all(|p| p == [40, 80, 120, 255]));
    }

    #[test]
    fn dither_field_is_preserved_not_blurred() {
        // A 2-colour checkerboard (like the water dither): opposite neighbours are equal,
        // so `is_edge` is false everywhere → no blending, the texture survives.
        let a = [10u8, 10, 200, 255];
        let c = [10u8, 10, 230, 255]; // a close shade (within tolerance)
        let mut src = Vec::new();
        for y in 0..4 {
            for x in 0..4 {
                src.extend_from_slice(if (x + y) % 2 == 0 { &a } else { &c });
            }
        }
        let out = xbr2x(&src, 4, 4);
        // Every output texel is one of the two original shades (no blended midtone).
        assert!(out.chunks_exact(4).all(|p| p == a || p == c));
    }

    #[test]
    fn diagonal_edge_gets_blended() {
        // Black above a diagonal, white below — the staircase corner should be blended to
        // a midtone (grey) rather than a hard black/white step.
        let blk = [0u8, 0, 0, 255];
        let wht = [255u8, 255, 255, 255];
        // 3x3 with a clear corner at the centre: top row black, bottom rows white.
        let rows = [[blk, blk, blk], [wht, blk, blk], [wht, wht, blk]];
        let mut src = Vec::new();
        for r in rows {
            for c in r {
                src.extend_from_slice(&c);
            }
        }
        let out = xbr2x(&src, 3, 3);
        // Somewhere a grey (blended) texel must appear — proof of anti-aliasing.
        let grey = out
            .chunks_exact(4)
            .any(|p| (1..=254).contains(&p[0]) && p[0] == p[1] && p[1] == p[2]);
        assert!(grey, "expected a blended (grey) texel on the diagonal");
    }
}
