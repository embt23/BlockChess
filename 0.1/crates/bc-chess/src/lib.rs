//! The rules of chess, as code.
//!
//! Lifted unchanged from version 0.0, where it was written from the Chess
//! Programming Wiki and verified against the published perft counts:
//! `perft(6) = 119,060,324` and Kiwipete `perft(5) = 193,690,690`, both exact.
//! Those tests came with it and still run.
//!
//! It is here rather than rewritten because it is *the rules of chess*, not
//! architecture. It was checked against an oracle somebody else published,
//! which is the standard this project holds itself to, and rewriting it would
//! buy nothing but a month.
//!
//! ## This crate defines the encoding
//!
//! `generate_legal` returns moves in a fixed order, and `bc-codec` encodes a
//! move as *its index in that list*. So the order is not an implementation
//! detail — it is part of the file format, and changing it silently changes
//! what every previously encoded game decodes to. See
//! `papers/04-permanence.md` §5, and the golden vectors in
//! `crates/bc-codec/tests/enumeration_order.rs` that pin it.

pub mod attacks;
pub mod bitboard;
pub mod movegen;
pub mod perft;
pub mod position;
pub mod types;

pub use bitboard::Bitboard;
pub use perft::{divide, perft};
pub use position::{FenError, Position};
pub use types::{Color, Move, MoveList, Piece, Square};
