//! E7 — the move's index among the legal moves, mixed-radix coded.
//!
//! See the crate docs for why this is a mixed radix rather than an arithmetic
//! coder. The short version: with a uniform model over `b` options they are
//! the same code, and this one has no carry logic to get wrong.

use bc_chess::{Move, Position};

use crate::bignum::Big;
use crate::order::canonical_moves;
use crate::CodecError;

pub fn encode(start: &Position, moves: &[Move]) -> Result<Vec<u8>, CodecError> {
    // Forward pass: replay the game, recording each move's index and how many
    // options it was chosen from. This is also where an illegal move is
    // caught, which is the encoder refusing to store a game that is not a
    // game.
    let mut pos = *start;
    let mut digits: Vec<(u32, u32)> = Vec::with_capacity(moves.len());
    for (i, &m) in moves.iter().enumerate() {
        let legal = canonical_moves(&pos);
        let Some(idx) = legal.iter().position(|&x| x == m) else {
            return Err(CodecError(format!(
                "ply {}: {} is not legal in {}",
                i + 1,
                m.to_uci(),
                pos.to_fen()
            )));
        };
        digits.push((idx as u32, legal.len() as u32));
        pos = pos.make_move(m);
    }

    // Reverse fold, so that the decoder peels the *first* ply off first.
    let mut n = Big::zero();
    for &(d, b) in digits.iter().rev() {
        n.mul_add(b, d);
    }
    Ok(n.to_bytes())
}

pub fn decode(start: &Position, bytes: &[u8], ply_count: usize) -> Result<Vec<Move>, CodecError> {
    let mut n = Big::from_bytes(bytes);
    let mut pos = *start;
    let mut out = Vec::with_capacity(ply_count);

    for i in 0..ply_count {
        let legal = canonical_moves(&pos);
        if legal.is_empty() {
            return Err(CodecError(format!(
                "ply {}: no legal moves in {}, but {} plies were claimed",
                i + 1,
                pos.to_fen(),
                ply_count
            )));
        }
        let d = n.div_rem(legal.len() as u32) as usize;
        let m = legal[d];
        out.push(m);
        pos = pos.make_move(m);
    }

    // The integer must be exhausted. Anything left over means the byte string
    // encoded something other than this many plies from this position, and
    // silently ignoring it would make the encoding non-canonical -- two byte
    // strings decoding to the same game. See bignum.rs.
    if !n.is_zero() {
        return Err(CodecError(
            "trailing data after the last ply: this is not a canonical encoding".into(),
        ));
    }
    Ok(out)
}
