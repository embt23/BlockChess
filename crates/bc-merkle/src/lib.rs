//! Episode 04 — Merkle trees.
//!
//! The attack this defeats: *"here's a different game history."*
//!
//! Two shapes, for two jobs:
//!
//! - [`tree`] — a Merkle tree over an **ordered list** (RFC 6962). Commits to
//!   the transactions in a block. Membership costs ⌈log₂ n⌉ hashes.
//! - [`smt`] — a **sparse** Merkle tree keyed by 256-bit hashes. Commits to
//!   chain state: accounts, channels, disputes, servers. Proves absence as
//!   cheaply as presence.
//!
//! Both are the same underlying idea — hash pairs upward until one value
//! commits to everything — and the same idea again as the hash chain in
//! episode 01 and the block chain in episode 05. A hash chain turns a statement
//! about one thing into a statement about everything *before* it; a Merkle tree
//! turns a statement about one thing into a statement about everything
//! *beside* it.

pub mod smt;
pub mod tree;

pub use bc_hash::Hash;
pub use smt::{Proof, Smt};
