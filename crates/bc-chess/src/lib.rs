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
pub mod pack;
pub mod perft;
pub mod position;
pub mod san;
pub mod terminal;
pub mod types;
pub mod uci;

pub use bitboard::Bitboard;
pub use pack::{unpack, PackError, Packed, MAX_PACKED_LEN};
pub use perft::{divide, perft};
pub use position::{FenError, Position};
pub use san::{parse_movetext, parse_san};
pub use terminal::Outcome;
pub use types::{Color, Move, MoveList, Piece, Square};
