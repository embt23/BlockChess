//! SAN parsing. The strongest test available without a corpus: parse a real
//! historical game end to end. A misparse selects the wrong move, which makes
//! the next move illegal, so any error surfaces within a ply or two — a game
//! that parses to a *checkmate* has validated the parser and `bc-chess`
//! together.

use bc_chess::san::{parse_movetext, parse_san};
use bc_chess::{Piece, Position};

/// Morphy — Duke of Brunswick & Count Isouard, Paris 1858. The "Opera Game".
/// Exercises captures, checks, castling long, and mate.
const OPERA: &str = "1.e4 e5 2.Nf3 d6 3.d4 Bg4 4.dxe5 Bxf3 5.Qxf3 dxe5 6.Bc4 Nf6 \
7.Qb3 Qe7 8.Nc3 c6 9.Bg5 b5 10.Nxb5 cxb5 11.Bxb5+ Nbd7 12.O-O-O Rd8 \
13.Rxd7 Rxd7 14.Rd1 Qe6 15.Bxd7+ Nxd7 16.Qb8+ Nxb8 17.Rd8# 1-0";

#[test]
fn opera_game_parses_and_ends_in_mate() {
    let start = Position::startpos();
    let moves = parse_movetext(&start, OPERA).expect("every token must be a legal move");
    assert_eq!(moves.len(), 33, "17 White moves and 16 Black");

    let mut pos = start;
    for m in &moves {
        assert!(
            pos.generate_legal().as_slice().contains(m),
            "illegal move reached: {}",
            m.to_uci()
        );
        pos = pos.make_move(*m);
    }
    assert!(pos.in_check(pos.side), "final position must be check");
    assert!(
        pos.generate_legal().is_empty(),
        "final position must have no legal replies — checkmate"
    );
}

#[test]
fn scholars_mate() {
    let pos = Position::startpos();
    let moves = parse_movetext(&pos, "1. e4 e5 2. Bc4 Nc6 3. Qh5 Nf6 4. Qxf7#").unwrap();
    assert_eq!(moves.len(), 7);
    let mut p = pos;
    for m in moves {
        p = p.make_move(m);
    }
    assert!(p.in_check(p.side) && p.generate_legal().is_empty());
}

#[test]
fn comments_variations_and_nags_are_skipped() {
    let pos = Position::startpos();
    let a = parse_movetext(&pos, "1. e4 e5 2. Nf3").unwrap();
    let b = parse_movetext(
        &pos,
        "1. e4 {best by test} e5 $1 (1... c5 2. Nf3) 2. Nf3 ; trailing\n",
    )
    .unwrap();
    assert_eq!(a, b);
}

#[test]
fn file_and_rank_disambiguation() {
    // Two knights on d2 and f2 can both reach e4; SAN must say which.
    let pos = Position::from_fen("4k3/8/8/8/8/8/3N1N2/4K3 w - - 0 1").unwrap();
    let nd = parse_san(&pos, "Nde4").expect("Nde4");
    let nf = parse_san(&pos, "Nfe4").expect("Nfe4");
    assert_eq!(nd.from() % 8, 3);
    assert_eq!(nf.from() % 8, 5);
    // Ambiguous notation must be refused rather than guessed at.
    assert!(parse_san(&pos, "Ne4").is_none());

    // Same file, different ranks — rank disambiguation.
    let pos = Position::from_fen("4k3/8/8/8/3N4/8/3N4/4K3 w - - 0 1").unwrap();
    assert_eq!(parse_san(&pos, "N2b3").unwrap().from() / 8, 1);
    assert_eq!(parse_san(&pos, "N4b3").unwrap().from() / 8, 3);
}

#[test]
fn promotions_and_underpromotions() {
    let pos = Position::from_fen("4k3/P7/8/8/8/8/8/4K3 w - - 0 1").unwrap();
    for (tok, want) in [
        ("a8=Q", Piece::Queen),
        ("a8=R", Piece::Rook),
        ("a8=B", Piece::Bishop),
        ("a8=N", Piece::Knight),
    ] {
        let m = parse_san(&pos, tok).unwrap_or_else(|| panic!("{tok}"));
        assert_eq!(m.promo(), want, "{tok}");
    }
}

#[test]
fn castling_both_spellings_and_sides() {
    let pos = Position::from_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").unwrap();
    assert_eq!(parse_san(&pos, "O-O").unwrap().to(), 6);
    assert_eq!(parse_san(&pos, "O-O-O").unwrap().to(), 2);
    assert_eq!(
        parse_san(&pos, "0-0").unwrap().to(),
        6,
        "digit-zero spelling"
    );
}

#[test]
fn en_passant_and_rejection_of_nonsense() {
    let pos = Position::from_fen("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1").unwrap();
    assert!(parse_san(&pos, "exd6").is_some());
    assert!(parse_san(&pos, "Qz9").is_none());
    assert!(parse_san(&pos, "Nf3").is_none(), "no knight on the board");
    assert!(parse_san(&pos, "").is_none());
}
