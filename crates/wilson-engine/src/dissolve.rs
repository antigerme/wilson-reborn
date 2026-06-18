// SPDX-License-Identifier: GPL-3.0-or-later
//! The original's **tiled dissolve** transition (opt-in).
//!
//! The 1992 `SCRANTIC.EXE` carries a transition effect that ships **disabled** (dead code,
//! gated by a flag that is statically 0 — see KB10 §10.2): a screen divided into cells, with
//! an **LFSR** visiting every cell in a fixed pseudo-random order, blitting each from the new
//! image over the old, paced over several frames. This module resurrects that effect for an
//! **opt-in** `--transition dissolve` (the faithful default stays a hard cut).
//!
//! The cell-visit order uses the **original's own LFSR feedback masks** (read from the data
//! segment at `seg14:0x27fe`): maximal-length taps for 2–11-bit registers, so the register
//! width is the smallest that covers the cell count and the sequence visits each cell once.

use crate::surface::Surface;

/// Maximal-length LFSR feedback masks for register widths 2..=11, transcribed verbatim from
/// the original's table at `seg14:0x27fe`. Index `w-2` is the mask for a `w`-bit register.
const LFSR_TAPS: [u16; 10] = [0x3, 0x6, 0xC, 0x14, 0x30, 0x60, 0xB8, 0x110, 0x240, 0x500];

/// The square cell size, in pixels (the dissolve granularity). 16 px over 640×480 gives
/// 40×30 = 1200 cells, comfortably inside the original's 11-bit (≤2047) LFSR range.
pub const CELL: u16 = 16;

/// How many frames the dissolve is spread over (~`STEPS × 16 ms` ≈ a third of a second).
pub const STEPS: u16 = 20;

/// The pseudo-random order in which the `n` cells are revealed, using the original's LFSR
/// masks. Returns a permutation of `0..n`. For `n > 2047` (beyond the original's 11-bit
/// table) it falls back to sequential order.
pub fn lfsr_order(n: usize) -> Vec<usize> {
    if n <= 1 {
        return (0..n).collect();
    }
    // Smallest register width whose maximal period (2^w - 1) covers every cell.
    let mut width = 2u32;
    while (1usize << width) - 1 < n {
        width += 1;
    }
    if width > 11 {
        return (0..n).collect(); // outside the original's table — defensive fallback
    }
    let mask = LFSR_TAPS[(width - 2) as usize];
    let mut order = Vec::with_capacity(n);
    let mut lfsr: u16 = 1; // any non-zero seed; the maximal sequence covers all non-zero values
    loop {
        let v = lfsr as usize; // in 1..=2^w-1
        if v <= n {
            order.push(v - 1); // 1-based LFSR value → 0-based cell index
        }
        // Galois step: shift right, XOR the mask back in when a 1 falls off the bottom.
        let lsb = lfsr & 1;
        lfsr >>= 1;
        if lsb != 0 {
            lfsr ^= mask;
        }
        if lfsr == 1 {
            break; // returned to the seed ⇒ full period visited
        }
    }
    order
}

/// An in-progress tiled dissolve from one surface to another.
#[derive(Debug)]
pub struct Dissolve {
    work: Surface,     // the progressively revealed image (starts as the "from" surface)
    to: Surface,       // the destination
    order: Vec<usize>, // cell-visit order (an LFSR permutation of 0..num_cells)
    revealed: usize,   // how many cells have been revealed so far
    per_step: usize,   // cells revealed per [`Dissolve::step`]
    cells_x: u16,      // cells per row
}

impl Dissolve {
    /// Begin a dissolve from `from` to `to` (same dimensions), spread over ~[`STEPS`] frames
    /// of [`CELL`]-pixel cells in the original's LFSR order.
    pub fn new(from: Surface, to: Surface) -> Self {
        let cells_x = from.width.div_ceil(CELL);
        let cells_y = from.height.div_ceil(CELL);
        let num = usize::from(cells_x) * usize::from(cells_y);
        let order = lfsr_order(num);
        let per_step = num.div_ceil(STEPS as usize).max(1);
        Dissolve {
            work: from,
            to,
            order,
            revealed: 0,
            per_step,
            cells_x,
        }
    }

    /// Whether every cell has been revealed (the dissolve has reached the destination).
    pub fn done(&self) -> bool {
        self.revealed >= self.order.len()
    }

    /// The current composited image (without advancing).
    pub fn image(&self) -> &Surface {
        &self.work
    }

    /// Reveal the next batch of cells and return the current composited image.
    pub fn step(&mut self) -> &Surface {
        let end = (self.revealed + self.per_step).min(self.order.len());
        for i in self.revealed..end {
            let cell = self.order[i]; // copied out, so `self` is free to borrow mutably below
            self.reveal(cell);
        }
        self.revealed = end;
        &self.work
    }

    /// Consume the dissolve, yielding the destination surface (for the final frame).
    pub fn into_destination(self) -> Surface {
        self.to
    }

    /// Copy one `CELL`-sized cell from the destination into the working image.
    fn reveal(&mut self, cell: usize) {
        let w = self.work.width;
        let h = self.work.height;
        let cx = (cell as u16 % self.cells_x) * CELL;
        let cy = (cell as u16 / self.cells_x) * CELL;
        let stride = usize::from(w);
        for y in cy..(cy + CELL).min(h) {
            let row = usize::from(y) * stride;
            let x0 = usize::from(cx);
            let x1 = usize::from((cx + CELL).min(w));
            self.work.pixels[row + x0..row + x1]
                .copy_from_slice(&self.to.pixels[row + x0..row + x1]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lfsr_order_is_a_full_permutation() {
        // The dissolve must reveal every cell exactly once (so it always reaches the
        // destination), for register widths 2..=11 and arbitrary in-between counts.
        for n in [2usize, 3, 7, 8, 16, 17, 100, 255, 256, 1200, 2047] {
            let order = lfsr_order(n);
            assert_eq!(order.len(), n, "n={n}: wrong length");
            let mut seen = vec![false; n];
            for &c in &order {
                assert!(c < n, "n={n}: index {c} out of range");
                assert!(!seen[c], "n={n}: index {c} repeated (not a permutation)");
                seen[c] = true;
            }
            assert!(seen.iter().all(|&s| s), "n={n}: not all cells visited");
        }
    }

    #[test]
    fn lfsr_order_is_not_trivially_sequential() {
        // It is a genuine pseudo-random walk, not 0,1,2,… (that's the whole point).
        let order = lfsr_order(1200);
        assert_ne!(order, (0..1200).collect::<Vec<_>>());
    }

    #[test]
    fn dissolve_reaches_the_destination_exactly() {
        // Start fully "from" (all 1s), end fully "to" (all 9s); stepping to completion must
        // yield the destination pixel-for-pixel, and `done()` must trip.
        let from = Surface::new(64, 48, 1);
        let to = Surface::new(64, 48, 9);
        let mut d = Dissolve::new(from, to.clone());
        let mut guard = 0;
        while !d.done() {
            d.step();
            guard += 1;
            assert!(guard < 10_000, "dissolve never completed");
        }
        assert_eq!(d.step(), &to); // a final step past completion is the destination
        assert_eq!(d.into_destination().pixels, to.pixels);
    }

    #[test]
    fn dissolve_is_partial_midway() {
        // Midway, the image is a mix of both — neither all-from nor all-to.
        let from = Surface::new(64, 48, 1);
        let to = Surface::new(64, 48, 9);
        let mut d = Dissolve::new(from, to);
        d.step(); // one batch revealed
        let mid = d.step().clone();
        assert!(
            mid.pixels.contains(&1),
            "expected some original still showing"
        );
        assert!(
            mid.pixels.contains(&9),
            "expected some destination revealed"
        );
    }
}
