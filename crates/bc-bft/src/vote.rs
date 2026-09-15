//! What a validator says, and what it signs when it says it.
//!
//! Two steps produce votes. **Prevote** is "I have seen a proposal I am
//! willing to accept"; **precommit** is "I am bound to this one". Only
//! precommits finalise, and the gap between them is the entire reason
//! Tendermint is safe across rounds: a validator may change its mind
//! between rounds about a prevote and may not about a precommit.
//!
//! ## Nil is a value
//!
//! `block: None` is a real vote and not an absence. A validator that has
//! seen no acceptable proposal votes nil, and nil can reach a quorum just
//! like a block can — which is how a round ends in "we agreed to agree on
//! nothing and move on" rather than in a timeout nobody voted about. A
//! protocol where silence and nil are the same thing cannot distinguish
//! "I am slow" from "I object", which is FLP wearing a different hat.

use bc_hash::{tagged_parts, Hash};
use bc_sig::{Signature, SigningKey, VerifyingKey};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Step {
    Prevote = 1,
    Precommit = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vote {
    pub height: u64,
    pub round: u32,
    pub step: Step,
    /// `None` is nil: a vote for no block at all.
    pub block: Option<Hash>,
    pub validator: VerifyingKey,
    pub signature: Signature,
}

/// The bytes a validator signs.
///
/// `height` and `round` are both in here, and both must be. Without the
/// round, a precommit from round 0 could be replayed as one from round 3,
/// which is exactly the move that breaks safety across a round change —
/// the attack the locking rules exist to prevent, handed to an attacker for
/// free by an encoding mistake.
pub fn vote_bytes(height: u64, round: u32, step: Step, block: Option<Hash>) -> Hash {
    let nil = [0u8; 32];
    tagged_parts(
        "BC/vote/v1",
        &[
            &height.to_le_bytes(),
            &round.to_le_bytes(),
            &[step as u8],
            // A nil vote and a vote for the all-zero block hash must differ.
            &[u8::from(block.is_some())],
            block.as_ref().unwrap_or(&nil),
        ],
    )
}

impl Vote {
    pub fn sign(
        key: &SigningKey,
        height: u64,
        round: u32,
        step: Step,
        block: Option<Hash>,
    ) -> Vote {
        let msg = vote_bytes(height, round, step, block);
        Vote {
            height,
            round,
            step,
            block,
            validator: key.verifying_key(),
            signature: key.sign(&msg),
        }
    }

    pub fn is_valid(&self) -> bool {
        let msg = vote_bytes(self.height, self.round, self.step, self.block);
        self.validator.verify(&msg, &self.signature)
    }

    pub fn is_nil(&self) -> bool {
        self.block.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(seed: u8) -> SigningKey {
        SigningKey::from_seed(&[seed; 32])
    }

    #[test]
    fn a_signed_vote_verifies_and_a_tampered_one_does_not() {
        let v = Vote::sign(&key(1), 7, 0, Step::Prevote, Some([9u8; 32]));
        assert!(v.is_valid());
        for mut bad in [v, v, v, v] {
            bad.round += 1;
            assert!(!bad.is_valid());
        }
    }

    /// Every field the vote means must be inside the signature, or an
    /// attacker rewrites it in flight.
    #[test]
    fn height_round_step_and_block_are_all_signed() {
        let base = vote_bytes(1, 2, Step::Prevote, Some([3u8; 32]));
        assert_ne!(base, vote_bytes(2, 2, Step::Prevote, Some([3u8; 32])));
        assert_ne!(base, vote_bytes(1, 3, Step::Prevote, Some([3u8; 32])));
        assert_ne!(base, vote_bytes(1, 2, Step::Precommit, Some([3u8; 32])));
        assert_ne!(base, vote_bytes(1, 2, Step::Prevote, Some([4u8; 32])));
    }

    /// The single most replayable confusion in the protocol: a precommit
    /// from an earlier round presented as one from a later round.
    #[test]
    fn a_precommit_cannot_be_replayed_into_another_round() {
        let v = Vote::sign(&key(2), 5, 0, Step::Precommit, Some([1u8; 32]));
        let mut replayed = v;
        replayed.round = 4;
        assert!(!replayed.is_valid());
    }

    /// Nil must not collide with a vote for the all-zero hash, or a
    /// validator abstaining can be counted as voting for a block.
    #[test]
    fn nil_is_distinct_from_a_vote_for_the_zero_hash() {
        assert_ne!(
            vote_bytes(1, 0, Step::Prevote, None),
            vote_bytes(1, 0, Step::Prevote, Some([0u8; 32]))
        );
    }

    #[test]
    fn a_prevote_and_a_precommit_for_the_same_block_are_different_statements() {
        let b = Some([7u8; 32]);
        assert_ne!(
            vote_bytes(1, 0, Step::Prevote, b),
            vote_bytes(1, 0, Step::Precommit, b)
        );
    }
}
