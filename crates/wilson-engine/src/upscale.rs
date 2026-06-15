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

/// "De-dither": collapse ordered-dither checkerboards to their intended blended tone,
/// leaving everything else (flat areas, real edges, detail) untouched. Returns a new
/// `w×h` RGBA buffer.
///
/// The 1992 art fakes extra colours by alternating two *high-contrast* shades in a
/// checkerboard (most visible on the sea/sky). A pixel is detected as part of such a
/// checkerboard when its four **diagonal** neighbours match it and its four
/// **orthogonal** neighbours are a single, *different* colour; it is then replaced by the
/// average of the two shades — so the dither melts into a smooth gradient while sprites
/// and outlines (which don't have that exact alternating signature) stay crisp.
///
/// A plain blur would smooth the dither too, but also soften every sprite; this targets
/// only the checkerboard pattern. Optional (off by default): the dithering is the
/// authentic look, this just offers a smoother sea.
pub fn dedither(src: &[u8], w: usize, h: usize) -> Vec<u8> {
    let mut out = src.to_vec(); // unchanged unless a pixel is a detected checkerboard
    if w == 0 || h == 0 {
        return out;
    }
    let idx = |x: i32, y: i32| -> usize {
        let xc = x.clamp(0, w as i32 - 1) as usize;
        let yc = y.clamp(0, h as i32 - 1) as usize;
        (yc * w + xc) * 4
    };
    let same = |i: usize, j: usize| dist(&src[i..], &src[j..]) <= TOLERANCE;
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let c = idx(x, y);
            let (up, dn, lf, rt) = (idx(x, y - 1), idx(x, y + 1), idx(x - 1, y), idx(x + 1, y));
            let (ul, ur, dl, dr) = (
                idx(x - 1, y - 1),
                idx(x + 1, y - 1),
                idx(x - 1, y + 1),
                idx(x + 1, y + 1),
            );
            // Checkerboard signature: diagonals match the centre; orthogonals are one
            // uniform, different colour.
            let diagonals_match = same(c, ul) && same(c, ur) && same(c, dl) && same(c, dr);
            let orthogonals_uniform = same(up, dn) && same(up, lf) && same(up, rt);
            if diagonals_match && orthogonals_uniform && !same(c, up) {
                out[c] = ((u16::from(src[c]) + u16::from(src[up])) / 2) as u8;
                out[c + 1] = ((u16::from(src[c + 1]) + u16::from(src[up + 1])) / 2) as u8;
                out[c + 2] = ((u16::from(src[c + 2]) + u16::from(src[up + 2])) / 2) as u8;
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

    #[test]
    fn dedither_leaves_flat_areas_untouched() {
        let mut src = Vec::new();
        for _ in 0..16 {
            src.extend_from_slice(&[40, 80, 120, 255]);
        }
        let out = dedither(&src, 4, 4);
        assert!(out.chunks_exact(4).all(|p| p == [40, 80, 120, 255]));
    }

    #[test]
    fn dedither_smooths_a_dither_checkerboard() {
        // Two high-contrast shades alternating (as the real art dithers) → every interior
        // pixel collapses to their blend, so the checkerboard becomes a flat mid-tone.
        let a = [0u8, 0, 255, 255];
        let b = [120u8, 160, 255, 255];
        let mut src = Vec::new();
        for y in 0..6 {
            for x in 0..6 {
                src.extend_from_slice(if (x + y) % 2 == 0 { &a } else { &b });
            }
        }
        let out = dedither(&src, 6, 6);
        let i = (3 * 6 + 3) * 4;
        assert_eq!(
            &out[i..i + 3],
            &[60, 80, 255],
            "checkerboard should collapse to the blended mid-tone"
        );
    }

    #[test]
    fn dedither_preserves_a_hard_edge() {
        // Black | white halves: across the boundary the far colour is excluded, so pixels
        // stay pure black or white (no grey bleed).
        let blk = [0u8, 0, 0, 255];
        let wht = [255u8, 255, 255, 255];
        let mut src = Vec::new();
        for _y in 0..4 {
            for x in 0..4 {
                src.extend_from_slice(if x < 2 { &blk } else { &wht });
            }
        }
        let out = dedither(&src, 4, 4);
        assert!(
            out.chunks_exact(4).all(|p| p == blk || p == wht),
            "a hard edge must stay hard (no grey bleed)"
        );
    }
}
