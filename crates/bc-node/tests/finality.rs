//! The claim the `reorg` demo makes, as assertions.
//!
//! The demo prints it; this runs the same scenario in the fast suite so a
//! regression is a red test rather than a binary nobody ran. Both build
//! the scene from [`bc_node::scenario`], so they cannot drift apart.

use bc_bft::Bft;
use bc_block::Consensus;
use bc_node::action::Action;
use bc_node::scenario::{self, Scene, EASY};
use bc_node::Node;
use bc_pow::{Difficulty, ProofOfWork};

fn scene() -> Scene {
    scenario::build()
}

/// Every transaction body the node executes survives a round trip. A
/// decoder that silently produced a different action would make the
/// canonical chain mean something other than what was signed.
#[test]
fn every_action_round_trips_through_a_transaction_body() {
    for tx in scene().txs {
        let back = Action::decode(tx.payload, &tx.body).expect("decodes");
        assert_eq!(back.encode(), tx.body, "{:?}", tx.payload);
        assert_eq!(back.payload(), tx.payload);
    }
}

#[test]
fn a_truncated_body_is_refused_rather_than_misread() {
    for tx in scene().txs {
        assert_eq!(
            Action::decode(tx.payload, &tx.body[..tx.body.len() - 1]),
            None
        );
        let mut long = tx.body.clone();
        long.push(0);
        assert_eq!(Action::decode(tx.payload, &long), None);
    }
}

/// **The episode's result, proof-of-work half.** Black replies inside the
/// window, is buried past the depth convention calls settled, and loses
/// the pot to a heavier fork that reaches past the deadline.
#[test]
fn under_proof_of_work_a_reorg_takes_a_defended_game_away() {
    let s = scene();
    let g = scenario::genesis();
    let mut node = Node::new(
        ProofOfWork::new(g.clone(), Difficulty::Fixed(EASY)),
        s.seed.clone(),
    );

    let b1 = scenario::mined(&g.header, 1, vec![s.open()]);
    node.submit(b1.clone()).unwrap();
    let b2 = scenario::mined(&b1.header, 1, vec![s.dispute_open()]);
    node.submit(b2.clone()).unwrap();
    let b3 = scenario::mined(&b2.header, 1, vec![s.white_move()]);
    node.submit(b3.clone()).unwrap();

    let deadline = node.ledger().dispute(&s.channel).unwrap().deadline_block;
    let defence = scenario::mined(&b3.header, 1, vec![s.black_defence()]);
    node.submit(defence.clone()).unwrap();
    assert!(4 < deadline, "the defence must be inside the window");

    let mut tip = defence.header;
    for _ in 5..=10 {
        let b = scenario::mined(&tip, 1, vec![]);
        tip = b.header;
        node.submit(b).unwrap();
    }
    assert!(
        node.consensus.deep_enough_by_convention(&defence.hash()),
        "six deep, which convention calls settled"
    );
    assert!(!node.consensus.is_final(&defence.hash()));

    let mut tip = b3.header;
    let mut deepest = 0;
    for h in 4..=deadline + 1 {
        let txs = if h == deadline + 1 {
            vec![s.finalize()]
        } else {
            vec![]
        };
        let b = scenario::mined(&tip, 2, txs);
        tip = b.header;
        node.submit(b).unwrap();
        deepest = deepest.max(node.consensus.last_reorg().depth());
    }

    assert!(deepest >= 6, "reorg only cut {deepest} deep");
    assert!(!node.consensus.is_canonical(&defence.hash()));
    assert!(
        node.consensus.block(&defence.hash()).is_some(),
        "a reorg is not amnesia"
    );
    assert!(!node.ledger().in_dispute(&s.channel), "it finalised");
    assert_eq!(node.balance(&s.white_pk), 1_100, "White took the pot");
    assert_eq!(node.balance(&s.black_pk), 900);
    assert_eq!(
        node.balance(&s.white_pk) + node.balance(&s.black_pk),
        2_000,
        "E1: nothing minted"
    );
}

/// **The BFT half.** The identical attack, with every fork block properly
/// signed by a real quorum, and the defence survives.
#[test]
fn under_bft_the_same_fork_is_refused_and_the_defence_stands() {
    let s = scene();
    let (keys, set) = scenario::validators();
    let g = scenario::genesis();
    let mut node = Node::new(Bft::new(g.clone(), set), s.seed.clone());

    let b1 = scenario::certified(&g.header, &keys, vec![s.open()]);
    node.submit(b1.clone()).unwrap();
    let b2 = scenario::certified(&b1.header, &keys, vec![s.dispute_open()]);
    node.submit(b2.clone()).unwrap();
    let b3 = scenario::certified(&b2.header, &keys, vec![s.white_move()]);
    node.submit(b3.clone()).unwrap();

    let deadline = node.ledger().dispute(&s.channel).unwrap().deadline_block;
    let defence = scenario::certified(&b3.header, &keys, vec![s.black_defence()]);
    node.submit(defence.clone()).unwrap();
    assert!(
        node.consensus.is_final(&defence.hash()),
        "committed means final, at depth one"
    );

    // The same fork, quorum-signed, every block refused.
    let mut tip = b3.header;
    let mut refused = 0;
    for _ in 4..=deadline + 1 {
        let b = scenario::certified(&tip, &keys, vec![]);
        tip = b.header;
        if node.submit(b).is_err() {
            refused += 1;
        }
    }
    assert_eq!(refused, (deadline + 1 - 3) as usize, "a fork block got in");
    assert!(node.consensus.is_canonical(&defence.hash()));

    // Time passes on the honest chain instead; White is the one who never
    // moved, so White is the one who loses on time.
    let mut tip = defence.header;
    for h in 5..=deadline + 2 {
        let txs = if h == deadline + 2 {
            vec![s.finalize()]
        } else {
            vec![]
        };
        let b = scenario::certified(&tip, &keys, txs);
        tip = b.header;
        node.submit(b).unwrap();
    }
    assert!(!node.ledger().in_dispute(&s.channel));
    assert_eq!(node.balance(&s.black_pk), 1_100, "the defence stood");
    assert_eq!(node.balance(&s.white_pk), 900);
}

/// The state is a function of the canonical chain and nothing else, so
/// replaying it twice gives the same answer — and a node that only ever
/// saw the winning fork agrees with one that lived through the reorg.
#[test]
fn state_is_a_pure_function_of_the_canonical_chain() {
    let s = scene();
    let g = scenario::genesis();
    let mut a = Node::new(
        ProofOfWork::new(g.clone(), Difficulty::Fixed(EASY)),
        s.seed.clone(),
    );
    let b1 = scenario::mined(&g.header, 1, vec![s.open()]);
    a.submit(b1.clone()).unwrap();
    let b2 = scenario::mined(&b1.header, 1, vec![s.dispute_open()]);
    a.submit(b2.clone()).unwrap();

    let before = (a.balance(&s.white_pk), a.balance(&s.black_pk));
    let history = a.history().to_vec();
    a.apply();
    assert_eq!((a.balance(&s.white_pk), a.balance(&s.black_pk)), before);
    assert_eq!(a.history(), history);

    // A second node fed the same blocks reaches the same state.
    let mut c = Node::new(
        ProofOfWork::new(g.clone(), Difficulty::Fixed(EASY)),
        s.seed.clone(),
    );
    c.submit(b1).unwrap();
    c.submit(b2).unwrap();
    assert_eq!((c.balance(&s.white_pk), c.balance(&s.black_pk)), before);
    assert_eq!(c.history(), history);
}
