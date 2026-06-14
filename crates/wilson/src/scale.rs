// SPDX-License-Identifier: GPL-3.0-or-later
//! Nearest-neighbour scaling of an RGBA frame into a softbuffer `0x00RRGGBB` buffer.

/// Scale `src` (RGBA8888, `sw`×`sh`) into `dst` (one `0x00RRGGBB` word per pixel,
/// `dw`×`dh`) using nearest-neighbour sampling.
pub fn scale_rgba_to_argb(src: &[u8], sw: usize, sh: usize, dst: &mut [u32], dw: usize, dh: usize) {
    if sw == 0 || sh == 0 || dw == 0 || dh == 0 {
        return;
    }
    for dy in 0..dh {
        let sy = dy * sh / dh;
        let drow = dy * dw;
        let srow = sy * sw;
        for dx in 0..dw {
            let sx = dx * sw / dw;
            let si = (srow + sx) * 4;
            let r = u32::from(src[si]);
            let g = u32::from(src[si + 1]);
            let b = u32::from(src[si + 2]);
            dst[drow + dx] = (r << 16) | (g << 8) | b;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upscales_2x() {
        // 1x1 red source -> 2x2 destination, all red.
        let src = [255u8, 0, 0, 255];
        let mut dst = [0u32; 4];
        scale_rgba_to_argb(&src, 1, 1, &mut dst, 2, 2);
        assert_eq!(dst, [0x00FF_0000; 4]);
    }

    #[test]
    fn samples_nearest() {
        // 2x1 source [red, green] -> 4x1 destination.
        let src = [255, 0, 0, 255, 0, 255, 0, 255];
        let mut dst = [0u32; 4];
        scale_rgba_to_argb(&src, 2, 1, &mut dst, 4, 1);
        assert_eq!(dst, [0x00FF_0000, 0x00FF_0000, 0x0000_FF00, 0x0000_FF00]);
    }

    #[test]
    fn zero_dims_are_safe() {
        let src = [0u8; 4];
        let mut dst = [0u32; 0];
        scale_rgba_to_argb(&src, 1, 1, &mut dst, 0, 0);
    }
}
