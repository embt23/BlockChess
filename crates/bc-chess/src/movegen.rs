//! Move generation.
//!
//! Strategy: generate pseudo-legal moves cheaply, then filter by making each
//! one and asking whether our own king is attacked. That is slower than
//! computing pins and check-evasions directly, but it is *obviously* correct,
//! and it handles the two positions that break naive generators for free:
//!
//!  - en passant discovering a rank check on your own king,
//!  - a pinned piece that may still move along the pin ray.
//!
//! Correctness before cleverness. `perft` is an exact test, so the fast version
//! can be written later against a known-good baseline.

use crate::attacks::*;
use crate::bitboard::*;
use crate::position::Position;
use crate::types::*;

/// Squares a piece on `sq` attacks, given the occupancy. Leapers ignore the
/// second argument; sliders need it.
type AttackFn = fn(Square, Bitboard) -> Bitboard;

/// One castling option. `empty` and `safe` differ: for queenside, b1/b8 must be
/// vacant but the king never crosses it, so it is not a square the king may not
/// be attacked on.
struct CastleOption {
    right: u8,
    king_from: Square,
    king_to: Square,
    empty: &'static [Square],
    safe: &'static [Square],
}

impl Position {
    /// All pseudo-legal moves: legal except that they may leave our king in
    /// check. Castling is generated fully legally here, because "castling out
    /// of, through, or into check" is a rule about squares rather than about
    /// the resulting position, and it is cheaper to check up front.
    pub fn generate_pseudo(&self, out: &mut MoveList) {
        let us = self.side;
        let them = us.flip();
        let occ = self.occupied();
        let ours = self.color_bb[us.idx()];
        let theirs = self.color_bb[them.idx()];

        self.gen_pawns(out, us, occ, theirs);

        for (kind, attack_fn) in [
            (Piece::Knight, (|s, _occ| knight_attacks(s)) as AttackFn),
            (Piece::Bishop, |s, occ| bishop_attacks(s, occ)),
            (Piece::Rook, |s, occ| rook_attacks(s, occ)),
            (Piece::Queen, |s, occ| queen_attacks(s, occ)),
            (Piece::King, |s, _occ| king_attacks(s)),
        ] {
            for from in Squares(self.pieces(us, kind)) {
                // Never move onto our own pieces.
                for to in Squares(attack_fn(from, occ) & !ours) {
                    out.push(Move::normal(from, to));
                }
            }
        }

        self.gen_castles(out, us);
    }

    fn gen_pawns(&self, out: &mut MoveList, us: Color, occ: Bitboard, theirs: Bitboard) {
        let pawns = self.pieces(us, Piece::Pawn);
        let empty = !occ;

        // Bitboard-parallel: shift the whole pawn set at once, then walk the
        // resulting target squares. `from` is recovered by shifting back.
        let (push, dbl_rank, promo_rank, cap_l, cap_r) = match us {
            Color::White => (8i8, RANK_2, RANK_8, 7i8, 9i8),
            Color::Black => (-8i8, RANK_7, RANK_1, -9i8, -7i8),
        };

        let shift = |b: Bitboard, d: i8| -> Bitboard {
            if d >= 0 {
                b << d
            } else {
                b >> (-d)
            }
        };

        // Single pushes.
        let single = shift(pawns, push) & empty;
        for to in Squares(single) {
            let from = (to as i8 - push) as u8;
            Self::push_pawn_move(out, from, to, promo_rank);
        }

        // Double pushes: only from the home rank, and only if both squares are
        // empty — hence chaining off `single` rather than testing separately.
        let dbl = shift(single & shift(dbl_rank, push), push) & empty;
        for to in Squares(dbl) {
            out.push(Move::normal((to as i8 - 2 * push) as u8, to));
        }

        // Captures. Mask the source file so the shift cannot wrap around the
        // board edge — a1 is adjacent to h1 in a u64 but not on a chessboard.
        for (d, guard) in [(cap_l, FILE_A), (cap_r, FILE_H)] {
            let targets = shift(pawns & !guard, d) & theirs;
            for to in Squares(targets) {
                Self::push_pawn_move(out, (to as i8 - d) as u8, to, promo_rank);
            }
        }

        // En passant. Ask which of our pawns attack the target square.
        if let Some(ep) = self.ep {
            for from in Squares(pawn_attacks(us.flip(), ep) & pawns) {
                out.push(Move::new(from, ep, FLAG_EP, 0));
            }
        }
    }

    #[inline]
    fn push_pawn_move(out: &mut MoveList, from: Square, to: Square, promo_rank: Bitboard) {
        if bit(to) & promo_rank != 0 {
            for promo in 0..4u16 {
                out.push(Move::new(from, to, FLAG_PROMO, promo));
            }
        } else {
            out.push(Move::normal(from, to));
        }
    }

    fn gen_castles(&self, out: &mut MoveList, us: Color) {
        let occ = self.occupied();
        let them = us.flip();

        let opts: [CastleOption; 2] = match us {
            Color::White => [
                CastleOption {
                    right: CASTLE_WK,
                    king_from: 4,
                    king_to: 6,
                    empty: &[5, 6],
                    safe: &[4, 5, 6],
                },
                CastleOption {
                    right: CASTLE_WQ,
                    king_from: 4,
                    king_to: 2,
                    empty: &[1, 2, 3],
                    safe: &[4, 3, 2],
                },
            ],
            Color::Black => [
                CastleOption {
                    right: CASTLE_BK,
                    king_from: 60,
                    king_to: 62,
                    empty: &[61, 62],
                    safe: &[60, 61, 62],
                },
                CastleOption {
                    right: CASTLE_BQ,
                    king_from: 60,
                    king_to: 58,
                    empty: &[57, 58, 59],
                    safe: &[60, 59, 58],
                },
            ],
        };

        for o in opts {
            if self.castling & o.right == 0 {
                continue;
            }
            if o.empty.iter().any(|&s| occ & bit(s) != 0) {
                continue;
            }
            if o.safe.iter().any(|&s| self.attacked(s, them)) {
                continue;
            }
            out.push(Move::new(o.king_from, o.king_to, FLAG_CASTLE, 0));
        }
    }

    /// Fully legal moves.
    pub fn generate_legal(&self) -> MoveList {
        let mut pseudo = MoveList::new();
        self.generate_pseudo(&mut pseudo);

        let mut legal = MoveList::new();
        for &m in &pseudo {
            if self.is_legal(m) {
                legal.push(m);
            }
        }
        legal
    }

    /// Does this pseudo-legal move leave our own king attacked?
    #[inline]
    pub fn is_legal(&self, m: Move) -> bool {
        let after = self.make_move(m);
        !after.in_check(self.side)
    }
}
