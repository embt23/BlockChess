//! Terminal claims, and the one move that answers them.
//!
//! Eight ways a game ends, split by what the chain has to compute. Six are
//! decided on the spot because six are O(1) — a signature check, a popcount, a
//! field read. Two are not, and those two are where the whole design lives.
//!
//! | Statement | Work |
//! |---|---|
//! | **Claim** mate: ∀ moves, still in check | up to 218 generations and check tests |
//! | **Refute** mate: ∃ a move, not in check | one move application, one check test |
//!
//! Two orders of magnitude, and the expensive side is the one that is usually
//! true and unchallenged — so the expensive computation is almost never
//! performed by anyone. That is `P3`.
//!
//! ## Why optimism is free here
//!
//! The obvious worry: can someone steal a win by falsely claiming mate against
//! an opponent who happens to be offline?
//!
//! No — because that opponent was *already* going to lose. A player who cannot
//! post a refutation within Δ blocks equally cannot post a legal move within Δ
//! blocks, and the timeout rule would have taken the game from them anyway.
//! The claim introduces **no new liveness requirement**; it rides entirely on
//! one the protocol had already made.
//!
//! That is the test to apply to any optimistic mechanism: *does this require
//! the honest party to be online at a time they did not already need to be?*
//! If no, the optimism costs nothing.

use super::{Dispute, DisputeError, PendingClaim};
use crate::dilation::FALSE_CLAIM_PENALTY_DIVISOR;
use crate::state::Status;
use crate::status::loser_is;
use bc_chess::{Color, Move};

/// How a game is claimed to have ended.
///
/// `Timeout` is absent on purpose. `spec/05` lists it as a claim kind, but a
/// timeout is not a claim about anything — it is a statement about block
/// heights that [`Dispute::verdict`] already computes, and which anyone can
/// read off the chain without being told. A variant here would also be the
/// only one whose outcome depends on the board rather than the claimant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimKind {
    /// Signed by the loser. One signature, and nobody lies to lose.
    Resign,
    /// Two signatures over the same ply.
    DrawAgreed,
    /// `halfmove == 100`. A field read.
    FiftyMove,
    /// Popcounts on the bitboards.
    InsufficientMaterial,
    /// Three countersigned states sharing a `rep_hash` at distinct plies.
    Threefold,
    /// **Optimistic.** Refutable by one legal move.
    Checkmate,
    /// **Optimistic.** Refutable by one legal move.
    Stalemate,
}

impl ClaimKind {
    /// Does this claim open a refutation window rather than settle at once?
    pub fn is_optimistic(self) -> bool {
        matches!(self, ClaimKind::Checkmate | ClaimKind::Stalemate)
    }

    /// The result if nobody refutes.
    pub fn uncontested_status(self, claimant: Color) -> Status {
        match self {
            ClaimKind::Resign => loser_is(claimant),
            ClaimKind::Checkmate => loser_is(claimant.flip()),
            _ => Status::Draw,
        }
    }

    /// Who is entitled to make this claim, given whose turn it is.
    ///
    /// Mate is claimed by the player who delivered it; stalemate by the player
    /// who has no move. Both are claims about the side to move.
    fn claimable_by(self, side_to_move: Color) -> Option<Color> {
        match self {
            ClaimKind::Checkmate => Some(side_to_move.flip()),
            ClaimKind::Stalemate => Some(side_to_move),
            _ => None,
        }
    }
}

/// What a successful refutation did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refutation {
    /// A mate claim was struck and the game resumed at the escaping move —
    /// which belonged to the refuter, so it is theirs to choose.
    ResumedAtMove,
    /// A stalemate claim was struck by exhibiting a legal move. The move is
    /// **not** played: it belongs to the claimant, and letting their opponent
    /// pick it would be absurd. They must now actually move.
    ClaimStruck,
}

impl Dispute {
    /// Claim the game is over.
    ///
    /// Returns `Some(status)` when the claim is decided immediately, and
    /// `None` when it is optimistic and a refutation window has opened.
    ///
    /// Signature-backed evidence — `Resign`, `DrawAgreed`, `Threefold` — is
    /// verified by the caller before it gets here; by this point those are
    /// established fact.
    pub fn claim_terminal(
        &mut self,
        kind: ClaimKind,
        claimant: Color,
        height: u64,
    ) -> Result<Option<Status>, DisputeError> {
        if self.claim.is_some() {
            return Err(DisputeError::ClaimPending);
        }
        match kind {
            ClaimKind::FiftyMove if !self.pos.fifty_move_claimable() => {
                return Err(DisputeError::UnsupportedEvidence)
            }
            ClaimKind::InsufficientMaterial if !self.pos.insufficient_material() => {
                return Err(DisputeError::UnsupportedEvidence)
            }
            _ => {}
        }

        if !kind.is_optimistic() {
            return Ok(Some(kind.uncontested_status(claimant)));
        }
        if kind.claimable_by(self.pos.side) != Some(claimant) {
            return Err(DisputeError::NotYourTurn);
        }
        self.claim = Some(PendingClaim {
            kind,
            claimant,
            refutable_until: height + self.delta_blocks as u64,
        });
        Ok(None)
    }

    /// Answer an optimistic claim with one legal move.
    ///
    /// `by` must be the player the claim is against — the one with something
    /// to lose. For a mate claim that is the allegedly mated player; for a
    /// stalemate claim it is their opponent, who is the only person with a
    /// reason to object.
    pub fn refute(&mut self, by: Color, mv: Move, height: u64) -> Result<Refutation, DisputeError> {
        let claim = self.claim.ok_or(DisputeError::NoClaimPending)?;
        if height > claim.refutable_until {
            return Err(DisputeError::RefutationWindowClosed);
        }
        if by != claim.claimant.flip() {
            return Err(DisputeError::NotYourTurn);
        }
        // The whole cost of the honest path: one move application and one
        // check test. The claimant's ∀ is never computed by anybody.
        if !self.pos.is_move_legal(mv) {
            return Err(DisputeError::NotRefutable);
        }

        self.claim = None;
        // A false claim is not free. Halving the budget makes speculative
        // mate claims a losing trade without making honest ones risky.
        let b = &mut self.budget[claim.claimant as usize];
        *b /= FALSE_CLAIM_PENALTY_DIVISOR;

        match claim.kind {
            ClaimKind::Checkmate => {
                // The refuting move is the refuter's own, so play it.
                self.spend(by, height);
                self.pos = self.pos.make_move(mv);
                self.ply += 1;
                self.arm(height);
                Ok(Refutation::ResumedAtMove)
            }
            _ => {
                // A stalemate refutation only exhibits that a move exists.
                self.arm(height);
                Ok(Refutation::ClaimStruck)
            }
        }
    }
}
