//! A small deterministic pseudo-random number generator (`SplitMix64`).
//!
//! Having its own implementation spares an external dependency and makes games
//! reproducible in tests when the seed is fixed.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::time::{SystemTime, UNIX_EPOCH};

/// Pseudo-random number generator.
#[derive(Clone, Debug)]
pub struct Rng {
    state: u64,
}

impl Rng {
    /// A generator with the given seed: the same seed gives the same sequence.
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// A generator with a random seed.
    #[must_use]
    pub fn from_entropy() -> Self {
        Self::new(entropy_seed())
    }

    pub const fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A uniform integer from `0..n`; returns 0 for `n == 0`.
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        // Multiply and shift instead of a remainder: no noticeable bias for small n.
        ((u128::from(self.next_u64()) * n as u128) >> 64) as usize
    }

    /// A uniform real number from `[0, 1)`.
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// True with probability `p`.
    pub fn chance(&mut self, p: f64) -> bool {
        self.unit() < p
    }

    /// A uniform integer from the closed range `[-amplitude, amplitude]`.
    pub fn symmetric(&mut self, amplitude: i32) -> i32 {
        if amplitude <= 0 {
            return 0;
        }
        self.below(2 * amplitude as usize + 1) as i32 - amplitude
    }

    /// A random permutation (Fisher–Yates).
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.below(i + 1);
            items.swap(i, j);
        }
    }

    /// A random element of a slice.
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        if items.is_empty() {
            None
        } else {
            items.get(self.below(items.len()))
        }
    }
}

/// A seed from system sources: the time and the random hashing keys of the standard library.
#[must_use]
pub fn entropy_seed() -> u64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as u64);
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u64(nanos);
    hasher.finish() ^ nanos.rotate_left(32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_sequence() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        let mut c = Rng::new(43);
        let xs: Vec<u64> = (0..8).map(|_| a.next_u64()).collect();
        let ys: Vec<u64> = (0..8).map(|_| b.next_u64()).collect();
        let zs: Vec<u64> = (0..8).map(|_| c.next_u64()).collect();
        assert_eq!(xs, ys);
        assert_ne!(xs, zs);
    }

    #[test]
    fn ranges_are_respected() {
        let mut rng = Rng::new(7);
        let mut seen = [false; 5];
        for _ in 0..1000 {
            let x = rng.below(5);
            assert!(x < 5);
            seen[x] = true;
            let u = rng.unit();
            assert!((0.0..1.0).contains(&u));
            let s = rng.symmetric(3);
            assert!((-3..=3).contains(&s));
        }
        assert!(seen.iter().all(|&s| s));
        assert_eq!(rng.below(0), 0);
        assert_eq!(rng.symmetric(0), 0);
        assert!(!rng.chance(0.0));
        assert!(rng.chance(1.0));
    }

    #[test]
    fn shuffle_keeps_elements() {
        let mut rng = Rng::new(1);
        let mut items: Vec<u32> = (0..20).collect();
        rng.shuffle(&mut items);
        let mut sorted = items.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, (0..20).collect::<Vec<u32>>());
        assert_ne!(items, sorted, "twenty items are almost certainly reordered");
        assert_eq!(rng.pick::<u32>(&[]), None);
        assert!(items.contains(rng.pick(&items).unwrap()));
    }
}
