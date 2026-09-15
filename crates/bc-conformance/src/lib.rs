//! D23 — the split oracle.
//!
//! `G1` says every layer is verified against an oracle someone else published.
//! Episode 08 is the first layer where that rule cannot be satisfied whole,
//! because nobody publishes adjudicator test vectors. `spec/09` D23 splits it
//! rather than pretending otherwise:
//!
//! | Half | Standard | Where |
//! |---|---|---|
//! | terminal predicates — they are chess, and others implement chess | **differential** against `shakmaty` | `tests/terminal_differential.rs` |
//! | dilation, budgets, deadlines, override-by-ply — invented here | **exhaustive model check** of the properties | `tests/dispute_model.rs` |
//!
//! Neither half is described as an oracle where it is not one. A differential
//! test is a real oracle: `shakmaty` is a separate implementation by a
//! different author, so agreement is evidence and not a tautology. A model
//! check is *not* an oracle — it proves the state machine satisfies properties
//! this project chose, and if the properties are wrong it will say so
//! cheerfully. `spec/05` argues termination in prose; that prose is a proof
//! obligation written in English, and the model check discharges it.
//!
//! ## Why this is its own crate
//!
//! `bc-adjudicator` is forbidden dev-dependencies — `D24`, enforced by its own
//! `tests/dependencies.rs`. That rule exists so nothing can quietly become the
//! reason a consensus type is `pub`. An oracle harness is precisely the kind
//! of good reason that would arrive to relax it, so the harness lives outside
//! and the rule stays absolute.
//!
//! Nothing depends on this crate. It is a leaf, and it is allowed to depend on
//! anything.

pub mod dispute_model;
pub mod playout;

pub use playout::{Playout, Rng};
