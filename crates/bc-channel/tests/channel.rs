//! The channel protocol, from both sides — including the side that lies.
//!
//! Each test here is one row of the threat model in `spec/00-overview.md`.
//! The happy path is one test; the rest are the reasons the happy path is
//! safe.

mod common;

use bc_channel::game::ChannelError;
use bc_channel::state::{GameState, Status, STATE_LEN};
use bc_channel::MoveMsg;
use bc_chess::{Color, Move, Position};
use bc_sig::SigningKey;
use common::{table, Table};

/// Fool's mate: the shortest complete game there is.
const FOOLS: [&str; 4] = ["f2f3", "e7e5", "g2g4", "d8h4"];

#[test]
fn a_whole_game_certifies_every_ply() {
    let mut t = table(Position::startpos());
    t.play(&FOOLS);

    assert_eq!(t.white.ply(), 4);
    assert_eq!(t.white.head().state.status, Status::BlackWins);
    assert!(t.white.is_over());

    // White was mated, so White is the one who must countersign the state
    // Black asserted. Until then it is a claim, not a certificate.
    assert!(!t.white.head().is_certified());
    let certified = t.white.countersign().expect("countersign");
    assert!(certified.is_certified());
    assert!(certified.verify_certified(&t.offer.white_pk, &t.offer.black_pk));
}

#[test]
fn signing_a_state_is_signing_the_whole_game() {
    // The hash chain: change any earlier ply and the current state's hash
    // changes, so a signature at ply n is a statement about plies 0..n.
    let mut t = table(Position::startpos());
    t.play(&FOOLS[..3]);
    let real = t.black.head().state;

    let mut forged = real;
    forged.prev_hash[0] ^= 1;
    assert_ne!(forged.hash(), real.hash());

    let mut moved = real;
    moved.mv ^= 1;
    assert_ne!(moved.hash(), real.hash());
}

#[test]
fn the_state_encoding_is_109_bytes_and_exact() {
    let mut t = table(Position::startpos());
    t.play(&FOOLS[..2]);
    let s = t.white.head().state;
    let bytes = s.encode();
    assert_eq!(bytes.len(), STATE_LEN);
    assert_eq!(STATE_LEN, 109);
    assert_eq!(GameState::decode(&bytes), Some(s));
    assert_eq!(GameState::decode(&bytes[..108]), None);

    // Every field is covered by the hash.
    let mut b = bytes;
    for i in 0..STATE_LEN {
        b[i] ^= 0xFF;
        if let Some(other) = GameState::decode(&b) {
            assert_ne!(other.hash(), s.hash(), "byte {i} does not affect the hash");
        }
        b[i] ^= 0xFF;
    }
}

// ---------------------------------------------------------------------------
// What a liar cannot do
// ---------------------------------------------------------------------------

/// Take a legitimate message and let a test mangle it before delivery.
fn tampered(t: &mut Table, uci: &str, f: impl FnOnce(&mut MoveMsg)) -> ChannelError {
    let white_to_move = t.white.position().side == Color::White;
    let (mover, other) = if white_to_move {
        (&mut t.white, &mut t.black)
    } else {
        (&mut t.black, &mut t.white)
    };
    let mv = mover.position().move_from_uci(uci).unwrap();
    let mut msg = mover.play(mv, 3_000).expect("play");
    f(&mut msg);
    other.receive(&msg, 3_100).expect_err("should be refused")
}

#[test]
fn a_state_for_another_channel_is_not_evidence_here() {
    let mut t = table(Position::startpos());
    let e = tampered(&mut t, "e2e4", |m| m.state.channel_id[0] ^= 1);
    assert_eq!(e, ChannelError::WrongChannel);
}

#[test]
fn the_chain_must_link() {
    let mut t = table(Position::startpos());
    t.play(&["e2e4"]);
    let e = tampered(&mut t, "e7e5", |m| m.state.prev_hash[3] ^= 1);
    assert_eq!(e, ChannelError::BrokenChain);
}

#[test]
fn plies_do_not_skip() {
    let mut t = table(Position::startpos());
    let e = tampered(&mut t, "e2e4", |m| m.state.ply = 7);
    assert_eq!(e, ChannelError::WrongPly);
}

#[test]
fn the_position_hash_must_be_the_position_the_move_reaches() {
    let mut t = table(Position::startpos());
    let e = tampered(&mut t, "e2e4", |m| m.state.pos_hash[0] ^= 1);
    assert_eq!(e, ChannelError::WrongPosition);
}

#[test]
fn an_illegal_move_is_refused_and_does_not_panic() {
    let mut t = table(Position::startpos());
    // A move the rules do not allow, wearing a valid signature.
    let e = tampered(&mut t, "e2e4", |m| {
        m.mv = Move::normal(12, 36); // e2e5
        m.state.mv = m.mv.0;
    });
    assert_eq!(e, ChannelError::IllegalMove);

    // And sixteen bits of nonsense from a hostile peer.
    let mut t = table(Position::startpos());
    let e = tampered(&mut t, "e2e4", |m| {
        m.mv = Move(0xFFFF);
        m.state.mv = m.mv.0;
    });
    assert_eq!(e, ChannelError::IllegalMove);
}

#[test]
fn a_signature_from_the_wrong_key_is_worthless() {
    let mut t = table(Position::startpos());
    let impostor = SigningKey::from_seed(&[9u8; 32]);
    let e = tampered(&mut t, "e2e4", |m| {
        m.sig = impostor.sign(&m.state.hash());
    });
    assert_eq!(e, ChannelError::BadSignature);
}

#[test]
fn you_cannot_award_yourself_the_opponents_clock() {
    let mut t = table(Position::startpos());
    let e = tampered(&mut t, "e2e4", |m| m.state.clock_b_ms += 1_000);
    assert_eq!(e, ChannelError::OpponentClockMoved);
}

#[test]
fn you_cannot_keep_time_you_spent() {
    let mut t = table(Position::startpos());
    // Assert a 0.1s think after the receiver watched 3.1s go by. Grace is
    // 300ms; three seconds is not jitter.
    let e = tampered(&mut t, "e2e4", |m| m.state.clock_w_ms = 181_900);
    assert_eq!(e, ChannelError::ImplausibleClock);
}

#[test]
fn you_cannot_declare_a_win_you_did_not_earn() {
    let mut t = table(Position::startpos());
    let e = tampered(&mut t, "e2e4", |m| m.state.status = Status::WhiteWins);
    assert_eq!(e, ChannelError::UnjustifiedStatus);

    // Nor a draw out of a perfectly ordinary position.
    let mut t = table(Position::startpos());
    let e = tampered(&mut t, "e2e4", |m| m.state.status = Status::Draw);
    assert_eq!(e, ChannelError::UnjustifiedStatus);
}

#[test]
fn you_may_always_declare_your_own_loss() {
    // Nobody lies to lose, so this needs no checking — which is exactly why
    // resignation is free.
    let mut t = table(Position::startpos());
    let white_to_move = t.white.position().side == Color::White;
    assert!(white_to_move);
    let mv = t.white.position().move_from_uci("e2e4").unwrap();
    let mut msg = t.white.play(mv, 3_000).unwrap();
    msg.state.status = Status::BlackWins;
    msg.sig = SigningKey::from_seed(&[1u8; 32]).sign(&msg.state.hash());
    t.black
        .receive(&msg, 3_100)
        .expect("a self-declared loss is always believed");
    assert!(t.black.is_over());
}

#[test]
fn moving_twice_in_a_row_is_refused() {
    let mut t = table(Position::startpos());
    let mv = t.white.position().move_from_uci("e2e4").unwrap();
    let msg = t.white.play(mv, 3_000).unwrap();
    t.black.receive(&msg, 3_100).unwrap();
    assert_eq!(
        t.white.play(Move::normal(11, 27), 3_000).unwrap_err(),
        ChannelError::NotYourTurn
    );
}

#[test]
fn a_move_without_the_owed_countersignature_is_refused() {
    let mut t = table(Position::startpos());
    t.play(&["e2e4"]);
    let e = tampered(&mut t, "e7e5", |m| m.countersig = None);
    assert_eq!(e, ChannelError::MissingCountersig);
}

// ---------------------------------------------------------------------------
// Endings
// ---------------------------------------------------------------------------

#[test]
fn the_flag_can_be_conceded_without_a_move() {
    let mut t = table(Position::startpos());
    t.play(&["e2e4", "e7e5"]);
    let msg = t.white.concede_time().expect("concede");
    assert_eq!(msg.state.status, Status::BlackWins);
    assert_eq!(msg.state.clock_w_ms, 0);
    assert_eq!(msg.state.pos_hash, t.black.head().state.pos_hash);

    t.black.receive(&msg, 400_000).expect("receive concession");
    assert!(t.black.is_over());
    assert_eq!(t.black.head().state.ply, 3);
}

#[test]
fn a_clock_that_runs_out_refuses_to_produce_a_move() {
    let mut t = table(Position::startpos());
    let mv = t.white.position().move_from_uci("e2e4").unwrap();
    assert_eq!(
        t.white.play(mv, 200_000).unwrap_err(),
        ChannelError::Flagged
    );
}

#[test]
fn stalemate_is_a_draw_both_players_reach_independently() {
    // Qf1-f7 stalemates: Black is not in check and has no legal move. White
    // asserts the draw; Black checks it against their own board and agrees.
    let start = Position::from_fen("7k/8/6K1/8/8/8/8/5Q2 w - - 0 1").unwrap();
    let mut t = table(start);
    t.play(&["f1f7"]);
    assert_eq!(t.white.head().state.status, Status::Draw);
    assert_eq!(t.black.head().state.status, Status::Draw);
    assert!(t.black.position().is_stalemate());
}

#[test]
fn threefold_repetition_is_three_signatures_not_a_history_replay() {
    let start = Position::from_fen("7k/8/8/8/8/8/8/R6K w - - 0 1").unwrap();
    let mut t = table(start);
    // Shuffle the rook back and forth. Each return to a1 repeats the
    // position; the third occurrence is claimable as a draw.
    t.play(&["a1a2", "h8g8", "a2a1", "g8h8"]);
    assert_eq!(t.white.repetitions(), 2, "the start position, twice over");
    assert!(!t.white.is_over());

    // The halfmove clock has been ticking the whole time, so these positions
    // do *not* share a `pos_hash`. Repetition compares `rep_hash`, which is
    // the same bytes with the clock cleared.
    assert_ne!(
        t.white.head().state.pos_hash,
        bc_channel::pos_hash(&Position::from_fen("7k/8/8/8/8/8/8/R6K w - - 0 1").unwrap())
    );

    t.play(&["a1a2", "h8g8", "a2a1", "g8h8"]);
    // Black's return to h8 is the third occurrence, and Black claimed the
    // draw on the move that produced it.
    assert_eq!(t.white.repetitions(), 3);
    assert_eq!(t.black.head().state.status, Status::Draw);
    assert_eq!(t.white.head().state.status, Status::Draw);
}

#[test]
fn the_side_not_to_move_is_the_one_holding_evidence() {
    let mut t = table(Position::startpos());
    t.play(&["e2e4"]);
    // Black, who is to move, holds a certificate one ply behind plus White's
    // half-signed state. White, who is waiting, holds only their own claim.
    assert_eq!(t.black.head().state.ply, 1);
    assert!(!t.black.head().is_certified());
    assert_eq!(t.black.certified().state.ply, 0);

    t.play(&["e7e5"]);
    // Black's move carried the countersignature, so ply 1 is now certified
    // for White — the player who is waiting.
    assert_eq!(t.white.certified().state.ply, 1);
    assert!(t.white.certified().is_certified());
    assert_eq!(t.white.head().state.ply, 2);
}

#[test]
fn colours_cannot_be_confused() {
    let t = table(Position::startpos());
    assert_eq!(t.white.me, Color::White);
    assert_eq!(t.black.me, Color::Black);
    assert_eq!(t.white.channel_id, t.black.channel_id);
}
