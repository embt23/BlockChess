//! Board vocabulary: squares, colours, pieces, moves.

/// Squares are indexed a1 = 0, b1 = 1, … h8 = 63.
/// So `rank = sq / 8` and `file = sq % 8`, and "one square north" is `+8`.
pub type Square = u8;

pub const fn sq(file: u8, rank: u8) -> Square {
    rank * 8 + file
}
pub const fn file_of(s: Square) -> u8 {
    s % 8
}
pub const fn rank_of(s: Square) -> u8 {
    s / 8
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    White = 0,
    Black = 1,
}

impl Color {
    pub const fn flip(self) -> Color {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
    pub const fn idx(self) -> usize {
        self as usize
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Piece {
    Pawn = 0,
    Knight = 1,
    Bishop = 2,
    Rook = 3,
    Queen = 4,
    King = 5,
}

impl Piece {
    pub const ALL: [Piece; 6] = [
        Piece::Pawn,
        Piece::Knight,
        Piece::Bishop,
        Piece::Rook,
        Piece::Queen,
        Piece::King,
    ];
    pub const fn idx(self) -> usize {
        self as usize
    }
    pub const fn from_idx(i: usize) -> Piece {
        Piece::ALL[i]
    }
    pub const fn ch(self) -> char {
        match self {
            Piece::Pawn => 'p',
            Piece::Knight => 'n',
            Piece::Bishop => 'b',
            Piece::Rook => 'r',
            Piece::Queen => 'q',
            Piece::King => 'k',
        }
    }
}

// Castling rights, one bit each.
pub const CASTLE_WK: u8 = 0b0001;
pub const CASTLE_WQ: u8 = 0b0010;
pub const CASTLE_BK: u8 = 0b0100;
pub const CASTLE_BQ: u8 = 0b1000;

/// A move, packed into 16 bits exactly as `spec/03-position.md` specifies.
///
/// ```text
/// bits  0-5   from square
/// bits  6-11  to square
/// bits 12-13  promotion piece (0=N 1=B 2=R 3=Q)
/// bits 14-15  flag (0=normal 1=promotion 2=en passant 3=castle)
/// ```
///
/// Two bytes per move matters: this is what gets hashed into every game state
/// and, in a dispute, what goes on-chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Move(pub u16);

pub const FLAG_NORMAL: u16 = 0;
pub const FLAG_PROMO: u16 = 1;
pub const FLAG_EP: u16 = 2;
pub const FLAG_CASTLE: u16 = 3;

impl Move {
    pub const fn new(from: Square, to: Square, flag: u16, promo: u16) -> Move {
        Move((from as u16) | ((to as u16) << 6) | (promo << 12) | (flag << 14))
    }
    pub const fn normal(from: Square, to: Square) -> Move {
        Move::new(from, to, FLAG_NORMAL, 0)
    }
    pub const fn from(self) -> Square {
        (self.0 & 0x3F) as Square
    }
    pub const fn to(self) -> Square {
        ((self.0 >> 6) & 0x3F) as Square
    }
    pub const fn flag(self) -> u16 {
        self.0 >> 14
    }
    /// Only meaningful when `flag() == FLAG_PROMO`.
    pub const fn promo(self) -> Piece {
        match (self.0 >> 12) & 3 {
            0 => Piece::Knight,
            1 => Piece::Bishop,
            2 => Piece::Rook,
            _ => Piece::Queen,
        }
    }

    /// Long algebraic notation, e.g. `e2e4`, `e7e8q`.
    pub fn to_uci(self) -> String {
        let name = |s: Square| {
            format!(
                "{}{}",
                (b'a' + file_of(s)) as char,
                (b'1' + rank_of(s)) as char
            )
        };
        let mut s = format!("{}{}", name(self.from()), name(self.to()));
        if self.flag() == FLAG_PROMO {
            s.push(self.promo().ch());
        }
        s
    }
}

/// A fixed-capacity move buffer.
///
/// 218 is the largest number of legal moves any chess position is known to
/// admit; 256 gives headroom for pseudo-legal generation. Using a stack array
/// rather than a `Vec` avoids an allocation per node, which at 119 million
/// nodes is the difference between seconds and minutes.
pub struct MoveList {
    moves: [Move; 256],
    len: usize,
}

impl Default for MoveList {
    fn default() -> Self {
        Self::new()
    }
}

impl MoveList {
    pub fn new() -> Self {
        MoveList {
            moves: [Move(0); 256],
            len: 0,
        }
    }
    #[inline]
    pub fn push(&mut self, m: Move) {
        debug_assert!(self.len < 256, "move list overflow");
        self.moves[self.len] = m;
        self.len += 1;
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn as_slice(&self) -> &[Move] {
        &self.moves[..self.len]
    }
}

impl<'a> IntoIterator for &'a MoveList {
    type Item = &'a Move;
    type IntoIter = core::slice::Iter<'a, Move>;
    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}
