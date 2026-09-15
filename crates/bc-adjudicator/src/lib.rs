#![cfg_attr(not(feature = "std"), no_std)]
//! Episode 08 — the adjudicator, as its own crate.
//!
//! `D24`. The node links this as its state transition function, and
//! **`bc-channel` links it too** — which is the part worth choosing
//! deliberately rather than falling into. A client that links the adjudicator
//! can run an entire dispute *locally before spending any gas*: post its best
//! certified state against a simulated chain, see the deadline it would get,
//! see whether its mate claim survives refutation. That turns the griefing
//! analysis in `spec/05` from an argument into something a client computes.
//!
//! ## Why a crate boundary rather than a module
//!
//! Consensus code must be bit-identical on every validator, which rules out
//! floating point, hash-map iteration order, and anything that varies with
//! the platform. Those are lints you can impose for free at a fresh crate
//! boundary and painfully inside a crate full of legitimate client-side
//! convenience. `P5` stops being a rule people remember and becomes a
//! dependency arrow.
//!
//! The dependency list is the specification: `bc-chess` for the rules,
//! `bc-hash` for state hashes, nothing else. In particular **no signatures** —
//! claims arrive here already verified, because deciding *whether* a
//! signature is good is the escrow's job and deciding *what follows* from it
//! is this crate's. `tests/dependencies.rs` fails if that list grows.
//!
//! ## What is here
//!
//! | Module | What |
//! |---|---|
//! | [`state`] | the 109-byte object both players sign |
//! | [`terms`] | what a channel commits to, including its time-control class |
//! | [`timecontrol`] | Δ and τ by class, so no window crosses the wire (`D22`) |
//! | [`dilation`] | the on-chain time base |
//! | [`ruleset`] | what an `adjudicator_ver` denotes (`D25`) |
//! | [`dispute`] | the state machine: the same game, on-chain, slowly |
//! | [`status`] | three lines of shared vocabulary |

extern crate alloc;

pub mod dilation;
pub mod dispute;
pub mod ruleset;
pub mod state;
pub mod status;
pub mod terms;
pub mod timecontrol;

pub use dispute::{ClaimKind, Dispute, DisputeError, MoveOutcome, Refutation};
pub use ruleset::{RegistryError, RulesetRegistry};
pub use state::{pos_hash, rep_hash, GameState, Status};
pub use terms::GameTerms;
pub use timecontrol::TimeControl;
