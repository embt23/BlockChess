//! Shared fixture: two funded players, an open channel, and their two views.
//!
//! Compiled separately into each test binary, so not every binary uses every
//! helper here.
#![allow(dead_code)]

use bc_channel::game::Channel;
use bc_channel::offer::{GameOffer, GameTerms};
use bc_channel::state::pos_hash;
use bc_channel::Ledger;
use bc_chess::{Color, Position};
use bc_sig::SigningKey;

pub const STAKE: u128 = 100;
pub const START_BALANCE: u128 = 1_000;

pub fn keys() -> (SigningKey, SigningKey, SigningKey) {
    (
        SigningKey::from_seed(&[1u8; 32]),
        SigningKey::from_seed(&[2u8; 32]),
        SigningKey::from_seed(&[3u8; 32]),
    )
}

pub fn offer_for(start: &Position, rake_bps: u16, server: &SigningKey) -> GameOffer {
    let (w, b, _) = keys();
    GameOffer {
        white_pk: w.verifying_key(),
        black_pk: b.verifying_key(),
        stake_white: STAKE,
        stake_black: STAKE,
        terms: GameTerms::for_clock(pos_hash(start), 180_000, 2_000),
        server_pk: server.verifying_key(),
        open_nonce: [7u8; 32],
        rake_bps,
        expiry_block: 100,
    }
}

/// A funded ledger with the channel already open, plus both players' views.
pub struct Table {
    pub ledger: Ledger,
    pub white: Channel,
    pub black: Channel,
    pub offer: GameOffer,
}

pub fn table(start: Position) -> Table {
    table_with_rake(start, 0)
}

pub fn table_with_rake(start: Position, rake_bps: u16) -> Table {
    let (wsk, bsk, ssk) = keys();
    let offer = offer_for(&start, rake_bps, &ssk);

    let mut ledger = Ledger::new();
    ledger.credit(&offer.white_pk, START_BALANCE);
    ledger.credit(&offer.black_pk, START_BALANCE);
    let aw = wsk.sign(&offer.signing_bytes());
    let ab = bsk.sign(&offer.signing_bytes());
    ledger.open_game(&offer, &aw, &ab).expect("open");

    Table {
        white: Channel::open(offer, wsk, Color::White, start),
        black: Channel::open(offer, bsk, Color::Black, start),
        ledger,
        offer,
    }
}

impl Table {
    /// Play one move from the side to move, delivering it to the other.
    pub fn ply(&mut self, uci: &str) {
        let white_to_move = self.white.position().side == Color::White;
        let (mover, other) = if white_to_move {
            (&mut self.white, &mut self.black)
        } else {
            (&mut self.black, &mut self.white)
        };
        let mv = mover
            .position()
            .move_from_uci(uci)
            .unwrap_or_else(|| panic!("{uci} is not legal here"));
        let msg = mover.play(mv, 3_000).expect("play");
        other.receive(&msg, 3_100).expect("receive");
    }

    pub fn play(&mut self, moves: &[&str]) {
        for m in moves {
            self.ply(m);
        }
    }
}

/// A terms block with a shorter ply cap, for testing the forced draw.
pub fn table_capped(start: Position, max_plies: u16) -> Table {
    let (wsk, bsk, ssk) = keys();
    let mut offer = offer_for(&start, 0, &ssk);
    offer.terms.max_plies = max_plies;

    let mut ledger = Ledger::new();
    ledger.credit(&offer.white_pk, START_BALANCE);
    ledger.credit(&offer.black_pk, START_BALANCE);
    ledger
        .open_game(
            &offer,
            &wsk.sign(&offer.signing_bytes()),
            &bsk.sign(&offer.signing_bytes()),
        )
        .expect("open");

    Table {
        white: Channel::open(offer, wsk, Color::White, start),
        black: Channel::open(offer, bsk, Color::Black, start),
        ledger,
        offer,
    }
}

impl Table {
    /// The best evidence a player holds: the highest state their opponent
    /// signed, plus the packed position it names.
    pub fn evidence(&self, who: Color) -> (bc_channel::Signed, Vec<u8>) {
        let ch = match who {
            Color::White => &self.white,
            Color::Black => &self.black,
        };
        let (signed, pos) = ch.evidence();
        (signed, pos.pack().as_slice().to_vec())
    }
}

impl Table {
    /// A player's highest *fully certified* state and its position — what a
    /// threefold claim is built from, since one signature is not evidence.
    pub fn certified(&self, who: Color) -> (bc_channel::Signed, Vec<u8>) {
        let ch = match who {
            Color::White => &self.white,
            Color::Black => &self.black,
        };
        (
            *ch.certified(),
            ch.certified_position().pack().as_slice().to_vec(),
        )
    }
}
