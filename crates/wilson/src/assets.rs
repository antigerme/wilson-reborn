// SPDX-License-Identifier: GPL-3.0-or-later
//! Asset loading: a built-in **recreated** (copyright-free, procedural) asset pack so
//! the app runs standalone, plus a loader for the user's original `RESOURCE.*` data.
//!
//! The demo pack uses simple coloured shapes (a sand island with a palm, an ocean, a
//! little walking figure). It is the seed of a redistributable pack; richer recreated
//! art can replace these sprites without touching the engine.

use std::fs;
use std::path::Path;

use wilson_dgds::{Ads, AdsRes, Archive, Bmp, BmpImage, Palette, ResourceMap, Scr, Tag, Ttm};

/// Palette index used as the transparent colour key in the demo pack.
pub const TRANSPARENT_INDEX: u8 = 15;

/// The demo palette (index 15 is the magenta transparency key the engine detects).
pub fn demo_palette() -> Palette {
    let mut colors = [[0u8; 3]; 256];
    colors[0] = [0, 0, 0];
    colors[1] = [240, 240, 255]; // foam / clouds
    colors[2] = [40, 90, 170]; // ocean
    colors[3] = [225, 205, 140]; // sand
    colors[4] = [120, 80, 40]; // trunk
    colors[5] = [40, 150, 60]; // leaves
    colors[6] = [235, 195, 150]; // skin
    colors[7] = [200, 60, 60]; // shirt
    colors[8] = [30, 60, 110]; // shadow on water
    colors[9] = [150, 110, 60]; // raft
    colors[10] = [10, 20, 60]; // night ocean
    colors[15] = [168, 0, 168]; // transparent key
    Palette { colors }
}

fn filled(w: u16, h: u16, color: u8) -> BmpImage {
    BmpImage {
        width: w,
        height: h,
        pixels: vec![color; w as usize * h as usize],
    }
}

fn transparent_px() -> BmpImage {
    filled(1, 1, TRANSPARENT_INDEX)
}

/// A simple 16×32 humanoid (head/body/legs) over a transparent background.
fn johnny() -> BmpImage {
    let (w, h) = (16usize, 32usize);
    let mut px = vec![TRANSPARENT_INDEX; w * h];
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if y < 8 && (5..11).contains(&x) {
                px[i] = 6; // head
            } else if (8..24).contains(&y) && (3..13).contains(&x) {
                px[i] = 7; // body
            } else if y >= 24 && (4..12).contains(&x) {
                px[i] = 4; // legs
            }
        }
    }
    BmpImage {
        width: w as u16,
        height: h as u16,
        pixels: px,
    }
}

fn backgrnd_sheet() -> Bmp {
    let mut images: Vec<BmpImage> = (0..42).map(|_| transparent_px()).collect();
    images[0] = filled(140, 80, 3); // island
    images[1] = filled(120, 16, 3); // low-tide shore
    images[2] = filled(40, 20, 8); // rock
    images[12] = filled(70, 44, 5); // leaves
    images[13] = filled(14, 90, 4); // trunk
    images[14] = filled(120, 14, 8); // shadow
    for img in images[3..12].iter_mut() {
        *img = filled(60, 10, 1); // high-tide wave frames
    }
    for img in images[30..42].iter_mut() {
        *img = filled(60, 10, 1); // low-tide wave frames
    }
    for img in images[15..18].iter_mut() {
        *img = filled(120, 40, 1); // clouds
    }
    Bmp {
        width: 140,
        height: 80,
        images,
    }
}

fn op(c: u16) -> [u8; 2] {
    c.to_le_bytes()
}

/// A TTM that draws the figure standing (bobbing) for a few frames, then ends.
fn jdemo_ttm() -> Ttm {
    let mut code = Vec::new();
    code.extend_from_slice(&op(0x1111)); // TAG 1
    code.extend_from_slice(&1u16.to_le_bytes());
    code.extend_from_slice(&op(0x1051)); // SET_BMP_SLOT 0
    code.extend_from_slice(&0u16.to_le_bytes());
    code.extend_from_slice(&op(0xF02F)); // LOAD_IMAGE "JDEMO.BMP"
    code.extend_from_slice(b"JDEMO.BMP\0"); // 10 bytes (even)
    code.extend_from_slice(&op(0x1021)); // SET_DELAY 8
    code.extend_from_slice(&8u16.to_le_bytes());
    for y in [250u16, 246, 250] {
        code.extend_from_slice(&op(0xA601)); // CLEAR_SCREEN 0
        code.extend_from_slice(&0u16.to_le_bytes());
        code.extend_from_slice(&op(0xA504)); // DRAW_SPRITE 312 y 0 0
        for v in [312u16, y, 0, 0] {
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
            description: "stand".into(),
        }],
    }
}

/// A minimal ADS that plays the standing TTM once (works for any requested tag).
fn demo_ads() -> Ads {
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
            name: "JDEMO.TTM".into(),
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
    let ocean = |idx: u8| Scr {
        width: 640,
        height: 480,
        pixels: vec![idx; 640 * 480],
    };
    let archive = Archive {
        bitmaps: vec![
            ("BACKGRND.BMP".to_string(), backgrnd_sheet()),
            (
                "MRAFT.BMP".to_string(),
                Bmp {
                    width: 46,
                    height: 18,
                    images: (0..5).map(|_| filled(46, 18, 9)).collect(),
                },
            ),
            (
                "HOLIDAY.BMP".to_string(),
                Bmp {
                    width: 30,
                    height: 44,
                    images: vec![
                        filled(28, 34, 5),
                        filled(28, 30, 5),
                        filled(30, 40, 5),
                        filled(24, 44, 1),
                    ],
                },
            ),
            (
                "JDEMO.BMP".to_string(),
                Bmp {
                    width: 16,
                    height: 32,
                    images: vec![johnny()],
                },
            ),
            (
                "JOHNWALK.BMP".to_string(),
                Bmp {
                    width: 16,
                    height: 32,
                    images: (0..40).map(|_| johnny()).collect(),
                },
            ),
        ],
        screens: vec![
            ("OCEAN00.SCR".to_string(), ocean(2)),
            ("OCEAN01.SCR".to_string(), ocean(2)),
            ("OCEAN02.SCR".to_string(), ocean(2)),
            ("NIGHT.SCR".to_string(), ocean(10)),
        ],
        ttms: vec![("JDEMO.TTM".to_string(), jdemo_ttm())],
        ads: ads_names
            .iter()
            .map(|n| (n.to_string(), demo_ads()))
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
    fn demo_archive_runs_and_shows_more_than_ocean() {
        let (archive, palette) = demo_archive();
        let director = Director::new(5, 100);
        let clock = Clock {
            yday: 100,
            hour: 12,
            month: 6,
            day: 14,
        };
        let mut show = Show::new(&archive, &palette, 640, 480, director, clock, 1);

        // Over a few hundred frames, at least one frame must contain non-ocean pixels
        // (the island/Johnny), proving the recreated pack actually renders.
        let mut saw_scenery = false;
        for _ in 0..300 {
            let f = show.next_frame(&archive);
            assert_eq!(f.surface.pixels.len(), 640 * 480);
            if f.surface.pixels.iter().any(|&p| p != 2 && p != 10) {
                saw_scenery = true;
            }
        }
        assert!(saw_scenery, "expected island/Johnny pixels over the ocean");
    }
}
