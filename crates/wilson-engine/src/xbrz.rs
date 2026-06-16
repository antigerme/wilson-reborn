// SPDX-License-Identifier: GPL-3.0-or-later
//! **xBRZ** edge-directed 2× upscaler — a Rust port of Zenju's xBRZ (the refined xBR
//! fork), for comparison against our [`crate::xbr2x`] (Hyllian xBR).
//!
//! Ported from the reference C++ (`xbrz.cpp`/`config.h`, HqMAME project, © Zenju), which
//! is distributed under **GPL-3.0** — compatible with this crate's GPL-3.0-or-later. Only
//! the 2× RGB scaler is ported (that's what our pipeline uses); the algorithm, the
//! `ScalerCfg` defaults (`luminanceWeight=1`, `equalColorTolerance=30`,
//! `dominantDirectionThreshold=3.6`, `steepDirectionThreshold=2.2`) and the ITU-R BT.2020
//! YCbCr colour metric are reproduced faithfully.
//!
//! Fidelity is **byte-verified**: our output matches Zenju's reference xBRZ binary
//! bit-for-bit on an 8×8 golden (see `xbrz_matches_reference_golden`) and on every real
//! 640×480 frame tested (2026-06-16). Matching the reference required reproducing its
//! lookup-table colour metric exactly (quantised diffs + f32 rounding — see [`dist`]).

/// xBRZ blend strength for one corner (fits in 2 bits).
const BLEND_NONE: u8 = 0;
const BLEND_NORMAL: u8 = 1;
const BLEND_DOMINANT: u8 = 2;

// ScalerCfg defaults (config.h). luminanceWeight (=1) is unused by the LUT colour metric.
const EQUAL_COLOR_TOLERANCE: f64 = 30.0;
const DOMINANT_DIRECTION_THRESHOLD: f64 = 3.6;
const STEEP_DIRECTION_THRESHOLD: f64 = 2.2;

/// ITU-R BT.2020 YCbCr colour distance — byte-exact with xBRZ's default `ColorDistanceRGB`.
///
/// The reference ships a 64 MB lookup table (`DistYCbCrBuffer`): it quantises each channel
/// diff `d` to the table index `(d+255)/2` (so the stored diff is `2·idx-255`), computes
/// the YCbCr magnitude from those quantised diffs, and stores the result as an **f32**. We
/// reproduce that exactly (quantise → f64 math → round to f32) so our output matches the
/// reference binary bit-for-bit, rather than using the (commented-out) exact `distYCbCr`.
#[inline]
fn dist(p1: [u8; 3], p2: [u8; 3]) -> f64 {
    let q = |a: u8, b: u8| -> f64 {
        let d = i32::from(a) - i32::from(b);
        f64::from(2 * ((d + 255) / 2) - 255) // LUT index (d+255)/2 → stored diff 2·idx-255
    };
    let r = q(p1[0], p2[0]);
    let g = q(p1[1], p2[1]);
    let b = q(p1[2], p2[2]);
    const K_B: f64 = 0.0593;
    const K_R: f64 = 0.2627;
    const K_G: f64 = 1.0 - K_B - K_R;
    const SCALE_B: f64 = 0.5 / (1.0 - K_B);
    const SCALE_R: f64 = 0.5 / (1.0 - K_R);
    let y = K_R * r + K_G * g + K_B * b; // the LUT applies no lumaWeight (== our default 1)
    let c_b = SCALE_B * (b - y);
    let c_r = SCALE_R * (r - y);
    let d = (y * y + c_b * c_b + c_r * c_r).sqrt();
    f64::from(d as f32) // the table stores f32; match that precision
}

#[inline]
fn eq(p1: [u8; 3], p2: [u8; 3]) -> bool {
    dist(p1, p2) < EQUAL_COLOR_TOLERANCE
}

/// Per-channel `gradientRGB<M,N>`: blend `front` over `back` with opacity M/N.
#[inline]
fn grad(block: &mut [[u8; 3]; 4], idx: usize, front: [u8; 3], m: i32, n: i32) {
    let back = block[idx];
    let mix = |fr: u8, bk: u8| ((i32::from(fr) * m + i32::from(bk) * (n - m)) / n) as u8;
    block[idx] = [
        mix(front[0], back[0]),
        mix(front[1], back[1]),
        mix(front[2], back[2]),
    ];
}

/// Blend directions for the four corners around the F/G/J/K point (the `preProcessCorners`
/// result): each is [`BLEND_NONE`]/[`BLEND_NORMAL`]/[`BLEND_DOMINANT`].
#[derive(Default, Clone, Copy)]
struct BlendResult {
    f: u8,
    g: u8,
    j: u8,
    k: u8,
}

// 4×4 kernel indices (a..p), F (the input pixel) at index 5.
//   a b c d  = 0 1 2 3
//   e f g h  = 4 5 6 7
//   i j k l  = 8 9 10 11
//   m n o p  = 12 13 14 15
/// Detect blend direction for the corner between F, G, J, K (`preProcessCorners`).
fn pre_process_corners(k: &[[u8; 3]; 16]) -> BlendResult {
    let mut r = BlendResult::default();
    let (f, g, j, kk) = (k[5], k[6], k[9], k[10]);
    if (f == g && j == kk) || (f == j && g == kk) {
        return r;
    }
    let w = 4.0;
    // jg = i-f, f-c, n-k, k-h, weight*(j-g)
    let jg = dist(k[8], k[5])
        + dist(k[5], k[2])
        + dist(k[13], k[10])
        + dist(k[10], k[7])
        + w * dist(k[9], k[6]);
    // fk = e-j, j-o, b-g, g-l, weight*(f-k)
    let fk = dist(k[4], k[9])
        + dist(k[9], k[14])
        + dist(k[1], k[6])
        + dist(k[6], k[11])
        + w * dist(k[5], k[10]);
    if jg < fk {
        let dom = DOMINANT_DIRECTION_THRESHOLD * jg < fk;
        let bt = if dom { BLEND_DOMINANT } else { BLEND_NORMAL };
        if f != g && f != j {
            r.f = bt;
        }
        if kk != j && kk != g {
            r.k = bt;
        }
    } else if fk < jg {
        let dom = DOMINANT_DIRECTION_THRESHOLD * fk < jg;
        let bt = if dom { BLEND_DOMINANT } else { BLEND_NORMAL };
        if j != f && j != kk {
            r.j = bt;
        }
        if g != f && g != kk {
            r.g = bt;
        }
    }
    r
}

// 3×3 kernel getters under each rotation (the reference's `get_x<rotDeg>` tables).
// Index: a b c / d e f / g h i = 0 1 2 / 3 4 5 / 6 7 8.
const KER_ROT: [[usize; 9]; 4] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8], // ROT_0
    [6, 3, 0, 7, 4, 1, 8, 5, 2], // ROT_90
    [8, 7, 6, 5, 4, 3, 2, 1, 0], // ROT_180
    [2, 5, 8, 1, 4, 7, 0, 3, 6], // ROT_270
];

/// Rotate the packed 4-corner blend byte to match a kernel rotation (`rotateBlendInfo`).
#[inline]
fn rotate_blend_info(b: u8, rot: usize) -> u8 {
    // Rotate the 4 two-bit corner fields by `rot` quarter-turns (2 bits each).
    b.rotate_left(2 * rot as u32)
}

/// Map an `OutputMatrix::ref<I,J>` access to a 2×2 block index (TL,TR,BL,BR = 0,1,2,3)
/// under the given rotation.
#[inline]
fn out_idx(rot: usize, i: usize, j: usize) -> usize {
    let (row, col) = match rot {
        1 => (1 - j, i),
        2 => (1 - i, 1 - j),
        3 => (j, 1 - i),
        _ => (i, j),
    };
    row * 2 + col
}

/// Blend one rotated corner of pixel `e` into the 2×2 output block (the reference's
/// `blendPixel` specialised to `Scaler2x`).
fn blend_pixel(ker3: &[[u8; 3]; 9], block: &mut [[u8; 3]; 4], blend_info: u8, rot: usize) {
    let m = &KER_ROT[rot];
    let (b, c) = (ker3[m[1]], ker3[m[2]]);
    let (d, e, f) = (ker3[m[3]], ker3[m[4]], ker3[m[5]]);
    let (g, h, i) = (ker3[m[6]], ker3[m[7]], ker3[m[8]]);

    let blend = rotate_blend_info(blend_info, rot);
    let bottom_r = (blend >> 4) & 0x3;
    if bottom_r < BLEND_NORMAL {
        return;
    }
    let top_r = (blend >> 2) & 0x3;
    let bottom_l = (blend >> 6) & 0x3;

    let do_line_blend = if bottom_r >= BLEND_DOMINANT {
        true
    } else if (top_r != BLEND_NONE && !eq(e, g)) || (bottom_l != BLEND_NONE && !eq(e, c)) {
        // A second blending in an adjacent rotation would conflict — corner only.
        false
    } else {
        // No full blending for L-shapes; blend corner only.
        !(!eq(e, i) && eq(g, h) && eq(h, i) && eq(i, f) && eq(f, c))
    };

    let px = if dist(e, f) <= dist(e, h) { f } else { h };

    if do_line_blend {
        let fg = dist(f, g);
        let hc = dist(h, c);
        let have_shallow = STEEP_DIRECTION_THRESHOLD * fg <= hc && e != g && d != g;
        let have_steep = STEEP_DIRECTION_THRESHOLD * hc <= fg && e != c && b != c;
        match (have_shallow, have_steep) {
            (true, true) => {
                // blendLineSteepAndShallow
                grad(block, out_idx(rot, 1, 0), px, 1, 4);
                grad(block, out_idx(rot, 0, 1), px, 1, 4);
                grad(block, out_idx(rot, 1, 1), px, 5, 6);
            }
            (true, false) => {
                // blendLineShallow
                grad(block, out_idx(rot, 1, 0), px, 1, 4);
                grad(block, out_idx(rot, 1, 1), px, 3, 4);
            }
            (false, true) => {
                // blendLineSteep
                grad(block, out_idx(rot, 0, 1), px, 1, 4);
                grad(block, out_idx(rot, 1, 1), px, 3, 4);
            }
            (false, false) => {
                // blendLineDiagonal
                grad(block, out_idx(rot, 1, 1), px, 1, 2);
            }
        }
    } else {
        // blendCorner: model a round corner (1 - pi/4 ≈ 0.2146).
        grad(block, out_idx(rot, 1, 1), px, 21, 100);
    }
}

/// Upscale an RGBA image by 2× with xBRZ. Returns the `2w × 2h` RGBA buffer (always fully
/// opaque). `src` must be `w*h*4` bytes.
pub fn xbrz2x(src: &[u8], w: usize, h: usize) -> Vec<u8> {
    let (dw, dh) = (w * 2, h * 2);
    let mut out = vec![0u8; dw * dh * 4];
    if w == 0 || h == 0 {
        return out;
    }
    let at = |x: i32, y: i32| -> [u8; 3] {
        let xc = x.clamp(0, w as i32 - 1) as usize;
        let yc = y.clamp(0, h as i32 - 1) as usize;
        let o = (yc * w + xc) * 4;
        [src[o], src[o + 1], src[o + 2]]
    };

    // Pass 1: per-pixel blend info (4 corners packed into a byte, accumulated from the
    // four corner results around each pixel).
    let mut blend = vec![0u8; w * h];
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let mut k4 = [[0u8; 3]; 16];
            let mut idx = 0;
            for dy in -1..=2 {
                for dx in -1..=2 {
                    k4[idx] = at(x + dx, y + dy);
                    idx += 1;
                }
            }
            let r = pre_process_corners(&k4);
            let (xu, yu) = (x as usize, y as usize);
            blend[yu * w + xu] |= r.f << 4; // BottomR of (x, y)
            if xu + 1 < w {
                blend[yu * w + xu + 1] |= r.g << 6; // BottomL of (x+1, y)
            }
            if yu + 1 < h {
                blend[(yu + 1) * w + xu] |= r.j << 2; // TopR of (x, y+1)
            }
            if xu + 1 < w && yu + 1 < h {
                blend[(yu + 1) * w + xu + 1] |= r.k; // TopL of (x+1, y+1)
            }
        }
    }

    // Pass 2: render each 2×2 block.
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let f = at(x, y);
            let mut block = [f; 4];
            let bi = blend[y as usize * w + x as usize];
            if bi != 0 {
                let ker3 = [
                    at(x - 1, y - 1),
                    at(x, y - 1),
                    at(x + 1, y - 1),
                    at(x - 1, y),
                    f,
                    at(x + 1, y),
                    at(x - 1, y + 1),
                    at(x, y + 1),
                    at(x + 1, y + 1),
                ];
                for rot in 0..4 {
                    blend_pixel(&ker3, &mut block, bi, rot);
                }
            }
            let tl = ((2 * y as usize) * dw + 2 * x as usize) * 4;
            for (o, col) in [
                (tl, block[0]),
                (tl + 4, block[1]),
                (tl + dw * 4, block[2]),
                (tl + dw * 4 + 4, block[3]),
            ] {
                out[o] = col[0];
                out[o + 1] = col[1];
                out[o + 2] = col[2];
                out[o + 3] = 255;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xbrz_matches_reference_golden() {
        // A synthetic 8×8 RGB fixture and the EXACT 16×16 output of Zenju's reference xBRZ
        // binary (factor 2), stored as raw bytes. Our port must reproduce it byte-for-byte
        // — this locks fidelity in CI (which has no xBRZ binary). Generated and verified
        // 2026-06-16 by compiling and running the reference; that day all 15 real 640×480
        // frames were also byte-identical. Both fixtures are synthetic (no game data).
        const GOLDEN_IN: &[u8] = include_bytes!("testdata/xbrz_golden_in.bin"); // 8×8×3
        const GOLDEN_OUT: &[u8] = include_bytes!("testdata/xbrz_golden_out.bin"); // 16×16×3
        let mut rgba = Vec::with_capacity(GOLDEN_IN.len() / 3 * 4);
        for px in GOLDEN_IN.chunks_exact(3) {
            rgba.extend_from_slice(&[px[0], px[1], px[2], 255]);
        }
        let out = xbrz2x(&rgba, 8, 8);
        let out_rgb: Vec<u8> = out
            .chunks_exact(4)
            .flat_map(|p| [p[0], p[1], p[2]])
            .collect();
        assert_eq!(
            out_rgb.len(),
            GOLDEN_OUT.len(),
            "16×16×3 expected from an 8×8 input"
        );
        assert!(
            out_rgb == GOLDEN_OUT,
            "xBRZ output must match the reference binary byte-for-byte"
        );
    }

    #[test]
    fn doubles_the_dimensions() {
        let out = xbrz2x(&[0u8; 3 * 2 * 4], 3, 2);
        assert_eq!(out.len(), 6 * 4 * 4);
    }

    #[test]
    fn flat_image_is_unchanged() {
        // No corner has differing F/G/J/K → no blending → every texel is the source colour.
        let mut src = Vec::new();
        for _ in 0..(4 * 4) {
            src.extend_from_slice(&[40, 80, 120, 255]);
        }
        let out = xbrz2x(&src, 4, 4);
        assert!(out.chunks_exact(4).all(|p| p == [40, 80, 120, 255]));
    }

    #[test]
    fn output_is_fully_opaque() {
        let mut src = Vec::new();
        for i in 0..(6 * 6) {
            let v = (i * 7) as u8;
            src.extend_from_slice(&[v, 255 - v, v / 2, 180]);
        }
        let out = xbrz2x(&src, 6, 6);
        assert!(out.chunks_exact(4).all(|p| p[3] == 255));
    }

    #[test]
    fn diagonal_edge_is_antialiased() {
        // Black/white diagonal → xBRZ must introduce intermediate (grey) tones.
        let (w, h) = (8usize, 8usize);
        let mut src = Vec::new();
        for y in 0..h {
            for x in 0..w {
                let c = if x as i32 > y as i32 { 255u8 } else { 0u8 };
                src.extend_from_slice(&[c, c, c, 255]);
            }
        }
        let out = xbrz2x(&src, w, h);
        let has_grey = out
            .chunks_exact(4)
            .any(|p| (1..=254).contains(&p[0]) && p[0] == p[1] && p[1] == p[2]);
        assert!(
            has_grey,
            "xBRZ must anti-alias the diagonal into grey midtones"
        );
    }

    #[test]
    fn straight_edge_stays_hard() {
        // A vertical 2-colour split: no diagonal corner gradient → only the two source
        // colours appear (xBRZ does not soften straight H/V edges).
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
        let out = xbrz2x(&src, w, h);
        assert!(
            out.chunks_exact(4).all(|p| p[0..3] == a || p[0..3] == b),
            "a straight vertical edge must stay hard"
        );
    }
}
