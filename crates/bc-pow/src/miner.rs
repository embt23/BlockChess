//! Finding a nonce, and what a proof-of-work hash is over.
//!
//! The seal for a proof-of-work block is eight bytes: a nonce. It lives in
//! [`crate::Block::seal`] rather than in the header because a BFT block has
//! no nonce and should not carry eight bytes of nothing (`bc-block`'s
//! `header` module says the same thing from the other side).

use bc_block::{Block, BlockHeader};
use bc_hash::{tagged_parts, Hash};

use crate::target::Target;

/// The hash a miner is trying to get under.
///
/// Domain-tagged and distinct from [`BlockHeader::hash`], which is the
/// block's *identity*. Two different questions: "what is this block called"
/// and "did someone pay for it". Conflating them would mean the identity
/// changes as the nonce is searched, so a miner could not refer to the block
/// it is working on.
pub fn pow_hash(header: &BlockHeader, nonce: u64) -> Hash {
    tagged_parts("BC/pow/v1", &[&header.encode(), &nonce.to_le_bytes()])
}

pub fn seal_of(nonce: u64) -> Vec<u8> {
    nonce.to_le_bytes().to_vec()
}

pub fn nonce_of(seal: &[u8]) -> Option<u64> {
    Some(u64::from_le_bytes(seal.try_into().ok()?))
}

/// Does this block's seal actually satisfy `target`?
pub fn seal_is_valid(block: &Block, target: Target) -> bool {
    match nonce_of(&block.seal) {
        Some(n) => target.meets(&pow_hash(&block.header, n)),
        None => false,
    }
}

/// Search for a nonce, giving up after `max_tries`.
///
/// Starting from `from` rather than zero so that two miners racing on the
/// same header do not trace the same search path — which is the whole
/// reason mining is a race and not a queue.
pub fn mine_from(header: &BlockHeader, target: Target, from: u64, max_tries: u64) -> Option<u64> {
    (0..max_tries)
        .map(|i| from.wrapping_add(i))
        .find(|&n| target.meets(&pow_hash(header, n)))
}

/// Mine from nonce zero. Convenient for tests, honest for nobody else.
pub fn mine(header: &BlockHeader, target: Target, max_tries: u64) -> Option<u64> {
    mine_from(header, target, 0, max_tries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bc_block::header::NO_PARENT;

    fn header() -> BlockHeader {
        BlockHeader {
            version: 1,
            height: 1,
            parent_hash: NO_PARENT,
            state_root: [0u8; 32],
            tx_root: [0u8; 32],
            timestamp_ms: 0,
            proposer: [3u8; 32],
        }
    }

    #[test]
    fn a_mined_nonce_satisfies_the_target_it_was_mined_for() {
        let t = Target(u128::MAX / 4_096);
        let n = mine(&header(), t, 1_000_000).expect("found a nonce");
        assert!(t.meets(&pow_hash(&header(), n)));
    }

    /// The identity of a block does not change while it is being mined.
    #[test]
    fn the_proof_of_work_hash_is_not_the_block_identity() {
        let h = header();
        assert_ne!(pow_hash(&h, 0), h.hash());
        assert_ne!(pow_hash(&h, 0), pow_hash(&h, 1));
        // …and the identity is the same whatever nonce is being tried.
        assert_eq!(h.hash(), header().hash());
    }

    /// Changing anything in the header invalidates the work. That is what
    /// makes the work a commitment to the block rather than to a number.
    #[test]
    fn work_does_not_survive_an_edit_to_the_header() {
        let t = Target(u128::MAX / 4_096);
        let h = header();
        let n = mine(&h, t, 1_000_000).unwrap();
        assert!(t.meets(&pow_hash(&h, n)));

        let mut tampered = h;
        tampered.state_root[0] ^= 1;
        assert!(!t.meets(&pow_hash(&tampered, n)));
    }

    #[test]
    fn a_seal_round_trips_and_a_malformed_one_is_rejected() {
        assert_eq!(nonce_of(&seal_of(0xdead_beef)), Some(0xdead_beef));
        assert_eq!(nonce_of(&[1, 2, 3]), None);
        assert_eq!(nonce_of(&[]), None);
    }

    #[test]
    fn two_miners_starting_at_different_nonces_find_different_blocks() {
        let t = Target(u128::MAX / 256);
        let h = header();
        let a = mine_from(&h, t, 0, 100_000).unwrap();
        let b = mine_from(&h, t, 500_000, 100_000).unwrap();
        assert_ne!(a, b);
    }

    /// Giving up is a real outcome and must not loop forever.
    #[test]
    fn an_impossible_target_gives_up_rather_than_hanging() {
        assert_eq!(mine(&header(), Target(0), 5_000), None);
    }
}
