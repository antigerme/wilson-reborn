// SPDX-License-Identifier: GPL-3.0-or-later
//! Nearest-neighbour scaling of an RGBA frame into a softbuffer `0x00RRGGBB` buffer,
//! in three modes (aspect-fit with letterbox, stretch-to-fill, integer-multiple).

/// How a frame is scaled into the window.
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
}

impl ScaleMode {
    /// Parse a mode name (`fit`/`stretch`/`integer`), case-insensitive.
    pub fn parse(s: &str) -> Option<ScaleMode> {
        match s.to_ascii_lowercase().as_str() {
            "fit" => Some(ScaleMode::Fit),
            "stretch" | "fill" => Some(ScaleMode::Stretch),
            "integer" | "int" => Some(ScaleMode::Integer),
            _ => None,
        }
    }

    /// The canonical name (round-trips with [`ScaleMode::parse`]).
    pub fn as_str(self) -> &'static str {
        match self {
            ScaleMode::Fit => "fit",
            ScaleMode::Stretch => "stretch",
            ScaleMode::Integer => "integer",
        }
    }
}

/// Scale `src` (RGBA8888, `sw`×`sh`) into `dst` (`0x00RRGGBB`, `dw`×`dh`) using `mode`.
pub fn scale_rgba_to_argb(
    src: &[u8],
    sw: usize,
    sh: usize,
    dst: &mut [u32],
    dw: usize,
    dh: usize,
    mode: ScaleMode,
) {
    match mode {
        ScaleMode::Fit => scale_rgba_to_argb_fit(src, sw, sh, dst, dw, dh),
        ScaleMode::Stretch => scale_rgba_to_argb_stretch(src, sw, sh, dst, dw, dh),
        ScaleMode::Integer => scale_rgba_to_argb_integer(src, sw, sh, dst, dw, dh),
    }
}

#[inline]
fn argb(src: &[u8], si: usize) -> u32 {
    (u32::from(src[si]) << 16) | (u32::from(src[si + 1]) << 8) | u32::from(src[si + 2])
}

/// Blit `src` (`sw`×`sh`) into `dst` (`dst_dims`) as the rectangle `rect` =
/// `(ox, oy, tw, th)`, nearest-neighbour scaled.
fn blit_scaled(
    src: &[u8],
    sw: usize,
    sh: usize,
    dst: &mut [u32],
    dst_dims: (usize, usize),
    rect: (usize, usize, usize, usize),
) {
    let (dw, dh) = dst_dims;
    let (ox, oy, tw, th) = rect;
    for ty in 0..th {
        if oy + ty >= dh {
            break;
        }
        let sy = ty * sh / th;
        let drow = (oy + ty) * dw + ox;
        let srow = sy * sw;
        for tx in 0..tw {
            if ox + tx >= dw {
                break;
            }
            dst[drow + tx] = argb(src, (srow + tx * sw / tw) * 4);
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
) {
    if sw == 0 || sh == 0 || dw == 0 || dh == 0 {
        for p in dst.iter_mut() {
            *p = 0;
        }
        return;
    }
    blit_scaled(src, sw, sh, dst, (dw, dh), (0, 0, dw, dh));
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
        scale_rgba_to_argb_fit(src, sw, sh, dst, dw, dh);
        return;
    }
    let (tw, th) = (sw * k, sh * k);
    let ox = (dw - tw) / 2;
    let oy = (dh - th) / 2;
    blit_scaled(src, sw, sh, dst, (dw, dh), (ox, oy, tw, th));
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
    blit_scaled(src, sw, sh, dst, (dw, dh), (ox, oy, tw, th));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_exact_aspect_fills_fully() {
        // 1x1 red into 3x3 (same aspect): fills everything, no bars.
        let src = [255u8, 0, 0, 255];
        let mut dst = [0u32; 9];
        scale_rgba_to_argb_fit(&src, 1, 1, &mut dst, 3, 3);
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
        scale_rgba_to_argb_fit(&src, 2, 2, &mut dst, 6, 2);
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
        scale_rgba_to_argb_fit(&src, 1, 1, &mut dst, 0, 0);
        scale_rgba_to_argb_stretch(&src, 1, 1, &mut dst, 0, 0);
        scale_rgba_to_argb_integer(&src, 1, 1, &mut dst, 0, 0);
    }

    #[test]
    fn stretch_fills_every_pixel() {
        // 1x1 red stretched into 4x3: every pixel red, no bars.
        let src = [255u8, 0, 0, 255];
        let mut dst = [0u32; 12];
        scale_rgba_to_argb_stretch(&src, 1, 1, &mut dst, 4, 3);
        assert!(dst.iter().all(|&p| p == 0x00FF_0000));
    }

    #[test]
    fn integer_uses_whole_multiple_and_centres() {
        // 1x1 red into 5x5: k=5 (a 5x5 block), so it fills fully here.
        let src = [255u8, 0, 0, 255];
        let mut dst = [0u32; 25];
        scale_rgba_to_argb_integer(&src, 1, 1, &mut dst, 5, 5);
        assert!(dst.iter().all(|&p| p == 0x00FF_0000));

        // 2x2 source into 5x5: k=2 → a 4x4 block centred at (0,0)+offset, 1px bar.
        let s2 = [
            255, 0, 0, 255, 0, 255, 0, 255, // red, green
            0, 0, 255, 255, 255, 255, 0, 255, // blue, yellow
        ];
        let mut d2 = [9u32; 25];
        scale_rgba_to_argb_integer(&s2, 2, 2, &mut d2, 5, 5);
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
        scale_rgba_to_argb_integer(&src, 4, 4, &mut dst, 2, 2);
        assert!(dst.iter().all(|&p| p == 0x00C8_C8C8));
    }

    #[test]
    fn mode_parse_round_trips() {
        for m in [ScaleMode::Fit, ScaleMode::Stretch, ScaleMode::Integer] {
            assert_eq!(ScaleMode::parse(m.as_str()), Some(m));
        }
        assert_eq!(ScaleMode::parse("FIT"), Some(ScaleMode::Fit));
        assert_eq!(ScaleMode::parse("fill"), Some(ScaleMode::Stretch));
        assert_eq!(ScaleMode::parse("nope"), None);
    }
}
