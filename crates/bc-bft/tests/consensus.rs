//! Four validators, one simulated network, one block.
//!
//! Every message really travels: signed, delayed by a seeded amount,
//! reordered, and dropped if a partition says so. Nothing is hand-fed.

use bc_bft::msg::Msg;
use bc_bft::{Bft, Commit, Node, Step, ValidatorSet, Vote};
use bc_block::header::NO_PARENT;
use bc_block::{Block, BlockHeader, Consensus, Finality, Payload, Tx};
use bc_net::Network;
use bc_sig::{SigningKey, VerifyingKey};

const N: usize = 4;

fn keys() -> Vec<SigningKey> {
    (0..N as u8)
        .map(|i| SigningKey::from_seed(&[i + 1; 32]))
        .collect()
}

fn set_of(keys: &[SigningKey]) -> ValidatorSet {
    ValidatorSet::uniform(&keys.iter().map(|k| k.verifying_key()).collect::<Vec<_>>())
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

fn candidate(parent: &Block, proposer: VerifyingKey, txs: Vec<Tx>) -> Block {
    Block {
        header: BlockHeader {
            version: 1,
            height: parent.header.height + 1,
            parent_hash: parent.hash(),
            state_root: [0u8; 32],
            tx_root: Block::tx_root(&txs),
            timestamp_ms: parent.header.timestamp_ms + 2_000,
            proposer: proposer.0,
        },
        txs,
        seal: vec![],
    }
}

/// Run one height to completion over the network. Returns each node's
/// conclusion.
fn run_height(seed: u64, txs: Vec<Tx>) -> (Vec<Option<bc_bft::Committed>>, ValidatorSet, Block) {
    let ks = keys();
    let set = set_of(&ks);
    let g = genesis();
    let mut net: Network<Msg> = Network::new(N, seed);
    let mut nodes: Vec<Node> = ks
        .iter()
        .map(|k| Node::new(k.clone(), set.clone(), 1))
        .collect();

    // Each node's own candidate names that node as proposer; only the
    // round's actual proposer will be listened to.
    let cands: Vec<Block> = ks
        .iter()
        .map(|k| candidate(&g, k.verifying_key(), txs.clone()))
        .collect();

    // A node's own message is delivered to itself immediately and its
    // consequences cascade — a validator counts its own vote, and counting
    // it can be what completes a quorum. Skipping that is a bug in a
    // harness, not a property of the protocol.
    fn dispatch(net: &mut Network<Msg>, node: &mut Node, i: usize, first: Vec<Msg>) {
        let mut pending = first;
        while let Some(m) = pending.pop() {
            net.broadcast(i, m.clone());
            pending.extend(node.receive(m));
        }
    }

    for (i, node) in nodes.iter_mut().enumerate() {
        let out = node.start(cands[i].clone());
        dispatch(&mut net, node, i, out);
    }

    for _ in 0..20_000 {
        let Some(e) = net.step() else { break };
        let out = nodes[e.to].receive(e.msg);
        dispatch(&mut net, &mut nodes[e.to], e.to, out);
    }

    let proposer = set.proposer(1, 0).expect("a proposer");
    let agreed = cands
        .iter()
        .find(|c| c.header.proposer == proposer.0)
        .cloned()
        .expect("the proposer's candidate");
    (
        nodes.iter().map(|n| n.committed().cloned()).collect(),
        set,
        agreed,
    )
}

#[test]
fn four_validators_commit_the_same_block() {
    let (out, set, expected) = run_height(1, vec![dispute_tx(0)]);
    assert!(
        out.iter().all(|c| c.is_some()),
        "not every validator committed: {:?}",
        out.iter().map(|c| c.is_some()).collect::<Vec<_>>()
    );
    let hashes: Vec<_> = out
        .iter()
        .map(|c| c.as_ref().unwrap().block.hash())
        .collect();
    assert!(
        hashes.windows(2).all(|w| w[0] == w[1]),
        "validators committed different blocks"
    );
    assert_eq!(hashes[0], expected.hash());

    // And every certificate produced is independently checkable.
    for c in out.iter().flatten() {
        assert!(c.commit.verifies(&set, 1, &c.block.hash()));
    }
}

/// The interleaving must not matter. Same block under every seed.
#[test]
fn the_outcome_does_not_depend_on_the_message_order() {
    let mut agreed = Vec::new();
    for seed in 1..25 {
        let (out, _, expected) = run_height(seed, vec![]);
        assert!(out.iter().all(|c| c.is_some()), "seed {seed} stalled");
        for c in out.iter().flatten() {
            assert_eq!(c.block.hash(), expected.hash(), "seed {seed} diverged");
        }
        agreed.push(expected.hash());
    }
    assert!(agreed.windows(2).all(|w| w[0] == w[1]));
}

#[test]
fn a_committed_block_is_final_and_a_pow_style_fork_is_refused() {
    let (out, set, _) = run_height(3, vec![dispute_tx(0)]);
    let committed = out[0].clone().expect("a commit");

    let g = genesis();
    let mut bft = Bft::new(g.clone(), set.clone());
    assert_eq!(bft.finality(), Finality::Deterministic);
    assert!(bft.finality().safe_under_deadline());

    let mut sealed = committed.block.clone();
    sealed.seal = committed.commit.encode();
    bft.submit(sealed.clone()).expect("a committed block");

    assert_eq!(bft.height(), 1);
    assert!(bft.is_final(&sealed.hash()));
    assert_eq!(bft.final_height(), 1);

    // Now try what worked against proof-of-work: a competing block at the
    // same height, with a perfectly good certificate of its own would be
    // impossible to build — but even offering one is refused outright.
    let rival = candidate(&g, set.proposer(1, 0).unwrap(), vec![dispute_tx(9)]);
    let mut rival_sealed = rival.clone();
    rival_sealed.seal = committed.commit.encode();
    assert!(bft.submit(rival_sealed).is_err());
    assert!(
        bft.is_final(&sealed.hash()),
        "finality survived the attempt"
    );
    assert!(bft.is_canonical(&sealed.hash()));
}

/// A block with no certificate, or somebody else's, is not a block.
#[test]
fn a_block_without_a_real_quorum_is_refused() {
    let (out, set, _) = run_height(4, vec![]);
    let committed = out[0].clone().unwrap();
    let mut bft = Bft::new(genesis(), set.clone());

    let mut bare = committed.block.clone();
    bare.seal = vec![];
    assert!(bft.submit(bare).is_err());

    // A certificate with one signature short of a quorum.
    let mut short = committed.commit.clone();
    short.votes.truncate(set.quorum() as usize - 1);
    let mut b = committed.block.clone();
    b.seal = short.encode();
    assert!(bft.submit(b).is_err());

    // The real one still works, so the refusals above were about the
    // certificate and not about the block.
    let mut good = committed.block.clone();
    good.seal = committed.commit.encode();
    assert!(bft.submit(good).is_ok());
}

/// One validator signing the same precommit four times is one vote, not a
/// quorum. This is the cheapest possible attack on a certificate checker.
#[test]
fn one_validator_cannot_fill_a_quorum_by_itself() {
    let ks = keys();
    let set = set_of(&ks);
    let (out, _, _) = run_height(5, vec![]);
    let real = out[0].clone().unwrap();
    let hash = real.block.hash();

    let solo = Vote::sign(&ks[0], 1, real.commit.round, Step::Precommit, Some(hash));
    let forged = Commit {
        round: real.commit.round,
        votes: vec![solo; 4],
    };
    assert!(!forged.verifies(&set, 1, &hash), "duplicates were counted");
}

/// Prevotes do not finalise anything. A certificate made of them must be
/// refused however much stake it carries.
#[test]
fn a_certificate_made_of_prevotes_finalises_nothing() {
    let ks = keys();
    let set = set_of(&ks);
    let (out, _, _) = run_height(6, vec![]);
    let real = out[0].clone().unwrap();
    let hash = real.block.hash();

    let prevotes = Commit {
        round: real.commit.round,
        votes: ks
            .iter()
            .map(|k| Vote::sign(k, 1, real.commit.round, Step::Prevote, Some(hash)))
            .collect(),
    };
    assert!(!prevotes.verifies(&set, 1, &hash));
}

#[test]
fn a_commit_certificate_round_trips_through_the_seal() {
    let (out, set, _) = run_height(7, vec![dispute_tx(1), dispute_tx(2)]);
    let c = out[0].clone().unwrap();
    let bytes = c.commit.encode();
    assert_eq!(Commit::decode(&bytes), Some(c.commit.clone()));
    assert_eq!(Commit::decode(&bytes[..bytes.len() - 1]), None);
    assert!(c.commit.verifies(&set, 1, &c.block.hash()));
}
