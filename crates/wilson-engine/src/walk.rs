// SPDX-License-Identifier: GPL-3.0-or-later
//! Johnny's walk animation between island spots.
//!
//! Faithful port of `walk.c` (`walkInit`/`walkAnimate`) over the data in
//! [`crate::walk_data`]. A [`Walker`] is a state machine that, given a start/end
//! spot+heading, yields one [`WalkFrame`] per call (turn → walk → arrive) until
//! Johnny reaches the destination. Rendering is left to the caller: each frame names
//! the sprite (from `JOHNWALK.BMP`), its position, whether it is horizontally
//! flipped, and whether Johnny is currently behind the palm tree.

use crate::path::calc_path;
use crate::rng::Rng;
use crate::walk_data::{
    WALK_BOOKMARKS, WALK_BOOKMARKS_TURNS, WALK_DATA, WALK_END_HEADINGS, WALK_START_HEADINGS,
};

/// One frame of the walk animation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WalkFrame {
    /// Whether the sprite is drawn horizontally flipped.
    pub flip: bool,
    /// X position (island-relative).
    pub x: i32,
    /// Y position (island-relative).
    pub y: i32,
    /// Sprite index within `JOHNWALK.BMP`.
    pub sprite: u16,
    /// How long to show this frame, in ticks (6 walking, 80 on arrival).
    pub delay: u16,
    /// Whether Johnny is passing behind the palm tree (caller redraws trunk/leaves).
    pub behind_tree: bool,
}

/// Walk-animation state machine between two spots.
#[derive(Debug, Clone)]
pub struct Walker {
    path: Vec<u8>,
    path_idx: usize,
    current_spot: i32,
    current_hdg: i32,
    next_spot: i32,
    next_hdg: i32,
    final_spot: i32,
    final_hdg: i32,
    increment: i32,
    last_turn: bool,
    has_arrived: bool,
    behind_tree: bool,
    data: usize,
}

impl Walker {
    /// Begin walking from `(from_spot, from_hdg)` to `(to_spot, to_hdg)`.
    pub fn new(from_spot: u8, from_hdg: u8, to_spot: u8, to_hdg: u8, rng: &mut Rng) -> Self {
        let path = calc_path(from_spot, to_spot, rng);

        let mut w = Walker {
            path,
            path_idx: 0,
            current_spot: i32::from(from_spot),
            current_hdg: i32::from(from_hdg),
            next_spot: -1,
            next_hdg: i32::from(to_hdg),
            final_spot: i32::from(to_spot),
            final_hdg: i32::from(to_hdg),
            increment: 0,
            last_turn: false,
            has_arrived: false,
            behind_tree: false,
            data: 0,
        };

        if w.current_spot == w.final_spot {
            w.next_spot = -1;
            w.next_hdg = w.final_hdg;
            w.last_turn = true;
        } else {
            w.path_idx = 1;
            w.next_spot = i32::from(w.path[w.path_idx]);
            w.next_hdg = WALK_START_HEADINGS[w.current_spot as usize][w.next_spot as usize];
            w.last_turn = false;
        }
        w.increment = turn_increment(w.next_hdg, w.current_hdg);
        w
    }

    /// Whether Johnny has reached the destination.
    pub fn has_arrived(&self) -> bool {
        self.has_arrived
    }

    /// Current spot (A–F as 0–5).
    pub fn current_spot(&self) -> u8 {
        self.current_spot as u8
    }

    /// Current heading (0–7).
    pub fn current_hdg(&self) -> u8 {
        self.current_hdg as u8
    }

    /// Advance the animation by one frame, or `None` once arrival is complete.
    pub fn next_frame(&mut self) -> Option<WalkFrame> {
        if self.has_arrived {
            return None;
        }

        if self.next_hdg != -1 {
            // Turning.
            if (((self.next_hdg - self.current_hdg) & 7) % 7) > 1 {
                self.current_hdg = (self.current_hdg + self.increment) & 7;
                self.data =
                    (WALK_BOOKMARKS_TURNS[self.current_spot as usize] + self.current_hdg) as usize;
                if self.last_turn {
                    self.data += 9;
                }
            } else if self.current_spot != self.final_spot {
                // Turn finished; start walking to the next spot.
                self.next_hdg = -1;
                self.behind_tree = (self.current_spot == 3 && self.next_spot == 4)
                    || (self.current_spot == 4 && self.next_spot == 3);
                self.data =
                    WALK_BOOKMARKS[self.current_spot as usize][self.next_spot as usize] as usize;
            } else {
                // Arrived at the final spot/heading (the final pose reflects
                // `final_hdg`; sync `current_hdg` so the accessor is accurate).
                self.data =
                    (WALK_BOOKMARKS_TURNS[self.final_spot as usize] + self.final_hdg) as usize + 9;
                self.current_hdg = self.final_hdg;
                self.has_arrived = true;
            }
        } else {
            // Walking forward.
            self.data += 1;
            if WALK_DATA[self.data][1] == 0 {
                // Reached a spot; begin a turn.
                self.current_hdg =
                    WALK_END_HEADINGS[self.current_spot as usize][self.next_spot as usize];
                self.current_spot = self.next_spot;

                if self.current_spot != self.final_spot {
                    self.path_idx += 1;
                    self.next_spot = i32::from(self.path[self.path_idx]);
                    self.next_hdg =
                        WALK_START_HEADINGS[self.current_spot as usize][self.next_spot as usize];
                } else {
                    self.next_hdg = self.final_hdg;
                    self.last_turn = true;
                }

                self.increment = turn_increment(self.next_hdg, self.current_hdg);
                self.current_hdg = (self.current_hdg + self.increment) & 7;
                self.data =
                    (WALK_BOOKMARKS_TURNS[self.current_spot as usize] + self.current_hdg) as usize;

                if self.last_turn {
                    self.data += 9;
                    if self.current_hdg == self.final_hdg {
                        self.has_arrived = true;
                    }
                }
            }
        }

        let d = WALK_DATA[self.data];
        Some(WalkFrame {
            flip: d[0] != 0,
            x: i32::from(d[1]) - 1,
            y: i32::from(d[2]),
            sprite: d[3],
            delay: if self.has_arrived { 80 } else { 6 },
            behind_tree: self.behind_tree,
        })
    }
}

/// The rotation step (+1, -1 or 0) to turn from `current` toward `next` heading.
fn turn_increment(next_hdg: i32, current_hdg: i32) -> i32 {
    let delta = (next_hdg - current_hdg) & 7;
    if delta == 0 {
        0
    } else if delta < 4 {
        1
    } else {
        -1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(from_spot: u8, from_hdg: u8, to_spot: u8, to_hdg: u8, seed: u64) -> Vec<WalkFrame> {
        let mut rng = Rng::new(seed);
        let mut w = Walker::new(from_spot, from_hdg, to_spot, to_hdg, &mut rng);
        let mut frames = Vec::new();
        let mut guard = 0;
        while let Some(f) = w.next_frame() {
            frames.push(f);
            guard += 1;
            assert!(guard < 5000, "walk did not terminate");
        }
        assert!(w.has_arrived());
        assert_eq!(w.current_spot(), to_spot);
        assert_eq!(w.current_hdg(), to_hdg);
        frames
    }

    #[test]
    fn walks_between_all_spots() {
        // Every spot pair, with a couple of start/end headings, terminates cleanly
        // and ends in the arrival pose (delay 80).
        for from in 0..6u8 {
            for to in 0..6u8 {
                let frames = run(from, 0, to, 4, 1);
                assert!(!frames.is_empty());
                assert_eq!(frames.last().unwrap().delay, 80);
                // Every frame references a real sprite position.
                for f in &frames {
                    assert!(f.x >= -1);
                    assert!(f.sprite < 64);
                }
            }
        }
    }

    #[test]
    fn turn_in_place_when_same_spot() {
        // Same spot, different heading: only turns, then arrives.
        let frames = run(2, 0, 2, 4, 3);
        assert!(!frames.is_empty());
        assert!(frames.iter().all(|f| !f.behind_tree));
    }

    #[test]
    fn behind_tree_between_d_and_e() {
        // The direct D(3)->E(4) route passes behind the palm tree. calc_path may pick
        // a longer route, so check that at least one seed takes the direct one.
        let found = (0..64u64).any(|seed| {
            let frames = run(3, 0, 4, 0, seed);
            frames.iter().any(|f| f.behind_tree)
        });
        assert!(found, "expected a behind-tree frame on some D->E route");
    }

    #[test]
    fn turn_increment_picks_shortest_rotation() {
        assert_eq!(turn_increment(0, 0), 0);
        assert_eq!(turn_increment(1, 0), 1); // +1
        assert_eq!(turn_increment(7, 0), -1); // -1 (shorter than +7)
        assert_eq!(turn_increment(4, 0), -1); // delta 4 -> -1 (`<4 ? 1 : -1`)
        assert_eq!(turn_increment(5, 0), -1); // delta 5 -> -1
        assert_eq!(turn_increment(3, 0), 1); // delta 3 -> +1
    }
}
