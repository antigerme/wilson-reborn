// SPDX-License-Identifier: GPL-3.0-or-later
//! Nearest-neighbour, aspect-preserving scaling of an RGBA frame into a softbuffer
//! `0x00RRGGBB` buffer (with black letterbox bars).

#[inline]
fn argb(src: &[u8], si: usize) -> u32 {
    (u32::from(src[si]) << 16) | (u32::from(src[si + 1]) << 8) | u32::from(src[si + 2])
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
    for ty in 0..th {
        let sy = ty * sh / th;
        let drow = (oy + ty) * dw + ox;
        let srow = sy * sw;
        for tx in 0..tw {
            dst[drow + tx] = argb(src, (srow + tx * sw / tw) * 4);
        }
    }
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
    }
}
