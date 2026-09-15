//! D23, the half that has an oracle: the terminal predicates, against
//! `shakmaty`.
//!
//! `perft` counts nodes. It is a superb oracle for move generation and it
//! says **nothing at all** about how games end — a bug in
//! `insufficient_material` cannot change a perft count, because perft never
//! asks. So despite `terminal.rs` having tests, insufficient material and the
//! fifty-move clock were the thinnest-covered logic in `bc-chess`, and they
//! are logic the chain settles money on.
//!
//! `shakmaty` is a chess library by a different author with a different
//! internal representation. Agreement between it and `bc-chess` over hundreds
//! of thousands of positions is evidence; agreement between `bc-chess` and a
//! reference this project wrote would be a tautology (`G1`).
//!
//! The bridge between them is FEN, which means every one of these assertions
//! also differentially tests `Position::to_fen`. That is a bonus and not an
//! accident: if the writer were wrong, `shakmaty` would be answering about a
//! different position and the predicates would disagree immediately.

use bc_chess::{Color, Outcome, Position};
use bc_conformance::Playout;
use shakmaty::fen::Fen;
use shakmaty::{CastlingMode, Chess, Position as _};

/// How many random games each predicate is checked over. Kept low enough to
/// stay in the fast suite; the `--ignored` test below runs the same checks at
/// a scale that reaches real endgames.
const GAMES: u64 = 120;
const PLIES: usize = 300;

fn theirs(pos: &Position) -> Chess {
    let fen = pos.to_fen();
    let parsed: Fen = fen
        .parse()
        .unwrap_or_else(|e| panic!("shakmaty rejected our FEN {fen:?}: {e}"));
    parsed
        .into_position(CastlingMode::Standard)
        .unwrap_or_else(|e| panic!("shakmaty rejected our position {fen:?}: {e}"))
}

/// Agreement on the whole predicate set, position by position.
fn compare(ours: &Position, at: &str) {
    let them = theirs(ours);

    assert_eq!(
        ours.is_checkmate(),
        them.is_checkmate(),
        "checkmate disagrees {at}: {}",
        ours.to_fen()
    );
    assert_eq!(
        ours.is_stalemate(),
        them.is_stalemate(),
        "stalemate disagrees {at}: {}",
        ours.to_fen()
    );
    assert_eq!(
        ours.insufficient_material(),
        them.is_insufficient_material(),
        "insufficient material disagrees {at}: {}",
        ours.to_fen()
    );
    assert_eq!(
        ours.halfmove,
        them.halfmoves() as u8,
        "halfmove clock disagrees {at}: {}",
        ours.to_fen()
    );
    assert_eq!(
        ours.generate_legal().len(),
        them.legal_moves().len(),
        "legal move count disagrees {at}: {}",
        ours.to_fen()
    );
}

fn sweep(games: u64, plies: usize) -> usize {
    let mut seen = 0;
    for seed in 0..games {
        for (i, (pos, _)) in Playout::new(seed, plies).enumerate() {
            compare(&pos, &format!("(seed {seed} ply {i})"));
            seen += 1;
        }
    }
    seen
}

#[test]
fn the_terminal_predicates_agree_with_shakmaty_over_random_games() {
    let seen = sweep(GAMES, PLIES);
    assert!(seen > 10_000, "only {seen} positions compared");
}

/// The same comparison at a scale that reaches deep endgames, where
/// insufficient material and the fifty-move clock actually fire. Slow because
/// it is two full move generations per position.
#[test]
#[ignore = "slow: ~1M positions against a second engine"]
fn the_terminal_predicates_agree_with_shakmaty_at_scale() {
    let seen = sweep(3_000, 400);
    println!("compared {seen} positions against shakmaty");
    assert!(seen > 500_000, "only {seen} positions compared");
}

/// A sweep is only meaningful if it reaches the states under test. Random
/// play from the start position does reach all four outcomes — this asserts
/// it rather than hoping, so that a future change to the generator cannot
/// silently turn the differential test into an opening-only test.
#[test]
fn the_sweep_actually_reaches_every_outcome() {
    let (mut mate, mut stale, mut insuf, mut fifty) = (0, 0, 0, 0);
    for seed in 0..GAMES {
        for (pos, _) in Playout::new(seed, 400) {
            match pos.outcome() {
                Some(Outcome::Checkmate { .. }) => mate += 1,
                Some(Outcome::Stalemate) => stale += 1,
                Some(Outcome::InsufficientMaterial) => insuf += 1,
                Some(Outcome::FiftyMove) => fifty += 1,
                None => {}
            }
        }
    }
    println!("mate {mate} stalemate {stale} insufficient {insuf} fifty {fifty}");
    assert!(mate > 0, "no checkmate in {GAMES} random games");
    assert!(stale > 0, "no stalemate in {GAMES} random games");
    assert!(
        insuf > 0,
        "no insufficient material in {GAMES} random games"
    );
    assert!(fifty > 0, "no fifty-move draw in {GAMES} random games");
}

/// Random play from the start position almost never produces the awkward
/// material configurations, so those are enumerated directly.
///
/// The labels are **assertions, not comments**. Three hand-written mate
/// positions in this project's history were not mate — see
/// `docs/build-log.md` §15 — so a case list whose labels are prose is a case
/// list that can quietly test the wrong thing. Here the expected outcome is
/// checked against `bc-chess`, and then `shakmaty` is asked whether both of
/// us are right.
#[test]
fn the_material_edge_cases_agree_with_shakmaty() {
    use Outcome::*;
    const W: Outcome = Checkmate {
        winner: Color::White,
    };
    #[rustfmt::skip]
    const CASES: [(&str, Option<Outcome>); 16] = [
        ("8/8/8/4k3/8/8/4K3/8 w - - 0 1",         Some(InsufficientMaterial)), // K vs K
        ("8/8/8/4k3/8/8/4K3/6B1 w - - 0 1",       Some(InsufficientMaterial)), // K+B vs K
        ("8/8/8/4k3/8/8/4K3/6N1 w - - 0 1",       Some(InsufficientMaterial)), // K+N vs K
        ("8/8/8/4k3/8/8/4K3/5NN1 w - - 0 1",      None),                       // K+N+N: not insufficient
        ("6b1/8/8/4k3/8/8/4K3/6B1 w - - 0 1",     None),                       // opposite-colour bishops
        ("5b2/8/8/4k3/8/8/4K3/6B1 w - - 0 1",     Some(InsufficientMaterial)), // same-colour bishops
        ("8/8/8/4k3/8/8/4K3/5BB1 w - - 0 1",      None),                       // two bishops, one side
        ("8/8/8/4k3/8/8/4K3/6R1 w - - 0 1",       None),                       // rook
        ("8/8/8/4k3/8/8/4K3/6Q1 w - - 0 1",       None),                       // queen
        ("8/4P3/8/4k3/8/8/4K3/8 w - - 0 1",       None),                       // a pawn is never insufficient
        ("8/8/8/4k3/8/8/4K3/4B1N1 w - - 0 1",     None),                       // bishop + knight
        ("R6k/8/6K1/8/8/8/8/8 b - - 0 1",         Some(W)),                    // back-rank rook mate
        ("7k/6Q1/6K1/8/8/8/8/8 b - - 0 1",        Some(W)),                    // queen mate, king-defended
        ("7k/5Q2/8/8/8/8/8/6K1 b - - 0 1",        Some(Stalemate)),            // the classic queen stalemate
        ("R5k1/8/8/8/8/8/8/6K1 b - - 0 1",        None),                       // same check, white king too far: not mate
        ("8/8/8/4k3/8/8/4K3/6R1 w - - 100 60",    Some(FiftyMove)),            // clock out, material on
    ];
    for (fen, want) in CASES {
        let pos = Position::from_fen(fen).unwrap_or_else(|e| panic!("bad test FEN {fen}: {e:?}"));
        assert_eq!(pos.outcome(), want, "label is wrong for {fen}");
        compare(&pos, fen);
    }
}

/// The fifty-move clock is a counter, and a counter is only right if it
/// resets on exactly the right events. `shakmaty` is asked at every ply of a
/// random game, so captures, pawn moves, castling, promotion and en passant
/// are all covered without being enumerated.
#[test]
fn the_halfmove_clock_agrees_with_shakmaty_through_captures_and_pawn_moves() {
    let mut resets = 0;
    for seed in 200..260 {
        let mut prev: Option<u8> = None;
        for (pos, _) in Playout::new(seed, 200) {
            let them = theirs(&pos);
            assert_eq!(pos.halfmove, them.halfmoves() as u8, "{}", pos.to_fen());
            if prev.is_some_and(|p| pos.halfmove == 0 && p != 0) {
                resets += 1;
            }
            prev = Some(pos.halfmove);
        }
    }
    assert!(resets > 100, "only {resets} clock resets seen");
}

/// Mate precedes every draw condition, including a fifty-move clock that ran
/// out on the mating move itself. FIDE says the game ended when mate was
/// delivered; a chain that got this backwards would pay out a draw on a won
/// game.
#[test]
fn mate_beats_the_fifty_move_clock_and_shakmaty_agrees() {
    let pos = Position::from_fen("R6k/8/6K1/8/8/8/8/8 b - - 100 60").unwrap();
    assert_eq!(pos.halfmove, 100);
    assert!(pos.fifty_move_claimable());
    assert_eq!(
        pos.outcome(),
        Some(Outcome::Checkmate {
            winner: Color::White
        }),
        "mate must win the precedence race against the clock"
    );
    assert!(theirs(&pos).is_checkmate());
}
