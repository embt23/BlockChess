//! The seam between "what a chain is" and "how this chain agrees".
//!
//! `spec/02` asks for proof-of-work first and Tendermint-style BFT second,
//! with the second justified by watching the first fail in a specific way.
//! That argument only lands if the same adjudicator, ledger and channel run
//! unchanged on both — otherwise the comparison is between two systems and
//! demonstrates nothing about either.
//!
//! So the layer above consensus is written against this trait, and the two
//! episodes are two implementations of it.
//!
//! ## The property that is not a method
//!
//! [`Consensus::finality`] is the whole of episode 06 expressed as an
//! associated value. Everything above the chain that depends on a deadline —
//! which here is everything that holds money — depends on it, and it is not
//! something an implementation can be polite about:
//!
//! > **Money under a deadline requires deterministic finality.**
//!
//! Under [`Finality::Probabilistic`], a transaction included before its
//! deadline can be un-included after it. The rule "respond within Δ blocks
//! or forfeit" then turns a liveness failure into a permanent financial
//! loss, which is not a risk the protocol above knows how to price.

use crate::block::Block;
use bc_hash::Hash;

/// What "confirmed" is worth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Finality {
    /// A block `k` deep is *very likely* permanent and never certainly so.
    /// An adversary who can reorg `k` blocks can un-include anything.
    Probabilistic,
    /// A block that is final is final. Safety holds while Byzantine stake
    /// is under the protocol's bound.
    Deterministic,
}

impl Finality {
    /// Is it safe to settle money that was held under a block deadline?
    ///
    /// The one question the layers above actually ask. `spec/05`'s whole
    /// timeout argument is conditioned on it, and `bc-pow` answers `false`.
    pub fn safe_under_deadline(self) -> bool {
        matches!(self, Finality::Deterministic)
    }
}

/// What every chain owes the layer above it.
pub trait Consensus {
    type Error: core::fmt::Debug;

    /// What confirmation is worth on this chain.
    fn finality(&self) -> Finality;

    /// The head of the chain this engine currently believes in.
    fn head(&self) -> Hash;

    /// The head's height. What `Δ` is counted in (`P4`).
    fn height(&self) -> u64;

    /// Offer a block. Engines differ entirely in what makes one acceptable.
    fn submit(&mut self, block: Block) -> Result<(), Self::Error>;

    /// A block the engine has accepted, canonical or not.
    ///
    /// Orphans are returned too, and must be: a reorg does not erase
    /// history, it changes which history is believed, and a node that
    /// threw away the losing branch could not serve a peer still on it.
    fn block(&self, hash: &Hash) -> Option<&Block>;

    /// Is `hash` on the chain the engine currently believes in?
    ///
    /// Distinct from "have I seen this block", and the distinction is the
    /// point: a block orphaned by a reorg is still known and no longer
    /// canonical, and a transaction inside it has been un-included.
    fn is_canonical(&self, hash: &Hash) -> bool;

    /// How many blocks deep `hash` is, if it is canonical.
    fn confirmations(&self, hash: &Hash) -> Option<u64>;

    /// Is `hash` beyond any possible reorg?
    ///
    /// Under deterministic finality this is a fact the protocol produces.
    /// Under probabilistic finality it is a judgement call, and the default
    /// here is deliberately the honest one: **never**. An engine with
    /// probabilistic finality may override it with a depth heuristic, but
    /// it has to do so explicitly, in its own file, where a reader can see
    /// the number that was picked and argue with it.
    fn is_final(&self, _hash: &Hash) -> bool {
        false
    }
}
