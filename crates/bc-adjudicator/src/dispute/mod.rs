//! Episode 08 — the adjudicator.
//!
//! The attack this defeats: *"I'll just stop replying."*
//!
//! ## The single rule
//!
//! > **On-chain, the game continues under the same rules, slowly.**
//!
//! This is not a dispute-resolution mechanism with logic of its own. It is the
//! same chess game at block speed, with the chain as referee. If your opponent
//! stops responding at ply 37, you post ply 37 and the game resumes at ply 38.
//!
//! That framing deletes an enormous amount of machinery. There is no arbitration,
//! no weighing of evidence, no appeal — only: *it is your move, here is the
//! board, move or forfeit.* Everything hard about disputes in other protocols is
//! hard because they try to decide **who was right**. This one never asks.
//!
//! ## The three things that make it work
//!
//! 1. **You post a state your opponent signed.** A state you signed yourself
//!    proves nothing. Because countersignatures ride along with moves
//!    (`spec/04`), the player who is *waiting* — the one who needs evidence —
//!    always holds one.
//! 2. **Clock dilation** ([`crate::clock::budget_blocks`]) changes the time base
//!    without changing the ratio, so deadlines become physically achievable
//!    without handing a flagging player their time back.
//! 3. **Higher ply wins.** A newer state overrides an older one and resets the
//!    window. Budgets only ever decrease, so the process terminates.
//!
//! The position goes on-chain **in plaintext** here. That is a deliberate v1
//! privacy cost: disputes are rare, and the alternative is a research project
//! (`spec/08`).

mod claim;

pub use claim::{ClaimKind, Refutation};
pub use PendingClaim as Claim;

use crate::dilation::{blocks_consumed, budget_blocks};
use crate::state::{GameState, Status};
use crate::status::loser_is;
use crate::terms::GameTerms;
use bc_chess::{Color, Move, Position};
use bc_hash::Hash;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisputeError {
    NotOngoing,
    PositionMismatch,
    NotYourTurn,
    DeadlinePassed,
    IllegalMove,
    ClaimPending,
    NoClaimPending,
    NotRefutable,
    RefutationWindowClosed,
    StaleState,
    UnsupportedEvidence,
    NotYetDecided,
}

impl core::fmt::Display for DisputeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}
#[cfg(feature = "std")]
impl std::error::Error for DisputeError {}

/// An optimistic claim, waiting out its refutation window.
///
/// Only mate and stalemate ever get here. Everything else in [`ClaimKind`] is
/// decided on the spot, because everything else is O(1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingClaim {
    pub kind: ClaimKind,
    pub claimant: Color,
    pub refutable_until: u64,
}

/// A game being played out on-chain.
#[derive(Debug, Clone)]
pub struct Dispute {
    pub channel_id: Hash,
    pub ply: u16,
    /// In plaintext. On a real chain this is the 26 packed bytes; here it is
    /// already unpacked, which is the same information.
    pub pos: Position,
    pub clock_ms: [u32; 2],
    /// Remaining on-chain blocks, indexed by colour.
    pub budget: [u32; 2],
    /// The current responder must act by this height.
    pub deadline_block: u64,
    pub claim: Option<PendingClaim>,
    pub initiator: Color,
    /// Height at which the current responder's turn began, for metering.
    pub turn_started: u64,
    delta_blocks: u32,
    max_plies: u16,
}

/// What a successful on-chain move did to the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveOutcome {
    /// The game continues; it is now the other player's turn.
    Continues,
    /// The move carried an optimistic claim, and its refutation window is
    /// open. Nothing is settled yet.
    Claimed,
    /// The game is over: an immediately-decidable claim, or the ply cap.
    Ended(Status),
}

impl Dispute {
    /// Open a dispute from a state the **opponent** signed.
    ///
    /// The caller checks the signature and that the packed position matches
    /// `state.pos_hash`; by the time we are here, `pos` is established fact.
    pub fn open(
        state: &GameState,
        pos: Position,
        terms: &GameTerms,
        initiator: Color,
        height: u64,
    ) -> Result<Dispute, DisputeError> {
        if state.status != Status::Ongoing {
            return Err(DisputeError::NotOngoing);
        }
        let tau = terms.budget_tau_ms();
        let mut d = Dispute {
            channel_id: state.channel_id,
            ply: state.ply,
            pos,
            clock_ms: [state.clock_w_ms, state.clock_b_ms],
            budget: [
                budget_blocks(state.clock_w_ms, tau),
                budget_blocks(state.clock_b_ms, tau),
            ],
            deadline_block: 0,
            claim: None,
            initiator,
            turn_started: height,
            delta_blocks: terms.delta_blocks(),
            max_plies: terms.max_plies,
        };
        d.arm(height);
        Ok(d)
    }

    /// Replace this dispute with one built from a strictly higher state.
    ///
    /// This is `P2` — higher ply wins — as a single comparison. A player who
    /// posts a stale state is simply overridden, and because each override
    /// resets the window while budgets only decrease, the process terminates.
    pub fn supersede(
        &mut self,
        state: &GameState,
        pos: Position,
        terms: &GameTerms,
        initiator: Color,
        height: u64,
    ) -> Result<(), DisputeError> {
        if state.ply <= self.ply {
            return Err(DisputeError::StaleState);
        }
        *self = Dispute::open(state, pos, terms, initiator, height)?;
        Ok(())
    }

    pub fn side_to_move(&self) -> Color {
        self.pos.side
    }

    pub fn budget_of(&self, c: Color) -> u32 {
        self.budget[c as usize]
    }

    /// Set the deadline for whoever must respond now: bounded both per-move
    /// by Δ and in total by what is left of their budget.
    fn arm(&mut self, height: u64) {
        let side = self.pos.side;
        let window = self.delta_blocks.min(self.budget[side as usize]) as u64;
        self.deadline_block = height + window;
        self.turn_started = height;
    }

    fn spend(&mut self, side: Color, height: u64) {
        let cost = blocks_consumed(self.turn_started, height);
        let b = &mut self.budget[side as usize];
        *b = b.saturating_sub(cost.min(u32::MAX as u64) as u32);
    }

    /// Play a move on-chain. This is the only place the adjudicator's chess
    /// engine ever runs.
    /// Play a move on-chain, optionally claiming the game ends with it.
    ///
    /// **`P3` lives here.** An earlier version detected mate and stalemate
    /// itself, by calling `Position::outcome()` after every move — which
    /// generates all ~218 legal moves, which is precisely the ∀ the invariant
    /// exists to keep off the chain. It returned correct answers and every
    /// test passed; it was the *cost* that was wrong, and on a real chain the
    /// difference is one check test against two hundred and eighteen move
    /// generations, per move, for every disputed game. `docs/build-log.md`
    /// §12.
    ///
    /// So the chain no longer looks. A player who has just delivered mate
    /// says so — `claim` rides along with the move, exactly as `spec/05`'s
    /// `DisputeMove { …, [new_status] }` always allowed — and the claim is
    /// optimistic and refutable like any other. Claiming nothing is always
    /// allowed; the game simply continues.
    pub fn apply_move(
        &mut self,
        mover: Color,
        mv: Move,
        height: u64,
        claim: Option<ClaimKind>,
    ) -> Result<MoveOutcome, DisputeError> {
        if self.claim.is_some() {
            return Err(DisputeError::ClaimPending);
        }
        if mover != self.pos.side {
            return Err(DisputeError::NotYourTurn);
        }
        if height > self.deadline_block {
            return Err(DisputeError::DeadlinePassed);
        }
        // An illegal move is rejected and the transaction reverts. The clock
        // keeps running, so submitting garbage costs gas *and* time.
        if !self.pos.is_move_legal(mv) {
            return Err(DisputeError::IllegalMove);
        }

        self.spend(mover, height);
        self.pos = self.pos.make_move(mv);
        self.ply += 1;
        self.arm(height);

        // The ply cap is an integer comparison, not a search, so it stays.
        // It is what bounds the worst case a validator must afford.
        if self.ply >= self.max_plies {
            return Ok(MoveOutcome::Ended(Status::Draw));
        }
        match claim {
            None => Ok(MoveOutcome::Continues),
            Some(kind) => match self.claim_terminal(kind, mover, height)? {
                Some(status) => Ok(MoveOutcome::Ended(status)),
                None => Ok(MoveOutcome::Claimed),
            },
        }
    }

    /// The verdict, if the clock has run out on somebody.
    ///
    /// Callable by anyone — there is no reason to restrict it, since it only
    /// reports what the heights already imply.
    pub fn verdict(&self, height: u64) -> Result<Status, DisputeError> {
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
