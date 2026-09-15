//! Episode 06, as an executable sentence: **money under a deadline
//! requires deterministic finality.**
//!
//! The same wagered game, the same dispute, the same defence, run twice —
//! once on proof-of-work and once on BFT. Only the consensus engine
//! differs. Nothing in `bc-channel`, `bc-adjudicator` or `bc-node::exec`
//! knows which one it is running on.
//!
//! ```sh
//! cargo run --release -p bc-node --bin reorg
//! ```
//!
//! ## The scenario
//!
//! White opens a dispute because Black stopped answering, and plays a move
//! on-chain. Black comes back **and defends in time**: their reply is
//! mined, and buried six deep, which every convention in the industry
//! calls settled.
//!
//! Then White publishes a fork they were mining in private, which does not
//! contain Black's reply and which reaches past the deadline.
//!
//! Under proof-of-work, Black's defence is un-included *after* the deadline
//! has already passed. Replaying the canonical chain, nobody ever answered
//! and White takes the pot. Black did everything right and lost anyway —
//! and waiting for more confirmations would not have helped, because the
//! deadline had already gone by.
//!
//! Under BFT the fork is refused before it is considered, and it is White,
//! who spent the window mining instead of moving, who loses on time.

use bc_adjudicator::state::Status;
use bc_bft::Bft;
use bc_block::Consensus;
use bc_node::scenario::{self, Scene, EASY, STAKE};
use bc_node::Node;
use bc_pow::{Difficulty, ProofOfWork};

fn main() {
    let scene = scenario::build();
    println!("BlockChess — episode 06: the same dispute on two chains\n");
    println!(
        "  stakes {STAKE} each · Δ = {} blocks · the pot is {}\n",
        scene.delta,
        STAKE * 2
    );

    let pow = run_pow(&scene);
    let bft = run_bft(&scene);

    println!("\n  ────────────────────────────────────────────────────────");
    println!("  proof-of-work   Black defended in time and {pow}");
    println!("  BFT             Black defended in time and {bft}");
    println!("  ────────────────────────────────────────────────────────\n");
    println!("  The dispute logic is byte-identical in both runs. The only");
    println!("  difference is whether a block that was confirmed can stop");
    println!("  being confirmed — which is why a chain holding money under");
    println!("  a deadline cannot use probabilistic finality, however many");
    println!("  confirmations it waits for. The deadline has already gone.");
}

fn run_pow(scene: &Scene) -> &'static str {
    println!("  ── proof-of-work ───────────────────────────────────────");
    let g = scenario::genesis();
    let mut node = Node::new(
        ProofOfWork::new(g.clone(), Difficulty::Fixed(EASY)),
        scene.seed.clone(),
    );

    let b1 = scenario::mined(&g.header, 1, vec![scene.open()]);
    node.submit(b1.clone()).unwrap();
    let b2 = scenario::mined(&b1.header, 1, vec![scene.dispute_open()]);
    node.submit(b2.clone()).unwrap();
    let b3 = scenario::mined(&b2.header, 1, vec![scene.white_move()]);
    node.submit(b3.clone()).unwrap();

    let deadline = node
        .ledger()
        .dispute(&scene.channel)
        .unwrap()
        .deadline_block;
    println!("    block 3   White moves on-chain; Black must reply by block {deadline}");

    let defence = scenario::mined(&b3.header, 1, vec![scene.black_defence()]);
    node.submit(defence.clone()).unwrap();
    println!(
        "    block 4   Black replies — {} blocks early",
        deadline - 4
    );

    let mut tip = defence.header;
    for _ in 5..=10 {
        let b = scenario::mined(&tip, 1, vec![]);
        tip = b.header;
        node.submit(b).unwrap();
    }
    let conf = node.consensus.confirmations(&defence.hash()).unwrap();
    println!("    block 10  Black's reply is {conf} deep — settled, by convention");
    assert!(node.consensus.deep_enough_by_convention(&defence.hash()));
    assert!(
        !node.consensus.is_final(&defence.hash()),
        "the chain itself never said final"
    );

    // The private fork: from block 3, so the dispute and White's move
    // survive, but Black's reply does not exist. It runs past the deadline
    // and finalises there.
    let mut tip = b3.header;
    let mut fork = Vec::new();
    for h in 4..=deadline + 1 {
        let txs = if h == deadline + 1 {
            vec![scene.finalize()]
        } else {
            vec![]
        };
        let b = scenario::mined(&tip, 2, txs);
        tip = b.header;
        fork.push(b);
    }
    println!(
        "    …meanwhile, White has been mining {} blocks in private, with",
        fork.len()
    );
    println!("      no reply from Black in any of them.\n");

    // The head flips on the block that overtakes, not on the last one.
    let mut deepest = 0;
    for b in &fork {
        node.submit(b.clone()).unwrap();
        deepest = deepest.max(node.consensus.last_reorg().depth());
    }
    println!("    published — reorg {deepest} blocks deep");
    assert!(deepest >= 6, "no reorg deep enough to matter");

    println!(
        "    block {}  Black's reply is GONE — it was never on this chain",
        node.consensus.height()
    );
    assert!(!node.consensus.is_canonical(&defence.hash()));
    assert!(
        node.consensus.block(&defence.hash()).is_some(),
        "still stored: a reorg is not amnesia"
    );

    report(&node, scene)
}

fn run_bft(scene: &Scene) -> &'static str {
    println!("\n  ── BFT ─────────────────────────────────────────────────");
    let (keys, set) = scenario::validators();
    let g = scenario::genesis();
    let mut node = Node::new(Bft::new(g.clone(), set), scene.seed.clone());

    let b1 = scenario::certified(&g.header, &keys, vec![scene.open()]);
    node.submit(b1.clone()).unwrap();
    let b2 = scenario::certified(&b1.header, &keys, vec![scene.dispute_open()]);
    node.submit(b2.clone()).unwrap();
    let b3 = scenario::certified(&b2.header, &keys, vec![scene.white_move()]);
    node.submit(b3.clone()).unwrap();

    let deadline = node
        .ledger()
        .dispute(&scene.channel)
        .unwrap()
        .deadline_block;
    println!("    block 3   White moves on-chain; Black must reply by block {deadline}");

    let defence = scenario::certified(&b3.header, &keys, vec![scene.black_defence()]);
    node.submit(defence.clone()).unwrap();
    println!(
        "    block 4   Black replies — {} blocks early",
        deadline - 4
    );
    assert!(node.consensus.is_final(&defence.hash()));
    println!("    block 4   …and it is FINAL. Not six deep. Final.");

    // The same attack. Every fork block is properly certified — the
    // validators really did sign them — and every one is still refused,
    // because it claims a height a quorum already committed.
    let mut tip = b3.header;
    let mut refused = 0;
    for _ in 4..=deadline + 1 {
        let b = scenario::certified(&tip, &keys, vec![]);
        tip = b.header;
        if node.submit(b).is_err() {
            refused += 1;
        }
    }
    println!("\n    White offers the same fork, properly signed by a quorum.");
    println!("    refused   {refused} blocks — every one of them");
    assert_eq!(refused, (deadline + 1 - 3) as usize, "a fork block got in");
    assert!(node.consensus.is_canonical(&defence.hash()));

    // Time passes on the honest chain instead. White spent the window
    // mining rather than moving, so White is the one who flags.
    let mut tip = defence.header;
    for h in 5..=deadline + 2 {
        let txs = if h == deadline + 2 {
            vec![scene.finalize()]
        } else {
            vec![]
        };
        let b = scenario::certified(&tip, &keys, txs);
        tip = b.header;
        node.submit(b).unwrap();
    }

    report(&node, scene)
}

fn report<C: Consensus>(node: &Node<C>, scene: &Scene) -> &'static str {
    let (w, b) = (node.balance(&scene.white_pk), node.balance(&scene.black_pk));
    println!("    balances  white {w} · black {b}");
    assert_eq!(w + b, 2_000, "E1: nothing minted, nothing lost");
    assert!(!node.ledger().in_dispute(&scene.channel), "it finalised");

    if w > b {
        println!(
            "    verdict   {:?} — Black lost a game they defended",
            Status::WhiteWins
        );
        "LOST the pot"
    } else {
        println!("    verdict   the defence stood");
        "KEPT their stake"
    }
}
