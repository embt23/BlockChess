//! How hard a block is to find.
//!
//! A target is a threshold: a block is valid when its proof-of-work hash,
//! read as a big-endian integer, is **at or below** it. Lower target, fewer
//! acceptable hashes, more work.
//!
//! ## Why 128 bits and not 256
//!
//! Bitcoin's target is a 256-bit integer and its arithmetic needs bignums.
//! Here the target is the **top 128 bits** of the 256-bit threshold, with
//! the bottom 128 implicitly zero. Everything — comparison, expected work,
//! the retarget multiply — is then exact `u128` arithmetic with no bignum
//! library, no allocation and no rounding anyone has to argue about.
//!
//! That is not a simplification made to save effort. Consensus arithmetic
//! that needs a dependency is consensus arithmetic two implementations can
//! do differently, and `u128` is the widest thing every target agrees about.
//! The cost is that difficulty cannot exceed 2¹²⁸, which is not a real
//! constraint for a chain nobody is mining with ASICs.

use bc_hash::Hash;

/// A proof-of-work threshold: the top 128 bits of a 256-bit bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Target(pub u128);

/// The easiest possible target: every hash satisfies it.
pub const MAX_TARGET: Target = Target(u128::MAX);

/// The first 16 bytes of a hash, big-endian. Big-endian because "leading
/// zeros mean more work" is the intuition everyone has, and it is only true
/// if the most significant byte comes first.
pub fn top128(h: &Hash) -> u128 {
    let mut b = [0u8; 16];
    b.copy_from_slice(&h[..16]);
    u128::from_be_bytes(b)
}

impl Target {
    /// Does this hash satisfy the target?
    pub fn meets(self, h: &Hash) -> bool {
        top128(h) <= self.0
    }

    /// Expected hashes needed to find one block at this target.
    ///
    /// This is the quantity that makes the longest-chain rule work, and the
    /// reason fork choice compares *cumulative work* and not block count: a
    /// chain of ten easy blocks must lose to a chain of three hard ones, or
    /// an attacker simply lowers their own difficulty.
    pub fn work(self) -> u128 {
        match self.0.checked_add(1) {
            // Saturating because a target of zero is unmineable, and
            // "infinite work" is the honest answer rather than a wrap to
            // one — which would make an impossible chain the heaviest.
            Some(d) => (u128::MAX / d).saturating_add(1),
            // The maximum target: one hash, always.
            None => 1,
        }
    }

    /// Multiply by `num / den`, saturating at [`MAX_TARGET`] and never
    /// reaching zero.
    ///
    /// Provided because every retarget rule needs it and getting it wrong is
    /// an overflow rather than a disagreement — the multiply is what
    /// overflows 128 bits when a block takes a long time, and the answer is
    /// to divide first when it would.
    pub fn scale(self, num: u64, den: u64) -> Target {
        debug_assert!(den > 0, "a retarget with zero expected time is not one");
        let (n, d) = (num.max(1) as u128, den.max(1) as u128);
        let scaled = match self.0.checked_mul(n) {
            Some(v) => v / d,
            // Would overflow: divide first. Loses a little precision and
            // cannot produce a wrong answer, which is the right trade for
            // consensus arithmetic.
            None => (self.0 / d).saturating_mul(n),
        };
        Target(scaled.clamp(1, MAX_TARGET.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash_with_prefix(b: u8) -> Hash {
        let mut h = [0xffu8; 32];
        h[0] = b;
        h
    }

    #[test]
    fn a_lower_hash_meets_a_lower_target() {
        let easy = Target(u128::MAX / 2);
        assert!(easy.meets(&hash_with_prefix(0x00)));
        assert!(!easy.meets(&hash_with_prefix(0xff)));
    }

    #[test]
    fn the_maximum_target_accepts_everything() {
        for b in 0..=255u8 {
            assert!(MAX_TARGET.meets(&hash_with_prefix(b)));
        }
        assert_eq!(MAX_TARGET.work(), 1);
    }

    /// Halving the target doubles the work. This is the relationship the
    /// longest-chain rule depends on being true.
    #[test]
    fn work_is_inversely_proportional_to_the_target() {
        let t = Target(u128::MAX / 1024);
        let harder = Target(t.0 / 2);
        let ratio = harder.work() / t.work();
        assert!((2..=3).contains(&ratio), "ratio was {ratio}");
        assert!(harder.work() > t.work());
    }

    #[test]
    fn work_never_divides_by_zero_at_either_extreme() {
        assert_eq!(MAX_TARGET.work(), 1);
        assert_eq!(Target(0).work(), u128::MAX);
    }

    #[test]
    fn scaling_up_makes_it_easier_and_scaling_down_makes_it_harder() {
        let t = Target(1 << 100);
        assert!(t.scale(4, 1) > t);
        assert!(t.scale(1, 4) < t);
        assert_eq!(t.scale(1, 1), t);
    }

    /// The multiply overflows 128 bits long before the answer does, and a
    /// retarget that overflows is a chain that stops.
    #[test]
    fn scaling_a_near_maximum_target_saturates_instead_of_wrapping() {
        let t = Target(u128::MAX - 1);
        assert_eq!(t.scale(4, 1), MAX_TARGET);
        assert!(t.scale(1_000_000, 1) <= MAX_TARGET);
    }

    /// A target of zero is unmineable and would halt the chain forever.
    #[test]
    fn scaling_down_never_reaches_zero() {
        let mut t = Target(1 << 30);
        for _ in 0..200 {
            t = t.scale(1, 4);
            assert!(t.0 >= 1, "target hit zero");
        }
        assert_eq!(t, Target(1));
    }
}
