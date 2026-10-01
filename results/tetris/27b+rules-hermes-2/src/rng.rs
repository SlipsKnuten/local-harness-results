//! A small deterministic-friendly xorshift64 PRNG (no external dependency).

use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    /// The raw seed (useful as a deterministic starting value).
    pub fn value(&self) -> u64 {
        self.0
    }

    /// Seed from the wall clock.
    pub fn seed() -> Self {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E3779B97F4A7C15);
        Self::with_seed(ts)
    }

    /// Seed from an explicit value (zero maps to a fixed constant so the
    /// internal state is never degenerate).
    pub fn with_seed(value: u64) -> Self {
        Self(if value == 0 {
            0x9E3779B97F4A7C15
        } else {
            value
        })
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    /// Fisher-Yates shuffle.
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = (self.next_u64() % (i as u64 + 1)) as usize;
            items.swap(i, j);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Kind;

    #[test]
    fn shuffle_is_a_permutation() {
        let mut rng = Rng::with_seed(42);
        let mut values = Kind::all();
        rng.shuffle(&mut values);
        let mut sorted = values;
        sorted.sort_by_key(|k| k.as_idx());
        assert_eq!(sorted, Kind::all());
    }

    #[test]
    fn same_seed_same_sequence() {
        let mut a = Rng::with_seed(7);
        let mut b = Rng::with_seed(7);
        for _ in 0..32 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn zero_seed_does_not_stay_zero() {
        let mut rng = Rng::with_seed(0);
        assert_ne!(rng.next_u64(), 0);
    }
}
