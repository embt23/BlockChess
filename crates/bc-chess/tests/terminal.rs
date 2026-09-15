//! Terminal conditions, against positions from published games and problems.
//!
//! The mates below are the ones every chess book opens with, which is the
//! point: they are positions someone else published a verdict on, so agreeing
//! with them is an oracle check and not a restatement of our own code (`G1`).

use bc_chess::terminal::Outcome;
use bc_chess::{Color, Move, Position};

fn pos(fen: &str) -> Position {
    Position::from_fen(fen).unwrap()
}

#[test]
fn checkmates() {
    // Fool's mate: 1.f3 e5 2.g4 Qh4#. The fastest mate in chess.
    let fools = pos("rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3");
    assert_eq!(
        fools.outcome(),
        Some(Outcome::Checkmate {
            winner: Color::Black
        })
    );

    // Scholar's mate: 1.e4 e5 2.Bc4 Nc6 3.Qh5 Nf6 4.Qxf7#.
    let scholars = pos("r1bqkb1r/pppp1Qpp/2n2n2/4p3/2B1P3/8/PPPP1PPP/RNB1K1NR b KQkq - 0 4");
    assert_eq!(
        scholars.outcome(),
        Some(Outcome::Checkmate {
            winner: Color::White
        })
    );

    // Back-rank mate: the rook gives check along the eighth and the king's
    // own pawns take every escape.
    let back_rank = pos("6k1/5ppp/8/8/8/8/8/R5K1 b - - 0 1");
    assert!(!back_rank.is_checkmate(), "the king can still run to f8/h8");
    let mated = pos("R5k1/5ppp/8/8/8/8/8/6K1 b - - 0 1");
    assert_eq!(
        mated.outcome(),
        Some(Outcome::Checkmate {
            winner: Color::White
        })
    );

    // Smothered mate: the knight mates a king boxed in by its own pieces.
    let smothered = pos("6rk/5Npp/8/8/8/8/8/6K1 b - - 0 1");
    assert_eq!(
        smothered.outcome(),
        Some(Outcome::Checkmate {
            winner: Color::White
        })
    );
}

#[test]
fn stalemates() {
    // The commonest practical stalemate: queen one square too close.
    let queen = pos("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1");
    assert!(!queen.in_check(Color::Black), "h8 is not attacked");
    assert_eq!(queen.outcome(), Some(Outcome::Stalemate));

    // Moving the queen back one file turns it into mate, which is the whole
    // lesson of the position.
    let mate = pos("7k/6Q1/6K1/8/8/8/8/8 b - - 0 1");
    assert_eq!(
        mate.outcome(),
        Some(Outcome::Checkmate {
            winner: Color::White
        })
    );

    // A king in the corner with a pawn it cannot push.
    let blocked = pos("k7/P7/K7/8/8/8/8/8 b - - 0 1");
    assert_eq!(blocked.outcome(), Some(Outcome::Stalemate));
}

#[test]
fn insufficient_material() {
    for fen in [
        "8/8/4k3/8/8/4K3/8/8 w - - 0 1",    // K v K
        "8/8/4k3/8/8/4KB2/8/8 w - - 0 1",   // K+B v K
        "8/8/4k3/8/8/4KN2/8/8 w - - 0 1",   // K+N v K
        "8/8/2b1k3/8/8/4K3/8/8 w - - 0 1",  // K v K+B
        "8/1b6/4k3/8/8/4KB2/8/8 w - - 0 1", // both bishops light
    ] {
        assert!(pos(fen).insufficient_material(), "{fen}");
        assert_eq!(pos(fen).outcome(), Some(Outcome::InsufficientMaterial));
    }

    for fen in [
        "8/2b5/4k3/8/8/4KB2/8/8 w - - 0 1", // bishops on opposite complexes
        "8/8/4k3/8/8/3NKN2/8/8 w - - 0 1",  // K+N+N v K — mate needs help,
        "8/8/4k3/8/8/2B1KB2/8/8 w - - 0 1", // but is reachable, so not this
        "8/8/4k3/8/8/4KR2/8/8 w - - 0 1",   // a rook mates
        "8/8/4k3/8/8/4K3/4P3/8 w - - 0 1",  // a pawn promotes
    ] {
        assert!(!pos(fen).insufficient_material(), "{fen}");
    }
}

#[test]
fn bishops_on_one_complex_is_about_the_squares_not_the_colours() {
    // f3 and b7 are both light squares, so neither bishop can ever attack the
    // other's territory and no mate exists.
    let same = pos("8/1b6/4k3/8/8/4KB2/8/8 w - - 0 1");
    assert!(same.insufficient_material());

    // Move one bishop one square and the position becomes winnable in
    // principle, so the draw claim disappears.
    let differ = pos("8/2b5/4k3/8/8/4KB2/8/8 w - - 0 1");
    assert!(!differ.insufficient_material());
}

#[test]
fn fifty_move_is_claimable_not_automatic() {
    let p = pos("8/8/4k3/8/8/4K3/4P3/4R3 w - - 99 80");
    assert!(!p.fifty_move_claimable());
    assert_eq!(p.outcome(), None);

    let at_100 = pos("8/8/4k3/8/8/4K3/4P3/4R3 w - - 100 80");
    assert!(at_100.fifty_move_claimable());
    assert_eq!(at_100.outcome(), Some(Outcome::FiftyMove));
}

#[test]
fn mate_beats_the_clock() {
    // Precedence matters: the mating move landed, so the game is over as a
    // win even though the halfmove clock had also run out.
    let p = pos("R5k1/5ppp/8/8/8/8/8/6K1 b - - 100 90");
    assert_eq!(
        p.outcome(),
        Some(Outcome::Checkmate {
            winner: Color::White
        })
    );
}

#[test]
fn refutation_is_the_cheap_direction() {
    // `P3`: the chain never proves mate, it only checks a refutation. In a
    // position wrongly claimed as mate, one legal move settles it.
    let not_mate = pos("6k1/5ppp/8/8/8/8/8/R5K1 b - - 0 1");
    let escape = Move::normal(62, 61); // Kg8-f8
    assert!(not_mate.refutes_terminal_claim(escape));

    // In a real mate nothing refutes, and that is exactly the ∀ the chain
    // refuses to compute — here it is the *client* paying for it.
    let mated = pos("R5k1/5ppp/8/8/8/8/8/6K1 b - - 0 1");
    assert!(mated.generate_legal().is_empty());
    for from in 0u8..64 {
        for to in 0u8..64 {
            assert!(!mated.refutes_terminal_claim(Move::normal(from, to)));
        }
    }

    // A move by the wrong side refutes nothing, however legal it looks.
    assert!(!not_mate.refutes_terminal_claim(Move::normal(0, 8)));
}

/// The position the `shakmaty` differential found — `docs/build-log.md` §15.
///
/// Two white bishops, both on light squares, against a bare king. The old
/// rule required exactly one bishop per side and so called this *sufficient*,
/// which on-chain means a dead draw keeps consuming a disputing player's
/// block budget until one of them flags.
#[test]
fn many_bishops_on_one_square_colour_are_still_insufficient() {
    let found = Position::from_fen("2B5/8/K7/5Bk1/8/8/8/8 b - - 0 194").unwrap();
    assert!(
        found.insufficient_material(),
        "two light bishops cannot mate"
    );

    for fen in [
        "8/8/8/4k3/8/8/4K3/1B1B4 w - - 0 1",     // two light bishops
        "8/8/8/4k3/8/8/4K3/2B1B1B1 w - - 0 1",   // three, all dark
        "2b1b3/8/8/4k3/8/8/4K3/1B1B4 w - - 0 1", // four, two a side, all light
    ] {
        let p = Position::from_fen(fen).unwrap();
        assert!(p.insufficient_material(), "{fen}");
    }

    // One bishop on each colour complex mates, so the count is never the
    // thing being tested — the square colour is.
    for fen in [
        "8/8/8/4k3/8/8/4K3/5BB1 w - - 0 1", // adjacent files: opposite colours
        "6b1/8/8/4k3/8/8/4K3/6B1 w - - 0 1", // one each, opposite colours
        "8/8/8/4k3/8/8/4K3/4B1N1 w - - 0 1", // bishop and knight
        "8/8/8/4k3/8/8/4K3/5NN1 w - - 0 1", // two knights: reachable, not forcible
    ] {
        let p = Position::from_fen(fen).unwrap();
        assert!(!p.insufficient_material(), "{fen}");
    }
}
