//! D23, the half that has no oracle: the dispute machine, model-checked.
//!
//! Clock dilation, block budgets, `deadline_block`, override-by-higher-ply
//! and the false-claim penalty have no external referent anywhere, because
//! they are mechanisms this project invented. Nobody publishes vectors for
//! them and there is no second implementation to differ against. So this half
//! gets the other standard: the properties `spec/05` argues **in prose** are
//! stated as predicates and checked over every reachable state.
//!
//! This is deliberately not called an oracle. A differential test can tell
//! you that you are wrong about chess. A model check can only tell you that
//! the machine satisfies the properties you wrote down — if the properties
//! are wrong it will agree with you enthusiastically. What it buys is the
//! word *every*: `spec/05` says "budgets only decrease, so the process
//! terminates", and that sentence is a proof obligation written in English.
//!
//! ## It drives the real code
//!
//! The state here **contains a real [`Dispute`]** and every transition is a
//! real call to `apply_move`, `refute`, `claim_terminal` or `verdict`. This
//! matters: a hand-written abstract model would prove things about the
//! abstraction, and the abstraction is exactly where a re-derivation can
//! quietly diverge from the shipped code. What is abstracted is only the
//! *environment* — which moves get tried and at which heights — never the
//! machine under test.
//!
//! ## What is bounded, and why that is honest
//!
//! Exhaustive means exhaustive over a finite space, so the space is made
//! finite on purpose:
//!
//! - **Clocks near zero.** A flagging player's budget is [`FLOOR_BLOCKS`],
//!   which is small. This is not only tractable, it is the interesting
//!   region: deadline arithmetic that is wrong is wrong at the bottom.
//! - **At most [`BRANCH`] legal moves are tried** from any position. The
//!   properties under test are about budgets, plies and deadlines; which
//!   legal move is played changes none of them. Chess correctness is the
//!   differential test's job, next door.
//! - **Three heights per turn**: as early as possible, exactly on the
//!   deadline, and one block late. Every off-by-one that matters is at a
//!   boundary, and those are the boundaries.
//! - **A small ply cap**, so the cap itself is reachable.

/// How many legal moves are explored from any position.
pub const BRANCH: usize = 2;
/// Hard stop on path length, so the checker is guaranteed to finish even if
/// a future change makes the ranking function stop decreasing.
pub const MAX_STEPS: u8 = 10;

use bc_adjudicator::dispute::{Dispute, MoveOutcome};
use bc_adjudicator::state::{GameState, Status};
use bc_adjudicator::terms::GameTerms;
use bc_chess::{Color, Position};
use bc_hash::ZERO_HASH;
use stateright::{Model, Property};

mod sim;

pub use sim::{Act, Claim, Sim, When};

// ---------------------------------------------------------------- the model

pub struct DisputeModel {
    terms: GameTerms,
    start: Position,
    clock_ms: u32,
}

impl DisputeModel {
    pub fn new(fen: &str, clock_ms: u32, max_plies: u16) -> DisputeModel {
        let start = Position::from_fen(fen).expect("model start position");
        let mut terms = GameTerms::for_clock([0u8; 32], clock_ms.max(1), 0);
        terms.max_plies = max_plies;
        DisputeModel {
            terms,
            start,
            clock_ms,
        }
    }

    /// The height an actor acts at, given the deadline they are working to.
    fn height(&self, s: &Sim, deadline: u64, when: When) -> u64 {
        let earliest = s.now.max(s.d.turn_started + 1);
        match when {
            When::AsEarlyAsPossible => earliest.min(deadline),
            When::OnTheDeadline => deadline.max(earliest),
            When::OneBlockLate => deadline + 1,
        }
    }
}

/// A dispute machine step that settled the game, or did not.
fn settle_if_decided(s: &mut Sim) {
    if s.settled.is_none() {
        if let Ok(v) = s.d.verdict(s.now) {
            s.settled = Some(v);
        }
    }
}

impl Model for DisputeModel {
    type State = Sim;
    type Action = Act;

    fn init_states(&self) -> Vec<Sim> {
        let state = GameState {
            channel_id: [0u8; 32],
            ply: 0,
            prev_hash: [0u8; 32],
            mv: 0,
            pos_hash: ZERO_HASH,
            clock_w_ms: self.clock_ms,
            clock_b_ms: self.clock_ms,
            status: Status::Ongoing,
        };
        let d = Dispute::open(&state, self.start, &self.terms, Color::White, 100)
            .expect("the model's opening dispute");
        vec![Sim {
            d,
            now: 100,
            settled: None,
            steps: 0,
            budgets_never_rose: true,
            ply_never_fell: true,
            settlement_stayed_put: true,
            rank_strictly_fell: true,
            deadline_was_meetable: true,
            late_action_was_rejected: true,
            saw_timeout: false,
            saw_claim_stand: false,
            saw_refutation: false,
            saw_ply_cap: false,
            saw_immediate_settlement: false,
        }]
    }

    fn actions(&self, s: &Sim, out: &mut Vec<Act>) {
        if s.settled.is_some() || s.steps >= MAX_STEPS {
            return;
        }
        const WHENS: [When; 3] = [
            When::AsEarlyAsPossible,
            When::OnTheDeadline,
            When::OneBlockLate,
        ];
        if s.d.claim.is_some() {
            for when in WHENS {
                for idx in 0..BRANCH as u8 {
                    out.push(Act::Refute { idx, when });
                }
            }
        } else {
            for when in WHENS {
                for idx in 0..BRANCH as u8 {
                    for claim in [
                        None,
                        Some(Claim::Checkmate),
                        Some(Claim::Stalemate),
                        Some(Claim::Resign),
                    ] {
                        out.push(Act::Move { idx, when, claim });
                    }
                }
            }
        }
        out.push(Act::LetTheClockRun);
    }

    fn next_state(&self, last: &Sim, act: Act) -> Option<Sim> {
        let mut s = last.clone();
        s.steps += 1;

        match act {
            Act::LetTheClockRun => {
                // Run past whichever window is open.
                let open = match last.d.claim {
                    Some(c) => c.refutable_until,
                    None => last.d.deadline_block,
                };
                if s.now > open {
                    return None; // already past it; not a new state
                }
                s.now = open + 1;
                let had_claim = last.d.claim.is_some();
                settle_if_decided(&mut s);
                if s.settled.is_some() {
                    if had_claim {
                        s.saw_claim_stand = true;
                    } else {
                        s.saw_timeout = true;
                    }
                }
            }

            Act::Move { idx, when, claim } => {
                let mover = last.d.side_to_move();
                let legal = last.d.pos.generate_legal();
                let mv = *legal.as_slice().get(idx as usize)?;
                let at = self.height(last, last.d.deadline_block, when);
                s.now = at;
                match s.d.apply_move(mover, mv, at, claim.map(Into::into)) {
                    Ok(MoveOutcome::Continues) => {}
                    Ok(MoveOutcome::Claimed) => {}
                    Ok(MoveOutcome::Ended(st)) => {
                        s.settled = Some(st);
                        if last.d.ply + 1 >= last.d.max_plies() {
                            s.saw_ply_cap = true;
                        } else {
                            s.saw_immediate_settlement = true;
                        }
                    }
                    Err(_) => {
                        // The transaction reverted. The chain height still
                        // moved, and that is the point: a rejected move costs
                        // gas and time but changes no state, so it must not
                        // be modelled as a no-op that resets anything.
                        // A late move being refused is the expected case;
                        // the property below only fires when one is accepted.
                        return None;
                    }
                }
                if when == When::OneBlockLate {
                    // It was accepted, and it should not have been.
                    s.late_action_was_rejected = false;
                }
                settle_if_decided(&mut s);
            }

            Act::Refute { idx, when } => {
                let claim = last.d.claim?;
                let by = claim.claimant.flip();
                let legal = last.d.pos.generate_legal();
                let mv = *legal.as_slice().get(idx as usize)?;
                let at = self.height(last, claim.refutable_until, when);
                s.now = at;
                match s.d.refute(by, mv, at) {
                    Ok(_) => {
                        s.saw_refutation = true;
                        if when == When::OneBlockLate {
                            s.late_action_was_rejected = false;
                        }
                    }
                    Err(_) => return None,
                }
                settle_if_decided(&mut s);
            }
        }

        s.observe(last);

        Some(s)
    }

    fn within_boundary(&self, s: &Sim) -> bool {
        s.steps <= MAX_STEPS
    }

    fn properties(&self) -> Vec<Property<Self>> {
        vec![
            // --- the four `spec/05` argues in prose -----------------------
            Property::always("budgets only ever decrease", |_: &DisputeModel, s: &Sim| {
                s.budgets_never_rose
            }),
            Property::always("ply never goes backwards", |_: &DisputeModel, s: &Sim| {
                s.ply_never_fell
            }),
            Property::always(
                "the pot is assigned exactly once",
                |_: &DisputeModel, s: &Sim| {
                    s.settlement_stayed_put && s.settled != Some(Status::Ongoing)
                },
            ),
            Property::always(
                "the measure falls, so the process terminates",
                |_: &DisputeModel, s: &Sim| s.rank_strictly_fell,
            ),
            // --- and two the prose assumes without saying -----------------
            Property::always(
                "no player is given a deadline they cannot meet",
                |_: &DisputeModel, s: &Sim| s.deadline_was_meetable,
            ),
            Property::always(
                "an action one block late is always refused",
                |_: &DisputeModel, s: &Sim| s.late_action_was_rejected,
            ),
            // --- coverage: the space actually contains the interesting bits
            Property::sometimes(
                "a silent player loses on time",
                |_: &DisputeModel, s: &Sim| s.saw_timeout,
            ),
            Property::sometimes("an unrefuted claim stands", |_: &DisputeModel, s: &Sim| {
                s.saw_claim_stand
            }),
            Property::sometimes("a claim is refuted", |_: &DisputeModel, s: &Sim| {
                s.saw_refutation
            }),
            Property::sometimes("the ply cap ends the game", |_: &DisputeModel, s: &Sim| {
                s.saw_ply_cap
            }),
            Property::sometimes(
                "a resignation settles at once",
                |_: &DisputeModel, s: &Sim| s.saw_immediate_settlement,
            ),
        ]
    }
}
