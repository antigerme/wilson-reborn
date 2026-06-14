// SPDX-License-Identifier: GPL-3.0-or-later
//! The island scenery: background, raft, clouds, palm tree, shore waves and holiday
//! props. Faithful port of `island.c` (`islandInit`/`islandAnimate`/`islandInitHoliday`).
//!
//! [`Island::build`] paints the static scene onto an opaque background [`Surface`]
//! (the screen `OCEAN/NIGHT`, the raft, clouds, the island/tree, and — at low tide —
//! the shore and rock), then primes the shore waves. [`Island::animate_waves`] steps
//! the looping wave animation. Holiday props are kept on a separate layer.

use wilson_dgds::{Archive, BmpImage, Palette};

use crate::error::{EngineError, Result};
use crate::rng::Rng;
use crate::story::{Holiday, IslandState};
use crate::surface::{Surface, TRANSPARENT};
use crate::ttm_exec::detect_transparent;

/// The rendered island scenery.
#[derive(Debug, Clone)]
pub struct Island {
    background: Surface,
    holiday: Option<Surface>,
    backgrnd: Vec<BmpImage>,
    low_tide: bool,
    dx: i32,
    dy: i32,
    wave_c1: i32,
    wave_c2: i32,
}

impl Island {
    /// Build the island scenery for the given [`IslandState`].
    pub fn build(
        archive: &Archive,
        state: &IslandState,
        palette: &Palette,
        width: u16,
        height: u16,
        rng: &mut Rng,
    ) -> Result<Self> {
        let transparent_src = detect_transparent(palette);
        let dx = state.x_pos;
        let dy = state.y_pos;

        // Background screen.
        let mut background = Surface::new(width, height, 0);
        let scr_name = if state.night {
            "NIGHT.SCR".to_string()
        } else {
            format!("OCEAN0{}.SCR", rng.below(3))
        };
        let scr = archive
            .scr(&scr_name)
            .ok_or_else(|| EngineError::ResourceNotFound(scr_name.clone()))?;
        for sy in 0..i32::from(scr.height) {
            for sx in 0..i32::from(scr.width) {
                let p = scr.pixels[(sy * i32::from(scr.width) + sx) as usize];
                background.put_pixel(sx, sy, p);
            }
        }

        // Raft (its own sheet), offset by the island position.
        let raft = load_remapped(archive, "MRAFT.BMP", transparent_src)?;
        if state.raft >= 1 {
            let (xr, yr) = if state.low_tide {
                (529, 281)
            } else {
                (512, 266)
            };
            draw(
                &mut background,
                &raft,
                (state.raft - 1) as usize,
                xr + dx,
                yr + dy,
                false,
            );
        }

        let backgrnd = load_remapped(archive, "BACKGRND.BMP", transparent_src)?;

        // Clouds (drawn at absolute coordinates, no island offset).
        let wind = rng.below(2) == 1;
        let num_clouds = pick_num_clouds(rng);
        for _ in 0..num_clouds {
            let cloud_no = rng.below(3);
            let (cx, cy) = match cloud_no {
                0 => (rng.below(640 - 129) as i32, rng.below(135 - 36) as i32),
                1 => (rng.below(640 - 192) as i32, rng.below(135 - 57) as i32),
                _ => (rng.below(640 - 264) as i32, rng.below(135 - 76) as i32),
            };
            draw(
                &mut background,
                &backgrnd,
                15 + cloud_no as usize,
                cx,
                cy,
                !wind,
            );
        }

        // The island, palm tree and (at low tide) shore + rock.
        draw(&mut background, &backgrnd, 0, 288 + dx, 279 + dy, false); // island
        draw(&mut background, &backgrnd, 13, 442 + dx, 148 + dy, false); // trunk
        draw(&mut background, &backgrnd, 12, 365 + dx, 122 + dy, false); // leaves
        draw(&mut background, &backgrnd, 14, 396 + dx, 279 + dy, false); // shadow
        if state.low_tide {
            draw(&mut background, &backgrnd, 1, 249 + dx, 303 + dy, false); // shore
            draw(&mut background, &backgrnd, 2, 150 + dx, 328 + dy, false); // rock
        }

        let mut island = Island {
            background,
            holiday: None,
            backgrnd,
            low_tide: state.low_tide,
            dx,
            dy,
            wave_c1: 0,
            wave_c2: 0,
        };

        // Prime the shore waves.
        for _ in 0..4 {
            island.animate_waves();
        }

        // Holiday prop on its own layer.
        if state.holiday != Holiday::None {
            let mut layer = Surface::new(width, height, TRANSPARENT);
            let holiday = load_remapped(archive, "HOLIDAY.BMP", transparent_src)?;
            let (sprite, x, y) = match state.holiday {
                Holiday::Halloween => (0, 410, 298),
                Holiday::StPatrick => (1, 333, 286),
                Holiday::Christmas => (2, 404, 267),
                Holiday::NewYear => (3, 361, 155),
                Holiday::None => unreachable!(),
            };
            draw(&mut layer, &holiday, sprite, x + dx, y + dy, false);
            island.holiday = Some(layer);
        }

        Ok(island)
    }

    /// The painted background (opaque).
    pub fn background(&self) -> &Surface {
        &self.background
    }

    /// The holiday prop layer (transparent), if a holiday is active.
    pub fn holiday_layer(&self) -> Option<&Surface> {
        self.holiday.as_ref()
    }

    /// The island position offset applied to scenery (and to be applied to Johnny).
    pub fn offset(&self) -> (i32, i32) {
        (self.dx, self.dy)
    }

    /// Redraw the palm trunk and leaves over `target` (used when Johnny walks behind
    /// the tree, so he is occluded by it).
    pub fn redraw_tree(&self, target: &mut Surface) {
        draw(
            target,
            &self.backgrnd,
            13,
            442 + self.dx,
            148 + self.dy,
            false,
        );
        draw(
            target,
            &self.backgrnd,
            12,
            365 + self.dx,
            122 + self.dy,
            false,
        );
    }

    /// Step the looping shore-wave animation (draws the next wave frame).
    pub fn animate_waves(&mut self) {
        if self.low_tide {
            self.wave_c2 = (self.wave_c2 + 1) % 4;
            let c1 = self.wave_c1 as usize;
            match self.wave_c2 {
                0 => draw(
                    &mut self.background,
                    &self.backgrnd,
                    39 + c1,
                    129 + self.dx,
                    340 + self.dy,
                    false,
                ),
                1 => draw(
                    &mut self.background,
                    &self.backgrnd,
                    30 + c1,
                    233 + self.dx,
                    323 + self.dy,
                    false,
                ),
                2 => draw(
                    &mut self.background,
                    &self.backgrnd,
                    33 + c1,
                    367 + self.dx,
                    356 + self.dy,
                    false,
                ),
                _ => draw(
                    &mut self.background,
                    &self.backgrnd,
                    36 + c1,
                    558 + self.dx,
                    323 + self.dy,
                    false,
                ),
            }
        } else {
            self.wave_c2 = (self.wave_c2 + 1) % 3;
            let c1 = self.wave_c1 as usize;
            match self.wave_c2 {
                0 => draw(
                    &mut self.background,
                    &self.backgrnd,
                    3 + c1,
                    270 + self.dx,
                    306 + self.dy,
                    false,
                ),
                1 => draw(
                    &mut self.background,
                    &self.backgrnd,
                    6 + c1,
                    364 + self.dx,
                    319 + self.dy,
                    false,
                ),
                _ => draw(
                    &mut self.background,
                    &self.backgrnd,
                    9 + c1,
                    518 + self.dx,
                    303 + self.dy,
                    false,
                ),
            }
        }
        if self.wave_c2 == 0 {
            self.wave_c1 = (self.wave_c1 + 1) % 3;
        }
    }
}

fn draw(target: &mut Surface, images: &[BmpImage], idx: usize, x: i32, y: i32, flip: bool) {
    if let Some(img) = images.get(idx) {
        target.blit(
            img.width,
            img.height,
            &img.pixels,
            x,
            y,
            Some(TRANSPARENT),
            flip,
            None,
        );
    }
}

fn load_remapped(
    archive: &Archive,
    name: &str,
    transparent_src: Option<u8>,
) -> Result<Vec<BmpImage>> {
    let bmp = archive
        .bmp(name)
        .ok_or_else(|| EngineError::ResourceNotFound(name.to_string()))?;
    Ok(bmp
        .images
        .iter()
        .map(|im| BmpImage {
            width: im.width,
            height: im.height,
            pixels: im
                .pixels
                .iter()
                .map(|&p| {
                    if Some(p) == transparent_src {
                        TRANSPARENT
                    } else {
                        p
                    }
                })
                .collect(),
        })
        .collect())
}

fn pick_num_clouds(rng: &mut Rng) -> u32 {
    if rng.below(2) == 1 {
        1
    } else if rng.below(2) == 1 {
        0
    } else if rng.below(4) != 0 {
        2
    } else if rng.below(4) != 0 {
        3
    } else if rng.below(4) != 0 {
        4
    } else {
        5
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wilson_dgds::{Bmp, Scr};

    fn solid_bmp(count: usize, value: u8) -> Bmp {
        Bmp {
            width: 2,
            height: 2,
            images: (0..count)
                .map(|_| BmpImage {
                    width: 2,
                    height: 2,
                    pixels: vec![value; 4],
                })
                .collect(),
        }
    }

    fn archive() -> Archive {
        // One distinct value per sprite kind so we can locate them.
        let island_sheet = {
            // 42 images so both high- and low-tide wave indices are valid.
            let mut b = solid_bmp(42, 1);
            b.images[0] = BmpImage {
                width: 2,
                height: 2,
                pixels: vec![7; 4],
            }; // island
            b
        };
        Archive {
            bitmaps: vec![
                ("BACKGRND.BMP".to_string(), island_sheet),
                ("MRAFT.BMP".to_string(), solid_bmp(5, 9)),
                ("HOLIDAY.BMP".to_string(), solid_bmp(4, 5)),
            ],
            screens: (0..3)
                .map(|i| {
                    (
                        format!("OCEAN0{i}.SCR"),
                        Scr {
                            width: 640,
                            height: 480,
                            pixels: vec![3; 640 * 480],
                        },
                    )
                })
                .collect(),
            ..Default::default()
        }
    }

    fn state(low_tide: bool, raft: u8, holiday: Holiday) -> IslandState {
        IslandState {
            low_tide,
            night: false,
            raft,
            holiday,
            x_pos: 0,
            y_pos: 0,
        }
    }

    #[test]
    fn builds_background_with_island_and_raft() {
        let arch = archive();
        let pal = Palette {
            colors: [[0u8; 3]; 256],
        };
        let mut rng = Rng::new(5);
        let isl = Island::build(
            &arch,
            &state(false, 5, Holiday::None),
            &pal,
            640,
            480,
            &mut rng,
        )
        .unwrap();

        let bg = isl.background();
        assert_eq!(bg.get(600, 400), Some(3)); // bare ocean (SCR)
        assert_eq!(bg.get(288, 279), Some(7)); // island sprite (value 7)
        assert_eq!(bg.get(512, 266), Some(9)); // raft sprite (high tide, value 9)
        assert!(isl.holiday_layer().is_none());
    }

    #[test]
    fn low_tide_and_animation_do_not_panic() {
        let arch = archive();
        let pal = Palette {
            colors: [[0u8; 3]; 256],
        };
        let mut rng = Rng::new(9);
        let mut isl = Island::build(
            &arch,
            &state(true, 3, Holiday::None),
            &pal,
            640,
            480,
            &mut rng,
        )
        .unwrap();
        for _ in 0..30 {
            isl.animate_waves();
        }
    }

    #[test]
    fn holiday_layer_present() {
        let arch = archive();
        let pal = Palette {
            colors: [[0u8; 3]; 256],
        };
        let mut rng = Rng::new(1);
        let isl = Island::build(
            &arch,
            &state(false, 1, Holiday::Christmas),
            &pal,
            640,
            480,
            &mut rng,
        )
        .unwrap();
        let layer = isl.holiday_layer().expect("holiday layer");
        assert_eq!(layer.get(404, 267), Some(5)); // Christmas tree sprite (value 5)
                                                  // Most of the layer is transparent.
        assert_eq!(layer.get(0, 0), Some(TRANSPARENT));
    }
}
