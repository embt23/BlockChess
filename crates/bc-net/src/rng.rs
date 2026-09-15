//! Seeded randomness, and nothing else.
//!
//! Deliberately four lines and deliberately not cryptographic. The only
//! property required is that the same seed gives the same sequence on every
//! machine forever — which rules out `rand`'s thread-local generators and
//! anything whose algorithm may be improved in a point release.

/// xorshift64*.
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        // Zero is xorshift's fixed point; nudge rather than trusting every
        // caller to avoid it.
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform in `lo..=hi`.
    pub fn between(&mut self, lo: u64, hi: u64) -> u64 {
        if hi <= lo {
            return lo;
        }
        lo + self.next_u64() % (hi - lo + 1)
    }

    /// True with probability `pct`/100.
    pub fn chance(&mut self, pct: u64) -> bool {
        self.next_u64() % 100 < pct
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_seed_gives_the_same_sequence() {
        let a: Vec<u64> = (0..50).map(|_| Rng::new(9).next_u64()).collect();
        let mut r = Rng::new(9);
        let b: Vec<u64> = (0..50).map(|_| r.next_u64()).collect();
        assert_eq!(a[0], b[0]);
        assert_ne!(b[0], b[1], "a generator that repeats is not one");
    }

    #[test]
    fn different_seeds_diverge() {
        assert_ne!(Rng::new(1).next_u64(), Rng::new(2).next_u64());
    }

    #[test]
    fn between_stays_in_range_including_the_degenerate_one() {
        let mut r = Rng::new(3);
        for _ in 0..1_000 {
            let v = r.between(10, 20);
            assert!((10..=20).contains(&v), "{v}");
        }
        assert_eq!(r.between(7, 7), 7);
        assert_eq!(r.between(9, 2), 9);
    }

    #[test]
    fn chance_is_never_and_always_at_the_ends() {
        let mut r = Rng::new(4);
        for _ in 0..200 {
            assert!(!r.chance(0));
            assert!(r.chance(100));
        }
    }
}
