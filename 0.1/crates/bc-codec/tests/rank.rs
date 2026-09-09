//! E8's ordering, checked against the specification in `rank.rs`.
//!
//! These tests are written from the prose, not from the code. That is the
//! point of `papers/04-permanence.md` §2's criterion: if the specification is
//! good enough for an independent reader to reimplement from, it is good
//! enough to write tests from, and a disagreement means the prose is wrong
//! rather than the test.

use bc_chess::Position;
use bc_codec::rank::{rank_of_move, ranked_moves};

fn pos(fen: &str) -> Position {
    Position::from_fen(fen).unwrap_or_else(|e| panic!("bad fen: {e}"))
}

fn san_order(fen: &str, n: usize) -> Vec<String> {
    let p = pos(fen);
    ranked_moves(&p)
        .iter()
        .take(n)
        .map(|&m| bc_pgn::to_san(&p, m))
        .collect()
}

#[test]
fn the_ordering_is_a_permutation_of_the_legal_moves() {
    // Whatever it does, it must not lose or invent a move — otherwise a rank
    // does not identify a move and the encoding is broken.
    let mut p = Position::startpos();
    let mut x: u64 = 0x1234_5678_9ABC_DEF0;
    for _ in 0..300 {
        let canon = bc_codec::order::canonical_moves(&p);
        let mut ranked = ranked_moves(&p);
        if canon.is_empty() {
            break;
        }
        assert_eq!(
            ranked.len(),
            canon.len(),
            "different count at {}",
            p.to_fen()
        );
        ranked.sort_unstable_by_key(|m| m.0);
        let mut c = canon.clone();
        c.sort_unstable_by_key(|m| m.0);
        assert_eq!(ranked, c, "not a permutation at {}", p.to_fen());

        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        p = p.make_move(canon[(x.wrapping_mul(2685821657736338717) as usize) % canon.len()]);
    }
}

#[test]
fn captures_and_promotions_come_before_quiet_moves() {
    // Rule 1. White can take on d5 and also has many quiet moves.
    let fen = "rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2";
    let p = pos(fen);
    let order = ranked_moves(&p);
    let first = bc_pgn::to_san(&p, order[0]);
    assert_eq!(first, "exd5", "the only capture must sort first");
}

#[test]
fn most_valuable_victim_least_valuable_attacker() {
    // Rule 2. Both a pawn and a queen can take the rook on d5, and a pawn can
    // also take a pawn. Order must be: rook-by-pawn, rook-by-queen, then the
    // pawn capture — 10*5-1 = 49, 10*5-9 = 41, 10*1-1 = 9.
    let fen = "4k3/8/8/3r1p2/4P3/8/8/3QK3 w - - 0 1";
    let got = san_order(fen, 3);
    assert_eq!(got[0], "exd5", "pawn takes rook is the best gain");
    assert_eq!(got[1], "Qxd5", "queen takes rook next");
    assert_eq!(got[2], "exf5", "pawn takes pawn last of the three");
}

#[test]
fn promotions_sort_queen_first_among_themselves() {
    // Rule 2's promotion term: value(promoted) - 1, so Q(8) > R(4) > B/N(2/2),
    // and the N/B tie falls to rule 5's promotion order, N before B.
    let got = san_order("8/P6k/8/8/8/8/7K/8 w - - 0 1", 4);
    assert_eq!(got, ["a8=Q", "a8=R", "a8=N", "a8=B"]);
}

#[test]
fn central_destinations_beat_edge_ones() {
    // Rule 3, among quiet moves of the same piece. A knight on b1 can go to a3
    // (edge, centrality 0) or c3 (centrality 4). c3 must come first.
    let p = pos("4k3/8/8/8/8/8/8/1N2K3 w - - 0 1");
    let order: Vec<String> = ranked_moves(&p)
        .iter()
        .map(|&m| bc_pgn::to_san(&p, m))
        .collect();
    let c3 = order.iter().position(|s| s == "Nc3").expect("Nc3 legal");
    let a3 = order.iter().position(|s| s == "Na3").expect("Na3 legal");
    assert!(c3 < a3, "central destination should rank first: {order:?}");
}

#[test]
fn small_pieces_before_big_ones_when_all_else_ties() {
    // Rule 4. From the start every move is quiet; a2a3 and b1a3 both land on
    // a3 with centrality 0, so the pawn must come first.
    let p = Position::startpos();
    let order: Vec<String> = ranked_moves(&p)
        .iter()
        .map(|&m| bc_pgn::to_san(&p, m))
        .collect();
    let pawn = order.iter().position(|s| s == "a3").unwrap();
    let knight = order.iter().position(|s| s == "Na3").unwrap();
    assert!(
        pawn < knight,
        "pawn before knight to the same square: {order:?}"
    );
}

#[test]
fn the_ordering_is_deterministic() {
    // Two runs must agree, or the format is not a format.
    let fen = "r1bqkbnr/pppp1ppp/2n5/4p3/2B1P3/5Q2/PPPP1PPP/RNB1K1NR w KQkq - 4 4";
    assert_eq!(san_order(fen, 12), san_order(fen, 12));
}

#[test]
fn played_moves_rank_better_than_chance() {
    // The claim E8 rests on: a heuristic ordering puts the move people
    // actually played near the front. Measured over the games in the repo.
    //
    // This is a weak corpus — four games — so the bar is deliberately low.
    // The real figure comes from `blockchess measure` on a real corpus, and
    // this test only guards against the ordering being useless or inverted.
    let text = include_str!("../../../corpus/classics.pgn");
    let (games, _) = bc_pgn::parse_all(text);

    let mut sum_rank = 0.0;
    let mut sum_mid = 0.0;
    let mut n = 0.0;
    for g in &games {
        let mut p = g.start;
        for &m in &g.moves {
            let b = bc_codec::order::canonical_moves(&p).len();
            let r = rank_of_move(&p, m).expect("played move must be legal");
            sum_rank += r as f64;
            sum_mid += (b as f64 - 1.0) / 2.0; // mean rank under no ordering
            n += 1.0;
            p = p.make_move(m);
        }
    }
    let mean_rank = sum_rank / n;
    let chance = sum_mid / n;
    assert!(
        mean_rank < chance,
        "heuristic mean rank {mean_rank:.2} should beat chance {chance:.2}"
    );
}
