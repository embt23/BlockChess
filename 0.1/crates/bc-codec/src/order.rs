//! The canonical order of the legal moves in a position.
//!
//! `papers/04-permanence.md` §5: under a legal-move-index encoding, the move
//! generator's *enumeration order* is part of the file format. Get it wrong
//! later and every game already stored decodes to different moves — silently,
//! with no error, into a different but perfectly legal game.
//!
//! So the format does not inherit whatever order the generator happens to
//! emit. It defines its own:
//!
//! > **Ascending by origin square; ties broken by destination square; ties
//! > broken by promotion piece in the order N, B, R, Q.**
//!
//! Squares are numbered a1 = 0 to h8 = 63, and a castling move is ordered by
//! its king's origin and destination like any other. No two legal moves in a
//! position share all three keys, so the order is total.
//!
//! ## Why not simply sort by the packed 16-bit move
//!
//! Because that sorts by the bit layout, which puts the flag first and the
//! *destination* above the origin — a correct order, but one whose written
//! specification would be "sort by our struct's field order", and which would
//! change format if the packing ever changed for an unrelated reason. The
//! three keys above are stated in terms of chess, so any implementation can
//! reproduce them from the sentence, which is the criterion
//! `papers/04-permanence.md` §2 sets for anything permanent.
//!
//! It is worth being explicit that this is a real decision and not an
//! accident: the alternative, ordering by some heuristic strength, would let
//! an encoder spend fewer bits on likely moves. That is scheme E8 in
//! `papers/02-encodings.md`, it is a different format, and it would need its
//! own `RuleSetId`.

use bc_chess::types::FLAG_PROMO;
use bc_chess::{Move, Position};

/// The three sort keys, in order of significance.
fn key(m: Move) -> (u8, u8, u8) {
    let promo = if m.flag() == FLAG_PROMO {
        // Knight, Bishop, Rook, Queen — the order the promotion field already
        // uses, stated here so it does not depend on that field.
        match m.promo() {
            bc_chess::Piece::Knight => 0,
            bc_chess::Piece::Bishop => 1,
            bc_chess::Piece::Rook => 2,
            _ => 3,
        }
    } else {
        0
    };
    (m.from(), m.to(), promo)
}

/// The legal moves, in the order the format defines.
pub fn canonical_moves(pos: &Position) -> Vec<Move> {
    let mut moves: Vec<Move> = pos.generate_legal().as_slice().to_vec();
    moves.sort_unstable_by_key(|&m| key(m));
    moves
}

/// The index of `m` among the legal moves, or `None` if it is not legal here.
pub fn index_of(pos: &Position, m: Move) -> Option<usize> {
    canonical_moves(pos).iter().position(|&x| x == m)
}
