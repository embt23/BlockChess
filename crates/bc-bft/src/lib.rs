//! Episode 06 — Tendermint-style BFT.
//!
//! The attack this defeats: *"I'll reorg away your dispute."*
//!
//! ## Why this episode exists at all
//!
//! Episode 05 built a working chain. This one replaces it, and the reason
//! is one sentence from `spec/02`:
//!
//! > **Money under a deadline requires deterministic finality.**
//!
//! Under Nakamoto consensus, confirmation is probabilistic. Normally that
//! is fine: wait longer for larger amounts. Here it is not, because
//! `spec/05`'s rule is *"respond within Δ blocks or forfeit"*. An adversary
//! who can reorg `k` blocks can un-include your dispute response **after**
//! Δ has passed, and you lose money you correctly defended. Waiting longer
//! does not help — the deadline already went by.
//!
//! `bc-pow` reports [`Finality::Probabilistic`] and answers `false` to
//! `is_final` at every depth. This crate reports
//! [`Finality::Deterministic`] and means it: a block with a valid commit
//! certificate is final, and [`Bft::submit`] refuses any block that would
//! reorganise one.
//!
//! ## What safety rests on
//!
//! Quorum intersection, and it is three sentences. Two quorums each above
//! ⅔ of stake overlap in more than ⅓. At most ⅓ is Byzantine. So the
//! overlap contains an honest validator, and an honest validator does not
//! precommit two blocks at the same height and round. `validators.rs`
//! checks that arithmetic at every set size rather than asserting it in a
//! comment.
//!
//! ## What is still an open hole
//!
//! [`locking`] — the rules that carry safety *across a failed round* — is
//! `G0`, episode 06's filmed subject. A single round needs none of it and
//! commits blocks with that file untouched; the moment a round fails, the
//! node calls into it and it says whose job that is.

pub mod commit;
pub mod locking;
pub mod msg;
pub mod node;
pub mod tally;
pub mod validators;
pub mod vote;

use bc_block::{Block, Chain, ChainError, Consensus, Finality};
use bc_hash::Hash;

pub use commit::Commit;
pub use locking::Lock;
pub use msg::{Committed, Msg};
pub use node::Node;
pub use tally::{Outcome, Tally};
pub use validators::{Validator, ValidatorSet};
pub use vote::{Step, Vote};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BftError {
    Chain(ChainError),
    /// The seal is missing, malformed, or does not carry a ⅔ precommit
    /// quorum for this exact block.
    NoCommitCertificate,
    /// The block would displace one that is already final. Under
    /// deterministic finality this is not a fork choice, it is an attack.
    WouldReorganiseFinality,
}

impl From<ChainError> for BftError {
    fn from(e: ChainError) -> BftError {
        BftError::Chain(e)
    }
}

impl std::fmt::Display for BftError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for BftError {}

/// A chain whose blocks are final when they are committed.
pub struct Bft {
    chain: Chain,
    set: ValidatorSet,
    /// Every block at or below this height is beyond reorganisation.
    final_height: u64,
}

impl Bft {
    pub fn new(genesis: Block, set: ValidatorSet) -> Bft {
        Bft {
            chain: Chain::new(genesis),
            set,
            final_height: 0,
        }
    }

    pub fn chain(&self) -> &Chain {
        &self.chain
    }

    pub fn validators(&self) -> &ValidatorSet {
        &self.set
    }

    pub fn final_height(&self) -> u64 {
        self.final_height
    }
}

impl Consensus for Bft {
    type Error = BftError;

    fn finality(&self) -> Finality {
        Finality::Deterministic
    }

    fn head(&self) -> Hash {
        self.chain.head()
    }

    fn height(&self) -> u64 {
        self.chain.height()
    }

    /// Accept a block only if it carries a quorum that finalises it, and
    /// only if it does not displace anything already final.
    ///
    /// The second check is what makes the type's promise true. Without it
    /// this is proof-of-work with signatures instead of hashes: the fork
    /// choice would happily follow a longer chain that discards a
    /// committed block, and a dispute could be un-included after its
    /// deadline exactly as under episode 05.
    fn submit(&mut self, block: Block) -> Result<(), BftError> {
        let hash = block.hash();
        let commit = Commit::decode(&block.seal).ok_or(BftError::NoCommitCertificate)?;
        if !commit.verifies(&self.set, block.header.height, &hash) {
            return Err(BftError::NoCommitCertificate);
        }
        // Anything at or below the finalised height is settled. A block
        // claiming one of those heights is claiming to replace a block a
        // quorum already committed.
        if block.header.height <= self.final_height {
            return Err(BftError::WouldReorganiseFinality);
        }
        if !self.chain.contains(&block.header.parent_hash) {
            return Err(BftError::Chain(ChainError::UnknownParent));
        }
        // A committed block must extend the finalised chain, not fork from
        // beside it.
        let parent_height = self
            .chain
            .block(&block.header.parent_hash)
            .map(|b| b.header.height)
            .unwrap_or(0);
        if parent_height < self.final_height {
            return Err(BftError::WouldReorganiseFinality);
        }

        // Score is height: with finality there is no work to weigh, and no
        // two blocks can ever compete for the same one.
        let score = block.header.height as u128;
        let reorg = self.chain.insert(block, score)?;
        debug_assert!(
            !reorg.is_rollback(),
            "deterministic finality must never roll back"
        );
        self.final_height = self.final_height.max(self.chain.height());
        Ok(())
    }

    fn block(&self, hash: &Hash) -> Option<&Block> {
        self.chain.block(hash)
    }

    fn is_canonical(&self, hash: &Hash) -> bool {
        self.chain.is_canonical(hash)
    }

    fn confirmations(&self, hash: &Hash) -> Option<u64> {
        self.chain.confirmations(hash)
    }

    /// A committed block is final. Not "very likely permanent" — final.
    fn is_final(&self, hash: &Hash) -> bool {
        self.chain
            .block(hash)
            .is_some_and(|b| self.is_canonical(hash) && b.header.height <= self.final_height)
    }
}
