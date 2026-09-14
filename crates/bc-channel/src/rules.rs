//! Which status a state may claim, and when a receiver should believe it.
//!
//! Kept apart from the state machine because this is the part with an opinion
//! in it. Everything in `game.rs` is mechanical; everything here is policy,
//! and policy is what changes.

use crate::state::{GameState, Status};
use bc_chess::terminal::Outcome;
use bc_chess::{Color, Position};

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

pub fn loser_is(c: Color) -> Status {
    match c {
        Color::White => Status::BlackWins,
        Color::Black => Status::WhiteWins,
    }
}

/// The status a mover may claim for the position they just reached.
///
/// Claiming nothing is always allowed: a player who overlooks a draw by
/// repetition has merely overlooked it, and the position is still there to
/// claim from next ply.
pub fn claimable_status(after: &Position, repetitions: usize) -> Status {
    if repetitions >= 3 {
        return Status::Draw;
    }
    match after.outcome() {
        Some(Outcome::Checkmate { winner }) => match winner {
            Color::White => Status::WhiteWins,
            Color::Black => Status::BlackWins,
        },
        Some(_) => Status::Draw,
        None => Status::Ongoing,
    }
}

/// Is a terminal claim by `mover` one the receiver should countersign?
///
/// Two ways to be justified. Either the position says so, or the mover is
/// declaring *their own* loss — and nobody lies to lose, so a self-declared
/// loss needs no checking at all. That second case is what makes resignation
/// and the self-declared flag free.
///
/// Note what this is not. `P3` says the **chain** never verifies mate: it
/// accepts the claim and waits to be shown one refuting move. A client is in
/// a different position — the computation is free off-chain and the money at
/// stake is its own — so here the full check is the cheap one. The two are
/// not in tension; they are the same asymmetry seen from the two sides.
pub fn justified(claimed: Status, after: &Position, repetitions: usize, mover: Color) -> bool {
    claimed == Status::Ongoing
        || claimed == loser_is(mover)
        || claimed == claimable_status(after, repetitions)
}
