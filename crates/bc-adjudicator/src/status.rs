//! Reading a status off a colour, and the one transition that is not a move.
//!
//! Three lines of consensus vocabulary, here rather than in `bc-channel`
//! because the dispute machine needs them and must not reach back across the
//! boundary to get them.

use crate::state::{GameState, Status};
use bc_chess::Color;

/// The sentinel `mv` for a state that advances the ply without a move: a
/// self-declared flag. Only ever terminal, and only ever against the player
/// who sent it.
pub const NO_MOVE: u16 = 0;

pub fn clock_of(s: &GameState, c: Color) -> u32 {
    match c {
        Color::White => s.clock_w_ms,
        Color::Black => s.clock_b_ms,
    }
}

/// The status in which `c` has lost.
pub fn loser_is(c: Color) -> Status {
    match c {
        Color::White => Status::BlackWins,
        Color::Black => Status::WhiteWins,
    }
}
