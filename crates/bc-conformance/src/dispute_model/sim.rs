//! The state the checker enumerates, and the actions that move it.
//!
//! The state **contains a real [`Dispute`]** — every transition in
//! [`super`] is an actual call into `bc-adjudicator`. What is abstracted is
//! only the environment: which legal move gets tried, and at which of three
//! heights.
//!
//! Transition properties ("budgets never increase") cannot be written as a
//! predicate on a single state, so [`Sim`] carries a ghost flag for each
//! one. Every transition evaluates the property and ands the answer in; a
//! `false` is a counterexample the checker reconstructs a path to.

use bc_adjudicator::dilation::MIN_MOVE_BLOCKS;
use bc_adjudicator::dispute::{ClaimKind, Dispute};
use bc_adjudicator::state::Status;
use bc_chess::Color;
use std::hash::Hasher;

/// Where in the turn the actor chose to act.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum When {
    /// The first block they could possibly act in.
    AsEarlyAsPossible,
    /// The last block they are allowed to act in.
    OnTheDeadline,
    /// One block too late. Must always be rejected.
    OneBlockLate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Act {
    /// The side to move plays a legal move, optionally claiming the game ends
    /// with it. A claim of mate or stalemate is **not** checked against the
    /// board — that is `P3`, and the false claim is the interesting case.
    Move {
        idx: u8,
        when: When,
        claim: Option<Claim>,
    },
    /// The player a claim is against exhibits one legal move.
    Refute { idx: u8, when: When },
    /// Nobody acts and the clock runs out.
    LetTheClockRun,
}

/// `ClaimKind` without the variants whose evidence is a signature the
/// adjudicator never sees.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Claim {
    Checkmate,
    Stalemate,
    Resign,
}

impl From<Claim> for ClaimKind {
    fn from(c: Claim) -> ClaimKind {
        match c {
            Claim::Checkmate => ClaimKind::Checkmate,
            Claim::Stalemate => ClaimKind::Stalemate,
            Claim::Resign => ClaimKind::Resign,
        }
    }
}

/// Everything that distinguishes one reachable state from another.
///
/// `Dispute` is not `Hash`, and giving it a derive would put a consensus
/// type's identity in the hands of a test. So the projection lives here:
/// the packed position, the ply, both budgets, the deadline, the height, the
/// settlement, the step count, and the pending claim.
#[derive(PartialEq, Eq, Hash)]
struct Key(
    Vec<u8>,
    u16,
    [u32; 2],
    u64,
    u64,
    Option<u8>,
    u8,
    Option<(u8, u8, u64)>,
);

/// The dispute, the chain height, and the ghost variables the safety
/// properties are read from.
///
/// Transition properties ("budgets never increase") cannot be expressed as a
/// predicate on one state, so each transition evaluates them and records the
/// answer. A `false` here is a counterexample the checker will reconstruct a
/// path to.
#[derive(Clone, Debug)]
pub struct Sim {
    pub d: Dispute,
    pub now: u64,
    pub settled: Option<Status>,
    pub steps: u8,

    // ghosts
    pub budgets_never_rose: bool,
    pub ply_never_fell: bool,
    pub settlement_stayed_put: bool,
    pub rank_strictly_fell: bool,
    pub deadline_was_meetable: bool,
    pub late_action_was_rejected: bool,

    // coverage
    pub saw_timeout: bool,
    pub saw_claim_stand: bool,
    pub saw_refutation: bool,
    pub saw_ply_cap: bool,
    pub saw_immediate_settlement: bool,
}

impl Sim {
    /// The termination measure. Every transition must strictly decrease it,
    /// and it is bounded below, so no infinite run exists. This is the
    /// formal content of `spec/05`'s "budgets only ever decrease, so the
    /// process terminates" — and writing it down is what showed that the
    /// English sentence was not quite true, because budgets are not the only
    /// thing moving.
    ///
    /// The ply term carries weight 2 for a reason worth keeping. A move that
    /// *opens* a claim spends a ply (down) and adds a pending claim (up); at
    /// weight 1 those cancel exactly and the measure stalls. Two plies are
    /// worth more than one claim because a claim can only be opened by
    /// spending a ply, and the ply is never refunded.
    pub fn rank(&self) -> u64 {
        self.d.budget_of(Color::White) as u64
            + self.d.budget_of(Color::Black) as u64
            + 2 * (self.d.max_plies().saturating_sub(self.d.ply)) as u64
            + u64::from(self.d.claim.is_some())
    }

    /// Evaluate every transition property against the state this one came
    /// from, and record the answers.
    ///
    /// Each ghost is `previous && holds_now`, so a violation anywhere on a
    /// path is still visible at the end of it — the checker then walks back
    /// and prints the path that got there.
    pub fn observe(&mut self, last: &Sim) {
        self.budgets_never_rose = last.budgets_never_rose
            && self.d.budget_of(Color::White) <= last.d.budget_of(Color::White)
            && self.d.budget_of(Color::Black) <= last.d.budget_of(Color::Black);

        self.ply_never_fell = last.ply_never_fell && self.d.ply >= last.d.ply;

        self.settlement_stayed_put = last.settlement_stayed_put
            && match (last.settled, self.settled) {
                (Some(a), Some(b)) => a == b,
                (Some(_), None) => false,
                _ => true,
            };

        // A settling step may leave the measure alone: the game is over,
        // there is no next step, and nothing can loop.
        self.rank_strictly_fell =
            last.rank_strictly_fell && (self.settled.is_some() || self.rank() < last.rank());

        // Whoever must respond now must be able to, *if they have anything
        // left to respond with*.
        //
        // This property used to read `window <= budget && window > 0`, and
        // it was too weak in the way that mattered: a one-block window is
        // nonzero and satisfies it, and a one-block window is two seconds
        // to get a transaction mined. It is what `build-log` §18 was, and
        // the model agreed with the bug for as long as the property allowed
        // it to. A model check cannot tell you your properties are wrong.
        //
        // What is actually required:
        //
        // - No budget left → no window. That is a loss on time, correctly.
        // - Budget left → a window a move can physically be made in, which
        //   means at least what a move is *charged*: `MIN_MOVE_BLOCKS`.
        // - And never more than Δ, except where the floor above raises it,
        //   or a player could sit past a window their terms never agreed.
        //
        // The old `window <= budget` clause is gone on purpose: the floor
        // deliberately breaks it, and nothing is lost, because budgets
        // decreasing monotonically is a separate property already checked.
        let side = self.d.side_to_move();
        let window = self.d.deadline_block.saturating_sub(self.d.turn_started);
        let budget = self.d.budget_of(side) as u64;
        let delta = self.d.delta_blocks() as u64;
        let meetable = if budget == 0 {
            window == 0
        } else {
            window >= MIN_MOVE_BLOCKS && window <= delta.max(MIN_MOVE_BLOCKS)
        };
        self.deadline_was_meetable =
            last.deadline_was_meetable && (self.settled.is_some() || meetable);
    }

    fn key(&self) -> Key {
        Key(
            self.d.pos.pack().as_slice().to_vec(),
            self.d.ply,
            [
                self.d.budget_of(Color::White),
                self.d.budget_of(Color::Black),
            ],
            self.d.deadline_block,
            self.now,
            self.settled.map(|s| s as u8),
            self.steps,
            self.d
                .claim
                .map(|c| (c.kind as u8, c.claimant as u8, c.refutable_until)),
        )
    }
}

impl PartialEq for Sim {
    fn eq(&self, other: &Sim) -> bool {
        self.key() == other.key()
    }
}
impl Eq for Sim {}
impl std::hash::Hash for Sim {
    fn hash<H: Hasher>(&self, h: &mut H) {
        self.key().hash(h)
    }
}
