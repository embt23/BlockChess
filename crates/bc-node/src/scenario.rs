//! The wagered game episode 06 argues about, and the two ways to seal a
//! block containing it.
//!
//! Shared by the `reorg` binary and `tests/finality.rs` because they make
//! the same claim and should not be able to drift: the demo narrates it
//! and the test asserts it, over one scenario built in one place.
//!
//! ## The setup
//!
//! Two players, 100 each. Two moves off-chain. Black stops answering, so
//! White opens a dispute with the highest state Black signed and plays the
//! move Black never acknowledged. Then **Black comes back and replies in
//! time** — which is the whole point. Every question after this is about
//! what the chain does to a defence that was made correctly.

use crate::action::Action;
use crate::exec::Seed;
use bc_bft::{Commit, Step, Validator, ValidatorSet, Vote};
use bc_block::header::NO_PARENT;
use bc_block::{Block, BlockHeader, Tx};
use bc_channel::game::Channel;
use bc_channel::offer::{GameOffer, GameTerms};
use bc_channel::state::pos_hash;
use bc_chess::{Color, Position};
use bc_hash::Hash;
use bc_pow::{miner, Target};
use bc_sig::{SigningKey, VerifyingKey};

/// Easy enough to mine a demo's worth of blocks, real enough to be a
/// nonce search rather than a counter.
pub const EASY: Target = Target(u128::MAX / 1_024);
pub const STAKE: u128 = 100;

pub fn genesis() -> Block {
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

fn header_on(parent: &BlockHeader, who: u8, txs: &[Tx]) -> BlockHeader {
    BlockHeader {
        version: 1,
        height: parent.height + 1,
        parent_hash: parent.hash(),
        state_root: [0u8; 32],
        tx_root: Block::tx_root(txs),
        timestamp_ms: parent.timestamp_ms + 2_000,
        proposer: [who; 32],
    }
}

/// A block sealed by proof-of-work: a real nonce search against [`EASY`].
pub fn mined(parent: &BlockHeader, who: u8, txs: Vec<Tx>) -> Block {
    let header = header_on(parent, who, &txs);
    let nonce = miner::mine(&header, EASY, 100_000_000).expect("a nonce exists");
    Block {
        header,
        txs,
        seal: miner::seal_of(nonce),
    }
}

/// A block sealed by a ⅔ precommit quorum. The validators really sign it.
pub fn certified(parent: &BlockHeader, keys: &[SigningKey], txs: Vec<Tx>) -> Block {
    let header = header_on(parent, 1, &txs);
    let hash = header.hash();
    Block {
        header,
        txs,
        seal: Commit {
            round: 0,
            votes: keys
                .iter()
                .map(|k| Vote::sign(k, header.height, 0, Step::Precommit, Some(hash)))
                .collect(),
        }
        .encode(),
    }
}

/// Four validators with equal stake, so `f = 1`.
pub fn validators() -> (Vec<SigningKey>, ValidatorSet) {
    let keys: Vec<SigningKey> = (0..4u8)
        .map(|i| SigningKey::from_seed(&[i + 40; 32]))
        .collect();
    let set = ValidatorSet::new(
        keys.iter()
            .map(|k| Validator {
                key: k.verifying_key(),
                stake: 1,
            })
            .collect(),
    );
    (keys, set)
}

/// Everything the two runs need, built once.
pub struct Scene {
    pub seed: Seed,
    /// In the order they are mined: open, dispute-open, White's move,
    /// Black's defence, finalize.
    pub txs: [Tx; 5],
    pub white_pk: VerifyingKey,
    pub black_pk: VerifyingKey,
    pub channel: Hash,
    pub delta: u32,
}

impl Scene {
    pub fn open(&self) -> Tx {
        self.txs[0].clone()
    }
    pub fn dispute_open(&self) -> Tx {
        self.txs[1].clone()
    }
    pub fn white_move(&self) -> Tx {
        self.txs[2].clone()
    }
    pub fn black_defence(&self) -> Tx {
        self.txs[3].clone()
    }
    pub fn finalize(&self) -> Tx {
        self.txs[4].clone()
    }
}

pub fn build() -> Scene {
    let white_sk = SigningKey::from_seed(&[1u8; 32]);
    let black_sk = SigningKey::from_seed(&[2u8; 32]);
    let (white_pk, black_pk) = (white_sk.verifying_key(), black_sk.verifying_key());

    let start = Position::startpos();
    let offer = GameOffer {
        white_pk,
        black_pk,
        stake_white: STAKE,
        stake_black: STAKE,
        // A short clock, so Δ is the bullet window and the run is eighty
        // blocks rather than two thousand.
        terms: GameTerms::for_clock(pos_hash(&start), 60_000, 0),
        server_pk: VerifyingKey([0u8; 32]),
        open_nonce: [11u8; 32],
        rake_bps: 0,
        expiry_block: 10_000,
    };
    let channel = offer.channel_id();

    let mut w = Channel::open(offer, white_sk.clone(), Color::White, start);
    let mut b = Channel::open(offer, black_sk.clone(), Color::Black, start);
    for (i, uci) in ["e2e4", "e7e5"].iter().enumerate() {
        let (mover, other) = if i % 2 == 0 {
            (&mut w, &mut b)
        } else {
            (&mut b, &mut w)
        };
        let mv = mover.position().move_from_uci(uci).expect("a legal move");
        let msg = mover.play(mv, 4_000).expect("play");
        other.receive(&msg, 4_100).expect("receive");
    }

    // White's evidence is the highest state *Black* signed — it rode along
    // with Black's own last move, so the waiting player always holds one.
    let (evidence, pos) = w.evidence();
    let white_mv = pos.move_from_uci("g1f3").expect("a legal move for White");
    let black_mv = pos.make_move(white_mv).generate_legal().as_slice()[0];

    Scene {
        seed: Seed::default()
            .credit(white_pk, 1_000)
            .credit(black_pk, 1_000),
        txs: [
            Action::OpenGame {
                offer: Box::new(offer),
                white_sig: white_sk.sign(&offer.signing_bytes()),
                black_sig: black_sk.sign(&offer.signing_bytes()),
            }
            .into_tx(0, white_pk.0),
            Action::DisputeOpen {
                initiator: Color::White,
                signed: Box::new(evidence),
                packed_pos: pos.pack().as_slice().to_vec(),
            }
            .into_tx(1, white_pk.0),
            Action::DisputeMove {
                channel_id: channel,
                mover: Color::White,
                mv: white_mv,
                claim: None,
            }
            .into_tx(2, white_pk.0),
            Action::DisputeMove {
                channel_id: channel,
                mover: Color::Black,
                mv: black_mv,
                claim: None,
            }
            .into_tx(0, black_pk.0),
            Action::DisputeFinalize {
                channel_id: channel,
            }
            .into_tx(3, white_pk.0),
        ],
        white_pk,
        black_pk,
        channel,
        delta: offer.terms.delta_blocks(),
    }
}
