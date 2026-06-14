// SPDX-License-Identifier: GPL-3.0-or-later
//! A tiny deterministic PRNG so scene selection is reproducible in tests.

/// A small xorshift64 generator.
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    /// Create a generator from `seed` (forced non-zero).
    pub fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }

    /// Next pseudo-random `u32`.
    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        (x >> 32) as u32
    }

    /// A value in `0..n` (returns 0 when `n == 0`), matching `rand() % n`.
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 {
            0
        } else {
            self.next_u32() % n
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..1000 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
    }

    #[test]
    fn bounded() {
        let mut r = Rng::new(7);
        for _ in 0..1000 {
            assert!(r.below(7) < 7);
        }
        assert_eq!(r.below(0), 0);
    }
}
