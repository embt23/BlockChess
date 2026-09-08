//! The board, and the function that advances it.
//!
//! `make_move` is consensus-critical: it runs in your client thousands of times
//! per game and on-chain during a dispute, and the two must agree bit for bit.
//! It is written once, here, and compiled for both. See `spec/03-position.md`.

use crate::attacks::*;
use crate::bitboard::*;
use crate::types::*;

/// Squares whose occupancy or vacation can cost castling rights. The mask is
/// the set of rights to *keep*, so updating is `castling &= M[from] & M[to]` —
/// which correctly handles a rook being captured on its home square as well as
/// a rook or king moving off it.
const fn castle_masks() -> [u8; 64] {
    let mut m = [0b1111u8; 64];
    m[0] &= !CASTLE_WQ; // a1
    m[4] &= !(CASTLE_WK | CASTLE_WQ); // e1
    m[7] &= !CASTLE_WK; // h1
    m[56] &= !CASTLE_BQ; // a8
    m[60] &= !(CASTLE_BK | CASTLE_BQ); // e8
    m[63] &= !CASTLE_BK; // h8
    m
}
static CASTLE_MASK: [u8; 64] = castle_masks();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    /// All pieces of each colour.
    pub color_bb: [Bitboard; 2],
    /// All pieces of each kind, both colours.
    pub piece_bb: [Bitboard; 6],
    pub side: Color,
    pub castling: u8,
    /// En-passant *target* square (the square the capturing pawn moves to).
    pub ep: Option<Square>,
    pub halfmove: u8,
    pub fullmove: u16,
}

#[derive(Debug, PartialEq, Eq)]
pub struct FenError(pub String);

impl std::fmt::Display for FenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "bad FEN: {}", self.0)
    }
}
impl std::error::Error for FenError {}

impl Position {
    pub const START_FEN: &'static str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

    pub fn startpos() -> Position {
        Position::from_fen(Position::START_FEN).unwrap()
    }

    pub fn empty() -> Position {
        Position {
            color_bb: [0; 2],
            piece_bb: [0; 6],
            side: Color::White,
            castling: 0,
            ep: None,
            halfmove: 0,
            fullmove: 1,
        }
    }

    #[inline]
    pub fn occupied(&self) -> Bitboard {
        self.color_bb[0] | self.color_bb[1]
    }

    #[inline]
    pub fn pieces(&self, c: Color, p: Piece) -> Bitboard {
        self.color_bb[c.idx()] & self.piece_bb[p.idx()]
    }

    #[inline]
    pub fn king_sq(&self, c: Color) -> Square {
        lsb(self.pieces(c, Piece::King))
    }

    pub fn piece_at(&self, s: Square) -> Option<(Color, Piece)> {
        let b = bit(s);
        if self.occupied() & b == 0 {
            return None;
        }
        let c = if self.color_bb[0] & b != 0 {
            Color::White
        } else {
            Color::Black
        };
        for p in Piece::ALL {
            if self.piece_bb[p.idx()] & b != 0 {
                return Some((c, p));
            }
        }
        None
    }

    #[inline]
    fn put(&mut self, c: Color, p: Piece, s: Square) {
        self.color_bb[c.idx()] |= bit(s);
        self.piece_bb[p.idx()] |= bit(s);
    }

    #[inline]
    fn clear(&mut self, c: Color, p: Piece, s: Square) {
        self.color_bb[c.idx()] &= !bit(s);
        self.piece_bb[p.idx()] &= !bit(s);
    }

    /// Is `s` attacked by any piece of colour `by`?
    ///
    /// The trick with pawns: instead of asking "which squares do enemy pawns
    /// attack", we ask "if *our* pawn stood on `s`, which squares would it
    /// attack" and intersect that with enemy pawns. Pawn attacks are symmetric
    /// under colour flip, so this works and saves a second table.
    pub fn attacked(&self, s: Square, by: Color) -> bool {
        let occ = self.occupied();

        if pawn_attacks(by.flip(), s) & self.pieces(by, Piece::Pawn) != 0 {
            return true;
        }
        if knight_attacks(s) & self.pieces(by, Piece::Knight) != 0 {
            return true;
        }
        if king_attacks(s) & self.pieces(by, Piece::King) != 0 {
            return true;
        }
        let bq = self.pieces(by, Piece::Bishop) | self.pieces(by, Piece::Queen);
        if bishop_attacks(s, occ) & bq != 0 {
            return true;
        }
        let rq = self.pieces(by, Piece::Rook) | self.pieces(by, Piece::Queen);
        if rook_attacks(s, occ) & rq != 0 {
            return true;
        }
        false
    }

    pub fn in_check(&self, c: Color) -> bool {
        let k = self.pieces(c, Piece::King);
        // Perft test positions always have kings, but be total rather than
        // panicking — this function is consensus-critical.
        k != 0 && self.attacked(lsb(k), c.flip())
    }

    /// Apply a pseudo-legal move, returning the new position.
    ///
    /// Copy-make rather than make/unmake: a `Position` is 104 bytes, copying it
    /// is a few instructions, and it removes an entire class of bug where the
    /// undo does not exactly reverse the do. That bug class is invisible in
    /// normal play and shows up as a consensus split.
    pub fn make_move(&self, m: Move) -> Position {
        let mut p = *self;
        let us = self.side;
        let them = us.flip();
        let from = m.from();
        let to = m.to();
        let flag = m.flag();

        let (_, moving) = self
            .piece_at(from)
            .expect("make_move called with no piece on the from-square");

        p.ep = None;
        p.halfmove = self.halfmove.saturating_add(1);

        // Remove any captured piece first.
        if flag == FLAG_EP {
            // The captured pawn is beside the moving pawn, not on `to`.
            let cap_sq = if us == Color::White { to - 8 } else { to + 8 };
            p.clear(them, Piece::Pawn, cap_sq);
            p.halfmove = 0;
        } else if let Some((cc, cp)) = self.piece_at(to) {
            debug_assert_eq!(cc, them, "cannot capture own piece");
            p.clear(cc, cp, to);
            p.halfmove = 0;
        }

        // Move the piece.
        p.clear(us, moving, from);
        if flag == FLAG_PROMO {
            p.put(us, m.promo(), to);
        } else {
            p.put(us, moving, to);
        }

        if moving == Piece::Pawn {
            p.halfmove = 0;
            // Double push sets the en-passant target.
            if to.abs_diff(from) == 16 {
                p.ep = Some((from + to) / 2);
            }
        }

        // Castling moves the rook too. The move encodes king from/to; the rook
        // is implied by which side of the board the king landed on.
        if flag == FLAG_CASTLE {
            let (rf, rt) = match to {
                6 => (7u8, 5u8),    // white kingside:  h1 -> f1
                2 => (0u8, 3u8),    // white queenside: a1 -> d1
                62 => (63u8, 61u8), // black kingside:  h8 -> f8
                58 => (56u8, 59u8), // black queenside: a8 -> d8
                _ => unreachable!("castle to non-castling square"),
            };
            p.clear(us, Piece::Rook, rf);
            p.put(us, Piece::Rook, rt);
        }

        p.castling &= CASTLE_MASK[from as usize] & CASTLE_MASK[to as usize];

        if us == Color::Black {
            p.fullmove += 1;
        }
        p.side = them;
        p
    }

    pub fn from_fen(fen: &str) -> Result<Position, FenError> {
        let mut parts = fen.split_whitespace();
        let board = parts.next().ok_or_else(|| FenError("empty".into()))?;
        let side = parts.next().unwrap_or("w");
        let castling = parts.next().unwrap_or("-");
        let ep = parts.next().unwrap_or("-");
        let halfmove = parts.next().unwrap_or("0");
        let fullmove = parts.next().unwrap_or("1");

        let mut p = Position::empty();

        let mut rank: i32 = 7;
        let mut file: i32 = 0;
        for ch in board.chars() {
            match ch {
                '/' => {
                    if file != 8 {
                        return Err(FenError(format!("rank {} has {} files", 8 - rank, file)));
                    }
                    rank -= 1;
                    file = 0;
                    if rank < 0 {
                        return Err(FenError("too many ranks".into()));
                    }
                }
                '1'..='8' => file += ch as i32 - '0' as i32,
                _ => {
                    let color = if ch.is_ascii_uppercase() {
                        Color::White
                    } else {
                        Color::Black
                    };
                    let piece = match ch.to_ascii_lowercase() {
                        'p' => Piece::Pawn,
                        'n' => Piece::Knight,
                        'b' => Piece::Bishop,
                        'r' => Piece::Rook,
                        'q' => Piece::Queen,
                        'k' => Piece::King,
                        _ => return Err(FenError(format!("bad piece '{ch}'"))),
                    };
                    if !(0..8).contains(&file) || !(0..8).contains(&rank) {
                        return Err(FenError("piece off board".into()));
                    }
                    p.put(color, piece, sq(file as u8, rank as u8));
                    file += 1;
                }
            }
        }
        if rank != 0 || file != 8 {
            return Err(FenError("wrong number of squares".into()));
        }

        p.side = if side == "b" {
            Color::Black
        } else {
            Color::White
        };

        for ch in castling.chars() {
            match ch {
                'K' => p.castling |= CASTLE_WK,
                'Q' => p.castling |= CASTLE_WQ,
                'k' => p.castling |= CASTLE_BK,
                'q' => p.castling |= CASTLE_BQ,
                '-' => {}
                _ => return Err(FenError(format!("bad castling '{ch}'"))),
            }
        }

        if ep != "-" {
            let b = ep.as_bytes();
            if b.len() != 2 {
                return Err(FenError(format!("bad ep '{ep}'")));
            }
            p.ep = Some(sq(b[0] - b'a', b[1] - b'1'));
        }

        p.halfmove = halfmove.parse().unwrap_or(0);
        p.fullmove = fullmove.parse().unwrap_or(1);
        Ok(p)
    }

    pub fn to_fen(&self) -> String {
        let mut s = String::new();
        for rank in (0..8).rev() {
            let mut run = 0;
            for file in 0..8 {
                match self.piece_at(sq(file, rank)) {
                    None => run += 1,
                    Some((c, p)) => {
                        if run > 0 {
                            s.push_str(&run.to_string());
                            run = 0;
                        }
                        let ch = p.ch();
                        s.push(if c == Color::White {
                            ch.to_ascii_uppercase()
                        } else {
                            ch
                        });
                    }
                }
            }
            if run > 0 {
                s.push_str(&run.to_string());
            }
            if rank > 0 {
                s.push('/');
            }
        }
        s.push(' ');
        s.push(if self.side == Color::White { 'w' } else { 'b' });
        s.push(' ');
        if self.castling == 0 {
            s.push('-');
        } else {
            for (bitmask, ch) in [
                (CASTLE_WK, 'K'),
                (CASTLE_WQ, 'Q'),
                (CASTLE_BK, 'k'),
                (CASTLE_BQ, 'q'),
            ] {
                if self.castling & bitmask != 0 {
                    s.push(ch);
                }
            }
        }
        s.push(' ');
        match self.ep {
            None => s.push('-'),
            Some(e) => {
                s.push((b'a' + file_of(e)) as char);
                s.push((b'1' + rank_of(e)) as char);
            }
        }
        format!("{} {} {}", s, self.halfmove, self.fullmove)
    }
}

impl std::fmt::Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for rank in (0..8).rev() {
            write!(f, "{} ", rank + 1)?;
            for file in 0..8 {
                let c = match self.piece_at(sq(file, rank)) {
                    None => '.',
                    Some((Color::White, p)) => p.ch().to_ascii_uppercase(),
                    Some((Color::Black, p)) => p.ch(),
                };
                write!(f, "{c} ")?;
            }
            writeln!(f)?;
        }
        writeln!(f, "  a b c d e f g h")?;
        write!(f, "{}", self.to_fen())
    }
}
