//! E8 — order the legal moves by a fixed heuristic, then encode the *rank*.
//!
//! E7 spends `log2(b)` bits because it treats every legal move as equally
//! likely. They are not. If the moves are sorted so that the ones people
//! actually play tend to come first, then the rank of the played move is a
//! small number far more often than chance, and small numbers are cheap.
//!
//! `papers/02-encodings.md` puts this at ~4 bits/ply against E7's ~4.6, from
//! the literature. This is our own version, so the figure becomes ours.
//!
//! # The specification
//!
//! `papers/04-permanence.md` §2 sets the bar for anything permanent: **an
//! independent reader must be able to reconstruct the decoder from the written
//! specification alone.** So the rule is written here, first, in prose, and the
//! code below implements exactly this and nothing else.
//!
//! Sort the legal moves ascending by the following key, comparing each field
//! only when all earlier fields are equal. Every value is an integer.
//!
//! 1. **Class.** `0` for a capture or a promotion, `1` for everything else.
//!    Captures and promotions change material, and a player considers them
//!    first.
//! 2. **Gain**, descending — only meaningful within class 0. Defined as
//!    `10 × value(victim) − value(attacker)`, where the piece values are
//!    `pawn 1, knight 3, bishop 3, rook 5, queen 9, king 0`, an en passant
//!    victim is a pawn, and a promotion adds `value(promoted piece) − 1`.
//!    Taking a queen with a pawn sorts above taking a pawn with a queen; this
//!    is the standard most-valuable-victim, least-valuable-attacker rule.
//! 3. **Centrality of the destination**, descending: `6` for the four centre
//!    squares d4 e4 d5 e5, `4` for the twelve squares of the ring around them,
//!    `2` for the next ring, `0` for the outer edge.
//! 4. **Piece order**, ascending: pawn, knight, bishop, rook, queen, king.
//!    A player looks at what the small pieces are doing first.
//! 5. **Origin square**, then **destination square**, then **promotion piece**
//!    in the order N, B, R, Q — ascending, and identical to the tie-break
//!    `order.rs` already defines, so that a fully-tied group falls back to the
//!    E7 order rather than to anything new.
//!
//! Nothing here consults a corpus, a search, or an evaluation function. It is
//! about a hundred integer comparisons and it fits on this page, which is the
//! whole point: it is the strongest ordering that still passes the
//! reconstruct-from-prose test. A neural policy would order the moves better
//! and would not pass it.
//!
//! # What this crate does and does not provide
//!
//! It provides the ordering, and therefore the rank of any played move. It
//! does **not** provide a code for those ranks — that needs a frequency table
//! measured over a real corpus, and inventing one from a guess would defeat
//! the purpose. `blockchess measure` reports the empirical entropy of the rank
//! distribution, which is the bits/ply an optimal static code over these ranks
//! would achieve, and that is the number E8 should be judged on.

use bc_chess::types::{file_of, rank_of, FLAG_EP, FLAG_PROMO};
use bc_chess::{Move, Piece, Position};

/// Piece values, in the order `Piece::idx()` uses.
const VALUE: [i32; 6] = [1, 3, 3, 5, 9, 0];

/// How central a square is: 6, 4, 2, 0 from the middle outward.
fn centrality(sq: u8) -> i32 {
    let r = rank_of(sq) as i32;
    let f = file_of(sq) as i32;
    // Distance from the centre of the board, measured to the nearer of the two
    // middle files/ranks. 0 for d4/e4/d5/e5, rising to 3 at the edge.
    let dr = (r - 3).abs().min((r - 4).abs());
    let df = (f - 3).abs().min((f - 4).abs());
    match dr.max(df) {
        0 => 6,
        1 => 4,
        2 => 2,
        _ => 0,
    }
}

/// The sort key. Fields that should sort descending are negated, so the whole
/// key sorts ascending — which keeps the comparison in one place and out of
/// the caller.
fn key(pos: &Position, m: Move) -> (i32, i32, i32, u8, u8, u8, u8) {
    let attacker = pos
        .piece_at(m.from())
        .map(|(_, p)| p)
        .unwrap_or(Piece::Pawn);
    let victim = if m.flag() == FLAG_EP {
        Some(Piece::Pawn)
    } else {
        pos.piece_at(m.to()).map(|(_, p)| p)
    };
    let promo = m.flag() == FLAG_PROMO;

    let class = i32::from(!(victim.is_some() || promo));
    let mut gain = 0;
    if let Some(v) = victim {
        gain += 10 * VALUE[v.idx()] - VALUE[attacker.idx()];
    }
    if promo {
        gain += VALUE[m.promo().idx()] - 1;
    }

    let promo_rank = if promo {
        match m.promo() {
            Piece::Knight => 0,
            Piece::Bishop => 1,
            Piece::Rook => 2,
            _ => 3,
        }
    } else {
        0
    };

    (
        class,
        -gain,
        -centrality(m.to()),
        attacker.idx() as u8,
        m.from(),
        m.to(),
        promo_rank,
    )
}

/// The legal moves, ordered by the specification above.
pub fn ranked_moves(pos: &Position) -> Vec<Move> {
    let mut moves = crate::order::canonical_moves(pos);
    moves.sort_by_key(|&m| key(pos, m));
    moves
}

/// Where the played move falls in that order. `None` if it is not legal.
pub fn rank_of_move(pos: &Position, m: Move) -> Option<usize> {
    ranked_moves(pos).iter().position(|&x| x == m)
}
