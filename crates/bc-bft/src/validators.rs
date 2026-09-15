//! Who votes, how much their vote weighs, and what a quorum is.
//!
//! ## The two thresholds, and why they are not the same number
//!
//! - **`f`**, the Byzantine bound: the most stake that may misbehave while
//!   safety still holds. `f < total/3`.
//! - **`quorum`**: the stake a decision needs. `> 2·total/3`.
//!
//! The relationship between them is the whole argument. Two quorums each
//! holding more than ⅔ of stake must overlap in more than ⅓ — and since at
//! most ⅓ is Byzantine, that overlap contains at least one honest
//! validator. An honest validator does not precommit two different blocks
//! at the same height and round. So two conflicting blocks cannot both
//! reach a quorum.
//!
//! That is **quorum intersection**, it is three sentences long, and it is
//! the reason this chain can hold money under a deadline.
//!
//! ## Integer arithmetic, and the off-by-one that eats it
//!
//! `> 2·total/3` in integers is `total * 2 / 3 + 1`, and the temptation is
//! `total * 2 / 3`. At `total = 3` those are 3 and 2: the wrong one lets
//! two out of three validators finalise a block, which is a bare majority
//! wearing a BFT costume. [`ValidatorSet::quorum`] is tested against every
//! small total for exactly this reason.

use bc_sig::VerifyingKey;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Validator {
    pub key: VerifyingKey,
    pub stake: u128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatorSet {
    /// Sorted by key, so proposer selection is the same on every node. A
    /// set whose order depends on insertion order is a consensus bug with
    /// a long fuse.
    validators: Vec<Validator>,
}

impl ValidatorSet {
    pub fn new(mut validators: Vec<Validator>) -> ValidatorSet {
        validators.sort_by_key(|v| v.key.0);
        validators.dedup_by_key(|v| v.key.0);
        ValidatorSet { validators }
    }

    /// Equal stake for everyone. What the tests use, and what a small
    /// permissioned set looks like in practice.
    pub fn uniform(keys: &[VerifyingKey]) -> ValidatorSet {
        ValidatorSet::new(
            keys.iter()
                .map(|k| Validator { key: *k, stake: 1 })
                .collect(),
        )
    }

    pub fn len(&self) -> usize {
        self.validators.len()
    }

    pub fn is_empty(&self) -> bool {
        self.validators.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Validator> {
        self.validators.iter()
    }

    pub fn total_stake(&self) -> u128 {
        self.validators.iter().map(|v| v.stake).sum()
    }

    pub fn stake_of(&self, key: &VerifyingKey) -> u128 {
        self.validators
            .iter()
            .find(|v| v.key == *key)
            .map(|v| v.stake)
            .unwrap_or(0)
    }

    pub fn contains(&self, key: &VerifyingKey) -> bool {
        self.validators.iter().any(|v| v.key == *key)
    }

    /// Stake a decision needs: strictly more than two thirds.
    pub fn quorum(&self) -> u128 {
        self.total_stake() * 2 / 3 + 1
    }

    /// The most Byzantine stake safety tolerates: strictly less than a third.
    pub fn byzantine_bound(&self) -> u128 {
        let total = self.total_stake();
        if total == 0 {
            return 0;
        }
        (total - 1) / 3
    }

    /// Whose turn it is to propose, round-robin over the sorted set.
    ///
    /// Round-robin rather than stake-weighted, and rather than random. Two
    /// reasons: it needs no shared randomness (episode 15's VRF is not
    /// built yet), and it is the version whose liveness argument is one
    /// sentence — every validator gets a turn within `len()` rounds, so an
    /// honest proposer is reached within `f + 1`.
    pub fn proposer(&self, height: u64, round: u32) -> Option<VerifyingKey> {
        if self.validators.is_empty() {
            return None;
        }
        let i = (height as u128 + round as u128) % self.validators.len() as u128;
        Some(self.validators[i as usize].key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bc_sig::SigningKey;

    fn keys(n: u8) -> Vec<VerifyingKey> {
        (0..n)
            .map(|i| SigningKey::from_seed(&[i + 1; 32]).verifying_key())
            .collect()
    }

    /// The off-by-one that turns BFT into majority rule. Checked against a
    /// direct restatement of the inequality rather than against itself.
    #[test]
    fn the_quorum_is_strictly_more_than_two_thirds() {
        for n in 1..=40u128 {
            let set = ValidatorSet::uniform(&keys(n as u8));
            let q = set.quorum();
            assert!(q * 3 > n * 2, "quorum {q} of {n} is not above two thirds");
            assert!(
                (q - 1) * 3 <= n * 2,
                "quorum {q} of {n} is larger than it needs to be"
            );
        }
    }

    /// Two quorums must share at least one honest validator. This is the
    /// safety argument, checked arithmetically at every small size rather
    /// than asserted in a comment.
    #[test]
    fn two_quorums_always_overlap_in_more_than_the_byzantine_bound() {
        for n in 1..=60u128 {
            let set = ValidatorSet::uniform(&keys(n.min(60) as u8));
            let total = set.total_stake();
            let q = set.quorum();
            let f = set.byzantine_bound();
            // |A ∩ B| >= |A| + |B| - total
            let overlap = 2 * q - total;
            assert!(
                overlap > f,
                "n={n}: two quorums overlap in {overlap}, Byzantine bound is {f}"
            );
        }
    }

    #[test]
    fn the_byzantine_bound_is_strictly_under_a_third() {
        for n in 1..=40u128 {
            let set = ValidatorSet::uniform(&keys(n as u8));
            assert!(set.byzantine_bound() * 3 < n, "n={n}");
        }
        // The classic sizes.
        assert_eq!(ValidatorSet::uniform(&keys(4)).byzantine_bound(), 1);
        assert_eq!(ValidatorSet::uniform(&keys(7)).byzantine_bound(), 2);
        assert_eq!(ValidatorSet::uniform(&keys(10)).byzantine_bound(), 3);
    }

    #[test]
    fn four_validators_need_three_and_three_need_all_three() {
        assert_eq!(ValidatorSet::uniform(&keys(4)).quorum(), 3);
        assert_eq!(ValidatorSet::uniform(&keys(3)).quorum(), 3);
        assert_eq!(ValidatorSet::uniform(&keys(7)).quorum(), 5);
    }

    /// Order must not depend on how the set was built, or two nodes
    /// disagree about whose turn it is and neither is wrong.
    #[test]
    fn proposer_selection_does_not_depend_on_insertion_order() {
        let k = keys(5);
        let a = ValidatorSet::uniform(&k);
        let mut reversed = k.clone();
        reversed.reverse();
        let b = ValidatorSet::uniform(&reversed);
        for h in 0..20 {
            for r in 0..5 {
                assert_eq!(a.proposer(h, r), b.proposer(h, r), "h={h} r={r}");
            }
        }
    }

    /// Everyone proposes within `len` rounds, which is what bounds the wait
    /// for an honest proposer.
    #[test]
    fn every_validator_proposes_within_one_pass() {
        let set = ValidatorSet::uniform(&keys(5));
        let seen: std::collections::BTreeSet<_> = (0..5)
            .filter_map(|r| set.proposer(0, r))
            .map(|k| k.0)
            .collect();
        assert_eq!(seen.len(), 5);
    }

    #[test]
    fn a_duplicate_key_cannot_buy_extra_votes() {
        let k = keys(1)[0];
        let set = ValidatorSet::new(vec![
            Validator { key: k, stake: 1 },
            Validator { key: k, stake: 1 },
        ]);
        assert_eq!(set.len(), 1);
        assert_eq!(set.total_stake(), 1);
    }

    #[test]
    fn an_empty_set_has_no_proposer_and_no_bound() {
        let set = ValidatorSet::new(vec![]);
        assert!(set.is_empty());
        assert_eq!(set.proposer(0, 0), None);
        assert_eq!(set.byzantine_bound(), 0);
    }
}
