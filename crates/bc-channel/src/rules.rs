//! Which terminal claims a *client* should believe.
//!
//! Kept apart from the state machine because this is the part with an opinion
//! in it. The consensus half — `NO_MOVE`, `clock_of`, `loser_is` — moved to
//! `bc_adjudicator::status` with the dispute machine (`D24`); they are
//! re-exported here so client code reads the same way it did.

pub use bc_adjudicator::status::{clock_of, loser_is, NO_MOVE};

use bc_adjudicator::state::Status;
use bc_chess::terminal::Outcome;
use bc_chess::{Color, Position};

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
