// SPDX-License-Identifier: GPL-3.0-or-later
//! Asset loading: a built-in **recreated** (copyright-free, procedural) asset pack so
//! the app runs standalone, plus a loader for the user's original `RESOURCE.*` data.
//!
//! The pack is drawn entirely in code (no copied data): a gradient ocean (day & night,
//! with a moon and stars), a shaded sand island with a palm tree and its shadow, puffy
//! clouds, animated shore foam, a raft that grows with the story day, simple holiday
//! props, and a little castaway. Sprite sizes and the values the engine blits them at
//! (see [`crate`]'s engine `island`/`show`) are matched so the scene composes correctly.

use std::fs;
use std::path::Path;

use wilson_dgds::{Ads, AdsRes, Archive, Bmp, BmpImage, Palette, ResourceMap, Scr, Tag, Ttm};

/// Palette index used as the transparent colour key (magenta — the engine detects it).
pub const TRANSPARENT_INDEX: u8 = 15;

// --- Named palette indices --------------------------------------------------
const BLACK: u8 = 0;
const FOAM: u8 = 1; // white foam / cloud / stars glint
const OCEAN: u8 = 2; // ocean mid (day)
const SAND: u8 = 3; // sand (light)
const TRUNK: u8 = 4; // palm trunk (dark)
const LEAF: u8 = 5; // palm frond (dark)
const SKIN: u8 = 6;
const SHIRT: u8 = 7; // ragged shirt
const SHADOWSAND: u8 = 8; // tree shadow on sand
const RAFT: u8 = 9; // raft log
const NIGHT_OCEAN: u8 = 10; // night ocean mid
const OCEAN_HI: u8 = 11; // ocean light (toward the horizon)
const OCEAN_LO: u8 = 12; // ocean deep
const OCEAN_LOLO: u8 = 13; // ocean deepest (foreground)
const HORIZON: u8 = 14; // hazy band at the horizon
const SAND_MID: u8 = 16;
const SAND_DK: u8 = 17; // wet sand at the waterline
const LEAF_HI: u8 = 18; // palm frond (light)
const TRUNK_HI: u8 = 19; // palm trunk (light side / rings)
const HAIR: u8 = 20;
const PANTS: u8 = 21; // ragged shorts
const NIGHT_HI: u8 = 22; // night ocean light
const NIGHT_LO: u8 = 23; // night ocean deep
const MOON: u8 = 24;
const ROPE: u8 = 25; // raft lashings
const STAR: u8 = 26;
const PUMPKIN: u8 = 27;
const GOLD: u8 = 28; // star / coins / fireworks
const ROCK: u8 = 29;
const SKY: u8 = 30; // day sky (cyan)
const SKY_HI: u8 = 31; // day sky near the horizon (paler)
const HAZE: u8 = 32; // hazy horizon band
const NSKY: u8 = 33; // night sky (deep blue)
const NSKY_HI: u8 = 34; // night sky near the horizon

/// The recreated palette (index 15 is the magenta transparency key).
pub fn demo_palette() -> Palette {
    let mut colors = [[0u8; 3]; 256];
    colors[BLACK as usize] = [0, 0, 0];
    colors[FOAM as usize] = [240, 248, 255];
    colors[OCEAN as usize] = [46, 104, 170];
    colors[SAND as usize] = [234, 206, 116]; // golden sand
    colors[TRUNK as usize] = [168, 80, 50]; // warm reddish palm trunk
    colors[LEAF as usize] = [34, 124, 52];
    colors[SKIN as usize] = [236, 196, 150];
    colors[SHIRT as usize] = [198, 74, 60];
    colors[SHADOWSAND as usize] = [176, 150, 98];
    colors[RAFT as usize] = [156, 116, 64];
    colors[NIGHT_OCEAN as usize] = [16, 30, 68];
    colors[OCEAN_HI as usize] = [92, 156, 208];
    colors[OCEAN_LO as usize] = [32, 78, 140];
    colors[OCEAN_LOLO as usize] = [22, 60, 112];
    colors[HORIZON as usize] = [150, 196, 224];
    colors[TRANSPARENT_INDEX as usize] = [168, 0, 168]; // transparent key
    colors[SAND_MID as usize] = [212, 180, 92];
    colors[SAND_DK as usize] = [176, 144, 74];
    colors[LEAF_HI as usize] = [74, 176, 84];
    colors[TRUNK_HI as usize] = [202, 112, 66];
    colors[HAIR as usize] = [78, 50, 28];
    colors[PANTS as usize] = [110, 140, 166];
    colors[NIGHT_HI as usize] = [36, 60, 108];
    colors[NIGHT_LO as usize] = [8, 16, 40];
    colors[MOON as usize] = [232, 232, 206];
    colors[ROPE as usize] = [112, 82, 46];
    colors[STAR as usize] = [220, 225, 240];
    colors[PUMPKIN as usize] = [230, 130, 30];
    colors[GOLD as usize] = [230, 200, 90];
    colors[ROCK as usize] = [120, 120, 132];
    colors[SKY as usize] = [78, 214, 226];
    colors[SKY_HI as usize] = [158, 236, 240];
    colors[HAZE as usize] = [150, 152, 140];
    colors[NSKY as usize] = [30, 42, 120];
    colors[NSKY_HI as usize] = [54, 70, 150];
    Palette { colors }
}

// --- A tiny indexed pixel-art canvas ----------------------------------------

/// A small indexed drawing surface (transparent by default) for building sprites.
struct Canvas {
    w: i32,
    h: i32,
    px: Vec<u8>,
}

impl Canvas {
    fn new(w: i32, h: i32) -> Self {
        Canvas {
            w,
            h,
            px: vec![TRANSPARENT_INDEX; (w * h) as usize],
        }
    }

    fn solid(w: i32, h: i32, color: u8) -> Self {
        Canvas {
            w,
            h,
            px: vec![color; (w * h) as usize],
        }
    }

    #[inline]
    fn set(&mut self, x: i32, y: i32, color: u8) {
        if x >= 0 && y >= 0 && x < self.w && y < self.h {
            self.px[(y * self.w + x) as usize] = color;
        }
    }

    fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: u8) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.set(xx, yy, color);
            }
        }
    }

    fn hspan(&mut self, x0: i32, x1: i32, y: i32, color: u8) {
        for x in x0..=x1 {
            self.set(x, y, color);
        }
    }

    fn ellipse(&mut self, cx: i32, cy: i32, rx: i32, ry: i32, color: u8) {
        if rx <= 0 || ry <= 0 {
            return;
        }
        for y in -ry..=ry {
            for x in -rx..=rx {
                if x * x * ry * ry + y * y * rx * rx <= rx * rx * ry * ry {
                    self.set(cx + x, cy + y, color);
                }
            }
        }
    }

    fn disc(&mut self, cx: i32, cy: i32, r: i32, color: u8) {
        self.ellipse(cx, cy, r, r, color);
    }

    fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: u8) {
        let (mut x0, mut y0) = (x0, y0);
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        loop {
            self.set(x0, y0, color);
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    fn into_image(self) -> BmpImage {
        BmpImage {
            width: self.w as u16,
            height: self.h as u16,
            pixels: self.px,
        }
    }
}

/// A cheap deterministic noise hash for sand/water texture.
fn hash2(x: i32, y: i32) -> u32 {
    let mut h = (x as u32).wrapping_mul(374_761_393) ^ (y as u32).wrapping_mul(668_265_263);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^ (h >> 16)
}

// --- Background screens (full-screen gradients) -----------------------------

const BAYER4: [[i32; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];

/// Ordered-dithered pick from a vertical colour ramp (`num/den` is top→bottom).
fn ramp_at(ramp: &[u8], num: i32, den: i32, x: i32, y: i32) -> u8 {
    let segs = (ramp.len() as i32 - 1).max(1);
    let scaled = num * segs;
    let i = (scaled / den) as usize;
    if i >= ramp.len() - 1 {
        return ramp[ramp.len() - 1];
    }
    let frac = scaled - (i as i32) * den;
    let bayer = BAYER4[(y & 3) as usize][(x & 3) as usize];
    if frac * 16 >= bayer * den {
        ramp[i + 1]
    } else {
        ramp[i]
    }
}

/// Y of the horizon (sky above, ocean below) in the 640×480 screens.
const HORIZON_Y: i32 = 168;

/// A daytime scene background: cyan sky, a hazy horizon band, then the ocean
/// gradient with scattered foam streaks (denser toward the foreground).
fn ocean_scr(seed: u32) -> Scr {
    let (w, h) = (640i32, 480i32);
    let sky = [SKY, SKY, SKY_HI];
    let sea = [OCEAN_HI, OCEAN, OCEAN_LO, OCEAN_LOLO];
    let mut px = vec![0u8; (w * h) as usize];
    for y in 0..h {
        for x in 0..w {
            let c = if y < HORIZON_Y - 10 {
                ramp_at(&sky, y, HORIZON_Y - 10, x, y)
            } else if y < HORIZON_Y {
                if ((x + y) & 1) == 0 {
                    HAZE
                } else {
                    SKY_HI
                }
            } else {
                ramp_at(&sea, y - HORIZON_Y, h - HORIZON_Y, x, y)
            };
            px[(y * w + x) as usize] = c;
        }
    }
    // Foam streaks on the water for texture.
    let mut s = seed | 1;
    let mut next = || {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        s
    };
    for _ in 0..170 {
        let r = next();
        let x = (r % 600) as i32 + 12;
        let y = HORIZON_Y + 6 + ((r >> 9) % (h - HORIZON_Y - 12) as u32) as i32;
        let len = 4 + ((r >> 20) % 16) as i32;
        let color = if (r >> 5) & 7 == 0 { FOAM } else { OCEAN_HI };
        for k in 0..len {
            let xx = x + k;
            if xx < w && y < h {
                px[(y * w + xx) as usize] = color;
            }
        }
    }
    Scr {
        width: w as u16,
        height: h as u16,
        pixels: px,
    }
}

/// The night scene background: a deep-blue starry sky, a moon, then a near-black
/// ocean with a broad wavy moon-glitter reflection.
fn night_scr() -> Scr {
    let (w, h) = (640i32, 480i32);
    let sky = [NSKY, NSKY, NSKY_HI];
    let sea = [NIGHT_OCEAN, NIGHT_LO, NIGHT_LO];
    let mut c = Canvas::solid(w, h, NIGHT_OCEAN);
    for y in 0..h {
        for x in 0..w {
            let col = if y < HORIZON_Y {
                ramp_at(&sky, y, HORIZON_Y, x, y)
            } else {
                ramp_at(&sea, y - HORIZON_Y, h - HORIZON_Y, x, y)
            };
            c.set(x, y, col);
        }
    }
    // Stars in the sky.
    for i in 0..110i32 {
        let r = hash2(i, 7);
        let x = (r % 640) as i32;
        let y = ((r >> 10) % (HORIZON_Y - 8) as u32) as i32;
        c.set(x, y, if (r >> 3) & 3 == 0 { FOAM } else { STAR });
    }
    // Moon + soft glow.
    let (mx, my) = (150, 60);
    c.disc(mx, my, 24, NSKY_HI);
    c.disc(mx, my, 20, MOON);
    c.disc(mx - 7, my - 6, 6, FOAM);
    // Broad wavy glitter reflection on the water below the moon.
    for y in HORIZON_Y + 4..h {
        let spread = 6 + (y - HORIZON_Y) / 8;
        for k in 0..7 {
            let r = hash2(mx + k * 13, y);
            let off = (r % (spread as u32 * 2 + 1)) as i32 - spread;
            if !(r >> 5).is_multiple_of(3) {
                let color = if (r >> 9) & 3 == 0 { MOON } else { NIGHT_HI };
                c.set(mx + off, y, color);
            }
        }
    }
    Scr {
        width: c.w as u16,
        height: c.h as u16,
        pixels: c.px,
    }
}

// --- BACKGRND.BMP sheet sprites ---------------------------------------------

/// The sand island (sprite 0): a flat golden islet ringed by foam, sitting on the
/// water — a shaded sand ellipse with a darker patch, wet rim and speckle texture.
fn island_sprite() -> BmpImage {
    let (w, h) = (240i32, 96i32);
    let mut c = Canvas::new(w, h);
    let (cx, cy, rx, ry) = (120, 40, 116, 34);
    // Foam ring, then the sand just inside it.
    c.ellipse(cx, cy + 2, rx, ry, FOAM);
    c.ellipse(cx, cy, rx - 6, ry - 5, SAND);
    // Shade: a darker beach patch, a wet rim, and speckle texture.
    for y in 0..h {
        for x in 0..w {
            if c.px[(y * w + x) as usize] != SAND {
                continue;
            }
            // Distance from the sand centre (normalised, ×100).
            let dx = (x - cx) * 100 / (rx - 6);
            let dy = (y - cy) * 100 / (ry - 5);
            let d2 = dx * dx + dy * dy;
            let color = if d2 > 8200 {
                SAND_DK // wet rim near the waterline
            } else if y > cy + 4 {
                SAND_MID // shaded lower beach
            } else {
                SAND
            };
            // A soft darker patch (a dune shadow) left of centre.
            let pdx = x - (cx - 28);
            let pdy = y - (cy + 6);
            let color = if pdx * pdx + pdy * pdy * 4 < 900 {
                SHADOWSAND
            } else {
                color
            };
            let color = match hash2(x, y) % 23 {
                0 => SAND_DK,
                1 | 2 => SAND,
                _ => color,
            };
            c.set(x, y, color);
        }
    }
    c.into_image()
}

/// The palm trunk (sprite 13): a slightly curved two-tone trunk with rings.
fn trunk_sprite() -> BmpImage {
    let (w, h) = (22i32, 152i32);
    let mut c = Canvas::new(w, h);
    for y in 0..h {
        // Gentle lean: the trunk bows to the right toward the top.
        let bend = (h - y) * (h - y) / 1400; // 0 at bottom, ~16 at top
        let center = 9 + bend / 2;
        let half = 4 + y / 60; // a touch thicker toward the base
        for x in center - half..=center + half {
            let color = if x <= center - half + 1 {
                TRUNK_HI
            } else {
                TRUNK
            };
            c.set(x, y, color);
        }
        if y % 13 == 0 {
            c.hspan(center - half, center + half, y, TRUNK_HI); // ring
        }
    }
    c.into_image()
}

/// The palm fronds (sprite 12): big drooping fronds radiating from the trunk top.
fn leaves_sprite() -> BmpImage {
    let (w, h) = (184i32, 104i32);
    let mut c = Canvas::new(w, h);
    let (ax, ay) = (78, 30); // attach point (the trunk top, in sprite-local coords)
                             // Frond directions (dx, dy) and lengths — wide spread, drooping down.
    let fronds = [
        (-72, 0, 78),
        (-60, 24, 74),
        (-34, 42, 66),
        (-6, 50, 60),
        (22, 44, 66),
        (52, 28, 74),
        (74, 4, 80),
        (-30, -24, 60),
        (26, -22, 64),
    ];
    for &(dx, dy, len) in &fronds {
        for s in 0..len {
            let t = s as f32 / len as f32;
            let droop = (t * t * 16.0) as i32; // tips bend downward
            let x = ax + dx * s / len;
            let y = ay + dy * s / len + droop;
            let thick = (3.5 * (1.0 - t)) as i32 + 1; // taper to a point
            for o in -thick..=thick {
                c.set(x, y + o, LEAF);
            }
            c.set(x, y - thick, LEAF_HI); // lit upper edge
            if s % 3 == 0 {
                c.set(x, y, LEAF_HI); // central rib
            }
        }
    }
    // Coconuts at the crown.
    c.disc(ax - 5, ay + 6, 3, TRUNK);
    c.disc(ax + 6, ay + 7, 3, TRUNK);
    c.disc(ax + 1, ay + 9, 3, TRUNK);
    c.into_image()
}

/// The tree shadow on the sand (sprite 14).
fn shadow_sprite() -> BmpImage {
    let (w, h) = (104i32, 20i32);
    let mut c = Canvas::new(w, h);
    c.ellipse(52, 10, 50, 8, SHADOWSAND);
    c.into_image()
}

/// The wet-sand strip exposed at low tide (sprite 1).
fn shore_sprite() -> BmpImage {
    let (w, h) = (150i32, 22i32);
    let mut c = Canvas::new(w, h);
    c.ellipse(75, 11, 73, 9, SAND_DK);
    c.ellipse(75, 8, 66, 6, SAND_MID);
    c.into_image()
}

/// A rock exposed at low tide (sprite 2).
fn rock_sprite() -> BmpImage {
    let (w, h) = (48i32, 30i32);
    let mut c = Canvas::new(w, h);
    c.ellipse(24, 20, 22, 10, ROCK);
    c.ellipse(20, 14, 14, 9, ROCK);
    // A couple of highlights.
    for i in 0..40 {
        let r = hash2(i, 3);
        let x = (r % 40) as i32 + 4;
        let y = ((r >> 8) % 22) as i32 + 6;
        if c.px[(y * w + x) as usize] == ROCK && r & 3 == 0 {
            c.set(x, y, FOAM);
        }
    }
    c.into_image()
}

/// A foam wave ribbon (`w` wide): dashed scallops of foam, scrolled by `phase`.
fn wave_sprite(w: i32, phase: i32) -> BmpImage {
    let h = 12i32;
    let mut c = Canvas::new(w, h);
    let mut x = 0;
    while x < w {
        let lift = (x / 10 + phase) % 3; // gentle height variation
        let y = 6 - lift;
        let dash = 3 + (hash2(x, phase) % 3) as i32;
        c.hspan(x, (x + dash).min(w - 1), y, FOAM);
        c.hspan(x + 1, (x + dash - 1).min(w - 1), y - 1, FOAM);
        c.hspan(x, (x + dash).min(w - 1), y + 1, OCEAN_HI);
        x += dash + 4 + (hash2(x, phase + 1) % 4) as i32; // gap between scallops
    }
    c.into_image()
}

/// A puffy cloud (`w`×`h`): a solid body with bumpy top lobes and a shaded base.
fn cloud_sprite(w: i32, h: i32) -> BmpImage {
    let mut c = Canvas::new(w, h);
    let by = h * 2 / 3; // baseline of the flat bottom
                        // Solid body so there are no internal gaps.
    c.rect(w / 10, by - h / 5, w * 8 / 10, h / 3, FOAM);
    // Bumpy top lobes.
    let lobes = (w / 30).max(3);
    for i in 0..=lobes {
        let x = (w * i / lobes).clamp(h / 3, w - h / 3);
        let r = h / 4 + (hash2(i, 1) % (h as u32 / 5 + 1)) as i32;
        c.disc(x, by - h / 5, r, FOAM);
    }
    // One clean shaded row along the underside.
    for x in 0..w {
        for y in (0..h - 1).rev() {
            if c.px[(y * w + x) as usize] == FOAM {
                c.set(x, y + 1, HORIZON);
                break;
            }
        }
    }
    c.into_image()
}

/// The 42-image `BACKGRND.BMP` sheet, at the indices the engine expects.
fn backgrnd_sheet() -> Bmp {
    let mut images: Vec<BmpImage> = (0..42).map(|_| Canvas::new(1, 1).into_image()).collect();
    images[0] = island_sprite();
    images[1] = shore_sprite();
    images[2] = rock_sprite();
    // High-tide waves: groups at 3.., 6.., 9.. (3 phases each).
    for g in 0..3 {
        let width = [104, 92, 92][g];
        for p in 0..3 {
            images[3 + g * 3 + p] = wave_sprite(width, p as i32);
        }
    }
    images[12] = leaves_sprite();
    images[13] = trunk_sprite();
    images[14] = shadow_sprite();
    images[15] = cloud_sprite(120, 40);
    images[16] = cloud_sprite(180, 56);
    images[17] = cloud_sprite(248, 74);
    // Low-tide waves: groups at 30.., 33.., 36.., 39.. (3 phases each).
    for g in 0..4 {
        let width = [150, 132, 150, 132][g];
        for p in 0..3 {
            images[30 + g * 3 + p] = wave_sprite(width, p as i32);
        }
    }
    Bmp {
        width: 240,
        height: 96,
        images,
    }
}

// --- Raft, holiday props and Johnny -----------------------------------------

/// One raft build stage (0–4): more, longer lashed logs as the story progresses
/// (parallel logs seen from above, tied with two ropes).
fn raft_frame(stage: usize) -> BmpImage {
    let (w, h) = (60i32, 26i32);
    let mut c = Canvas::new(w, h);
    let logs = 3 + stage as i32; // 3..7 logs
    let log_h = 12 + stage as i32 * 2;
    let (x0, y0) = (6, 4);
    for i in 0..logs {
        let x = x0 + i * 6;
        if x + 4 >= w {
            break;
        }
        c.rect(x, y0, 5, log_h.min(h - y0 - 2), RAFT);
        c.line(x, y0, x, y0 + log_h.min(h - y0 - 2) - 1, ROPE); // gap between logs
    }
    // Two rope lashings across the logs.
    let span = (x0 + logs * 6 - 2).min(w - 2);
    for ry in [y0 + 2, y0 + log_h.min(h - y0 - 2) - 3] {
        c.hspan(x0, span, ry, ROPE);
    }
    c.into_image()
}

/// `MRAFT.BMP`: five raft build stages.
fn raft_sheet() -> Bmp {
    Bmp {
        width: 60,
        height: 26,
        images: (0..5).map(raft_frame).collect(),
    }
}

/// `HOLIDAY.BMP`: pumpkin, pot-o'-gold, Christmas tree, fireworks.
fn holiday_sheet() -> Bmp {
    // 0: Halloween pumpkin.
    let pumpkin = {
        let mut c = Canvas::new(34, 34);
        c.ellipse(17, 20, 15, 12, PUMPKIN);
        c.rect(16, 5, 3, 6, TRUNK); // stem
        c.set(11, 18, BLACK);
        c.set(12, 18, BLACK);
        c.set(22, 18, BLACK);
        c.set(23, 18, BLACK); // eyes
        c.hspan(13, 21, 25, BLACK); // mouth
        c.into_image()
    };
    // 1: St Patrick's pot of gold.
    let pot = {
        let mut c = Canvas::new(34, 34);
        c.ellipse(17, 24, 13, 8, BLACK); // pot
        c.rect(5, 16, 24, 8, BLACK);
        c.ellipse(17, 15, 11, 4, GOLD); // gold
        c.disc(13, 13, 2, GOLD);
        c.disc(21, 12, 2, GOLD);
        c.disc(17, 11, 2, GOLD);
        c.into_image()
    };
    // 2: Christmas tree.
    let tree = {
        let (w, h) = (34i32, 46i32);
        let mut c = Canvas::new(w, h);
        c.rect(15, h - 8, 4, 8, TRUNK);
        for (i, ty) in [8, 20, 30].iter().enumerate() {
            let half = 6 + i as i32 * 5;
            for y in 0..12 {
                let hw = half * y / 12;
                c.hspan(17 - hw, 17 + hw, ty + y, LEAF);
            }
        }
        c.disc(17, 6, 2, GOLD); // star
        for i in 0..10 {
            let r = hash2(i, 9);
            let x = (r % 24) as i32 + 5;
            let y = ((r >> 8) % 32) as i32 + 10;
            if c.px[(y * w + x) as usize] == LEAF {
                c.set(x, y, [SHIRT, GOLD, FOAM][(r % 3) as usize]);
            }
        }
        c.into_image()
    };
    // 3: New Year fireworks burst.
    let fireworks = {
        let (w, h) = (44i32, 44i32);
        let mut c = Canvas::new(w, h);
        let (cx, cy) = (22, 22);
        let colors = [GOLD, SHIRT, FOAM, LEAF_HI];
        for k in 0..16 {
            let ang = k as f32 * std::f32::consts::PI / 8.0;
            let ex = cx + (ang.cos() * 19.0) as i32;
            let ey = cy + (ang.sin() * 19.0) as i32;
            c.line(cx, cy, ex, ey, colors[(k % 4) as usize]);
            c.set(ex, ey, FOAM);
        }
        c.disc(cx, cy, 2, GOLD);
        c.into_image()
    };
    Bmp {
        width: 44,
        height: 46,
        images: vec![pumpkin, pot, tree, fireworks],
    }
}

/// A recreated castaway pose for the standalone vignettes.
#[derive(Clone, Copy)]
enum Pose {
    Stand,
    Wave,
    Fish,
    Read,
}

/// Bottom-anchor a 16×32 figure into a 16×64 sprite, so that — blitted by its top-left
/// at the walk/scene Y — the feet land on the island (the spots sit well above the
/// island's top row).
fn embed_tall(fig: &Canvas) -> BmpImage {
    let mut c = Canvas::new(16, 64);
    for y in 0..32 {
        for x in 0..16 {
            let p = fig.px[(y * 16 + x) as usize];
            if p != TRANSPARENT_INDEX {
                c.set(x, y + 32, p);
            }
        }
    }
    c.into_image()
}

/// Head, torso, ragged shorts and legs (with an optional walk `stride`). No arms.
fn draw_castaway_base(c: &mut Canvas, stride: u8) {
    // Head: hair, face, eyes, a hint of a castaway beard.
    c.rect(5, 1, 6, 3, HAIR);
    c.set(4, 2, HAIR);
    c.set(11, 2, HAIR);
    c.rect(5, 4, 6, 4, SKIN);
    c.set(6, 5, BLACK);
    c.set(9, 5, BLACK);
    c.hspan(5, 10, 7, HAIR); // beard
    c.set(7, 8, SKIN);
    c.set(8, 8, SKIN); // neck
                       // Torso: a ragged shirt with a torn hem.
    c.rect(4, 9, 8, 7, SHIRT);
    for x in 4..12 {
        if x % 2 == 0 {
            c.set(x, 16, SHIRT);
        }
    }
    // Ragged shorts.
    c.rect(4, 17, 8, 4, PANTS);
    // Legs + a small stride offset (only the walk poses stride).
    let (lf, rf) = match stride % 4 {
        1 => (2, -1),
        3 => (-1, 2),
        _ => (0, 0),
    };
    c.rect(5, 21, 2, 5, SKIN);
    c.rect(9, 21, 2, 5, SKIN);
    for y in 26..31 {
        c.set(5 + lf.clamp(-1, 0), y, SKIN);
        c.set(6, y, SKIN);
        c.set(9, y, SKIN);
        c.set(10 + rf.clamp(0, 1), y, SKIN);
    }
    c.hspan(4 + lf, 7 + lf, 31, SKIN);
    c.hspan(9 + rf, 12 + rf, 31, SKIN);
}

/// Both arms hanging at the sides.
fn arms_down(c: &mut Canvas) {
    c.rect(3, 9, 1, 7, SKIN);
    c.rect(12, 9, 1, 7, SKIN);
    c.set(3, 16, SKIN);
    c.set(12, 16, SKIN);
}

/// The castaway in a walk phase (used by `JOHNWALK.BMP`), 16×64 bottom-anchored.
fn johnny_pose(phase: u8) -> BmpImage {
    let mut c = Canvas::new(16, 32);
    draw_castaway_base(&mut c, phase);
    arms_down(&mut c);
    embed_tall(&c)
}

/// A recreated action pose for the standalone scenes, 16×64 bottom-anchored. `phase`
/// drives sub-frame motion (e.g. the wave).
fn johnny_action(pose: Pose, phase: u8) -> BmpImage {
    let mut c = Canvas::new(16, 32);
    draw_castaway_base(&mut c, 0);
    match pose {
        Pose::Stand => arms_down(&mut c),
        Pose::Wave => {
            // Left arm down, right arm raised and waving.
            c.rect(3, 9, 1, 7, SKIN);
            c.set(3, 16, SKIN);
            let up = if phase.is_multiple_of(2) { 0 } else { 1 };
            c.line(12, 10, 14, 4 + up, SKIN); // raised arm
            c.set(14, 3 + up, SKIN); // hand
        }
        Pose::Fish => {
            // Both hands forward (right), holding a rod angled up to the corner.
            c.rect(12, 11, 1, 3, SKIN);
            c.set(13, 11, SKIN);
            c.line(12, 12, 15, 3, TRUNK_HI); // rod
            c.line(15, 3, 15, 15, FOAM); // line down to the water
        }
        Pose::Read => {
            // Hands forward holding an open book at chest height.
            c.rect(3, 11, 1, 3, SKIN);
            c.rect(12, 11, 1, 3, SKIN);
            c.rect(4, 11, 8, 4, FOAM); // pages
            c.set(7, 11, ROCK); // spine
            c.set(8, 11, ROCK);
        }
    }
    embed_tall(&c)
}

/// Mary, the mermaid (`JDEMO.BMP` frame 4): she sits in the water beside the island —
/// flowing hair, a shell top and a green fish tail. Bottom-anchored in 16×64.
fn mary_sprite() -> BmpImage {
    let mut c = Canvas::new(16, 64);
    // She floats higher than Johnny's feet, so draw from y≈30 down.
    // Hair + head.
    c.rect(5, 30, 7, 4, GOLD); // long blonde hair
    c.set(4, 31, GOLD);
    c.set(12, 31, GOLD);
    c.rect(6, 33, 5, 4, SKIN); // face
    c.set(7, 35, BLACK);
    c.set(9, 35, BLACK); // eyes
    c.hspan(4, 5, 37, GOLD); // hair down the shoulders
    c.hspan(11, 12, 37, GOLD);
    // Torso + shell top.
    c.rect(6, 38, 5, 4, SKIN);
    c.set(6, 39, SHIRT);
    c.set(10, 39, SHIRT); // shell cups
    c.rect(4, 39, 1, 3, SKIN); // arms
    c.rect(12, 39, 1, 3, SKIN);
    // Green tail: widening, then a fin.
    for y in 42..56 {
        let half = 2 + (y - 42) / 5;
        let color = if (y - 42) % 3 == 0 { LEAF_HI } else { LEAF };
        c.hspan(8 - half, 8 + half, y, color);
    }
    // Tail fin.
    c.hspan(2, 6, 58, LEAF_HI);
    c.hspan(10, 14, 58, LEAF_HI);
    c.hspan(3, 6, 60, LEAF);
    c.hspan(10, 13, 60, LEAF);
    c.into_image()
}

fn op(c: u16) -> [u8; 2] {
    c.to_le_bytes()
}

/// A two-figure TTM for Mary's scenes: Johnny on the island and the mermaid in the
/// water beside him, both gently bobbing.
fn mary_ttm() -> Ttm {
    let mut code = Vec::new();
    code.extend_from_slice(&op(0x1111)); // TAG 1
    code.extend_from_slice(&1u16.to_le_bytes());
    code.extend_from_slice(&op(0x1051)); // SET_BMP_SLOT 0
    code.extend_from_slice(&0u16.to_le_bytes());
    code.extend_from_slice(&op(0xF02F)); // LOAD_IMAGE "JDEMO.BMP"
    code.extend_from_slice(b"JDEMO.BMP\0");
    code.extend_from_slice(&op(0x1021)); // SET_DELAY 8
    code.extend_from_slice(&8u16.to_le_bytes());
    // (johnny_y, mary_y) per step — they bob out of phase.
    for &(jy, my) in &[(250u16, 300u16), (248, 302), (250, 300), (248, 298)] {
        code.extend_from_slice(&op(0xA601)); // CLEAR
        code.extend_from_slice(&0u16.to_le_bytes());
        // Johnny (frame 0) at the island.
        code.extend_from_slice(&op(0xA504));
        for v in [330u16, jy, 0, 0] {
            code.extend_from_slice(&v.to_le_bytes());
        }
        // Mary (frame 4) in the water to his left.
        code.extend_from_slice(&op(0xA504));
        for v in [250u16, my, 4, 0] {
            code.extend_from_slice(&v.to_le_bytes());
        }
        code.extend_from_slice(&op(0x0FF0)); // UPDATE
    }
    Ttm {
        version: "1.20".into(),
        num_pages: 1,
        bytecode: code,
        tags: vec![Tag {
            id: 1,
            description: "mary".into(),
        }],
    }
}

/// A TTM that animates the castaway through `steps` of `(sprite_frame, y)`, drawing
/// `JDEMO.BMP` frame `sprite_frame` at `(312, y)` each step (gently bobbing/acting).
fn vignette_ttm(steps: &[(u16, u16)]) -> Ttm {
    let mut code = Vec::new();
    code.extend_from_slice(&op(0x1111)); // TAG 1
    code.extend_from_slice(&1u16.to_le_bytes());
    code.extend_from_slice(&op(0x1051)); // SET_BMP_SLOT 0
    code.extend_from_slice(&0u16.to_le_bytes());
    code.extend_from_slice(&op(0xF02F)); // LOAD_IMAGE "JDEMO.BMP"
    code.extend_from_slice(b"JDEMO.BMP\0"); // 10 bytes (even)
    code.extend_from_slice(&op(0x1021)); // SET_DELAY 8
    code.extend_from_slice(&8u16.to_le_bytes());
    for &(frame, y) in steps {
        code.extend_from_slice(&op(0xA601)); // CLEAR_SCREEN
        code.extend_from_slice(&0u16.to_le_bytes());
        code.extend_from_slice(&op(0xA504)); // DRAW_SPRITE 312 y frame 0
        for v in [312u16, y, frame, 0] {
            code.extend_from_slice(&v.to_le_bytes());
        }
        code.extend_from_slice(&op(0x0FF0)); // UPDATE
    }
    Ttm {
        version: "1.20".into(),
        num_pages: 1,
        bytecode: code,
        tags: vec![Tag {
            id: 1,
            description: "vignette".into(),
        }],
    }
}

/// The recreated vignette TTMs, by name (each plays a distinct action).
fn vignette_ttms() -> Vec<(String, Ttm)> {
    // JDEMO.BMP frames: 0 = stand, 1 = wave, 2 = fish, 3 = read.
    vec![
        (
            "STAND.TTM".to_string(),
            vignette_ttm(&[(0, 250), (0, 247), (0, 250)]),
        ),
        // Wave: alternate the raised-arm frame with the arm-down stand frame.
        (
            "WAVE.TTM".to_string(),
            vignette_ttm(&[(1, 250), (0, 250), (1, 250), (0, 250)]),
        ),
        (
            "FISH.TTM".to_string(),
            vignette_ttm(&[(2, 250), (2, 248), (2, 250)]),
        ),
        (
            "READ.TTM".to_string(),
            vignette_ttm(&[(3, 250), (3, 249), (3, 250)]),
        ),
        ("MARY.TTM".to_string(), mary_ttm()),
    ]
}

/// The recreated action that best fits each `.ADS` category.
fn ttm_for_ads(ads_name: &str) -> &'static str {
    match ads_name {
        "FISHING.ADS" => "FISH.TTM",
        "ACTIVITY.ADS" => "READ.TTM",
        "MARY.ADS" => "MARY.TTM", // the mermaid appears beside Johnny
        "STAND.ADS" | "WALKSTUF.ADS" | "BUILDING.ADS" => "STAND.TTM",
        // Story/character/visitor/gag scenes: a friendly wave.
        _ => "WAVE.TTM",
    }
}

/// A minimal ADS that plays `ttm_name` once (works for any requested tag).
fn demo_ads(ttm_name: &str) -> Ads {
    let mut code = Vec::new();
    code.extend_from_slice(&op(0x0001)); // tag 1
    code.extend_from_slice(&op(0x2005)); // ADD_SCENE slot1 tag1 0 0
    for v in [1u16, 1, 0, 0] {
        code.extend_from_slice(&v.to_le_bytes());
    }
    code.extend_from_slice(&op(0x1510)); // PLAY_SCENE
    code.extend_from_slice(&op(0xFFFF)); // END
    Ads {
        version: "1.20".into(),
        resources: vec![AdsRes {
            id: 1,
            name: ttm_name.to_string(),
        }],
        bytecode: code,
        tags: vec![Tag {
            id: 1,
            description: "demo".into(),
        }],
    }
}

/// Build the built-in recreated asset pack.
pub fn demo_archive() -> (Archive, Palette) {
    let ads_names = [
        "ACTIVITY.ADS",
        "BUILDING.ADS",
        "FISHING.ADS",
        "JOHNNY.ADS",
        "MARY.ADS",
        "MISCGAG.ADS",
        "STAND.ADS",
        "SUZY.ADS",
        "VISITOR.ADS",
        "WALKSTUF.ADS",
    ];
    let archive = Archive {
        bitmaps: vec![
            ("BACKGRND.BMP".to_string(), backgrnd_sheet()),
            ("MRAFT.BMP".to_string(), raft_sheet()),
            ("HOLIDAY.BMP".to_string(), holiday_sheet()),
            (
                "JDEMO.BMP".to_string(),
                Bmp {
                    width: 16,
                    height: 64,
                    // Frames: 0 = stand, 1 = wave, 2 = fish, 3 = read, 4 = Mary.
                    images: vec![
                        johnny_action(Pose::Stand, 0),
                        johnny_action(Pose::Wave, 0),
                        johnny_action(Pose::Fish, 0),
                        johnny_action(Pose::Read, 0),
                        mary_sprite(),
                    ],
                },
            ),
            (
                "JOHNWALK.BMP".to_string(),
                Bmp {
                    width: 16,
                    height: 64,
                    // 64 frames (walk sprite ids are < 64); cycle four walk poses.
                    images: (0..64).map(|i| johnny_pose((i % 4) as u8)).collect(),
                },
            ),
        ],
        screens: vec![
            ("OCEAN00.SCR".to_string(), ocean_scr(0x1234_5678)),
            ("OCEAN01.SCR".to_string(), ocean_scr(0x2BAD_F00D)),
            ("OCEAN02.SCR".to_string(), ocean_scr(0x0C0F_FEE1)),
            ("NIGHT.SCR".to_string(), night_scr()),
        ],
        ttms: vignette_ttms(),
        // Each scene category plays its fitting recreated action.
        ads: ads_names
            .iter()
            .map(|n| (n.to_string(), demo_ads(ttm_for_ads(n))))
            .collect(),
        ..Default::default()
    };
    (archive, demo_palette())
}

/// Load the user's original `RESOURCE.MAP` + data file from `dir`.
pub fn load_real(dir: &Path) -> Result<(Archive, Palette), String> {
    let map_bytes = fs::read(dir.join("RESOURCE.MAP")).map_err(|e| format!("RESOURCE.MAP: {e}"))?;
    let map = ResourceMap::parse(&map_bytes).map_err(|e| e.to_string())?;
    let archive_bytes = fs::read(dir.join(&map.data_file_name))
        .map_err(|e| format!("{}: {e}", map.data_file_name))?;
    let archive = Archive::parse(&map_bytes, &archive_bytes).map_err(|e| e.to_string())?;
    let palette = archive.palette().cloned().unwrap_or_else(demo_palette);
    Ok((archive, palette))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wilson_engine::{Clock, Director, Show};

    #[test]
    fn palette_has_transparency_key() {
        // The engine detects transparency by the magenta key; it must be present.
        assert_eq!(
            demo_palette().colors[TRANSPARENT_INDEX as usize],
            [168, 0, 168]
        );
    }

    #[test]
    fn backgrnd_sheet_has_all_indices_the_engine_blits() {
        let sheet = backgrnd_sheet();
        assert_eq!(sheet.images.len(), 42);
        // The island, tree and a high- and low-tide wave must be real sprites.
        for idx in [0usize, 12, 13, 14, 3, 30, 41] {
            assert!(sheet.images[idx].width > 1, "sprite {idx} is a placeholder");
        }
    }

    #[test]
    fn recreated_scenes_vary_by_category() {
        // Different scene categories play different recreated actions.
        assert_eq!(ttm_for_ads("FISHING.ADS"), "FISH.TTM");
        assert_eq!(ttm_for_ads("ACTIVITY.ADS"), "READ.TTM");
        assert_eq!(ttm_for_ads("STAND.ADS"), "STAND.TTM");
        assert_eq!(ttm_for_ads("MARY.ADS"), "MARY.TTM");
        assert_ne!(ttm_for_ads("FISHING.ADS"), ttm_for_ads("ACTIVITY.ADS"));

        // Every referenced TTM exists in the pack.
        let (archive, _) = demo_archive();
        for name in ["STAND.TTM", "WAVE.TTM", "FISH.TTM", "READ.TTM", "MARY.TTM"] {
            assert!(archive.ttm(name).is_some(), "missing {name}");
        }
        // Each ADS references the action chosen for its category.
        for (name, ads) in &archive.ads {
            assert_eq!(ads.resources[0].name, ttm_for_ads(name));
        }
    }

    #[test]
    fn action_poses_are_distinct() {
        // JDEMO has the four action poses plus Mary, and they actually differ.
        let (archive, _) = demo_archive();
        let jdemo = archive.bmp("JDEMO.BMP").unwrap();
        assert_eq!(jdemo.images.len(), 5);
        let stand = &johnny_action(Pose::Stand, 0).pixels;
        let fish = &johnny_action(Pose::Fish, 0).pixels;
        let read = &johnny_action(Pose::Read, 0).pixels;
        let mary = &mary_sprite().pixels;
        assert_ne!(stand, fish);
        assert_ne!(stand, read);
        assert_ne!(fish, read);
        assert_ne!(stand, mary);
    }

    #[test]
    fn demo_archive_renders_island_and_castaway() {
        let (archive, palette) = demo_archive();
        let director = Director::new(5, 100);
        let clock = Clock {
            yday: 100,
            hour: 12,
            month: 6,
            day: 14,
        };
        let mut show = Show::new(&archive, &palette, 640, 480, director, clock, 1);

        // Over a few hundred frames, some frame must contain sand/leaf/skin pixels —
        // colours that only exist in the island, palm and castaway, never the ocean.
        let scenery = [SAND, SAND_MID, SAND_DK, LEAF, LEAF_HI, TRUNK, SKIN];
        let mut saw_scenery = false;
        for _ in 0..300 {
            let f = show.next_frame(&archive);
            assert_eq!(f.surface.pixels.len(), 640 * 480);
            if f.surface.pixels.iter().any(|p| scenery.contains(p)) {
                saw_scenery = true;
            }
        }
        assert!(saw_scenery, "expected island/palm/castaway pixels");
    }

    /// End-to-end render check / screenshot tool. A no-op unless `WILSON_DUMP=<dir>`
    /// is set, in which case it writes representative scenes (day/night/Christmas) as
    /// PPM into `<dir>`. Set `WILSON_REAL_DIR=<data dir>` to render the user's original
    /// `RESOURCE.*` instead of the recreated pack (used to compare against the original).
    #[test]
    fn dump_scenes_when_requested() {
        let Ok(out_dir) = std::env::var("WILSON_DUMP") else {
            return;
        };
        let (archive, palette, prefix) = match std::env::var("WILSON_REAL_DIR") {
            Ok(dir) => {
                let (a, p) = load_real(Path::new(&dir)).expect("load real data");
                (a, p, "real")
            }
            Err(_) => {
                let (a, p) = demo_archive();
                (a, p, "demo")
            }
        };
        for (name, hour, month, day) in [
            ("day", 12u8, 6u8, 14u8),
            ("night", 0, 6, 14),
            ("xmas", 12, 12, 24),
        ] {
            let director = Director::new(5, 100);
            let clock = Clock {
                yday: 100,
                hour,
                month,
                day,
            };
            let mut show = Show::new(&archive, &palette, 640, 480, director, clock, 3);
            // Pick the frame with the most sand that also shows the castaway.
            let mut best = show.next_frame(&archive).surface;
            let mut best_sand = best.pixels.iter().filter(|&&p| p == SAND).count();
            for _ in 0..500 {
                let f = show.next_frame(&archive);
                let sand = f.surface.pixels.iter().filter(|&&p| p == SAND).count();
                if sand > best_sand && f.surface.pixels.contains(&SKIN) {
                    best_sand = sand;
                    best = f.surface;
                }
            }
            let mut out = format!("P6\n{} {}\n255\n", best.width, best.height).into_bytes();
            for &p in &best.pixels {
                out.extend_from_slice(&palette.colors[p as usize]);
            }
            let path = format!("{out_dir}/wilson_{prefix}_{name}.ppm");
            std::fs::write(&path, out).expect("write ppm");
        }
    }
}
