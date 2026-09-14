//! Milestone D: a full game played off-chain, signed at every ply, settled
//! cooperatively.
//!
//! The oracle is the game itself. Morphy's Opera Game has been published for
//! 168 years and ends in mate on move 17; if this run disagrees about the
//! result or the length, the protocol is wrong and not the record.

mod common;

use bc_channel::state::Status;
use bc_chess::Position;
use common::{table_with_rake, STAKE, START_BALANCE};

/// One shared copy of the move list, so the demo binary and this test can
/// never drift apart.
pub fn opera_moves() -> Vec<&'static str> {
    include_str!("../games/opera-1858.txt")
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .flat_map(|l| l.split_whitespace())
        .collect()
}

#[test]
fn the_opera_game_plays_signs_and_settles() {
    let moves = opera_moves();
    assert_eq!(moves.len(), 33, "seventeen moves, mate on the last");

    let mut t = table_with_rake(Position::startpos(), 200);
    let before = t.ledger.total_supply();
    t.play(&moves);

    assert_eq!(t.white.ply(), 33);
    assert_eq!(t.white.head().state.status, Status::WhiteWins);
    assert!(t.black.position().is_checkmate());
    assert_eq!(
        t.white.position().to_fen(),
        "1n1Rkb1r/p4ppp/4q3/4p1B1/4P3/8/PPP2PPP/2K5 b k - 1 17"
    );

    // Both players independently arrived at the same 33rd state hash, which
    // is the only thing either of them needs to have kept.
    assert_eq!(t.white.head().state.hash(), t.black.head().state.hash());

    let certified = t.black.countersign().expect("Black countersigns the mate");
    assert!(certified.verify_certified(&t.offer.white_pk, &t.offer.black_pk));

    let payout = t.ledger.close_game(&certified).expect("close");
    assert_eq!(payout.rake, 4, "2% of a pot of 200");
    assert_eq!(payout.white, 196);
    assert_eq!(
        t.ledger.balance(&t.offer.white_pk),
        START_BALANCE - STAKE + 196
    );
    assert_eq!(t.ledger.total_supply(), before);
}

#[test]
fn every_ply_is_a_link_in_one_chain() {
    // Replay the game keeping each state, then check the chain: state n's
    // prev_hash is state n−1's hash, all the way back to the opening.
    let mut t = table_with_rake(Position::startpos(), 0);
    let mut hashes = vec![t.white.head().state.hash()];
    for m in opera_moves() {
        t.ply(m);
        let s = t.white.head().state;
        assert_eq!(s.prev_hash, *hashes.last().unwrap(), "ply {}", s.ply);
        assert_eq!(s.ply as usize, hashes.len());
        hashes.push(s.hash());
    }
    assert_eq!(hashes.len(), 34);
}
