//! Reading the outcome off the heights.
//!
//! Separate from `mod.rs` because it answers a different question.
//! `apply_move` and `refute` *change* a dispute; this only reports what the
//! block heights already imply, and it is the one place a settlement is
//! decided — which is why the ply cap lives here rather than only in the
//! functions that happen to add a ply (`docs/build-log.md` §16).

use super::{Dispute, DisputeError};
use crate::state::Status;
use crate::status::loser_is;

impl Dispute {
    /// The verdict, if the clock has run out on somebody.
    ///
    /// Callable by anyone — there is no reason to restrict it, since it only
    /// reports what the heights already imply.
    pub fn verdict(&self, height: u64) -> Result<Status, DisputeError> {
        // The cap is checked here and not only where plies are added,
        // because `refute` adds one too and used not to look — see
        // `docs/build-log.md` §16. A bound that each caller has to remember
        // to apply is not a bound.
        if self.ply >= self.max_plies {
            return Ok(Status::Draw);
        }
        if let Some(c) = self.claim {
            return if height > c.refutable_until {
                // Unrefuted within the window, so it stands.
                Ok(c.kind.uncontested_status(c.claimant))
            } else {
                Err(DisputeError::NotYetDecided)
            };
        }
        if height > self.deadline_block {
            // The side to move did not move. That is a loss on time, and it
            // is the same rule whether they crashed or chose to vanish — the
            // chain cannot tell the difference and does not need to.
            Ok(loser_is(self.pos.side))
        } else {
            Err(DisputeError::NotYetDecided)
        }
    }
}
