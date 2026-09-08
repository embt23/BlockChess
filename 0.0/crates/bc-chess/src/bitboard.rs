//! The board is 64 squares. The machine word is 64 bits. Nobody planned that,
//! but it means a set of squares is a `u64` and set operations on the board are
//! single instructions.
//!
//! "Every square attacked by a white knight" is a handful of shifts and ORs.
//! "Is the king in check" is one AND against zero.

pub type Bitboard = u64;

pub const FILE_A: Bitboard = 0x0101_0101_0101_0101;
pub const FILE_H: Bitboard = FILE_A << 7;
pub const RANK_1: Bitboard = 0xFF;
pub const RANK_2: Bitboard = RANK_1 << 8;
pub const RANK_4: Bitboard = RANK_1 << 24;
pub const RANK_5: Bitboard = RANK_1 << 32;
pub const RANK_7: Bitboard = RANK_1 << 48;
pub const RANK_8: Bitboard = RANK_1 << 56;

#[inline]
pub const fn bit(sq: u8) -> Bitboard {
    1u64 << sq
}

/// Index of the least significant set bit.
#[inline]
pub const fn lsb(b: Bitboard) -> u8 {
    b.trailing_zeros() as u8
}

/// Index of the most significant set bit.
#[inline]
pub const fn msb(b: Bitboard) -> u8 {
    63 - b.leading_zeros() as u8
}

/// Remove and return the least significant set bit's index.
#[inline]
pub fn pop_lsb(b: &mut Bitboard) -> u8 {
    let s = lsb(*b);
    *b &= *b - 1; // clears the lowest set bit — the classic trick
    s
}

/// Iterate the set squares of a bitboard.
pub struct Squares(pub Bitboard);

impl Iterator for Squares {
    type Item = u8;
    #[inline]
    fn next(&mut self) -> Option<u8> {
        if self.0 == 0 {
            None
        } else {
            Some(pop_lsb(&mut self.0))
        }
    }
}

/// Debug helper: print a bitboard as a board, rank 8 at the top.
pub fn render(b: Bitboard) -> String {
    let mut s = String::new();
    for rank in (0..8).rev() {
        for file in 0..8 {
            s.push(if b & bit(rank * 8 + file) != 0 {
                'X'
            } else {
                '.'
            });
            s.push(' ');
        }
        s.push('\n');
    }
    s
}
