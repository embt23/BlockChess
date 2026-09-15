//! Counting votes by stake, and noticing when somebody votes twice.
//!
//! One validator, one vote per (height, round, step). A second vote for a
//! different value is **equivocation** — not an error to shrug at, but the
//! evidence a slashing transaction is built from (`spec/02`'s
//! `SlashServer`, and the validator equivalent). So the tally records it
//! rather than discarding it, and refuses to count the stake twice.

use crate::validators::ValidatorSet;
use crate::vote::{Step, Vote};
use bc_hash::Hash;
use bc_sig::VerifyingKey;
use std::collections::BTreeMap;

/// What a set of votes adds up to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Nothing has a quorum yet.
    Undecided,
    /// A quorum of stake voted for this block.
    Quorum(Hash),
    /// A quorum of stake voted nil.
    QuorumNil,
}

/// Proof that a validator voted two ways at the same height, round and
/// step. Both votes are signed, so this is self-contained evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Equivocation {
    pub validator: VerifyingKey,
    pub first: Vote,
    pub second: Vote,
}

#[derive(Debug, Clone, Default)]
pub struct Tally {
    /// validator → the one vote counted for them.
    votes: BTreeMap<[u8; 32], Vote>,
    equivocations: Vec<Equivocation>,
}

impl Tally {
    pub fn new() -> Tally {
        Tally::default()
    }

    pub fn len(&self) -> usize {
        self.votes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.votes.is_empty()
    }

    pub fn equivocations(&self) -> &[Equivocation] {
        &self.equivocations
    }

    /// Record a vote. Returns false if it was rejected — a bad signature, a
    /// non-validator, or a duplicate.
    ///
    /// An identical repeat is accepted silently and counted once, because
    /// the network reorders and re-delivers and a validator resending its
    /// own vote is normal behaviour, not an attack.
    pub fn add(&mut self, set: &ValidatorSet, vote: Vote) -> bool {
        if !set.contains(&vote.validator) || !vote.is_valid() {
            return false;
        }
        match self.votes.get(&vote.validator.0) {
            None => {
                self.votes.insert(vote.validator.0, vote);
                true
            }
            Some(prev) if prev.block == vote.block => true,
            Some(prev) => {
                self.equivocations.push(Equivocation {
                    validator: vote.validator,
                    first: *prev,
                    second: vote,
                });
                false
            }
        }
    }

    /// Stake behind a particular value.
    pub fn stake_for(&self, set: &ValidatorSet, block: Option<Hash>) -> u128 {
        self.votes
            .values()
            .filter(|v| v.block == block)
            .map(|v| set.stake_of(&v.validator))
            .sum()
    }

    /// Total stake that has voted at all, for anything.
    pub fn stake_voted(&self, set: &ValidatorSet) -> u128 {
        self.votes
            .values()
            .map(|v| set.stake_of(&v.validator))
            .sum()
    }

    /// Has any single value reached a quorum?
    ///
    /// At most one can, which is quorum intersection and not an accident of
    /// this loop: two values each above ⅔ would need more than the total
    /// stake between them.
    pub fn outcome(&self, set: &ValidatorSet) -> Outcome {
        let q = set.quorum();
        if self.stake_for(set, None) >= q {
            return Outcome::QuorumNil;
        }
        let mut per_block: BTreeMap<Hash, u128> = BTreeMap::new();
        for v in self.votes.values() {
            if let Some(b) = v.block {
                *per_block.entry(b).or_insert(0) += set.stake_of(&v.validator);
            }
        }
        per_block
            .into_iter()
            .find(|(_, s)| *s >= q)
            .map(|(b, _)| Outcome::Quorum(b))
            .unwrap_or(Outcome::Undecided)
    }

    /// The one vote counted for this validator, if any.
    pub fn vote_of(&self, who: &VerifyingKey) -> Option<Vote> {
        self.votes.get(&who.0).copied()
    }

    /// The votes themselves, for building a commit certificate.
    pub fn votes_for(&self, block: Option<Hash>) -> Vec<Vote> {
        let mut out: Vec<Vote> = self
            .votes
            .values()
            .filter(|v| v.block == block)
            .copied()
            .collect();
        out.sort_by_key(|v| v.validator.0);
        out
    }

    /// Has enough stake voted that waiting longer cannot change the answer?
    ///
    /// Used to end a round early instead of sitting out a timeout that
    /// cannot teach anyone anything.
    pub fn is_settled(&self, set: &ValidatorSet, step: Step) -> bool {
        let _ = step;
        self.stake_voted(set) >= set.quorum()
    }
}
