//! Golden vectors pinning the canonical move order.
//!
//! `papers/04-permanence.md` §5: under a legal-move-index encoding the order
//! of the move list *is* the file format. If it ever changes, every game
//! already stored decodes to different moves — silently, with no error, into
//! a different but perfectly legal game.
//!
//! So it gets a test that fails loudly if anybody reorders anything. If this
//! test fails and the change was deliberate, the change needs a new
//! `RuleSetId`, not a new expectation here.

use bc_chess::Position;
use bc_codec::order::canonical_moves;

fn uci_list(fen: &str) -> Vec<String> {
    let pos = Position::from_fen(fen).expect("fen");
    canonical_moves(&pos).iter().map(|m| m.to_uci()).collect()
}

#[test]
fn starting_position_order_is_pinned() {
    let got = uci_list(Position::START_FEN);
    // Ascending by origin square (a1 = 0 … h8 = 63), then destination.
    let want = [
        "b1a3", "b1c3", "g1f3", "g1h3", "a2a3", "a2a4", "b2b3", "b2b4", "c2c3", "c2c4", "d2d3",
        "d2d4", "e2e3", "e2e4", "f2f3", "f2f4", "g2g3", "g2g4", "h2h3", "h2h4",
    ];
    assert_eq!(
        got, want,
        "the canonical order changed — this is a format break"
    );
}

#[test]
fn promotions_are_ordered_knight_bishop_rook_queen() {
    // Promotion is the third sort key, under origin and destination. Any
    // generator that emitted queen-first would silently renumber every
    // promotion in the corpus.
    let got = uci_list("8/P6k/8/8/8/8/7K/8 w - - 0 1");
    let promos: Vec<&String> = got.iter().filter(|s| s.starts_with("a7a8")).collect();
    assert_eq!(
        promos,
        ["a7a8n", "a7a8b", "a7a8r", "a7a8q"],
        "promotion ordering changed — this is a format break"
    );
}

#[test]
fn castling_sorts_by_king_destination() {
    // Castling is encoded king-from/king-to, so it sorts among the king's
    // ordinary moves rather than after them: c1 before d1, g1 after f1.
    // Worth pinning — it is the one move whose encoding does not look like
    // what it does.
    let got = uci_list("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1");
    let king: Vec<&String> = got.iter().filter(|s| s.starts_with("e1")).collect();
    assert_eq!(
        king,
        ["e1c1", "e1d1", "e1f1", "e1g1", "e1d2", "e1e2", "e1f2"]
    );
}

#[test]
fn every_position_lists_each_move_once() {
    // A duplicate in the list would make two indices decode to the same move,
    // which makes the encoding non-injective and loses data silently.
    for fen in [
        Position::START_FEN,
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
    ] {
        let moves = canonical_moves(&Position::from_fen(fen).unwrap());
        let mut sorted: Vec<u16> = moves.iter().map(|m| m.0).collect();
        let before = sorted.len();
        sorted.dedup();
        assert_eq!(before, sorted.len(), "duplicate move in {fen}");
    }
}
