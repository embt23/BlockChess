//! The canonical packed position, and the symmetry group that acts on it.
//!
//! Two jobs that belong together because the second is defined in terms of the
//! first.
//!
//! ## Packing
//!
//! `papers/01-position-space.md` measured a position at ~150 bits of real
//! content. This is the practical encoding of that:
//!
//! ```text
//! occupancy   8 bytes    which squares hold a man
//! nibbles     ceil(n/2)  4 bits per occupied square, ascending, one of 12 kinds
//! flags       1 byte     side to move | castling rights | en passant file
//! ```
//!
//! 26 bytes at most, ~20 in a typical middlegame. The `12` matters: there are
//! twelve (colour, kind) pairs, and a nibble holds sixteen values, so four
//! codes are spare. They are rejected on decode rather than ignored.
//!
//! ### Canonicality is the whole point
//!
//! This is an index key, so the *only* requirement that really bites is that
//! one position has exactly one encoding. Two spellings of the same position
//! hash differently, the transposition index silently splits, and the database
//! quietly answers "nobody has played this" about a line thousands of people
//! have played.
//!
//! Two places where that nearly goes wrong, both called out in
//! `papers/03-corpus.md`:
//!
//! - **En passant.** A FEN may name an ep target square when no ep capture is
//!   actually available, and most do. So the ep field here is set only when
//!   some legal move is genuinely an ep capture — which needs the move
//!   generator, which is why this lives above `bc-chess`.
//! - **The halfmove clock is excluded.** `papers/05-index.md` §3: a player
//!   studying a structure does not care that one game arrived with the clock
//!   at 4 and another at 11. The clock stays in the posting, not the key.
//!
//! ## The symmetry group, as code
//!
//! `papers/01-position-space.md` §5 worked out what survives contact with the
//! rules. Pawns kill rank reversal; castling rights kill file reversal; the
//! diagonals need both dead. What is left:
//!
//! | regime | group | order |
//! |---|---|---|
//! | no pawns, no castling rights | `D4 × ⟨colour swap⟩` | 16 |
//! | pawns, no castling rights | `⟨file mirror⟩ × ⟨colour swap⟩` | 4 |
//! | castling rights outstanding | `⟨colour swap⟩` | 2 |
//! | the general case | trivial | 1 |
//!
//! `canonical_key` applies every surviving element and keeps the smallest
//! packing. `papers/01` measured this as worth ~0 bits of storage and ~4×
//! fewer distinct keys in an index — which is why it is here, in the index
//! path, and not in `bc-codec`'s game encoding.

use bc_chess::bitboard::Squares;
use bc_chess::types::{file_of, rank_of, FLAG_EP};
use bc_chess::types::{CASTLE_BK, CASTLE_BQ, CASTLE_WK, CASTLE_WQ};
use bc_chess::{Color, Piece, Position, Square};

use crate::CodecError;

/// The twelve piece codes, in a fixed order: white P N B R Q K, then black.
fn code_of(c: Color, p: Piece) -> u8 {
    (c.idx() * 6 + p.idx()) as u8
}

fn decode_code(v: u8) -> Option<(Color, Piece)> {
    if v >= 12 {
        return None;
    }
    let c = if v < 6 { Color::White } else { Color::Black };
    Some((c, Piece::from_idx((v % 6) as usize)))
}

/// Is an en passant capture actually available? The ep field is only canonical
/// when the answer is yes.
pub fn ep_is_real(pos: &Position) -> bool {
    pos.ep.is_some()
        && pos
            .generate_legal()
            .into_iter()
            .any(|m| m.flag() == FLAG_EP)
}

/// Pack a position into its canonical byte string.
///
/// The halfmove clock and the fullmove number are deliberately absent; see the
/// module docs.
pub fn pack(pos: &Position) -> Vec<u8> {
    let occ = pos.occupied();
    let mut out = Vec::with_capacity(26);
    out.extend_from_slice(&occ.to_le_bytes());

    let mut nibbles: Vec<u8> = Vec::with_capacity(32);
    for sq in Squares(occ) {
        let (c, p) = pos.piece_at(sq).expect("occupied square holds a man");
        nibbles.push(code_of(c, p));
    }
    for pair in nibbles.chunks(2) {
        let hi = if pair.len() > 1 { pair[1] } else { 0 };
        out.push(pair[0] | (hi << 4));
    }

    // side (1) | castling (4) | ep file present + file (3 more would overflow,
    // so ep file goes in the top 3 bits and "no ep" is encoded by file 7 being
    // impossible — instead we use a separate sentinel byte only when needed).
    let mut flags = (pos.side as u8) | (pos.castling << 1);
    let ep = if ep_is_real(pos) {
        Some(file_of(pos.ep.expect("ep_is_real implies Some")))
    } else {
        None
    };
    // Bit 5 says an ep byte follows. Keeping it out-of-band rather than
    // squeezing a file into three spare bits keeps the flags byte readable and
    // costs one byte in the rare positions that have a live ep capture.
    if ep.is_some() {
        flags |= 1 << 5;
    }
    out.push(flags);
    if let Some(f) = ep {
        out.push(f);
    }
    out
}

/// Every square permutation we may need, as a table `perm[from] = to`.
type Perm = [u8; 64];

fn build(f: impl Fn(u8, u8) -> (u8, u8)) -> Perm {
    let mut p = [0u8; 64];
    for s in 0..64u8 {
        let (r, fl) = f(rank_of(s), file_of(s));
        p[s as usize] = r * 8 + fl;
    }
    p
}

/// The eight elements of D4, as permutations of the squares.
pub struct D4 {
    pub e: Perm,
    pub m_file: Perm,
    pub m_rank: Perm,
    pub rot180: Perm,
    pub d_main: Perm,
    pub d_anti: Perm,
    pub rot90: Perm,
    pub rot270: Perm,
}

impl D4 {
    pub fn new() -> D4 {
        D4 {
            e: build(|r, f| (r, f)),
            m_file: build(|r, f| (r, 7 - f)),
            m_rank: build(|r, f| (7 - r, f)),
            rot180: build(|r, f| (7 - r, 7 - f)),
            d_main: build(|r, f| (f, r)),
            d_anti: build(|r, f| (7 - f, 7 - r)),
            rot90: build(|r, f| (f, 7 - r)),
            rot270: build(|r, f| (7 - f, r)),
        }
    }
}

impl Default for D4 {
    fn default() -> Self {
        D4::new()
    }
}

/// One element of the acting group: permute the squares, and optionally swap
/// the colours (which also swaps the side to move and the castling rights).
#[derive(Clone, Copy)]
pub struct GroupElement {
    pub perm: Perm,
    pub swap_colours: bool,
    /// A short name, for reporting which element a canonical form used.
    pub name: &'static str,
}

/// Apply a group element to a position.
///
/// Note what is *not* transformed: the halfmove and fullmove counters, which
/// are not part of the key, and the castling rights under anything but a plain
/// colour swap — because the group is only allowed to contain rank-reversing
/// elements when there are no castling rights left to move.
pub fn transform(pos: &Position, g: &GroupElement) -> Position {
    let mut out = Position::empty();
    for sq in Squares(pos.occupied()) {
        let (c, p) = pos.piece_at(sq).expect("occupied");
        let c = if g.swap_colours { c.flip() } else { c };
        let to = g.perm[sq as usize];
        out.color_bb[c.idx()] |= 1u64 << to;
        out.piece_bb[p.idx()] |= 1u64 << to;
    }
    out.side = if g.swap_colours {
        pos.side.flip()
    } else {
        pos.side
    };
    out.castling = if g.swap_colours {
        // Rank reversal maps e1->e8 and h1->h8, so a white king-side right
        // becomes a black king-side right. Files are untouched, so K stays K.
        let mut c = 0;
        if pos.castling & CASTLE_WK != 0 {
            c |= CASTLE_BK;
        }
        if pos.castling & CASTLE_WQ != 0 {
            c |= CASTLE_BQ;
        }
        if pos.castling & CASTLE_BK != 0 {
            c |= CASTLE_WK;
        }
        if pos.castling & CASTLE_BQ != 0 {
            c |= CASTLE_WQ;
        }
        c
    } else {
        pos.castling
    };
    out.ep = pos.ep.map(|s| g.perm[s as usize]);
    out.halfmove = pos.halfmove;
    out.fullmove = pos.fullmove;
    out
}

/// The group that actually acts on this position, per `papers/01` §5.
pub fn surviving_group(pos: &Position) -> Vec<GroupElement> {
    let d4 = D4::new();
    let pawnless = pos.piece_bb[Piece::Pawn.idx()] == 0;
    let castling = pos.castling != 0;

    if castling {
        // Only the colour swap survives; the file mirror would put the king on
        // d1 with castling rights, which is not a chess position.
        return vec![
            GroupElement {
                perm: d4.e,
                swap_colours: false,
                name: "e",
            },
            GroupElement {
                perm: d4.m_rank,
                swap_colours: true,
                name: "m_rank+swap",
            },
        ];
    }

    if pawnless {
        // Nothing picks out a direction, so all of D4 acts, and independently
        // so does the colour swap.
        let mut out = Vec::with_capacity(16);
        for (perm, name) in [
            (d4.e, "e"),
            (d4.m_file, "m_file"),
            (d4.m_rank, "m_rank"),
            (d4.rot180, "rot180"),
            (d4.d_main, "d_main"),
            (d4.d_anti, "d_anti"),
            (d4.rot90, "rot90"),
            (d4.rot270, "rot270"),
        ] {
            out.push(GroupElement {
                perm,
                swap_colours: false,
                name,
            });
            out.push(GroupElement {
                perm,
                swap_colours: true,
                name,
            });
        }
        return out;
    }

    // Pawns, no castling rights: the file mirror is fine on its own, and rank
    // reversal is fine when paired with a colour swap so the pawns still move
    // the right way.
    vec![
        GroupElement {
            perm: d4.e,
            swap_colours: false,
            name: "e",
        },
        GroupElement {
            perm: d4.m_file,
            swap_colours: false,
            name: "m_file",
        },
        GroupElement {
            perm: d4.m_rank,
            swap_colours: true,
            name: "m_rank+swap",
        },
        GroupElement {
            perm: d4.rot180,
            swap_colours: true,
            name: "rot180+swap",
        },
    ]
}

/// The canonical index key: the smallest packing over the surviving group.
///
/// Returns the key and the name of the element that produced it, so a display
/// layer can un-mirror what it shows.
pub fn canonical_key(pos: &Position) -> (Vec<u8>, &'static str) {
    let mut best: Option<(Vec<u8>, &'static str)> = None;
    for g in surviving_group(pos) {
        let key = pack(&transform(pos, &g));
        match &best {
            Some((b, _)) if *b <= key => {}
            _ => best = Some((key, g.name)),
        }
    }
    best.expect("the group always contains the identity")
}

/// Decode a packed position. The clock is not in the encoding, so it comes
/// back as zero — this is for checking round trips and for display, not for
/// resuming a game.
pub fn unpack(bytes: &[u8]) -> Result<Position, CodecError> {
    if bytes.len() < 9 {
        return Err(CodecError("packed position is too short".into()));
    }
    let occ = u64::from_le_bytes(bytes[..8].try_into().expect("8 bytes"));
    let n = occ.count_ones() as usize;
    let nibble_bytes = n.div_ceil(2);
    if bytes.len() < 8 + nibble_bytes + 1 {
        return Err(CodecError("packed position ends mid-board".into()));
    }

    let mut pos = Position::empty();
    for (i, sq) in Squares(occ).enumerate() {
        let byte = bytes[8 + i / 2];
        let code = if i % 2 == 0 { byte & 0xF } else { byte >> 4 };
        let (c, p) = decode_code(code)
            .ok_or_else(|| CodecError(format!("piece code {code} is not one of the twelve")))?;
        pos.color_bb[c.idx()] |= 1u64 << sq;
        pos.piece_bb[p.idx()] |= 1u64 << sq;
    }

    // The spare high nibble of the last byte must be zero when the count is
    // odd, or the same position would have sixteen spellings.
    if n % 2 == 1 && bytes[8 + nibble_bytes - 1] >> 4 != 0 {
        return Err(CodecError(
            "padding nibble is not zero — not canonical".into(),
        ));
    }

    let flags = bytes[8 + nibble_bytes];
    pos.side = if flags & 1 == 0 {
        Color::White
    } else {
        Color::Black
    };
    pos.castling = (flags >> 1) & 0xF;
    if flags & (1 << 5) != 0 {
        let f = *bytes
            .get(9 + nibble_bytes)
            .ok_or_else(|| CodecError("ep flag set but no ep byte".into()))?;
        if f > 7 {
            return Err(CodecError(format!("ep file {f} is off the board")));
        }
        let rank = if pos.side == Color::White { 5 } else { 2 };
        pos.ep = Some(rank * 8 + f);
    }
    if flags >> 6 != 0 {
        return Err(CodecError(
            "reserved flag bits are set — not canonical".into(),
        ));
    }
    Ok(pos)
}

/// Convenience: the key as a fixed-width hex string, for use as a map key or
/// in output. Not a hash — the packing itself, which is short enough to carry.
pub fn key_hex(key: &[u8]) -> String {
    let mut s = String::with_capacity(key.len() * 2);
    for b in key {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// A square's name, for display.
pub fn square_name(s: Square) -> String {
    format!(
        "{}{}",
        (b'a' + file_of(s)) as char,
        (b'1' + rank_of(s)) as char
    )
}
