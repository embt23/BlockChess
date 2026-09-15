//! Episode 10 — reserved blockspace, and the half it does not buy.
//!
//! The attack: *"I'll censor your dispute."*
//!
//! ```sh
//! cargo run --release -p bc-node --bin censor
//! ```
//!
//! Three runs of one scenario. Black is disputing and must get a reply
//! on-chain before a deadline. In each run something is in the way.
//!
//! | run | in the way | outcome |
//! |---|---|---|
//! | 1 | a flood of ordinary traffic, **no reserve** | Black is crowded out and loses |
//! | 2 | the same flood, **25% reserved** | Black gets in and defends |
//! | 3 | a proposer who simply omits Black | Black loses **anyway** |
//!
//! Run 3 is the honest part. The reserve is a real defence against being
//! *squeezed out* and no defence at all against being *left out* — a block
//! containing nothing satisfies every rule in `bc-block::gas`. What covers
//! run 3 is the rotation argument in `bc_bft::censorship`, which is `G0`
//! and is not written yet.

use bc_block::gas::GasSchedule;
use bc_block::{Block, BlockHeader, Tx};
use bc_node::build::{self, Proposer};
use bc_node::scenario::{self, Scene, EASY};
use bc_node::Node;
use bc_pow::{miner, Difficulty, ProofOfWork};

/// Small enough that a flood is a readable number of transactions.
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

/// Run the scenario to its verdict. Returns whether Black's reply was ever
/// included, and the final balances.
fn run(scene: &Scene, gas: GasSchedule, proposer: Proposer) -> (bool, u128, u128) {
    let g = scenario::genesis();
    let mut node = Node::new(
        ProofOfWork::new(g.clone(), Difficulty::Fixed(EASY)).with_gas(gas),
        scene.seed.clone(),
    );

    // Open the game and the dispute, and let White move on-chain. None of
    // this is contested, so it goes in uncontested blocks.
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

    // Black broadcasts the reply immediately. It sits in a mempool behind a
    // flood of ordinary traffic — the worst ordering for Black, and the one
    // an attacker would arrange.
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
            // Once it lands, drop it from the mempool and stop flooding —
            // the rest of the window is uneventful.
            mempool.retain(|t| !t.is_dispute());
        }
    }

    // Settle whatever the heights imply.
    let fin = mine(&tip, 2, vec![scene.finalize()]);
    let _ = node.submit(fin);

    (
        included,
        node.balance(&scene.white_pk),
        node.balance(&scene.black_pk),
    )
}

fn report(label: &str, included: bool, w: u128, b: u128) -> bool {
    let defended = b > w;
    println!(
        "    reply included   {}",
        if included { "yes" } else { "NO" }
    );
    println!("    balances         white {w} · black {b}");
    println!(
        "    verdict          {}\n",
        if defended {
            "Black defended"
        } else {
            "Black loses a game they answered in time"
        }
    );
    assert_eq!(w + b, 2_000, "{label}: E1, nothing minted");
    defended
}

fn main() {
    let scene = scenario::build();
    let gas = protected();
    println!("BlockChess — episode 10: reserved blockspace\n");
    println!(
        "  gas limit {LIMIT} · reserve {}% = {} · ordinary traffic capped at {}",
        gas.dispute_reserve_bps / 100,
        gas.reserve(),
        gas.non_dispute_cap()
    );
    println!(
        "  the flood is 40 transactions of {FLOOD_GAS} gas — {} in total, \n  \
         which is {}× the whole block\n",
        40 * FLOOD_GAS as u64,
        40 * FLOOD_GAS as u64 / LIMIT
    );

    println!("  ── 1. no reserve, and a flood ──────────────────────────");
    println!("    Nobody censors anything. Ordinary traffic simply fills");
    println!("    every block before Black's reply is reached.");
    let (i1, w1, b1) = run(&scene, GasSchedule::unprotected(LIMIT), Proposer::Honest);
    let d1 = report("run 1", i1, w1, b1);

    println!("  ── 2. 25% reserved, same flood ─────────────────────────");
    println!("    The builder stops taking ordinary traffic at the cap, so");
    println!("    there is room left when Black's reply comes up.");
    let (i2, w2, b2) = run(&scene, protected(), Proposer::Honest);
    let d2 = report("run 2", i2, w2, b2);

    println!("  ── 3. 25% reserved, and a proposer who omits ───────────");
    println!("    Same reserve. The proposer just does not include Black.");
    println!("    Every block it produces is valid.");
    let (i3, w3, b3) = run(&scene, protected(), Proposer::censoring_disputes());
    let d3 = report("run 3", i3, w3, b3);

    assert!(!d1, "run 1 should have lost");
    assert!(d2, "run 2 should have defended");
    assert!(!d3, "run 3 should have lost");

    println!("  ────────────────────────────────────────────────────────");
    println!("  The reserve closes the squeeze-out and nothing else. A");
    println!("  block containing no disputes is a valid block, so run 3 is");
    println!("  not a bug in the rule — it is the rule's edge.");
    println!();
    println!("  What covers run 3 is rotation: a censoring proposer only");
    println!("  holds the pen for its own turns, and with at most f of them");
    println!("  in a row, a window longer than f reaches somebody honest.");
    println!("  That bound is `bc_bft::censorship`, it is `G0`, and it is");
    println!("  not written — see docs/g0-holes.md.");
    println!();
    println!("  It also has a sting the reserve does not: the window that");
    println!("  has to beat f is not Δ. `Dispute::arm` shrinks it toward");
    println!("  MIN_MOVE_BLOCKS as a budget runs down, so a channel that");
    println!("  negotiated Δ = 2048 is defended, in its last moves, by 8.");
}
