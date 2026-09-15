//! Episode 10's claims, as assertions.
//!
//! The `censor` demo prints these; this runs them in the fast suite so a
//! regression is a red test rather than a binary nobody ran. Both build the
//! scene from [`bc_node::scenario`], so they cannot drift apart.

use bc_block::gas::{GasError, GasSchedule};
use bc_block::{Block, BlockHeader, Consensus, Payload, Tx};
use bc_node::build::{self, Proposer};
use bc_node::scenario::{self, Scene, EASY};
use bc_node::Node;
use bc_pow::{miner, Difficulty, ProofOfWork};

const LIMIT: u64 = 1_000_000;
const FLOOD_GAS: u32 = 50_000;

fn protected() -> GasSchedule {
    GasSchedule {
        limit: LIMIT,
        dispute_reserve_bps: 2_500,
    }
}

fn mine(parent: &BlockHeader, who: u8, txs: Vec<Tx>) -> Block {
    let header = BlockHeader {
        version: 1,
        height: parent.height + 1,
        parent_hash: parent.hash(),
        state_root: [0u8; 32],
        tx_root: Block::tx_root(&txs),
        timestamp_ms: parent.timestamp_ms + 2_000,
        proposer: [who; 32],
    };
    let nonce = miner::mine(&header, EASY, 100_000_000).expect("a nonce");
    Block {
        header,
        txs,
        seal: miner::seal_of(nonce),
    }
}

/// Returns (Black's reply was included, white balance, black balance).
fn run(scene: &Scene, gas: GasSchedule, proposer: Proposer) -> (bool, u128, u128) {
    let g = scenario::genesis();
    let mut node = Node::new(
        ProofOfWork::new(g.clone(), Difficulty::Fixed(EASY)).with_gas(gas),
        scene.seed.clone(),
    );
    let b1 = mine(&g.header, 1, vec![scene.open()]);
    node.submit(b1.clone()).unwrap();
    let b2 = mine(&b1.header, 1, vec![scene.dispute_open()]);
    node.submit(b2.clone()).unwrap();
    let b3 = mine(&b2.header, 1, vec![scene.white_move()]);
    node.submit(b3.clone()).unwrap();

    let deadline = node
        .ledger()
        .dispute(&scene.channel)
        .unwrap()
        .deadline_block;
    let mut mempool = scenario::flood(40, FLOOD_GAS);
    mempool.push(scene.black_defence());

    let mut tip = b3.header;
    let mut included = false;
    for _ in 4..=deadline + 1 {
        let txs = build::select(&mempool, &gas, proposer);
        included |= txs.iter().any(|t| t.is_dispute());
        let b = mine(&tip, 2, txs);
        tip = b.header;
        node.submit(b).expect("the builder respects the schedule");
        if included {
            mempool.retain(|t| !t.is_dispute());
        }
    }
    let fin = mine(&tip, 2, vec![scene.finalize()]);
    let _ = node.submit(fin);
    (
        included,
        node.balance(&scene.white_pk),
        node.balance(&scene.black_pk),
    )
}

/// Run 1. No censorship at all — ordinary traffic is simply enough to
/// starve a dispute, and the victim loses a game they answered in time.
#[test]
fn without_a_reserve_a_flood_alone_steals_the_game() {
    let s = scenario::build();
    let (included, w, b) = run(&s, GasSchedule::unprotected(LIMIT), Proposer::Honest);
    assert!(!included, "the flood should have crowded the reply out");
    assert_eq!((w, b), (1_100, 900), "White took the pot");
}

/// Run 2. The same flood, against a reserve.
#[test]
fn a_reserve_defeats_the_flood() {
    let s = scenario::build();
    let (included, w, b) = run(&s, protected(), Proposer::Honest);
    assert!(included, "the reserve should have held room");
    assert_eq!((w, b), (900, 1_100), "Black defended");
}

/// Run 3, and the honest one. A proposer that omits you beats the reserve,
/// because a block with no disputes in it breaks no rule.
#[test]
fn a_reserve_does_not_survive_a_proposer_who_omits_you() {
    let s = scenario::build();
    let (included, w, b) = run(&s, protected(), Proposer::censoring_disputes());
    assert!(!included);
    assert_eq!((w, b), (1_100, 900), "Black loses despite the reserve");
}

/// The engine enforces the schedule, so a proposer cannot opt out of the
/// reserve by building a block that ignores it.
#[test]
fn an_over_stuffed_block_is_rejected_by_the_chain() {
    let g = scenario::genesis();
    let mut pow = ProofOfWork::new(g.clone(), Difficulty::Fixed(EASY)).with_gas(protected());

    // 16 × 50_000 = 800_000 of ordinary traffic, over the 750_000 cap.
    let greedy = scenario::flood(16, FLOOD_GAS);
    assert_eq!(
        pow.submit(mine(&g.header, 1, greedy)),
        Err(bc_pow::PowError::Gas(GasError::CrowdsOutDisputes {
            non_dispute: 800_000,
            cap: 750_000,
        }))
    );
    assert_eq!(pow.height(), 0, "nothing got in");

    // At the cap it is fine.
    let ok = scenario::flood(15, FLOOD_GAS);
    assert!(pow.submit(mine(&g.header, 1, ok)).is_ok());
}

/// …and the same rule under BFT, because the reserve is a property of a
/// block rather than of how the block was agreed.
#[test]
fn the_reserve_is_enforced_under_bft_too() {
    use bc_bft::Bft;
    let (keys, set) = scenario::validators();
    let g = scenario::genesis();
    let mut bft = Bft::new(g.clone(), set).with_gas(protected());
    let over = scenario::certified(&g.header, &keys, scenario::flood(16, FLOOD_GAS));
    assert!(matches!(
        bft.submit(over),
        Err(bc_bft::BftError::Gas(GasError::CrowdsOutDisputes { .. }))
    ));
    let ok = scenario::certified(&g.header, &keys, scenario::flood(15, FLOOD_GAS));
    assert!(bft.submit(ok).is_ok());
}

/// Dispute traffic may take the whole block. The reserve is a floor, and a
/// chain that capped disputes at 25% would have the same bug with the sign
/// flipped.
#[test]
fn a_block_of_nothing_but_disputes_is_valid() {
    let g = scenario::genesis();
    let mut pow = ProofOfWork::new(g.clone(), Difficulty::Fixed(EASY)).with_gas(protected());
    let all_disputes: Vec<Tx> = (0..20)
        .map(|i| Tx {
            version: 1,
            nonce: i,
            sender: [9u8; 32],
            fee: 1,
            gas_limit: FLOOD_GAS,
            payload: Payload::DisputeMove,
            body: vec![i as u8],
            signature: [0u8; 64],
        })
        .collect();
    assert_eq!(
        all_disputes.iter().map(|t| t.gas_limit as u64).sum::<u64>(),
        LIMIT
    );
    assert!(pow.submit(mine(&g.header, 1, all_disputes)).is_ok());
}
