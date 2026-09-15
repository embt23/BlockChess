//! The ply cap, on every path that adds a ply.
//!
//! `max_plies` is what bounds the worst case a validator has to be able to
//! afford, so it is a consensus parameter and not a convenience. It was
//! checked in `apply_move` and nowhere else; `refute` also adds a ply, and
//! the `stateright` model walked past the cap by alternating false mate
//! claims with refutations (`docs/build-log.md` §16).
//!
//! The authoritative check now lives in `verdict`, which is total over every
//! path into a state rather than over the paths somebody remembered.

use bc_adjudicator::dispute::{ClaimKind, Dispute, MoveOutcome, Refutation};
use bc_adjudicator::state::{GameState, Status};
use bc_adjudicator::terms::GameTerms;
use bc_chess::{Color, Position};

const H: u64 = 1_000;

fn dispute(fen: &str, max_plies: u16) -> Dispute {
    let pos = Position::from_fen(fen).expect("test FEN");
    let mut terms = GameTerms::for_clock([0u8; 32], 60_000, 0);
    terms.max_plies = max_plies;
    let state = GameState {
        channel_id: [0u8; 32],
        ply: 0,
        prev_hash: [0u8; 32],
        mv: 0,
        pos_hash: [0u8; 32],
        clock_w_ms: 60_000,
        clock_b_ms: 60_000,
        status: Status::Ongoing,
    };
    Dispute::open(&state, pos, &terms, Color::White, H).expect("open")
}

fn first_legal(d: &Dispute) -> bc_chess::Move {
    d.pos.generate_legal().as_slice()[0]
}

/// The direct path: moves alone reach the cap and the game is a draw.
#[test]
fn a_move_that_reaches_the_cap_ends_the_game() {
    let mut d = dispute("7k/5Q2/6K1/8/8/8/8/8 w - - 0 1", 1);
    let mv = first_legal(&d);
    assert_eq!(
        d.apply_move(Color::White, mv, H + 1, None),
        Ok(MoveOutcome::Ended(Status::Draw))
    );
}

/// The path the model found. A false mate claim followed by a refutation
/// advances the ply, and used to do so without ever consulting the cap.
#[test]
fn a_refutation_cannot_walk_past_the_ply_cap() {
    let mut d = dispute("7k/5Q2/6K1/8/8/8/8/8 w - - 0 1", 2);

    // White moves and falsely claims mate. `P3`: the chain does not look.
    let mv = first_legal(&d);
    assert_eq!(
        d.apply_move(Color::White, mv, H + 1, Some(ClaimKind::Checkmate)),
        Ok(MoveOutcome::Claimed)
    );
    assert_eq!(d.ply, 1);

    // Black refutes with one legal move, which is played. That is ply 2, and
    // ply 2 is the cap.
    let escape = first_legal(&d);
    assert_eq!(
        d.refute(Color::Black, escape, H + 2),
        Ok(Refutation::CapReached)
    );
    assert_eq!(d.ply, 2);

    // And the verdict agrees, without waiting for any deadline.
    assert_eq!(d.verdict(H + 2), Ok(Status::Draw));
}

/// `verdict` is the authoritative check, so it reports the cap however the
/// dispute got there — including states reached by a caller this test does
/// not know about.
#[test]
fn the_verdict_reports_the_cap_before_anything_else() {
    let mut d = dispute("7k/5Q2/6K1/8/8/8/8/8 w - - 0 1", 2);
    let mv = first_legal(&d);
    d.apply_move(Color::White, mv, H + 1, Some(ClaimKind::Checkmate))
        .unwrap();
    let escape = first_legal(&d);
    d.refute(Color::Black, escape, H + 2).unwrap();

    // A pending claim would normally make this `NotYetDecided`, and a passed
    // deadline would make it a loss on time. The cap outranks both, because
    // past it there is no game left to have an opinion about.
    assert_eq!(d.verdict(H + 2), Ok(Status::Draw));
    assert_eq!(d.verdict(H + 10_000), Ok(Status::Draw));
}

/// Below the cap nothing changes: a refutation resumes the game as before.
#[test]
fn a_refutation_below_the_cap_still_resumes_the_game() {
    let mut d = dispute("7k/5Q2/6K1/8/8/8/8/8 w - - 0 1", 200);
    let mv = first_legal(&d);
    d.apply_move(Color::White, mv, H + 1, Some(ClaimKind::Checkmate))
        .unwrap();
    let escape = first_legal(&d);
    assert_eq!(
        d.refute(Color::Black, escape, H + 2),
        Ok(Refutation::ResumedAtMove)
    );
    assert!(d.verdict(H + 2).is_err(), "the game is not over");
}
