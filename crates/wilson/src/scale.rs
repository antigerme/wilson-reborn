// SPDX-License-Identifier: GPL-3.0-or-later
//! Scaling of an RGBA frame into a softbuffer `0x00RRGGBB` buffer.
//!
//! Two independent choices:
//! * [`ScaleMode`] — the destination rectangle: aspect-fit with letterbox, stretch-to-
//!   fill, largest integer multiple, or `extend` (aspect-fit but the bars are filled by
//!   extending the scene's edges — fills a widescreen with no black bars, no distortion).
//! * [`Filter`] — how source pixels are sampled into that rectangle: `Nearest` (crisp,
//!   the original 1992 blocky look), `Linear` (bilinear, smooths the upscaled pixels so
//!   it looks less "80s/90s"), `Xbr` (edge-directed pixel-art 2× upscale then a bilinear
//!   fit — smooth *and* sharp, dissolves the dither, the "HD remaster" look), or `Xbrz`
//!   (edge-directed too, but keeps the dither texture — cleaner, closer to the 1992 feel).

/// How a frame is scaled into the window (the destination rectangle).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ScaleMode {
    /// Preserve aspect ratio, centred, with black letterbox bars (default).
    #[default]
    Fit,
    /// Stretch to fill the whole window, ignoring aspect ratio.
    Stretch,
    /// Largest whole-number multiple that fits, centred (crisp pixels); falls back
    /// to [`ScaleMode::Fit`] when the window is smaller than the source.
    Integer,
    /// Like [`ScaleMode::Fit`] (aspect-correct, centred) but the letterbox/pillarbox
    /// bars are filled by extending the scene's edge pixels — so the sea/sky/horizon
    /// continue to the screen edges and fill a widescreen with no black bars and no
    /// distortion (unlike `stretch`).
    Extend,
}

impl ScaleMode {
    /// Parse a mode name (`fit`/`stretch`/`integer`/`extend`), case-insensitive.
    pub fn parse(s: &str) -> Option<ScaleMode> {
        match s.to_ascii_lowercase().as_str() {
            "fit" => Some(ScaleMode::Fit),
            "stretch" | "fill" => Some(ScaleMode::Stretch),
            "integer" | "int" => Some(ScaleMode::Integer),
            "extend" | "widescreen" => Some(ScaleMode::Extend),
            _ => None,
        }
    }

    /// The canonical name (round-trips with [`ScaleMode::parse`]).
    pub fn as_str(self) -> &'static str {
        match self {
            ScaleMode::Fit => "fit",
            ScaleMode::Stretch => "stretch",
            ScaleMode::Integer => "integer",
            ScaleMode::Extend => "extend",
        }
    }
}

/// How source pixels are sampled when scaling.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Filter {
    /// Nearest-neighbour: crisp, blocky — the authentic 1992 look.
    Nearest,
    /// Bilinear: smooths the upscaled pixels (softer, less "retro grid").
    Linear,
    /// Real **xBR (Hyllian)** edge-directed 2× upscale, then bilinear fit: smooth *and*
    /// sharp on sprites and edges, dissolving the 1992 dither (the "HD remaster" look) —
    /// the **default**. A bit heavier per frame (negligible for a screensaver).
    #[default]
    Xbr,
    /// **xBRZ** edge-directed 2× upscale, then bilinear fit: like `xbr` it rounds edges,
    /// but it keeps the original dither texture instead of dissolving it (cleaner, less
    /// "plastic" — closer to the 1992 feel).
    Xbrz,
}

impl Filter {
    /// Parse a filter name (`nearest`/`linear`), case-insensitive. Common synonyms
    /// (`crisp`, `pixel`; `smooth`, `bilinear`) are accepted too.
    pub fn parse(s: &str) -> Option<Filter> {
        match s.to_ascii_lowercase().as_str() {
            "nearest" | "crisp" | "pixel" | "none" => Some(Filter::Nearest),
            "linear" | "smooth" | "bilinear" => Some(Filter::Linear),
            "xbr" | "hd" => Some(Filter::Xbr),
            "xbrz" => Some(Filter::Xbrz),
            _ => None,
        }
    }

    /// The canonical name (round-trips with [`Filter::parse`]).
    pub fn as_str(self) -> &'static str {
        match self {
            Filter::Nearest => "nearest",
            Filter::Linear => "linear",
            Filter::Xbr => "xbr",
            Filter::Xbrz => "xbrz",
        }
    }
}

/// Scale `src` (RGBA8888, `sw`×`sh`) into `dst` (`0x00RRGGBB`, `dw`×`dh`) using `mode`
/// for the destination rectangle and `filter` for pixel sampling.
// Source dims + destination dims + mode + filter are all genuinely needed here.
#[allow(clippy::too_many_arguments)]
pub fn scale_rgba_to_argb(
    src: &[u8],
    sw: usize,
    sh: usize,
    dst: &mut [u32],
    dw: usize,
    dh: usize,
    mode: ScaleMode,
    filter: Filter,
) {
    // xBR/xBRZ are edge-directed 2× prescales done once, then a bilinear fit into the
    // window.
    if sw > 0 && sh > 0 {
        let up = match filter {
            Filter::Xbr => Some(wilson_engine::xbr2x(src, sw, sh)),
            Filter::Xbrz => Some(wilson_engine::xbrz2x(src, sw, sh)),
            _ => None,
        };
        if let Some(up) = up {
            scale_rgba_to_argb(&up, sw * 2, sh * 2, dst, dw, dh, mode, Filter::Linear);
            return;
        }
    }
    match mode {
        ScaleMode::Fit => scale_rgba_to_argb_fit(src, sw, sh, dst, dw, dh, filter),
        ScaleMode::Stretch => scale_rgba_to_argb_stretch(src, sw, sh, dst, dw, dh, filter),
        ScaleMode::Integer => scale_rgba_to_argb_integer(src, sw, sh, dst, dw, dh, filter),
        ScaleMode::Extend => scale_rgba_to_argb_extend(src, sw, sh, dst, dw, dh, filter),
    }
}

/// Aspect-correct, centred (like [`scale_rgba_to_argb_fit`]) but with the bars filled by
/// extending the scene's edge pixels — fills a widescreen window with the sea/sky/horizon
/// instead of black bars, and without the distortion of `stretch`.
pub fn scale_rgba_to_argb_extend(
    src: &[u8],
    sw: usize,
    sh: usize,
    dst: &mut [u32],
    dw: usize,
    dh: usize,
    filter: Filter,
) {
    if sw == 0 || sh == 0 || dw == 0 || dh == 0 {
        for p in dst.iter_mut() {
            *p = 0;
        }
        return;
    }
    // Same centred rectangle as Fit.
    let (tw, th) = if dw * sh <= dh * sw {
        (dw, (dw * sh / sw).max(1))
    } else {
        ((dh * sw / sh).max(1), dh)
    };
    let ox = (dw - tw) / 2;
    let oy = (dh - th) / 2;
    blit_scaled(src, sw, sh, dst, (dw, dh), (ox, oy, tw, th), filter);
    // Fill the bars by clamping each bar pixel to the nearest edge pixel of the scene
    // rectangle (so the edge column/row — ocean/sky/horizon — extends outward).
    if ox == 0 && oy == 0 {
        return; // the rectangle already covers the whole window (no bars)
    }
    for y in 0..dh {
        let cy = y.clamp(oy, oy + th - 1);
        let in_rows = y >= oy && y < oy + th;
        for x in 0..dw {
            if in_rows && x >= ox && x < ox + tw {
                continue; // inside the scene rectangle
            }
            let cx = x.clamp(ox, ox + tw - 1);
            dst[y * dw + x] = dst[cy * dw + cx];
        }
    }
}

#[inline]
fn argb(src: &[u8], si: usize) -> u32 {
    (u32::from(src[si]) << 16) | (u32::from(src[si + 1]) << 8) | u32::from(src[si + 2])
}

/// Bilinearly sample `src` at fractional source position `(fx, fy)` (in source-pixel
/// units), returning a packed `0x00RRGGBB`. Coordinates are clamped to the image.
#[inline]
fn sample_bilinear(src: &[u8], sw: usize, sh: usize, fx: f32, fy: f32) -> u32 {
    let fx = fx.clamp(0.0, (sw - 1) as f32);
    let fy = fy.clamp(0.0, (sh - 1) as f32);
    let x0 = fx.floor() as usize;
    let y0 = fy.floor() as usize;
    let x1 = (x0 + 1).min(sw - 1);
    let y1 = (y0 + 1).min(sh - 1);
    let wx = fx - x0 as f32;
    let wy = fy - y0 as f32;
    let i00 = (y0 * sw + x0) * 4;
    let i01 = (y0 * sw + x1) * 4;
    let i10 = (y1 * sw + x0) * 4;
    let i11 = (y1 * sw + x1) * 4;
    let chan = |o: usize| -> u32 {
        let top = src[i00 + o] as f32 * (1.0 - wx) + src[i01 + o] as f32 * wx;
        let bot = src[i10 + o] as f32 * (1.0 - wx) + src[i11 + o] as f32 * wx;
        (top * (1.0 - wy) + bot * wy).round() as u32
    };
    (chan(0) << 16) | (chan(1) << 8) | chan(2)
}

/// Blit `src` (`sw`×`sh`) into `dst` (`dst_dims`) as the rectangle `rect` =
/// `(ox, oy, tw, th)`, sampling with `filter`.
fn blit_scaled(
    src: &[u8],
    sw: usize,
    sh: usize,
    dst: &mut [u32],
    dst_dims: (usize, usize),
    rect: (usize, usize, usize, usize),
    filter: Filter,
) {
    let (dw, dh) = dst_dims;
    let (ox, oy, tw, th) = rect;
    for ty in 0..th {
        if oy + ty >= dh {
            break;
        }
        let drow = (oy + ty) * dw + ox;
        match filter {
            Filter::Nearest => {
                let sy = ty * sh / th;
                let srow = sy * sw;
                for tx in 0..tw {
                    if ox + tx >= dw {
                        break;
                    }
                    dst[drow + tx] = argb(src, (srow + tx * sw / tw) * 4);
                }
            }
            // `Xbr`/`Xbrz` are intercepted in `scale_rgba_to_argb` (they prescale then
            // recurse with `Linear`), so here they only fit the already-upscaled image.
            Filter::Linear | Filter::Xbr | Filter::Xbrz => {
                // Map the destination pixel centre back to source space, then blend the
                // four surrounding source texels. `-0.5` centres the sample so the image
                // is not shifted half a pixel.
                let fy = (ty as f32 + 0.5) * sh as f32 / th as f32 - 0.5;
                for tx in 0..tw {
                    if ox + tx >= dw {
                        break;
                    }
                    let fx = (tx as f32 + 0.5) * sw as f32 / tw as f32 - 0.5;
                    dst[drow + tx] = sample_bilinear(src, sw, sh, fx, fy);
                }
            }
        }
    }
}

/// Stretch to fill the whole window (no bars, aspect ratio not preserved).
pub fn scale_rgba_to_argb_stretch(
    src: &[u8],
    sw: usize,
    sh: usize,
    dst: &mut [u32],
    dw: usize,
    dh: usize,
    filter: Filter,
) {
    if sw == 0 || sh == 0 || dw == 0 || dh == 0 {
        for p in dst.iter_mut() {
            *p = 0;
        }
        return;
    }
    blit_scaled(src, sw, sh, dst, (dw, dh), (0, 0, dw, dh), filter);
}

/// Largest whole-number multiple that fits, centred; falls back to fit when the
/// window is smaller than the source.
pub fn scale_rgba_to_argb_integer(
    src: &[u8],
    sw: usize,
    sh: usize,
    dst: &mut [u32],
    dw: usize,
    dh: usize,
    filter: Filter,
) {
    for p in dst.iter_mut() {
        *p = 0;
    }
    if sw == 0 || sh == 0 || dw == 0 || dh == 0 {
        return;
    }
    let k = (dw / sw).min(dh / sh);
    if k == 0 {
        // Window smaller than the source: best-effort aspect fit.
        scale_rgba_to_argb_fit(src, sw, sh, dst, dw, dh, filter);
        return;
    }
    let (tw, th) = (sw * k, sh * k);
    let ox = (dw - tw) / 2;
    let oy = (dh - th) / 2;
    blit_scaled(src, sw, sh, dst, (dw, dh), (ox, oy, tw, th), filter);
}

/// Scale `src` (RGBA8888, `sw`×`sh`) into `dst` (`0x00RRGGBB`, `dw`×`dh`) preserving
/// aspect ratio, centred, filling the remainder with black letterbox bars.
pub fn scale_rgba_to_argb_fit(
    src: &[u8],
    sw: usize,
    sh: usize,
    dst: &mut [u32],
    dw: usize,
    dh: usize,
    filter: Filter,
) {
    for p in dst.iter_mut() {
        *p = 0;
    }
    if sw == 0 || sh == 0 || dw == 0 || dh == 0 {
        return;
    }
    // Largest dst rectangle with the source aspect ratio.
    let (tw, th) = if dw * sh <= dh * sw {
        (dw, (dw * sh / sw).max(1))
    } else {
        ((dh * sw / sh).max(1), dh)
    };
    let ox = (dw - tw) / 2;
    let oy = (dh - th) / 2;
    blit_scaled(src, sw, sh, dst, (dw, dh), (ox, oy, tw, th), filter);
}

#[cfg(test)]
mod tests {
    use super::*;

    // Most geometry tests use Nearest so exact pixel positions are asserted without
    // blending; the Linear path has its own tests below.
    const N: Filter = Filter::Nearest;

    #[test]
    fn fit_exact_aspect_fills_fully() {
        // 1x1 red into 3x3 (same aspect): fills everything, no bars.
        let src = [255u8, 0, 0, 255];
        let mut dst = [0u32; 9];
        scale_rgba_to_argb_fit(&src, 1, 1, &mut dst, 3, 3, N);
        assert!(dst.iter().all(|&p| p == 0x00FF_0000));
    }

    #[test]
    fn fit_letterboxes_wide_target() {
        // 2x2 source into 6x2: a 2x2 image centred (ox = 2) with black side bars.
        let src = [
            255, 0, 0, 255, 0, 255, 0, 255, // row 0: red, green
            0, 0, 255, 255, 255, 255, 0, 255, // row 1: blue, yellow
        ];
        let mut dst = [123u32; 12]; // pre-filled; bars must be cleared to black
        scale_rgba_to_argb_fit(&src, 2, 2, &mut dst, 6, 2, N);
        assert_eq!(dst[0], 0);
        assert_eq!(dst[1], 0);
        assert_eq!(dst[2], 0x00FF_0000); // red
        assert_eq!(dst[3], 0x0000_FF00); // green
        assert_eq!(dst[4], 0);
        assert_eq!(dst[5], 0);
        assert_eq!(dst[8], 0x0000_00FF); // blue
        assert_eq!(dst[9], 0x00FF_FF00); // yellow
    }

    #[test]
    fn zero_dims_are_safe() {
        let src = [0u8; 4];
        let mut dst = [0u32; 0];
        scale_rgba_to_argb_fit(&src, 1, 1, &mut dst, 0, 0, N);
        scale_rgba_to_argb_stretch(&src, 1, 1, &mut dst, 0, 0, N);
        scale_rgba_to_argb_integer(&src, 1, 1, &mut dst, 0, 0, N);
        scale_rgba_to_argb_fit(&src, 1, 1, &mut dst, 0, 0, Filter::Linear);
    }

    #[test]
    fn stretch_fills_every_pixel() {
        // 1x1 red stretched into 4x3: every pixel red, no bars.
        let src = [255u8, 0, 0, 255];
        let mut dst = [0u32; 12];
        scale_rgba_to_argb_stretch(&src, 1, 1, &mut dst, 4, 3, N);
        assert!(dst.iter().all(|&p| p == 0x00FF_0000));
    }

    #[test]
    fn integer_uses_whole_multiple_and_centres() {
        // 1x1 red into 5x5: k=5 (a 5x5 block), so it fills fully here.
        let src = [255u8, 0, 0, 255];
        let mut dst = [0u32; 25];
        scale_rgba_to_argb_integer(&src, 1, 1, &mut dst, 5, 5, N);
        assert!(dst.iter().all(|&p| p == 0x00FF_0000));

        // 2x2 source into 5x5: k=2 → a 4x4 block centred at (0,0)+offset, 1px bar.
        let s2 = [
            255, 0, 0, 255, 0, 255, 0, 255, // red, green
            0, 0, 255, 255, 255, 255, 0, 255, // blue, yellow
        ];
        let mut d2 = [9u32; 25];
        scale_rgba_to_argb_integer(&s2, 2, 2, &mut d2, 5, 5, N);
        // ox = (5-4)/2 = 0, oy = 0 → top-left of the 4x4 block is at (0,0).
        assert_eq!(d2[0], 0x00FF_0000); // red (top-left)
        assert_eq!(d2[2], 0x0000_FF00); // green
                                        // The last column/row are black bars.
        assert_eq!(d2[4], 0);
        assert_eq!(d2[24], 0);
    }

    #[test]
    fn integer_falls_back_when_window_too_small() {
        // 4x4 source into 2x2: k=0 → falls back to fit (fills the 2x2).
        let src = vec![200u8; 4 * 4 * 4];
        let mut dst = [0u32; 4];
        scale_rgba_to_argb_integer(&src, 4, 4, &mut dst, 2, 2, N);
        assert!(dst.iter().all(|&p| p == 0x00C8_C8C8));
    }

    #[test]
    fn mode_parse_round_trips() {
        for m in [
            ScaleMode::Fit,
            ScaleMode::Stretch,
            ScaleMode::Integer,
            ScaleMode::Extend,
        ] {
            assert_eq!(ScaleMode::parse(m.as_str()), Some(m));
        }
        assert_eq!(ScaleMode::parse("FIT"), Some(ScaleMode::Fit));
        assert_eq!(ScaleMode::parse("fill"), Some(ScaleMode::Stretch));
        assert_eq!(ScaleMode::parse("widescreen"), Some(ScaleMode::Extend));
        assert_eq!(ScaleMode::parse("nope"), None);
    }

    #[test]
    fn extend_fills_bars_with_edge_pixels_not_black() {
        // 1x2 source (red over blue) into a 4x2 window: aspect-fit centres a 2x2 scene
        // (red row, blue row) with side bars — which Extend fills with the row's edge
        // colour instead of black, so every pixel is red (top) or blue (bottom).
        let src = [255u8, 0, 0, 255, 0, 0, 255, 255]; // (0,0)=red, (0,1)=blue
        let mut dst = [0u32; 8]; // 4x2
        scale_rgba_to_argb_extend(&src, 1, 2, &mut dst, 4, 2, N);
        for x in 0..4 {
            assert_eq!(dst[x], 0x00FF_0000, "top row should be all red at x={x}");
            assert_eq!(
                dst[4 + x],
                0x0000_00FF,
                "bottom row should be all blue at x={x}"
            );
        }
        assert!(!dst.contains(&0), "no black bars in extend mode");
    }

    #[test]
    fn filter_parse_round_trips() {
        for f in [Filter::Nearest, Filter::Linear, Filter::Xbr, Filter::Xbrz] {
            assert_eq!(Filter::parse(f.as_str()), Some(f));
        }
        assert_eq!(Filter::parse("NEAREST"), Some(Filter::Nearest));
        assert_eq!(Filter::parse("crisp"), Some(Filter::Nearest));
        assert_eq!(Filter::parse("smooth"), Some(Filter::Linear));
        assert_eq!(Filter::parse("bilinear"), Some(Filter::Linear));
        assert_eq!(Filter::parse("xbr"), Some(Filter::Xbr));
        assert_eq!(Filter::parse("HD"), Some(Filter::Xbr));
        assert_eq!(Filter::parse("xbrz"), Some(Filter::Xbrz));
        assert_eq!(Filter::parse("nope"), None);
        assert_eq!(Filter::default(), Filter::Xbr); // xBR ("HD") by default
    }

    #[test]
    fn xbr_and_xbrz_filter_paths_fill_without_panic() {
        // Exercises the xBR/xBRZ prescale (wilson_engine::xbr2x / xbrz2x) + bilinear fit.
        let src = [
            255, 0, 0, 255, 0, 255, 0, 255, // red, green
            0, 0, 255, 255, 255, 255, 0, 255, // blue, yellow
        ];
        for f in [Filter::Xbr, Filter::Xbrz] {
            let mut dst = [0u32; 64];
            scale_rgba_to_argb(&src, 2, 2, &mut dst, 8, 8, ScaleMode::Stretch, f);
            assert!(dst.iter().any(|&p| p != 0), "{f:?} path rendered nothing");
        }
    }

    #[test]
    fn linear_flat_image_keeps_the_colour() {
        // Bilinear of a single colour must reproduce exactly that colour everywhere.
        let src = [10u8, 20, 30, 255];
        let mut dst = [0u32; 16];
        scale_rgba_to_argb_stretch(&src, 1, 1, &mut dst, 4, 4, Filter::Linear);
        assert!(dst.iter().all(|&p| p == 0x000A_141E));
    }

    #[test]
    fn linear_blends_between_neighbours() {
        // A 2x1 source (black | white) stretched wide must produce intermediate greys in
        // the middle (proof that it interpolates rather than hard-stepping).
        let src = [0u8, 0, 0, 255, 255, 255, 255, 255]; // black, white
        let mut dst = [0u32; 8];
        scale_rgba_to_argb_stretch(&src, 2, 1, &mut dst, 8, 1, Filter::Linear);
        // Ends stay near black/white; somewhere in the middle is a true grey blend.
        let grey = |p: u32| {
            let (r, g, b) = ((p >> 16) & 0xFF, (p >> 8) & 0xFF, p & 0xFF);
            r == g && g == b && (1..=254).contains(&r)
        };
        assert!(
            dst.iter().any(|&p| grey(p)),
            "expected a blended grey: {dst:?}"
        );
        // Nearest, by contrast, only ever emits pure black or white.
        let mut dn = [0u32; 8];
        scale_rgba_to_argb_stretch(&src, 2, 1, &mut dn, 8, 1, Filter::Nearest);
        assert!(dn.iter().all(|&p| p == 0 || p == 0x00FF_FFFF));
    }
}
