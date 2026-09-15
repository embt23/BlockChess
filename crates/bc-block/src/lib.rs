//! What a block is, before anyone has decided how to agree on one.
//!
//! Episodes 05 and 06 build two consensus mechanisms that disagree about
//! almost everything — one is an economic race with probabilistic finality,
//! the other is a voting protocol with deterministic finality. They agree
//! about exactly this crate: what a header contains, how transactions are
//! committed to, and what a chain is obliged to offer the layer above.
//!
//! Splitting that out is not tidiness. `spec/02` says to build proof-of-work
//! first, then demonstrate that its probabilistic finality breaks the
//! challenge-window argument, and then replace it. That demonstration is only
//! possible if the *same* adjudicator can run on both, unchanged — otherwise
//! the comparison is between two different systems and proves nothing.
//! [`Consensus`] is the seam that makes it one system with two engines.
//!
//! ## Determinism
//!
//! Everything here is consensus code. `tests/determinism.rs` greps for
//! floats and hash maps, for the same reason `bc-adjudicator` does: a
//! validator that disagrees about a byte disagrees about money.

pub mod block;
pub mod chain;
pub mod consensus;
pub mod gas;
pub mod header;
pub mod tx;

pub use block::Block;
pub use chain::{Chain, ChainError};
pub use consensus::{Consensus, Finality};
pub use gas::{GasError, GasSchedule};
pub use header::{BlockHeader, HEADER_LEN};
pub use tx::{Payload, Tx};
