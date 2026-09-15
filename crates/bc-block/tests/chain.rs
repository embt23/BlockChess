//! `Chain`: what "seen" and "canonical" mean, and what a reorg reports.
//!
//! These live outside `src/` because `chain.rs` was over the `G2` line with
//! them in it, and because every one of them drives the public API — there
//! is nothing here a user of the crate could not write.

use bc_block::chain::Reorg;
use bc_block::header::{BlockHeader, NO_PARENT};
use bc_block::tx::{Payload, Tx};
use bc_block::{Block, Chain, ChainError};
/// Blocks are distinguished by their proposer, so a test can fork by
/// mining "the same" height twice with different miners.
fn block(parent: &Block, height: u64, miner: u8, txs: Vec<Tx>) -> Block {
    Block {
        header: BlockHeader {
            version: 1,
            height,
            parent_hash: parent.hash(),
            state_root: [0u8; 32],
            tx_root: Block::tx_root(&txs),
            timestamp_ms: height * 2_000,
            proposer: [miner; 32],
        },
        txs,
        seal: vec![],
    }
}

fn genesis() -> Block {
    Block {
        header: BlockHeader {
            version: 1,
            height: 0,
            parent_hash: NO_PARENT,
            state_root: [0u8; 32],
            tx_root: Block::tx_root(&[]),
            timestamp_ms: 0,
            proposer: [0u8; 32],
        },
        txs: vec![],
        seal: vec![],
    }
}

fn dispute_tx(nonce: u64) -> Tx {
    Tx {
        version: 1,
        nonce,
        sender: [1u8; 32],
        fee: 1,
        gas_limit: 100,
        payload: Payload::DisputeMove,
        body: vec![nonce as u8],
        signature: [0u8; 64],
    }
}

#[test]
fn a_fresh_chain_is_its_genesis() {
    let g = genesis();
    let c = Chain::new(g.clone());
    assert_eq!(c.head(), g.hash());
    assert_eq!(c.height(), 0);
    assert!(c.is_canonical(&g.hash()));
    assert_eq!(c.confirmations(&g.hash()), Some(1));
}

#[test]
fn extending_the_head_is_not_a_reorg() {
    let g = genesis();
    let mut c = Chain::new(g.clone());
    let b1 = block(&g, 1, 1, vec![]);
    let r = c.insert(b1.clone(), 1).unwrap();
    assert!(!r.is_rollback());
    assert_eq!(r.applied, vec![b1.hash()]);
    assert_eq!(c.height(), 1);
}

#[test]
fn a_block_with_an_unknown_parent_is_refused() {
    let g = genesis();
    let mut c = Chain::new(g.clone());
    let orphan_parent = block(&g, 1, 9, vec![]);
    let orphan = block(&orphan_parent, 2, 9, vec![]);
    assert_eq!(c.insert(orphan, 2), Err(ChainError::UnknownParent));
}

#[test]
fn a_block_whose_body_does_not_match_its_header_is_refused() {
    let g = genesis();
    let mut c = Chain::new(g.clone());
    let mut b = block(&g, 1, 1, vec![]);
    b.txs.push(dispute_tx(0));
    assert_eq!(c.insert(b, 1), Err(ChainError::BodyDoesNotMatchHeader));
}

#[test]
fn the_same_block_twice_is_refused() {
    let g = genesis();
    let mut c = Chain::new(g.clone());
    let b = block(&g, 1, 1, vec![]);
    c.insert(b.clone(), 1).unwrap();
    assert_eq!(c.insert(b, 1), Err(ChainError::Duplicate));
}

/// A lower-scoring fork is stored but does not move the head. This is
/// what "seen but not canonical" means, and it is the state a chain
/// spends most of its life in.
#[test]
fn a_losing_fork_is_kept_and_ignored() {
    let g = genesis();
    let mut c = Chain::new(g.clone());
    let main = block(&g, 1, 1, vec![]);
    let fork = block(&g, 1, 2, vec![]);
    c.insert(main.clone(), 10).unwrap();
    let r = c.insert(fork.clone(), 5).unwrap();
    assert_eq!(r, Reorg::default());
    assert!(c.contains(&fork.hash()));
    assert!(!c.is_canonical(&fork.hash()));
    assert!(c.is_canonical(&main.hash()));
    assert_eq!(c.confirmations(&fork.hash()), None);
}

/// Equal score keeps the incumbent. A miner who arrives late with the
/// same work does not get to reorganise anyone.
#[test]
fn an_equal_score_does_not_displace_the_incumbent() {
    let g = genesis();
    let mut c = Chain::new(g.clone());
    let first = block(&g, 1, 1, vec![]);
    let second = block(&g, 1, 2, vec![]);
    c.insert(first.clone(), 10).unwrap();
    assert_eq!(c.insert(second.clone(), 10).unwrap(), Reorg::default());
    assert_eq!(c.head(), first.hash());
}

/// The whole reason this type reports anything. A heavier fork arrives
/// and two confirmed blocks stop being confirmed.
#[test]
fn a_heavier_fork_un_includes_the_blocks_it_replaces() {
    let g = genesis();
    let mut c = Chain::new(g.clone());

    // The chain everyone can see: two blocks, one carrying a dispute.
    let a1 = block(&g, 1, 1, vec![dispute_tx(0)]);
    let a2 = block(&a1, 2, 1, vec![]);
    c.insert(a1.clone(), 10).unwrap();
    c.insert(a2.clone(), 20).unwrap();
    assert_eq!(c.confirmations(&a1.hash()), Some(2));

    // A fork built in private, with more work behind it.
    let b1 = block(&g, 1, 2, vec![]);
    let b2 = block(&b1, 2, 2, vec![]);
    let b3 = block(&b2, 3, 2, vec![]);
    // b1 alone is lighter than the visible chain, so nothing moves and
    // the fork is merely stored. This is what a private fork looks like
    // from outside right up until the moment it is not.
    assert_eq!(c.insert(b1.clone(), 11).unwrap(), Reorg::default());

    // b2 overtakes, and *this* is the block that un-includes a1 and a2.
    let r = c.insert(b2.clone(), 22).unwrap();
    assert!(r.is_rollback());
    assert_eq!(r.depth(), 2);
    assert_eq!(r.reverted, vec![a2.hash(), a1.hash()]);
    assert_eq!(r.applied, vec![b1.hash(), b2.hash()]);

    // b3 then simply extends the new best chain.
    let r = c.insert(b3.clone(), 33).unwrap();
    assert!(!r.is_rollback());
    assert_eq!(r.applied, vec![b3.hash()]);

    // And the dispute transaction that was two-deep confirmed is now in
    // a block nobody is building on.
    assert!(!c.is_canonical(&a1.hash()));
    assert_eq!(c.confirmations(&a1.hash()), None);
    assert!(c.contains(&a1.hash()), "still stored, just not believed");
    assert_eq!(c.height(), 3);
    assert_eq!(c.at_height(1), Some(b1.hash()));
}

/// The canonical index is rebuilt, not patched, so it can never carry a
/// stale entry from a chain that lost.
#[test]
fn the_canonical_index_never_keeps_an_abandoned_block() {
    let g = genesis();
    let mut c = Chain::new(g.clone());
    let a1 = block(&g, 1, 1, vec![]);
    let a2 = block(&a1, 2, 1, vec![]);
    c.insert(a1.clone(), 10).unwrap();
    c.insert(a2, 20).unwrap();

    let b1 = block(&g, 1, 2, vec![]);
    let b2 = block(&b1, 2, 2, vec![]);
    let b3 = block(&b2, 3, 2, vec![]);
    c.insert(b1, 11).unwrap();
    c.insert(b2, 22).unwrap();
    c.insert(b3, 33).unwrap();

    for h in 0..=c.height() {
        let hash = c.at_height(h).expect("every height is occupied");
        assert!(c.is_canonical(&hash));
        assert_eq!(c.block(&hash).unwrap().header.height, h);
    }
}
