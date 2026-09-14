//! Episode 03 — the rules of chess, as code.
//!
//! The attack this defeats: *"that move was legal, trust me."*
//!
//! This crate is the referee. It is used twice: in the client, thousands of
//! times per game, and inside the on-chain adjudicator when a game is disputed.
//! Those two must agree exactly, so there is exactly one implementation and it
//! lives here. See `spec/03-position.md`.

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
