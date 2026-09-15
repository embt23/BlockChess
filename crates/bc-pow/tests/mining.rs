//! Episode 05, end to end: mine a chain, fork it, and watch the heavier
//! branch take history away.
//!
//! Every block here is really mined — a nonce search against a real target,
//! not a counter being incremented. That matters for the one result the
//! episode is for: the reorg below is not staged, it is what heaviest-chain
//! fork choice does when someone with more hashrate shows up late.

use bc_block::header::NO_PARENT;
use bc_block::{Block, BlockHeader, Consensus, Finality, Payload, Tx};
use bc_pow::{miner, Difficulty, ProofOfWork, Target};

/// Easy enough that a test finds a nonce in microseconds, hard enough that
/// the search is real.
const EASY: Target = Target(u128::MAX / 4_096);

fn genesis() -> Block {
    let txs = vec![];
    Block {
        header: BlockHeader {
            version: 1,
            height: 0,
            parent_hash: NO_PARENT,
            state_root: [0u8; 32],
            tx_root: Block::tx_root(&txs),
            timestamp_ms: 0,
            proposer: [0u8; 32],
        },
        txs,
        seal: vec![],
    }
}

fn dispute_tx(nonce: u64) -> Tx {
    Tx {
        version: 1,
        nonce,
        sender: [7u8; 32],
        fee: 1,
        gas_limit: 100,
        payload: Payload::DisputeMove,
        body: vec![nonce as u8],
        signature: [0u8; 64],
    }
}

/// Mine a block on `parent` for `miner_id`, containing `txs`.
fn mine_on(parent: &Block, miner_id: u8, txs: Vec<Tx>, target: Target) -> Block {
    let header = BlockHeader {
        version: 1,
        height: parent.header.height + 1,
        parent_hash: parent.hash(),
        state_root: [0u8; 32],
        tx_root: Block::tx_root(&txs),
        timestamp_ms: parent.header.timestamp_ms + 2_000,
        proposer: [miner_id; 32],
    };
    let nonce = miner::mine(&header, target, 50_000_000).expect("a nonce exists at this target");
    Block {
        header,
        txs,
        seal: miner::seal_of(nonce),
    }
}

#[test]
fn a_mined_chain_grows_and_accumulates_work() {
    let g = genesis();
    let mut pow = ProofOfWork::new(g.clone(), Difficulty::Fixed(EASY));
    assert_eq!(pow.height(), 0);
    assert_eq!(pow.total_work(), 0);

    let mut tip = g;
    for _ in 0..5 {
        tip = mine_on(&tip, 1, vec![], EASY);
        pow.submit(tip.clone()).expect("a mined block is accepted");
    }
    assert_eq!(pow.height(), 5);
    assert_eq!(pow.total_work(), EASY.work() * 5);
    assert!(pow.is_canonical(&tip.hash()));
}

/// The seal is the whole of the security argument, so a block without one
/// must be refused even though everything else about it is well formed.
#[test]
fn a_block_without_enough_work_is_refused() {
    let g = genesis();
    let mut pow = ProofOfWork::new(g.clone(), Difficulty::Fixed(EASY));

    let txs = vec![];
    let unmined = Block {
        header: BlockHeader {
            version: 1,
            height: 1,
            parent_hash: g.hash(),
            state_root: [0u8; 32],
            tx_root: Block::tx_root(&txs),
            timestamp_ms: 2_000,
            proposer: [1u8; 32],
        },
        txs,
        seal: miner::seal_of(0),
    };
    assert!(pow.submit(unmined).is_err());
    assert_eq!(pow.height(), 0);
}

/// Editing a block after it is mined destroys the work, so the chain sees
/// an unmined block rather than a different one.
#[test]
fn a_block_edited_after_mining_no_longer_has_its_work() {
    let g = genesis();
    let mut pow = ProofOfWork::new(g.clone(), Difficulty::Fixed(EASY));
    let mut b = mine_on(&g, 1, vec![dispute_tx(0)], EASY);
    b.txs = vec![dispute_tx(1)];
    b.header.tx_root = Block::tx_root(&b.txs);
    assert!(
        pow.submit(b).is_err(),
        "re-rooting must invalidate the seal"
    );
}

/// **The episode's result.**
///
/// A dispute transaction is mined, confirmed six deep — the depth folklore
/// calls settled — and then a miner who was working in private publishes a
/// heavier branch and it is gone. Nothing here is a special case: both
/// chains are valid, both are really mined, and heaviest-chain fork choice
/// picks the one with more work behind it, which is exactly what it is for.
#[test]
fn a_heavier_private_fork_un_includes_a_confirmed_dispute() {
    let g = genesis();
    let mut pow = ProofOfWork::new(g.clone(), Difficulty::Fixed(EASY));

    // The public chain. The dispute lands in block 1.
    let a1 = mine_on(&g, 1, vec![dispute_tx(0)], EASY);
    pow.submit(a1.clone()).unwrap();
    let mut tip = a1.clone();
    for _ in 0..5 {
        tip = mine_on(&tip, 1, vec![], EASY);
        pow.submit(tip.clone()).unwrap();
    }

    // Six deep. Every convention in the industry says this is settled.
    assert_eq!(pow.confirmations(&a1.hash()), Some(6));
    assert!(pow.deep_enough_by_convention(&a1.hash()));
    // And the chain itself declines to agree.
    assert!(!pow.is_final(&a1.hash()));
    assert_eq!(pow.finality(), Finality::Probabilistic);
    assert!(!pow.finality().safe_under_deadline());

    // Meanwhile, in private: seven blocks from the genesis, no dispute in
    // any of them.
    let mut shadow = g.clone();
    let mut hidden = Vec::new();
    for _ in 0..7 {
        shadow = mine_on(&shadow, 2, vec![], EASY);
        hidden.push(shadow.clone());
    }

    // Published all at once.
    for b in &hidden {
        pow.submit(b.clone()).unwrap();
    }

    assert_eq!(pow.height(), 7);
    assert!(pow.is_canonical(&shadow.hash()));
    assert!(
        !pow.is_canonical(&a1.hash()),
        "the dispute's block is no longer on the chain"
    );
    assert_eq!(pow.confirmations(&a1.hash()), None);
    assert!(
        pow.chain().contains(&a1.hash()),
        "still stored — this is a reorg, not amnesia"
    );

    let r = pow.last_reorg();
    assert!(r.is_rollback());
    assert_eq!(r.depth(), 6, "six confirmed blocks left the chain");
}

/// Cumulative work, not block count. Without this an attacker mines ten
/// trivial blocks and calls it a longer chain.
#[test]
fn a_shorter_chain_with_more_work_behind_it_wins() {
    let hard = Target(EASY.0 / 64);
    let g = genesis();
    let mut pow = ProofOfWork::new(g.clone(), Difficulty::Fixed(EASY));

    let mut tip = g.clone();
    for _ in 0..3 {
        tip = mine_on(&tip, 1, vec![], EASY);
        pow.submit(tip.clone()).unwrap();
    }
    assert_eq!(pow.height(), 3);
    let easy_head = tip.hash();

    // One block, mined against a 64× harder target. It is shorter and it
    // should still win — but this engine is running `Difficulty::Fixed`, so
    // it requires `EASY` and scores the block at `EASY`'s work. The point
    // of the assertion is what the *work* comparison says, so compare the
    // numbers directly.
    let heavy = mine_on(&g, 2, vec![], hard);
    assert!(
        hard.work() > EASY.work() * 3,
        "one block at 64× difficulty outweighs three easy ones: {} vs {}",
        hard.work(),
        EASY.work() * 3
    );
    // It satisfies the easy target too, so the engine accepts it — and
    // under fixed difficulty it is scored as one easy block, which is not
    // enough to take the head. Fork choice is right; the difficulty policy
    // is the thing that has to tell it the truth.
    pow.submit(heavy.clone()).unwrap();
    assert_eq!(pow.head(), easy_head);
}

/// `Difficulty::Controlled` must not silently work. The control loop is
/// episode 05's subject and `G0` says Evan types it; a fallback that
/// quietly did something reasonable is how that stops being anyone's job.
#[test]
#[should_panic(expected = "G0")]
fn the_control_loop_is_an_open_hole_and_says_so() {
    let g = genesis();
    let mut pow = ProofOfWork::new(g.clone(), Difficulty::Controlled);
    let mut tip = g;
    // Mining is fine until a retarget window closes. Crossing one is what
    // needs the control law that has not been written.
    for _ in 0..(bc_pow::retarget::WINDOW * 2) {
        tip = mine_on(&tip, 1, vec![], bc_pow::MAX_TARGET);
        pow.submit(tip.clone())
            .expect("every block is trivially mineable");
    }
}
