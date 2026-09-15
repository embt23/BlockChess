//! A node: consensus underneath, the escrow and the adjudicator on top.
//!
//! This crate exists because the interesting question is not "does the
//! chain work" or "does the adjudicator work" — both were answered in
//! their own episodes — but **what happens to a dispute when the chain
//! changes its mind**. Answering that needs both halves in one place, and
//! neither half should have to know about the other to get there. So the
//! channel keeps depending only on `bc-block`'s [`Consensus`] trait, the
//! engines keep knowing nothing about chess, and the wiring lives here.
//!
//! ## State is the canonical chain, replayed
//!
//! [`Node::apply`] rebuilds the whole ledger from the genesis balances
//! every time the head moves, rather than journalling and undoing. For a
//! real chain that is absurd and for this one it is exactly right: it is
//! obviously correct, it has no undo logic to be wrong, and it makes the
//! definition operational — **the state is a function of the canonical
//! chain and nothing else.** A reorg is then not an event to handle, it is
//! simply a different input.
//!
//! That is also what makes the demonstration honest. Nothing in
//! [`Node::apply`] knows that a reorg happened or treats one specially.

pub mod action;
pub mod exec;
pub mod scenario;

pub use action::Action;
pub use exec::{Node, Seed};
pub use scenario::Scene;
