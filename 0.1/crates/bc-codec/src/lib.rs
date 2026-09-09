//! Turning games into bytes, and back.
//!
//! Two encodings, so that every claim about one is measured against the other
//! rather than asserted:
//!
//! | | scheme | what it does | cost |
//! |---|---|---|---|
//! | **E3** | `Fixed` | the 16-bit packed move, written out | 16 bits/ply |
//! | **E7** | `Index` | the move's index among the legal moves, mixed-radix coded | `log2 b` bits/ply, exactly |
//!
//! E3 needs no chess knowledge to decode. E7 needs the full rules, and pays
//! for them with roughly a 3× saving. `papers/02-encodings.md` scores both.
//!
//! ## Why mixed radix rather than an arithmetic coder
//!
//! E7 has to spend `log2 b` bits on a choice between `b` equally-treated
//! options. Rounding that up to whole bits wastes ~0.5 bits per ply, which is
//! 10% of the payload, so the rounding has to go.
//!
//! An arithmetic coder is the usual answer. It is also several hundred lines
//! with famously subtle carry handling. But when every model is *uniform* over
//! its alphabet, arithmetic coding degenerates into something much simpler and
//! exactly optimal: interpret the whole game as one integer in a mixed radix,
//!
//! ```text
//! N = ((… d₃)·b₂ + d₂)·b₁ + d₁
//! ```
//!
//! where `dᵢ` is the move index at ply `i` and `bᵢ` the number of legal moves
//! there. Encoding folds the plies in reverse; decoding peels them off the
//! front with `d = N mod b; N /= b`, and `b` is always known because the
//! decoder has already replayed every earlier move.
//!
//! The output is `⌈log2 ∏ bᵢ⌉` bits — the information-theoretic floor for
//! this model, with no rounding waste anywhere except once at the end of the
//! whole game. It needs no probability tables, no renormalisation, and no
//! carry logic; it needs arbitrary-precision multiply-and-add, which is
//! `bignum.rs` and is forty lines.
//!
//! The cost is that a game must be encoded and decoded whole rather than
//! streamed. At eighty plies that is not a cost.

pub mod bignum;
pub mod order;
pub mod position;
pub mod rank;

mod fixed;
mod index;

pub use fixed::{decode as decode_fixed, encode as encode_fixed};
pub use index::{decode as decode_index, encode as encode_index};

use bc_chess::{Move, Position};

/// Which encoding a byte string is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheme {
    /// E3 — 16 bits per ply, decodable without the rules.
    Fixed,
    /// E7 — `log2 b` bits per ply, decodable only under the rules.
    Index,
}

impl Scheme {
    pub fn name(self) -> &'static str {
        match self {
            Scheme::Fixed => "E3 fixed 16-bit",
            Scheme::Index => "E7 legal index",
        }
    }
}

#[derive(Debug)]
pub struct CodecError(pub String);

impl std::fmt::Display for CodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for CodecError {}

/// Encode a move sequence from a starting position.
pub fn encode(scheme: Scheme, start: &Position, moves: &[Move]) -> Result<Vec<u8>, CodecError> {
    match scheme {
        Scheme::Fixed => Ok(encode_fixed(moves)),
        Scheme::Index => encode_index(start, moves),
    }
}

/// Decode `ply_count` moves. The count is stored by the container, not by the
/// payload: a mixed-radix integer does not know where it ends, and inventing a
/// terminator would cost more than the two bytes a length costs.
pub fn decode(
    scheme: Scheme,
    start: &Position,
    bytes: &[u8],
    ply_count: usize,
) -> Result<Vec<Move>, CodecError> {
    match scheme {
        Scheme::Fixed => decode_fixed(bytes, ply_count),
        Scheme::Index => decode_index(start, bytes, ply_count),
    }
}

/// The exact size in bits that E7 will spend, without encoding anything.
///
/// This is `Σ log2 bᵢ`, the quantity `papers/02-encodings.md` calls
/// `E[log2 b]` when averaged over plies. Reported by `blockchess measure`.
pub fn index_bits(start: &Position, moves: &[Move]) -> f64 {
    let mut pos = *start;
    let mut bits = 0.0;
    for &m in moves {
        let n = order::canonical_moves(&pos).len();
        bits += (n as f64).log2();
        pos = pos.make_move(m);
    }
    bits
}
