//! The canonical wire encoding of a position — `spec/03-position.md`.
//!
//! Bitboards are 104 bytes and we sign a position hash every ply, so the wire
//! form is packed to ≤ 26:
//!
//! ```text
//! occupancy   u64                             8 bytes
//! nibbles     4 bits × popcount(occupancy)   ≤ 16 bytes
//! flags       u16                             2 bytes
//! ```
//!
//! **This encoding must be the only valid one for a given position.** It is
//! hashed, and the hash is what threefold repetition compares. If one position
//! has two encodings it has two hashes, and a repetition that happened is not
//! detected — or worse, a claim that no repetition happened survives a
//! dispute. So [`unpack`] rejects everything it can: spare nibble bits,
//! out-of-range piece codes, impossible castling rights, en-passant files
//! where no capture is possible, and positions with the wrong number of kings.
//!
//! The subtle one is en passant. [`Position::ep`] is set by `make_move` after
//! *every* double pawn push. But "white played e2e4" and "white played e2e3
//! then e3e4" reach boards that are identical in every way a player can act
//! on, and FIDE counts them as the same position for repetition. So the packed
//! form stores the ep file only when a legal en-passant capture actually
//! exists — see [`Position::ep_effective`].

use crate::attacks::pawn_attacks;
use crate::bitboard::*;
use crate::position::Position;
use crate::types::*;

/// Longest possible packed position: 8 occupancy + 16 nibbles + 2 flags.
pub const MAX_PACKED_LEN: usize = 26;

/// `ep_file` value meaning "no en-passant capture is available".
const EP_NONE: u16 = 8;

#[derive(Debug, PartialEq, Eq)]
pub struct PackError(pub &'static str);

impl core::fmt::Display for PackError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "bad packed position: {}", self.0)
    }
}
#[cfg(feature = "std")]
impl std::error::Error for PackError {}

/// A packed position: a fixed buffer plus a length, so that packing needs no
/// allocator. The adjudicator runs this code on-chain.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Packed {
    buf: [u8; MAX_PACKED_LEN],
    len: u8,
}

impl Packed {
    pub fn as_slice(&self) -> &[u8] {
        &self.buf[..self.len as usize]
    }
    pub fn len(&self) -> usize {
        self.len as usize
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl Packed {
    /// The same position with the halfmove clock zeroed.
    ///
    /// Repetition identity is **not** position identity. Two occurrences of a
    /// board differ in the halfmove clock the moment anything has happened
    /// between them — which, in a repetition, is precisely the case. FIDE
    /// compares pieces, side to move, castling rights and en-passant
    /// availability, and says nothing about the fifty-move counter.
    ///
    /// The clock has to stay in the packed form, because the adjudicator
    /// replays moves from it and the fifty-move rule reads it. So the
    /// repetition key is derived by clearing it rather than by leaving it
    /// out, and the two hashes are separately domain-tagged.
    pub fn without_halfmove(mut self) -> Packed {
        let n = self.len as usize;
        let flags = u16::from_le_bytes([self.buf[n - 2], self.buf[n - 1]]);
        let bytes = (flags & 0x1FF).to_le_bytes();
        self.buf[n - 2] = bytes[0];
        self.buf[n - 1] = bytes[1];
        self
    }
}

impl core::fmt::Debug for Packed {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for b in self.as_slice() {
            write!(f, "{b:02x}")?;
        }
        Ok(())
    }
}

/// Piece codes 0–11: white P N B R Q K, then black P N B R Q K.
const fn code_of(c: Color, p: Piece) -> u8 {
    (c as u8) * 6 + p as u8
}

fn decode_code(n: u8) -> Option<(Color, Piece)> {
    if n > 11 {
        return None;
    }
    let c = if n < 6 { Color::White } else { Color::Black };
    Some((c, Piece::from_idx((n % 6) as usize)))
}

/// Which castling rights are *structurally* possible: the king on its home
/// square and the relevant rook on its corner. A right without its pieces is
/// unreachable, so it is a second encoding of a position that already has a
/// canonical one.
fn castling_possible(p: &Position) -> u8 {
    let mut mask = 0u8;
    let wk = p.pieces(Color::White, Piece::King) & bit(4) != 0;
    let bk = p.pieces(Color::Black, Piece::King) & bit(60) != 0;
    let wr = p.pieces(Color::White, Piece::Rook);
    let br = p.pieces(Color::Black, Piece::Rook);
    if wk && wr & bit(7) != 0 {
        mask |= CASTLE_WK;
    }
    if wk && wr & bit(0) != 0 {
        mask |= CASTLE_WQ;
    }
    if bk && br & bit(63) != 0 {
        mask |= CASTLE_BK;
    }
    if bk && br & bit(56) != 0 {
        mask |= CASTLE_BQ;
    }
    mask
}

impl Position {
    /// The en-passant target square, but only when a **legal** en-passant
    /// capture onto it exists.
    ///
    /// Three things can make a recorded ep target meaningless: no enemy pawn
    /// beside the pushed one, a pawn that is beside it but pinned, and the
    /// exotic case where the capture exposes the king along the fifth rank
    /// because *two* pawns leave it at once. All three are handled by simply
    /// asking the move generator, which is the point of having exactly one
    /// implementation of the rules (`P5`).
    pub fn ep_effective(&self) -> Option<Square> {
        let target = self.ep?;
        // Candidate captures come from the squares a friendly pawn would have
        // to stand on: the squares an *enemy* pawn on the target attacks.
        let mut from_bb =
            pawn_attacks(self.side.flip(), target) & self.pieces(self.side, Piece::Pawn);
        while from_bb != 0 {
            let from = pop_lsb(&mut from_bb);
            if self.is_legal(Move::new(from, target, FLAG_EP, 0)) {
                return Some(target);
            }
        }
        None
    }

    /// Pack into the canonical wire form.
    pub fn pack(&self) -> Packed {
        let mut buf = [0u8; MAX_PACKED_LEN];
        let occ = self.occupied();
        buf[..8].copy_from_slice(&occ.to_le_bytes());

        let mut n = 0usize;
        let mut bb = occ;
        while bb != 0 {
            let s = pop_lsb(&mut bb);
            let (c, p) = self.piece_at(s).expect("occupancy disagrees with pieces");
            let nib = code_of(c, p);
            // Nibble i lives in byte i/2: low half when i is even.
            let byte = 8 + n / 2;
            buf[byte] |= if n.is_multiple_of(2) { nib } else { nib << 4 };
            n += 1;
        }

        let nibble_bytes = n.div_ceil(2);
        let flags = self.flags_u16();
        buf[8 + nibble_bytes..8 + nibble_bytes + 2].copy_from_slice(&flags.to_le_bytes());

        Packed {
            buf,
            len: (8 + nibble_bytes + 2) as u8,
        }
    }

    /// `side | castling | ep_file | halfmove`, low bits first.
    fn flags_u16(&self) -> u16 {
        let ep_file = match self.ep_effective() {
            Some(s) => file_of(s) as u16,
            None => EP_NONE,
        };
        (self.side as u16)
            | ((self.castling as u16 & 0xF) << 1)
            | (ep_file << 5)
            | ((self.halfmove.min(100) as u16) << 9)
    }
}

/// Decode a packed position, rejecting every non-canonical encoding.
///
/// The returned position has `fullmove = 1`: the move number is deliberately
/// not part of a position, because two positions differing only in move number
/// are the same position for repetition (`spec/03`).
pub fn unpack(bytes: &[u8]) -> Result<Position, PackError> {
    if bytes.len() < 10 {
        return Err(PackError("too short"));
    }
    let occ = u64::from_le_bytes(bytes[..8].try_into().unwrap());
    let count = occ.count_ones() as usize;
    if count > 32 {
        return Err(PackError("more than 32 pieces"));
    }
    let nibble_bytes = count.div_ceil(2);
    if bytes.len() != 8 + nibble_bytes + 2 {
        return Err(PackError("length disagrees with occupancy"));
    }

    let mut p = Position::empty();
    let mut bb = occ;
    let mut n = 0usize;
    while bb != 0 {
        let s = pop_lsb(&mut bb);
        let byte = bytes[8 + n / 2];
        let nib = if n.is_multiple_of(2) {
            byte & 0xF
        } else {
            byte >> 4
        };
        let (c, piece) = decode_code(nib).ok_or(PackError("piece code 12-15"))?;
        p.color_bb[c.idx()] |= bit(s);
        p.piece_bb[piece.idx()] |= bit(s);
        n += 1;
    }
    // An odd piece count leaves a high nibble spare. It must be zero, or the
    // same position has sixteen encodings.
    if count % 2 == 1 && bytes[8 + count / 2] >> 4 != 0 {
        return Err(PackError("spare nibble not zero"));
    }

    let flags = u16::from_le_bytes(bytes[8 + nibble_bytes..].try_into().unwrap());
    p.side = if flags & 1 == 0 {
        Color::White
    } else {
        Color::Black
    };
    p.castling = ((flags >> 1) & 0xF) as u8;
    let ep_file = (flags >> 5) & 0xF;
    let halfmove = flags >> 9;

    if halfmove > 100 {
        return Err(PackError("halfmove over 100"));
    }
    p.halfmove = halfmove as u8;

    if p.pieces(Color::White, Piece::King).count_ones() != 1
        || p.pieces(Color::Black, Piece::King).count_ones() != 1
    {
        return Err(PackError("not exactly one king each"));
    }
    if (p.piece_bb[Piece::Pawn.idx()] & (RANK_1 | RANK_8)) != 0 {
        return Err(PackError("pawn on the back rank"));
    }
    if p.castling & !castling_possible(&p) != 0 {
        return Err(PackError("castling right without its king and rook"));
    }
    // The side that just moved must not be in check: no legal move leaves it
    // so, therefore no reachable position has it.
    if p.in_check(p.side.flip()) {
        return Err(PackError("side not to move is in check"));
    }

    if ep_file != EP_NONE {
        if ep_file > 7 {
            return Err(PackError("ep file 9-15"));
        }
        let rank = if p.side == Color::White { 5 } else { 2 };
        p.ep = Some(sq(ep_file as u8, rank));
        if p.ep_effective().is_none() {
            return Err(PackError("ep file with no legal capture"));
        }
    }

    Ok(p)
}
