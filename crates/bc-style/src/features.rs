//! Turning a (position, move) pair into a sparse feature vector.
//!
//! Everything is expressed from the **mover's** point of view: if Black is to
//! move the board is mirrored vertically, so "advancing a pawn" is one feature
//! rather than two. Style should not depend on which colour you were assigned.
//!
//! Features are sparse indicator functions — at most a handful fire per move —
//! so a move is represented as a short list of indices and scoring is a few
//! array lookups and an add.

use bc_chess::types::{Color, Piece, FLAG_CASTLE, FLAG_EP, FLAG_PROMO};
use bc_chess::{Move, Position};

// Feature block layout.
pub const F_PIECE: usize = 0; //   6  which piece type moved
pub const F_TO: usize = 6; // 384  (piece, destination square)
pub const F_FROM: usize = 390; // 384  (piece, origin square)
pub const F_VICTIM: usize = 774; //   7  nothing captured, or the captured type
pub const F_CHECK: usize = 781; //   1  gives check
pub const F_PROMO: usize = 782; //   4  promotion piece
pub const F_CASTLE: usize = 786; //   2  castles, by side
pub const F_PHASE: usize = 788; //  18  (phase bucket, piece type)
pub const N_FEATURES: usize = 806;

/// The most indicator functions any single move can activate.
pub const MAX_ACTIVE: usize = 8;

/// A sparse feature vector: indices only, all weights implicitly 1.
#[derive(Clone, Copy, Debug)]
pub struct Feats {
    idx: [u16; MAX_ACTIVE],
    len: u8,
}

impl Feats {
    #[inline]
    fn new() -> Feats {
        Feats {
            idx: [0; MAX_ACTIVE],
            len: 0,
        }
    }
    #[inline]
    fn push(&mut self, i: usize) {
        debug_assert!(i < N_FEATURES);
        debug_assert!((self.len as usize) < MAX_ACTIVE);
        self.idx[self.len as usize] = i as u16;
        self.len += 1;
    }
    #[inline]
    pub fn as_slice(&self) -> &[u16] {
        &self.idx[..self.len as usize]
    }
    /// Dot product with a weight vector.
    #[inline]
    pub fn score(&self, w: &[f32]) -> f32 {
        self.as_slice().iter().map(|&i| w[i as usize]).sum()
    }
}

/// Vertical mirror. a1 <-> a8. Applied when Black is to move so that the mover
/// always advances "up" the board.
#[inline]
fn orient(sq: u8, side: Color) -> usize {
    (if side == Color::White { sq } else { sq ^ 56 }) as usize
}

/// 0 = opening, 1 = middlegame, 2 = endgame, by remaining material.
fn phase(pos: &Position) -> usize {
    let n = pos.occupied().count_ones();
    if n >= 28 {
        0
    } else if n >= 14 {
        1
    } else {
        2
    }
}

/// Extract the features of one legal move.
pub fn extract(pos: &Position, m: Move) -> Feats {
    let mut f = Feats::new();
    let side = pos.side;

    let moved = match pos.piece_at(m.from()) {
        Some((_, p)) => p,
        // Callers always pass legal moves; be total rather than panic.
        None => return f,
    };
    let pi = moved.idx();

    f.push(F_PIECE + pi);
    f.push(F_TO + pi * 64 + orient(m.to(), side));
    f.push(F_FROM + pi * 64 + orient(m.from(), side));

    let victim = if m.flag() == FLAG_EP {
        Some(Piece::Pawn)
    } else {
        pos.piece_at(m.to()).map(|(_, p)| p)
    };
    f.push(F_VICTIM + victim.map_or(0, |p| 1 + p.idx()));

    // Whether the move gives check. This is the single most style-bearing
    // cheap feature: how often a player forces matters more than where.
    let after = pos.make_move(m);
    if after.in_check(side.flip()) {
        f.push(F_CHECK);
    }

    if m.flag() == FLAG_PROMO {
        f.push(F_PROMO + (m.promo().idx() - 1).min(3));
    }
    if m.flag() == FLAG_CASTLE {
        f.push(F_CASTLE + usize::from(m.to() % 8 < 4));
    }
    f.push(F_PHASE + phase(pos) * 6 + pi);

    f
}

/// Features for every legal move in a position, in generation order.
pub fn extract_all(pos: &Position, moves: &[Move]) -> Vec<Feats> {
    moves.iter().map(|&m| extract(pos, m)).collect()
}
