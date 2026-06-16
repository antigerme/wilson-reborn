// SPDX-License-Identifier: GPL-3.0-or-later
//! **xBR (Hyllian)** edge-directed 2× upscaler for the indexed art once it's been turned
//! into RGBA, plus an optional dither smoother.
//!
//! [`xbr2x`] is a faithful CPU port of the real **xBR level-2** algorithm from ffmpeg's
//! `libavfilter/vf_xbr.c` (Hyllian's xBR). ffmpeg's file is LGPL-2.1-or-later, which is
//! compatible with this crate's GPL-3.0-or-later. Unlike the simpler Scale2x/EPX family it
//! replaced, real xBR detects edges in **luma+chroma** space, dissolves the 1992 ordered
//! dithering into solid colour, and anti-aliases diagonal/curved edges into smooth ramps —
//! the "HD remaster" look — while leaving flat areas and straight horizontal/vertical
//! edges crisp. The app runs it once per frame (cheap at the screensaver's frame rate),
//! then does a bilinear fit into the window.
//!
//! TODO(xBRZ): once this is validated against the originals, add **xBRZ** (the refined xBR
//! fork) as an alternative `--filter` and compare it head-to-head with this (per the
//! 2026-06-16 plan with the user). xBRZ tends to give even cleaner gradients on sprites,
//! but its upstream licence needs checking — prefer a clean MIT/Apache/GPL Rust port.

/// Squared, luma-weighted RGB distance between two pixels (slices starting at an RGBA
/// pixel; alpha is ignored). Green is weighted highest, matching human luma sensitivity.
/// Used by [`dedither`] (xBR itself uses the YUV [`Px::df`] metric below).
#[inline]
fn dist(a: &[u8], b: &[u8]) -> i32 {
    let dr = i32::from(a[0]) - i32::from(b[0]);
    let dg = i32::from(a[1]) - i32::from(b[1]);
    let db = i32::from(a[2]) - i32::from(b[2]);
    2 * dr * dr + 4 * dg * dg + 3 * db * db
}

/// Two pixels count as "the same colour" for [`dedither`] when within this squared-distance
/// tolerance (`~2·dr²+4·dg²+3·db²`); chosen so close shades merge but distinct colours stay
/// separate. Tuned by eye on the original Johnny Castaway frames.
const TOLERANCE: i32 = 12_000;

/// A source pixel carried with its precomputed Y'UV (so xBR's distance metric is a few
/// integer subtractions instead of re-deriving Y'UV on every one of the ~80 comparisons a
/// pixel takes part in).
#[derive(Clone, Copy)]
struct Px {
    rgb: [u8; 3],
    yuv: [i32; 3],
}

impl Px {
    /// xBR's pixel difference: sum of absolute Y'UV component differences (ffmpeg's `df`).
    #[inline]
    fn df(self, o: Px) -> i32 {
        (self.yuv[0] - o.yuv[0]).abs()
            + (self.yuv[1] - o.yuv[1]).abs()
            + (self.yuv[2] - o.yuv[2]).abs()
    }
    /// "Close enough to be the same colour" (ffmpeg's `eq`, threshold 155 in Y'UV units).
    #[inline]
    fn eq(self, o: Px) -> bool {
        self.df(o) < 155
    }
    /// Exact-colour inequality (ffmpeg compares the packed RGB, not the Y'UV).
    #[inline]
    fn ne_rgb(self, o: Px) -> bool {
        self.rgb != o.rgb
    }
}

/// BT.601 RGB→Y'UV (integer), matching the weights ffmpeg's xBR builds its lookup from.
#[inline]
fn rgb_to_yuv(p: [u8; 3]) -> [i32; 3] {
    let (r, g, b) = (i32::from(p[0]), i32::from(p[1]), i32::from(p[2]));
    let y = (299 * r + 587 * g + 114 * b) / 1000;
    let u = (-169 * r - 331 * g + 500 * b) / 1000 + 128;
    let v = (500 * r - 419 * g - 81 * b) / 1000 + 128;
    [y, u, v]
}

/// Per-channel linear interpolation `a + (b-a)·m/2^s` — the integer form of ffmpeg's
/// `ALPHA_BLEND_*_W` macros (e.g. `64_W` = m=1,s=2 → ¼ of `b`; `192_W` = m=3,s=2 → ¾;
/// `224_W` = m=7,s=3 → ⅞; `128_W` = m=1,s=1 → ½).
#[inline]
fn blend(a: [u8; 3], b: [u8; 3], m: i32, s: u32) -> [u8; 3] {
    let mix = |av: u8, bv: u8| (i32::from(av) + (((i32::from(bv) - i32::from(av)) * m) >> s)) as u8;
    [mix(a[0], b[0]), mix(a[1], b[1]), mix(a[2], b[2])]
}

/// One quadrant of the xBR kernel — a direct port of ffmpeg's `FILT2` macro. Writes up to
/// three of the four output sub-pixels in `e` (`[TL, TR, BL, BR]`); the caller invokes it
/// four times with rotated neighbourhoods to cover all corners. `n1`/`n2`/`n3` select which
/// sub-pixels this rotation may touch.
// A faithful port of a 25-argument C macro; the neighbourhood pixels are irreducible.
#[allow(clippy::too_many_arguments)]
#[inline]
fn filt2(
    e: &mut [[u8; 3]; 4],
    pe: Px,
    pi: Px,
    ph: Px,
    pf: Px,
    pg: Px,
    pc: Px,
    pd: Px,
    pb: Px,
    f4: Px,
    i4: Px,
    h5: Px,
    i5: Px,
    n1: usize,
    n2: usize,
    n3: usize,
) {
    if !pe.ne_rgb(ph) || !pe.ne_rgb(pf) {
        return; // PE == PH or PE == PF → no edge to round here.
    }
    let e_ = pe.df(pc) + pe.df(pg) + pi.df(h5) + pi.df(f4) + (ph.df(pf) << 2);
    let i_ = ph.df(pd) + ph.df(i5) + pf.df(i4) + pf.df(pb) + (pe.df(pi) << 2);
    if e_ > i_ {
        return;
    }
    // Choose the nearer edge colour to bleed in.
    let px = if pe.df(pf) <= pe.df(ph) {
        pf.rgb
    } else {
        ph.rgb
    };
    let strong = e_ < i_
        && ((!pf.eq(pb) && !ph.eq(pd))
            || (pe.eq(pi) && !pf.eq(i4) && !ph.eq(i5))
            || pe.eq(pg)
            || pe.eq(pc));
    if !strong {
        // Weak edge: a plain 50/50 on the outer corner.
        e[n3] = blend(e[n3], px, 1, 1);
        return;
    }
    let ke = pf.df(pg);
    let ki = ph.df(pc);
    let left = ke << 1 <= ki && pe.ne_rgb(pg) && pd.ne_rgb(pg);
    let up = ke >= ki << 1 && pe.ne_rgb(pc) && pb.ne_rgb(pc);
    if left && up {
        e[n3] = blend(e[n3], px, 7, 3); // 224/256
        e[n2] = blend(e[n2], px, 1, 2); // 64/256
        e[n1] = e[n2];
    } else if left {
        e[n3] = blend(e[n3], px, 3, 2); // 192/256
        e[n2] = blend(e[n2], px, 1, 2); // 64/256
    } else if up {
        e[n3] = blend(e[n3], px, 3, 2);
        e[n1] = blend(e[n1], px, 1, 2);
    } else {
        e[n3] = blend(e[n3], px, 1, 1); // diagonal: 128/256
    }
}

/// Upscale an RGBA image by 2× with real xBR (Hyllian, level 2). Returns the `2w × 2h`
/// RGBA buffer (always fully opaque). `src` must be `w*h*4` bytes.
pub fn xbr2x(src: &[u8], w: usize, h: usize) -> Vec<u8> {
    let (dw, dh) = (w * 2, h * 2);
    let mut out = vec![0u8; dw * dh * 4];
    if w == 0 || h == 0 {
        return out;
    }
    // Precompute Y'UV once per source pixel (each is compared ~80× across the 4 rotations).
    let yuv: Vec<[i32; 3]> = src
        .chunks_exact(4)
        .map(|p| rgb_to_yuv([p[0], p[1], p[2]]))
        .collect();
    let at = |x: i32, y: i32| -> Px {
        let xc = x.clamp(0, w as i32 - 1) as usize;
        let yc = y.clamp(0, h as i32 - 1) as usize;
        let i = yc * w + xc;
        let o = i * 4;
        Px {
            rgb: [src[o], src[o + 1], src[o + 2]],
            yuv: yuv[i],
        }
    };

    for y in 0..h as i32 {
        for x in 0..w as i32 {
            // The 5×5 neighbourhood (minus far corners), named as in ffmpeg/Hyllian.
            let pe = at(x, y);
            let pb = at(x, y - 1);
            let pd = at(x - 1, y);
            let pf = at(x + 1, y);
            let ph = at(x, y + 1);
            let pa = at(x - 1, y - 1);
            let pc = at(x + 1, y - 1);
            let pg = at(x - 1, y + 1);
            let pi = at(x + 1, y + 1);
            let a0 = at(x - 2, y - 1);
            let a1 = at(x - 1, y - 2);
            let b1 = at(x, y - 2);
            let c1 = at(x + 1, y - 2);
            let c4 = at(x + 2, y - 1);
            let d0 = at(x - 2, y);
            let f4 = at(x + 2, y);
            let g0 = at(x - 2, y + 1);
            let g5 = at(x - 1, y + 2);
            let h5 = at(x, y + 2);
            let i4 = at(x + 2, y + 1);
            let i5 = at(x + 1, y + 2);

            // Four output sub-pixels [TL, TR, BL, BR], each starting as the centre colour.
            let mut e = [pe.rgb; 4];
            // Four rotations of the same rule (NW, NE, SW, SE).
            filt2(
                &mut e, pe, pi, ph, pf, pg, pc, pd, pb, f4, i4, h5, i5, 1, 2, 3,
            );
            filt2(
                &mut e, pe, pc, pf, pb, pi, pa, ph, pd, b1, c1, f4, c4, 0, 3, 1,
            );
            filt2(
                &mut e, pe, pa, pb, pd, pc, pg, pf, ph, d0, a0, b1, a1, 2, 1, 0,
            );
            filt2(
                &mut e, pe, pg, pd, ph, pa, pi, pb, pf, h5, g5, d0, g0, 3, 0, 2,
            );

            let tl = ((2 * y as usize) * dw + 2 * x as usize) * 4;
            for (o, c) in [
                (tl, e[0]),
                (tl + 4, e[1]),
                (tl + dw * 4, e[2]),
                (tl + dw * 4 + 4, e[3]),
            ] {
                out[o] = c[0];
                out[o + 1] = c[1];
                out[o + 2] = c[2];
                out[o + 3] = 255;
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
/// authentic look, this just offers a smoother sea. xBR already dissolves most dithering;
/// this remains for use with `--filter nearest`/`linear`.
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
        // A uniform field has no edges (PE == PH == PF everywhere), so every output texel
        // is the source colour — opaque.
        let mut src = Vec::new();
        for _ in 0..(4 * 4) {
            src.extend_from_slice(&[40, 80, 120, 255]);
        }
        let out = xbr2x(&src, 4, 4);
        assert!(out.chunks_exact(4).all(|p| p == [40, 80, 120, 255]));
    }

    #[test]
    fn straight_edge_stays_hard() {
        // A vertical two-colour split: on a straight H/V edge either PH or PF equals PE, so
        // the kernel never fires — no intermediate colours are invented (xBR only rounds
        // diagonals/corners, not straight edges).
        let a = [10u8, 10, 200];
        let b = [220u8, 30, 30];
        let (w, h) = (6usize, 6usize);
        let mut src = Vec::new();
        for _y in 0..h {
            for x in 0..w {
                let c = if x < w / 2 { a } else { b };
                src.extend_from_slice(&[c[0], c[1], c[2], 255]);
            }
        }
        let out = xbr2x(&src, w, h);
        assert!(
            out.chunks_exact(4).all(|p| p[0..3] == a || p[0..3] == b),
            "a straight edge must stay hard (no blended colours)"
        );
    }

    #[test]
    fn diagonal_edge_is_antialiased() {
        // Black/white diagonal split → xBR must blend the staircase into intermediate greys.
        let (w, h) = (8usize, 8usize);
        let mut src = Vec::new();
        for y in 0..h {
            for x in 0..w {
                let c = if x as i32 > y as i32 { 255u8 } else { 0u8 };
                src.extend_from_slice(&[c, c, c, 255]);
            }
        }
        let out = xbr2x(&src, w, h);
        let has_grey = out
            .chunks_exact(4)
            .any(|p| (1..=254).contains(&p[0]) && p[0] == p[1] && p[1] == p[2]);
        assert!(
            has_grey,
            "xBR must anti-alias the diagonal into grey midtones"
        );
    }

    #[test]
    fn output_is_fully_opaque() {
        // Whatever the content, every output pixel's alpha is 255.
        let mut src = Vec::new();
        for i in 0..(5 * 5) {
            let v = (i * 9) as u8;
            src.extend_from_slice(&[v, 255 - v, v / 2, 200]);
        }
        let out = xbr2x(&src, 5, 5);
        assert!(out.chunks_exact(4).all(|p| p[3] == 255));
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
