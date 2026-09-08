//! Attack tables.
//!
//! Leaping pieces (knight, king, pawn) have attack sets that depend only on
//! their square, so they are pure lookup tables built at compile time.
//!
//! Sliding pieces (bishop, rook, queen) are the hard case, because their attack
//! set depends on what is in the way. We use the "classical" method: precompute
//! the full ray from each square in each direction, AND it with the occupancy
//! to find the blockers, take the nearest one with a bit-scan, and subtract off
//! the ray beyond it.
//!
//! Magic bitboards are faster (one multiply and a table lookup instead of a
//! bit-scan per direction) but they are an optimisation, not a different idea.
//! Correctness first; `perft` will still pass either way, which is exactly why
//! having an exact test makes optimisation safe later.

use crate::bitboard::*;
use crate::types::Color;

/// (file delta, rank delta) for the eight directions, in clockwise order
/// starting north.
const DIRS: [(i8, i8); 8] = [
    (0, 1),   // 0 N
    (1, 1),   // 1 NE
    (1, 0),   // 2 E
    (1, -1),  // 3 SE
    (0, -1),  // 4 S
    (-1, -1), // 5 SW
    (-1, 0),  // 6 W
    (-1, 1),  // 7 NW
];

/// Directions in which ray squares have *higher* index than the origin, so the
/// nearest blocker is the least significant set bit.
const POSITIVE: [bool; 8] = [true, true, true, false, false, false, false, true];

const fn build_rays() -> [[Bitboard; 64]; 8] {
    let mut rays = [[0u64; 64]; 8];
    let mut d = 0;
    while d < 8 {
        let (df, dr) = DIRS[d];
        let mut s = 0usize;
        while s < 64 {
            let mut f = (s % 8) as i8;
            let mut r = (s / 8) as i8;
            let mut bb = 0u64;
            loop {
                f += df;
                r += dr;
                if f < 0 || f > 7 || r < 0 || r > 7 {
                    break;
                }
                bb |= 1u64 << (r * 8 + f) as u8;
            }
            rays[d][s] = bb;
            s += 1;
        }
        d += 1;
    }
    rays
}

/// Build a leaper table from a list of (file, rank) offsets.
const fn build_leaper(offsets: &[(i8, i8)]) -> [Bitboard; 64] {
    let mut t = [0u64; 64];
    let mut s = 0usize;
    while s < 64 {
        let f0 = (s % 8) as i8;
        let r0 = (s / 8) as i8;
        let mut bb = 0u64;
        let mut i = 0;
        while i < offsets.len() {
            let (df, dr) = offsets[i];
            let f = f0 + df;
            let r = r0 + dr;
            if f >= 0 && f <= 7 && r >= 0 && r <= 7 {
                bb |= 1u64 << (r * 8 + f) as u8;
            }
            i += 1;
        }
        t[s] = bb;
        s += 1;
    }
    t
}

pub static RAYS: [[Bitboard; 64]; 8] = build_rays();

pub static KNIGHT_ATTACKS: [Bitboard; 64] = build_leaper(&[
    (1, 2),
    (2, 1),
    (2, -1),
    (1, -2),
    (-1, -2),
    (-2, -1),
    (-2, 1),
    (-1, 2),
]);

pub static KING_ATTACKS: [Bitboard; 64] = build_leaper(&[
    (0, 1),
    (1, 1),
    (1, 0),
    (1, -1),
    (0, -1),
    (-1, -1),
    (-1, 0),
    (-1, 1),
]);

/// `PAWN_ATTACKS[c][s]` = the squares a pawn of colour `c` standing on `s`
/// attacks.
pub static PAWN_ATTACKS: [[Bitboard; 64]; 2] = [
    build_leaper(&[(-1, 1), (1, 1)]),   // white captures forward-up
    build_leaper(&[(-1, -1), (1, -1)]), // black captures forward-down
];

#[inline]
fn ray_attacks(dir: usize, s: u8, occ: Bitboard) -> Bitboard {
    let full = RAYS[dir][s as usize];
    let blockers = full & occ;
    if blockers == 0 {
        return full;
    }
    // Nearest blocker along the ray, then remove everything past it. The
    // blocker square itself stays set — you can capture it.
    let nearest = if POSITIVE[dir] {
        lsb(blockers)
    } else {
        msb(blockers)
    };
    full ^ RAYS[dir][nearest as usize]
}

#[inline]
pub fn bishop_attacks(s: u8, occ: Bitboard) -> Bitboard {
    ray_attacks(1, s, occ)
        | ray_attacks(3, s, occ)
        | ray_attacks(5, s, occ)
        | ray_attacks(7, s, occ)
}

#[inline]
pub fn rook_attacks(s: u8, occ: Bitboard) -> Bitboard {
    ray_attacks(0, s, occ)
        | ray_attacks(2, s, occ)
        | ray_attacks(4, s, occ)
        | ray_attacks(6, s, occ)
}

#[inline]
pub fn queen_attacks(s: u8, occ: Bitboard) -> Bitboard {
    bishop_attacks(s, occ) | rook_attacks(s, occ)
}

#[inline]
pub fn knight_attacks(s: u8) -> Bitboard {
    KNIGHT_ATTACKS[s as usize]
}

#[inline]
pub fn king_attacks(s: u8) -> Bitboard {
    KING_ATTACKS[s as usize]
}

#[inline]
pub fn pawn_attacks(c: Color, s: u8) -> Bitboard {
    PAWN_ATTACKS[c.idx()][s as usize]
}
