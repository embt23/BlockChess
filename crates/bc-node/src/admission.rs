//! Whether the chain will take a channel's business at all.
//!
//! `D22` established the shape of this argument for time controls: an
//! offer naming a Δ the class table does not support is simply not a valid
//! offer, checked rather than trusted. Episode 10 adds the other half of
//! the same instinct — **the chain should decline a channel whose deadlines
//! it cannot honour against the validator set it actually has.**
//!
//! The alternative considered and rejected was a warning: compute the
//! bound, expose it, let clients decide. That is exactly the position D22
//! closed for time controls, and it fails the same way — a client that
//! does not check accepts hostile terms, and the player who is robbed is
//! the one who trusted their software.
//!
//! ## This depends on the `G0` hole
//!
//! [`bc_bft::censorship::assess`] is `todo!()`, so every function here
//! panics until episode 10's subject is written. Its tests are `#[ignore]`d
//! to match. A version that returned [`Admission::Accepted`] while the
//! bound was unwritten would be a censorship check that checks nothing,
//! which is worse than not having one.

use bc_adjudicator::dilation::MIN_MOVE_BLOCKS;
use bc_adjudicator::terms::GameTerms;
use bc_bft::censorship::{self, Assessment};
use bc_bft::ValidatorSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Admission {
    Accepted(Assessment),
    /// The smallest window this channel can produce does not outlast the
    /// longest run of Byzantine proposers the set permits, so late in a
    /// dispute a response could be censored to the deadline.
    Refused(Assessment),
}

impl Admission {
    pub fn is_accepted(self) -> bool {
        matches!(self, Admission::Accepted(_))
    }

    pub fn assessment(self) -> Assessment {
        match self {
            Admission::Accepted(a) | Admission::Refused(a) => a,
        }
    }
}

/// Assess an offer's terms against the live validator set.
///
/// Note what is *not* passed in: the stakes, the players, or how long the
/// game is. Censorship safety is a property of the terms and the set, and
/// nothing else — a large wager on a safe channel is safe, and a tiny one
/// on an unsafe channel is not.
pub fn admits(set: &ValidatorSet, terms: &GameTerms) -> Admission {
    let a = censorship::assess(set, terms.delta_blocks(), MIN_MOVE_BLOCKS as u32);
    if a.safe {
        Admission::Accepted(a)
    } else {
        Admission::Refused(a)
    }
}

/// The largest validator set these terms can be secured against.
///
/// Searching rather than solving, because the bound is `G0` and this must
/// not encode a guess at its shape. Bounded so it terminates whatever the
/// hole turns out to contain.
pub fn largest_securable_set(terms: &GameTerms, ceiling: usize) -> Option<usize> {
    use bc_sig::{SigningKey, VerifyingKey};
    (1..=ceiling).rfind(|n| {
        let keys: Vec<VerifyingKey> = (0..*n)
            .map(|i| SigningKey::from_seed(&[(i % 251) as u8 + 1; 32]).verifying_key())
            .collect();
        admits(&ValidatorSet::uniform(&keys), terms).is_accepted()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use bc_sig::{SigningKey, VerifyingKey};

    const G0: &str = "G0: episode 10's subject — Evan types the censorship bound";

    fn set(n: u8) -> ValidatorSet {
        let keys: Vec<VerifyingKey> = (0..n)
            .map(|i| SigningKey::from_seed(&[i + 1; 32]).verifying_key())
            .collect();
        ValidatorSet::uniform(&keys)
    }

    fn terms(base_ms: u32) -> GameTerms {
        GameTerms::for_clock([0u8; 32], base_ms, 0)
    }

    #[test]
    #[ignore = "G0: episode 10's subject — Evan types the censorship bound"]
    fn a_small_set_is_admitted() {
        let a = admits(&set(4), &terms(60_000));
        assert!(a.is_accepted(), "{G0}: four validators should be securable");
        assert_eq!(a.assessment().byzantine_run, 1);
        assert_eq!(a.assessment().smallest_window, MIN_MOVE_BLOCKS as u32);
    }

    /// The result worth filming. Past some size the set cannot be secured,
    /// and **no time control rescues it**, because every class shrinks to
    /// the same floor.
    #[test]
    #[ignore = "G0: episode 10's subject — Evan types the censorship bound"]
    fn a_large_set_is_refused_at_every_time_control() {
        let largest =
            largest_securable_set(&terms(60_000), 80).expect("something should be securable");
        println!("largest securable validator set: {largest}");

        let too_big = set((largest + 1) as u8);
        // Bullet through correspondence: 60 s to three days.
        for base in [60_000u32, 300_000, 1_800_000, 7_200_000, 259_200_000] {
            let a = admits(&too_big, &terms(base));
            assert!(
                !a.is_accepted(),
                "{G0}: base {base} ms (Δ={}) appeared to rescue a set of {}",
                terms(base).delta_blocks(),
                largest + 1
            );
            assert_eq!(
                a.assessment().smallest_window,
                MIN_MOVE_BLOCKS as u32,
                "{G0}: Δ changed the floor"
            );
        }
    }

    /// Admission depends on the terms and the set, and on nothing else.
    #[test]
    #[ignore = "G0: episode 10's subject — Evan types the censorship bound"]
    fn admission_is_monotone_in_the_set_size() {
        let t = terms(60_000);
        let mut refused_yet = false;
        for n in 1..=60u8 {
            let ok = admits(&set(n), &t).is_accepted();
            if refused_yet {
                assert!(!ok, "{G0}: n={n} admitted after a smaller set was refused");
            }
            refused_yet |= !ok;
        }
    }
}
