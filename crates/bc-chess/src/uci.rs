//! Long-algebraic (UCI) move notation: `e2e4`, `e1g1`, `e7e8q`.
//!
//! Parsing is done by generating the legal moves and matching, rather than by
//! working out the flags from the squares. That is slower and completely
//! worth it: castling, en passant and promotion all encode their flag in the
//! `Move`, and deriving those flags here would be a second, quieter copy of
//! the rules. There is exactly one implementation of the rules (`P5`), and
//! this is not it.

use crate::position::Position;
use crate::types::*;

fn square_from(s: &[u8]) -> Option<Square> {
    let file = s.first()?.checked_sub(b'a')?;
    let rank = s.get(1)?.checked_sub(b'1')?;
    (file < 8 && rank < 8).then(|| sq(file, rank))
}

impl Position {
    /// Parse a UCI move in this position, or `None` if it is not legal here.
    pub fn move_from_uci(&self, text: &str) -> Option<Move> {
        let b = text.as_bytes();
        if b.len() < 4 || b.len() > 5 {
            return None;
        }
        let from = square_from(&b[0..2])?;
        let to = square_from(&b[2..4])?;
        let promo = match b.get(4) {
            None => None,
            Some(c) => Some(match c.to_ascii_lowercase() {
                b'n' => Piece::Knight,
                b'b' => Piece::Bishop,
                b'r' => Piece::Rook,
                b'q' => Piece::Queen,
                _ => return None,
            }),
        };

        self.generate_legal().as_slice().iter().copied().find(|m| {
            m.from() == from
                && m.to() == to
                && match promo {
                    Some(p) => m.flag() == FLAG_PROMO && m.promo() == p,
                    None => m.flag() != FLAG_PROMO,
                }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_to_uci() {
        let p = Position::startpos();
        for &m in &p.generate_legal() {
            assert_eq!(p.move_from_uci(&m.to_uci()), Some(m));
        }
    }

    #[test]
    fn castling_is_king_from_king_to() {
        let p = Position::from_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").unwrap();
        let m = p.move_from_uci("e1g1").unwrap();
        assert_eq!(m.flag(), FLAG_CASTLE);
        assert_eq!(p.move_from_uci("e1h1"), None, "not how castling is written");
    }

    #[test]
    fn promotion_needs_its_piece() {
        let p = Position::from_fen("8/4P3/8/8/8/8/8/K6k w - - 0 1").unwrap();
        assert_eq!(p.move_from_uci("e7e8"), None);
        let q = p.move_from_uci("e7e8q").unwrap();
        assert_eq!(q.flag(), FLAG_PROMO);
        assert_eq!(q.promo(), Piece::Queen);
        assert_eq!(p.move_from_uci("e7e8k"), None);
    }

    #[test]
    fn illegal_and_malformed_input_is_rejected_not_trusted() {
        let p = Position::startpos();
        assert_eq!(p.move_from_uci("e2e5"), None);
        assert_eq!(p.move_from_uci(""), None);
        assert_eq!(p.move_from_uci("z9z9"), None);
        assert_eq!(p.move_from_uci("e2e4e4"), None);
    }
}
