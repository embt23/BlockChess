//! What the ⅓ bound actually buys, demonstrated rather than asserted.
//!
//! Three situations, in increasing order of malice: a network that splits,
//! a validator that crashes, and a validator that says two different things
//! to two different people. In none of them do two validators commit
//! different blocks — which is the only promise BFT makes, and the reason
//! `spec/05`'s deadlines are safe on top of it.
//!
//! Liveness is a different promise and these tests are careful not to
//! confuse them. A partitioned network **stops**, and stopping is correct:
//! FLP says you cannot have both, and this protocol is always safe and
//! eventually live. A chain that halts costs everyone time; a chain that
//! forks costs someone money.

use bc_bft::msg::Msg;
use bc_bft::{Node, Step, ValidatorSet, Vote};
use bc_block::header::NO_PARENT;
use bc_block::{Block, BlockHeader};
use bc_net::Network;
use bc_sig::{SigningKey, VerifyingKey};

const N: usize = 4;

fn keys() -> Vec<SigningKey> {
    (0..N as u8)
        .map(|i| SigningKey::from_seed(&[i + 1; 32]))
        .collect()
}

fn set_of(ks: &[SigningKey]) -> ValidatorSet {
    ValidatorSet::uniform(&ks.iter().map(|k| k.verifying_key()).collect::<Vec<_>>())
}

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

fn candidate(parent: &Block, proposer: VerifyingKey, tag: u8) -> Block {
    Block {
        header: BlockHeader {
            version: 1,
            height: parent.header.height + 1,
            parent_hash: parent.hash(),
            state_root: [tag; 32],
            tx_root: Block::tx_root(&[]),
            timestamp_ms: 2_000,
            proposer: proposer.0,
        },
        txs: vec![],
        seal: vec![],
    }
}

fn dispatch(net: &mut Network<Msg>, node: &mut Node, i: usize, first: Vec<Msg>) {
    let mut pending = first;
    while let Some(m) = pending.pop() {
        net.broadcast(i, m.clone());
        pending.extend(node.receive(m));
    }
}

/// Every node commits the same block, or no node commits at all. Never two
/// different ones. Returns how many committed.
fn committed_agree(nodes: &[Node]) -> usize {
    let hashes: Vec<_> = nodes
        .iter()
        .filter_map(|n| n.committed().map(|c| c.block.hash()))
        .collect();
    assert!(
        hashes.windows(2).all(|w| w[0] == w[1]),
        "SAFETY VIOLATION: validators committed different blocks"
    );
    hashes.len()
}

/// A 2-2 split. Neither side holds a quorum of 3, so neither side can
/// commit — and neither side commits something the other would disagree
/// with, because neither commits anything.
///
/// This is the shape of the whole argument. Safety does not depend on the
/// network behaving; liveness does.
#[test]
fn a_network_split_in_half_halts_rather_than_forking() {
    let ks = keys();
    let set = set_of(&ks);
    let g = genesis();
    let mut net: Network<Msg> = Network::new(N, 21);
    net.split(vec![vec![0, 1], vec![2, 3]]);

    let mut nodes: Vec<Node> = ks
        .iter()
        .map(|k| Node::new(k.clone(), set.clone(), 1))
        .collect();
    let cands: Vec<Block> = ks
        .iter()
        .enumerate()
        .map(|(i, k)| candidate(&g, k.verifying_key(), i as u8))
        .collect();

    for (i, node) in nodes.iter_mut().enumerate() {
        let out = node.start(cands[i].clone());
        dispatch(&mut net, node, i, out);
    }
    for _ in 0..20_000 {
        let Some(e) = net.step() else { break };
        let out = nodes[e.to].receive(e.msg);
        dispatch(&mut net, &mut nodes[e.to], e.to, out);
    }

    assert_eq!(
        committed_agree(&nodes),
        0,
        "a half-network must not reach a quorum of three"
    );
    assert!(net.dropped > 0, "the partition did nothing");
}

/// Heal the split and the chain finishes. Safety was never at risk;
/// liveness comes back with the network, which is what partial synchrony
/// means in one test.
#[test]
fn healing_the_split_lets_the_same_height_finish() {
    let ks = keys();
    let set = set_of(&ks);
    let g = genesis();
    let mut net: Network<Msg> = Network::new(N, 22);
    net.split(vec![vec![0, 1], vec![2, 3]]);

    let mut nodes: Vec<Node> = ks
        .iter()
        .map(|k| Node::new(k.clone(), set.clone(), 1))
        .collect();
    let cands: Vec<Block> = ks
        .iter()
        .map(|k| candidate(&g, k.verifying_key(), 0))
        .collect();

    for (i, node) in nodes.iter_mut().enumerate() {
        let out = node.start(cands[i].clone());
        dispatch(&mut net, node, i, out);
    }
    for _ in 0..200 {
        let Some(e) = net.step() else { break };
        let out = nodes[e.to].receive(e.msg);
        dispatch(&mut net, &mut nodes[e.to], e.to, out);
    }
    assert_eq!(committed_agree(&nodes), 0);

    // The partition lifts. Everyone re-sends what they have already said —
    // re-broadcast on reconnect, which is ordinary protocol hygiene and
    // not a special case: a vote is a signed statement and repeating it
    // costs nothing (`Tally::add` counts an identical repeat once).
    net.heal();
    for i in 0..N {
        let out = nodes[i].start(cands[i].clone());
        dispatch(&mut net, &mut nodes[i], i, out);
        let votes = nodes[i].resend();
        dispatch(&mut net, &mut nodes[i], i, votes);
    }
    for _ in 0..20_000 {
        let Some(e) = net.step() else { break };
        let out = nodes[e.to].receive(e.msg);
        dispatch(&mut net, &mut nodes[e.to], e.to, out);
    }

    assert_eq!(committed_agree(&nodes), N, "healed and still stuck");
}

/// One of four crashed is `f = 1`, which is exactly what this set
/// tolerates. Three is a quorum, so the height completes.
#[test]
fn one_crashed_validator_out_of_four_does_not_stop_the_chain() {
    let ks = keys();
    let set = set_of(&ks);
    let g = genesis();
    let mut net: Network<Msg> = Network::new(N, 23);
    // Node 3 is in no group: it neither hears nor is heard.
    net.split(vec![vec![0, 1, 2]]);

    let mut nodes: Vec<Node> = ks
        .iter()
        .map(|k| Node::new(k.clone(), set.clone(), 1))
        .collect();
    let cands: Vec<Block> = ks
        .iter()
        .map(|k| candidate(&g, k.verifying_key(), 0))
        .collect();

    // The round-0 proposer must be one of the survivors for round 0 to
    // succeed; with a crashed proposer the chain needs a round change,
    // which is the `G0` hole.
    let proposer = set.proposer(1, 0).unwrap();
    assert_ne!(proposer, ks[3].verifying_key(), "test setup");

    for (i, node) in nodes.iter_mut().enumerate() {
        let out = node.start(cands[i].clone());
        dispatch(&mut net, node, i, out);
    }
    for _ in 0..20_000 {
        let Some(e) = net.step() else { break };
        let out = nodes[e.to].receive(e.msg);
        dispatch(&mut net, &mut nodes[e.to], e.to, out);
    }

    assert_eq!(committed_agree(&nodes), 3, "three of four should commit");
    assert!(nodes[3].committed().is_none());
}

/// A validator that precommits two different blocks at the same height and
/// round has equivocated. The tally counts it once, keeps both signed
/// votes as evidence, and the quorum arithmetic is unmoved.
#[test]
fn an_equivocating_validator_is_counted_once_and_recorded() {
    let ks = keys();
    let set = set_of(&ks);
    let g = genesis();
    let a = candidate(&g, ks[0].verifying_key(), 1).hash();
    let b = candidate(&g, ks[0].verifying_key(), 2).hash();
    assert_ne!(a, b);

    let mut tally = bc_bft::Tally::new();
    assert!(tally.add(&set, Vote::sign(&ks[0], 1, 0, Step::Precommit, Some(a))));
    // The same statement again is fine — the network re-delivers.
    assert!(tally.add(&set, Vote::sign(&ks[0], 1, 0, Step::Precommit, Some(a))));
    // A different one is not.
    assert!(!tally.add(&set, Vote::sign(&ks[0], 1, 0, Step::Precommit, Some(b))));

    assert_eq!(tally.len(), 1, "one validator, one vote");
    assert_eq!(tally.stake_for(&set, Some(a)), 1);
    assert_eq!(tally.stake_for(&set, Some(b)), 0);
    assert_eq!(tally.equivocations().len(), 1);

    let e = tally.equivocations()[0];
    assert_eq!(e.validator, ks[0].verifying_key());
    assert!(
        e.first.is_valid() && e.second.is_valid(),
        "evidence stands alone"
    );
}

/// The equivocator's whole point would be to get two quorums. With `f = 1`
/// it cannot: even counting its vote on both sides, neither side reaches
/// three.
#[test]
fn one_equivocator_out_of_four_cannot_manufacture_two_quorums() {
    let ks = keys();
    let set = set_of(&ks);
    let g = genesis();
    let a = candidate(&g, ks[0].verifying_key(), 1).hash();
    let b = candidate(&g, ks[0].verifying_key(), 2).hash();

    // The best case for the attacker: the honest validators split evenly
    // and the equivocator tells each side what it wants to hear.
    let mut left = bc_bft::Tally::new();
    left.add(&set, Vote::sign(&ks[1], 1, 0, Step::Precommit, Some(a)));
    left.add(&set, Vote::sign(&ks[0], 1, 0, Step::Precommit, Some(a)));

    let mut right = bc_bft::Tally::new();
    right.add(&set, Vote::sign(&ks[2], 1, 0, Step::Precommit, Some(b)));
    right.add(&set, Vote::sign(&ks[3], 1, 0, Step::Precommit, Some(b)));
    right.add(&set, Vote::sign(&ks[0], 1, 0, Step::Precommit, Some(b)));

    assert_eq!(left.stake_for(&set, Some(a)), 2);
    assert!(left.stake_for(&set, Some(a)) < set.quorum(), "left forked");

    // The right side does reach three — and that is correct: three honest
    // and equivocating votes for one block is a real quorum, and there is
    // no second one, because the left side could not also reach three.
    assert_eq!(right.stake_for(&set, Some(b)), 3);
    assert!(right.stake_for(&set, Some(b)) >= set.quorum());
}

/// A vote from outside the set weighs nothing, however well signed.
#[test]
fn an_outsider_cannot_vote() {
    let ks = keys();
    let set = set_of(&ks);
    let outsider = SigningKey::from_seed(&[99u8; 32]);
    let mut tally = bc_bft::Tally::new();
    let v = Vote::sign(&outsider, 1, 0, Step::Precommit, Some([1u8; 32]));
    assert!(v.is_valid(), "the signature itself is fine");
    assert!(!tally.add(&set, v), "and it still does not count");
    assert_eq!(tally.stake_voted(&set), 0);
}
