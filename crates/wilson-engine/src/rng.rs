// SPDX-License-Identifier: GPL-3.0-or-later
//! A tiny deterministic PRNG so scene selection is reproducible in tests.

/// A small xorshift64 generator.
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    /// Create a generator from `seed`.
    ///
    /// The seed is run through the splitmix64 finalizer first: a raw small seed (e.g. `0..64`,
    /// common in tests) otherwise leaves xorshift64's high bits unmixed, so the very first
    /// `next_u32` (which returns those high bits) would be `0` for *every* such seed —
    /// collapsing all small seeds to the same first draw. Scrambling spreads any seed across
    /// all 64 bits so the first value is already well distributed.
    pub fn new(seed: u64) -> Self {
        let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        Rng(z | 1)
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

    #[test]
    fn small_seeds_give_a_varied_first_draw() {
        // Regression: before scrambling the seed, `next_u32()` was 0 for every seed in 0..64
        // (xorshift64's high bits stay unmixed for tiny states), so e.g. a weighted pick
        // always took its first option. Confirm the first draw now varies across small seeds.
        let firsts: std::collections::HashSet<u32> =
            (0..64u64).map(|s| Rng::new(s).next_u32()).collect();
        assert!(
            firsts.len() > 50,
            "first draws not varied: {} distinct",
            firsts.len()
        );
        assert!(firsts.iter().any(|&v| v != 0), "first draw is always 0");
    }
}
