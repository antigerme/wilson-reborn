// SPDX-License-Identifier: GPL-3.0-or-later
//! An indexed-color drawing surface and 2D primitives.
//!
//! The primitives are faithful ports of `repos/jc_reborn/graphics.c`, but operate on
//! palette indices instead of RGB so they can be tested without a real renderer.
//! [`TRANSPARENT`] marks holes in sprite layers (the original used a magenta colour
//! key); it is the sentinel skipped when blitting and when composing layers.

use wilson_dgds::Palette;

/// Sentinel palette value meaning "transparent" in a layer.
pub const TRANSPARENT: u8 = 0xFF;

/// An axis-aligned clip rectangle in surface coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    /// Left edge.
    pub x: i32,
    /// Top edge.
    pub y: i32,
    /// Width.
    pub w: i32,
    /// Height.
    pub h: i32,
}

impl Rect {
    /// Whether `(x, y)` lies inside the rectangle.
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }
}

/// An indexed-color framebuffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Surface {
    /// Width in pixels.
    pub width: u16,
    /// Height in pixels.
    pub height: u16,
    /// One palette index per pixel (row-major). May contain [`TRANSPARENT`].
    pub pixels: Vec<u8>,
}

impl Surface {
    /// Create a `width`×`height` surface filled with `fill`.
    pub fn new(width: u16, height: u16, fill: u8) -> Self {
        Surface {
            width,
            height,
            pixels: vec![fill; width as usize * height as usize],
        }
    }

    fn index(&self, x: i32, y: i32) -> Option<usize> {
        if x >= 0 && y >= 0 && x < i32::from(self.width) && y < i32::from(self.height) {
            Some(y as usize * self.width as usize + x as usize)
        } else {
            None
        }
    }

    /// Get the pixel at `(x, y)`, or `None` if out of bounds.
    pub fn get(&self, x: i32, y: i32) -> Option<u8> {
        self.index(x, y).map(|i| self.pixels[i])
    }

    /// Fill the whole surface with `color`.
    pub fn fill(&mut self, color: u8) {
        self.pixels.iter_mut().for_each(|p| *p = color);
    }

    /// Write a single pixel (bounds-checked to the surface).
    pub fn put_pixel(&mut self, x: i32, y: i32, color: u8) {
        if let Some(i) = self.index(x, y) {
            self.pixels[i] = color;
        }
    }

    /// Fill a rectangle with `color`, respecting an optional clip rectangle.
    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: u8, clip: Option<Rect>) {
        for yy in y..y + h {
            for xx in x..x + w {
                if let Some(c) = clip {
                    if !c.contains(xx, yy) {
                        continue;
                    }
                }
                self.put_pixel(xx, yy, color);
            }
        }
    }

    fn draw_h_line(&mut self, x1: i32, x2: i32, y: i32, color: u8) {
        for x in x1..=x2 {
            self.put_pixel(x, y, color);
        }
    }

    /// Draw a line using the same Bresenham variant as the original engine.
    pub fn draw_line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, color: u8) {
        let dx = (x2 - x1).abs();
        let dy = (y2 - y1).abs();
        let xinc = if x2 > x1 { 1 } else { -1 };
        let yinc = if y2 > y1 { 1 } else { -1 };
        let mut x = x1;
        let mut y = y1;

        if dy < dx {
            let mut cumul = (dx + 1) / 2;
            for _ in 0..dx {
                self.put_pixel(x, y, color);
                x += xinc;
                cumul += dy;
                if cumul > dx {
                    cumul -= dx;
                    y += yinc;
                }
            }
        } else {
            let mut cumul = (dy + 1) / 2;
            for _ in 0..dy {
                self.put_pixel(x, y, color);
                y += yinc;
                cumul += dx;
                if cumul > dy {
                    cumul -= dy;
                    x += xinc;
                }
            }
        }
    }

    /// Draw a filled circle (`fg` outline over `bg` fill), faithful to the original.
    /// Only regular circles with an even diameter are drawn (as in the original).
    pub fn draw_circle(&mut self, x1: i32, y1: i32, width: i32, height: i32, fg: u8, bg: u8) {
        if width != height || width < 2 || width % 2 != 0 {
            return;
        }
        let r = (width / 2) - 1;
        let xc = x1 + r;
        let yc = y1 + r;

        let (mut x, mut y, mut d) = (0, r, 1 - r);
        loop {
            self.draw_h_line(xc - x, xc + x + 1, yc + y + 1, bg);
            self.draw_h_line(xc - x, xc + x + 1, yc - y, bg);
            self.draw_h_line(xc - y, xc + y + 1, yc + x + 1, bg);
            self.draw_h_line(xc - y, xc + y + 1, yc - x, bg);
            if y - x <= 1 {
                break;
            }
            if d < 0 {
                d += (x << 1) + 3;
            } else {
                d += ((x - y) << 1) + 5;
                y -= 1;
            }
            x += 1;
        }

        if fg != bg {
            let (mut x, mut y, mut d) = (0, r, 1 - r);
            loop {
                for (px, py) in [
                    (xc - x, yc + y + 1),
                    (xc + x + 1, yc + y + 1),
                    (xc - x, yc - y),
                    (xc + x + 1, yc - y),
                    (xc - y, yc + x + 1),
                    (xc + y + 1, yc + x + 1),
                    (xc - y, yc - x),
                    (xc + y + 1, yc - x),
                ] {
                    self.put_pixel(px, py, fg);
                }
                if y - x <= 1 {
                    break;
                }
                if d < 0 {
                    d += (x << 1) + 3;
                } else {
                    d += ((x - y) << 1) + 5;
                    y -= 1;
                }
                x += 1;
            }
        }
    }

    /// Blit a sprite (`src_w`×`src_h` palette indices) at `(x, y)`.
    ///
    /// Pixels equal to `transparent` are skipped; `flip` mirrors horizontally;
    /// an optional `clip` rectangle bounds the destination.
    #[allow(clippy::too_many_arguments)]
    pub fn blit(
        &mut self,
        src_w: u16,
        src_h: u16,
        src: &[u8],
        x: i32,
        y: i32,
        transparent: Option<u8>,
        flip: bool,
        clip: Option<Rect>,
    ) {
        let sw = src_w as i32;
        let sh = src_h as i32;
        for sy in 0..sh {
            for sx in 0..sw {
                let read_col = if flip { sw - 1 - sx } else { sx };
                let pixel = src[(sy * sw + read_col) as usize];
                if Some(pixel) == transparent {
                    continue;
                }
                let (dx, dy) = (x + sx, y + sy);
                if let Some(c) = clip {
                    if !c.contains(dx, dy) {
                        continue;
                    }
                }
                self.put_pixel(dx, dy, pixel);
            }
        }
    }

    /// Copy the non-transparent pixels of the rectangle `(x, y, w, h)` from `src` onto
    /// `self` (used by the TTM `COPY_ZONE_TO_BG` opcode to build the persistent
    /// "saved zones" layer, mirroring `jc_reborn`'s `grCopyZoneToBg`).
    pub fn blit_zone(&mut self, src: &Surface, x: i32, y: i32, w: i32, h: i32) {
        for yy in y..y + h {
            for xx in x..x + w {
                if let Some(p) = src.get(xx, yy) {
                    if p != TRANSPARENT {
                        self.put_pixel(xx, yy, p);
                    }
                }
            }
        }
    }

    /// Compose `top` over a copy of `self`: `top`'s non-transparent pixels win.
    pub fn compose_over(&self, top: &Surface) -> Surface {
        let mut out = self.clone();
        for (o, &t) in out.pixels.iter_mut().zip(top.pixels.iter()) {
            if t != TRANSPARENT {
                *o = t;
            }
        }
        out
    }

    /// Convert to RGBA8888 using `palette` ([`TRANSPARENT`] → fully transparent).
    pub fn to_rgba(&self, palette: &Palette) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.pixels.len() * 4);
        for &p in &self.pixels {
            if p == TRANSPARENT {
                out.extend_from_slice(&[0, 0, 0, 0]);
            } else {
                let [r, g, b] = palette.rgb(p as usize);
                out.extend_from_slice(&[r, g, b, 255]);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn put_and_get_bounds() {
        let mut s = Surface::new(3, 2, 0);
        s.put_pixel(1, 1, 7);
        assert_eq!(s.get(1, 1), Some(7));
        s.put_pixel(-1, 0, 9); // ignored
        s.put_pixel(3, 0, 9); // ignored (out of bounds)
        assert_eq!(s.get(3, 0), None);
        assert_eq!(s.pixels, vec![0, 0, 0, 0, 7, 0]);
    }

    #[test]
    fn fill_rect_with_clip() {
        let mut s = Surface::new(4, 4, 0);
        let clip = Rect {
            x: 1,
            y: 1,
            w: 2,
            h: 2,
        };
        s.fill_rect(0, 0, 4, 4, 5, Some(clip));
        // Only the clip region is painted.
        assert_eq!(s.get(0, 0), Some(0));
        assert_eq!(s.get(1, 1), Some(5));
        assert_eq!(s.get(2, 2), Some(5));
        assert_eq!(s.get(3, 3), Some(0));
    }

    #[test]
    fn blit_with_transparency_and_flip() {
        let mut s = Surface::new(4, 2, 0);
        // 2x2 sprite: [1, T, 2, 3]
        let sprite = [1u8, TRANSPARENT, 2, 3];
        s.blit(2, 2, &sprite, 0, 0, Some(TRANSPARENT), false, None);
        assert_eq!(s.get(0, 0), Some(1));
        assert_eq!(s.get(1, 0), Some(0)); // transparent kept background
        assert_eq!(s.get(0, 1), Some(2));
        assert_eq!(s.get(1, 1), Some(3));

        let mut f = Surface::new(2, 2, 0);
        f.blit(2, 2, &sprite, 0, 0, Some(TRANSPARENT), true, None);
        // Flipped: row0 -> [T, 1] -> (0,0) stays 0, (1,0)=1
        assert_eq!(f.get(0, 0), Some(0));
        assert_eq!(f.get(1, 0), Some(1));
        assert_eq!(f.get(0, 1), Some(3));
        assert_eq!(f.get(1, 1), Some(2));
    }

    #[test]
    fn compose_over_layers() {
        let bg = Surface::new(2, 1, 9);
        let mut top = Surface::new(2, 1, TRANSPARENT);
        top.put_pixel(0, 0, 4);
        let out = bg.compose_over(&top);
        assert_eq!(out.pixels, vec![4, 9]);
    }

    #[test]
    fn to_rgba_uses_palette_and_transparency() {
        let mut colors = [[0u8; 3]; 256];
        colors[1] = [10, 20, 30];
        let pal = Palette { colors };
        let mut s = Surface::new(2, 1, 1);
        s.put_pixel(1, 0, TRANSPARENT);
        assert_eq!(s.to_rgba(&pal), vec![10, 20, 30, 255, 0, 0, 0, 0]);
    }

    #[test]
    fn draw_line_horizontal() {
        let mut s = Surface::new(5, 1, 0);
        s.draw_line(0, 0, 4, 0, 7);
        // The original draws `dx` pixels (start inclusive, end exclusive).
        assert_eq!(s.pixels, vec![7, 7, 7, 7, 0]);
    }
}
