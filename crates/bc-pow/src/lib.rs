//! Episode 05 — proof of work.
//!
//! The attack this defeats: *"my version of history is the real one."*
//!
//! ## This is built to be replaced
//!
//! `spec/02` is unusually explicit about it:
//!
//! > Build proof-of-work first — one episode, ~300 lines, and it is the
//! > clearest possible demonstration of "consensus as an economic race".
//! > Then show that its probabilistic finality breaks the challenge-window
//! > argument, and replace it. Building the wrong thing on purpose and then
//! > explaining precisely why it is wrong is better pedagogy than never
//! > building it.
//!
//! So this crate is not a stepping stone that gets deleted. It stays,
//! implementing the same [`Consensus`] trait as `bc-bft`, because episode
//! 06's argument is *"watch a reorg eat a dispute"* and an argument you can
//! run is worth more than one you can recall. `bc-channel`'s reorg demo
//! drives this crate and then drives the other one, and the only thing that
//! changes between them is which engine is underneath.
//!
//! ## The one thing it gets wrong, on purpose
//!
//! [`ProofOfWork::is_final`] returns `false`. Always, at any depth.
//!
//! That is not pedantry, it is the answer. Nakamoto confirmation is
//! probabilistic: a block `k` deep is very likely permanent and never
//! certainly so. Normally that is fine — wait longer for larger amounts.
//! Here it is not, because `spec/05`'s deadlines turn "un-included after
//! the fact" into permanent financial loss, and there is no `k` at which
//! that stops being true.
//!
//! [`ProofOfWork::deep_enough_by_convention`] is provided separately, and it
//! is the six-confirmations folklore everyone actually uses. It exists so
//! the demo can call it, believe it, and lose the money anyway.

pub mod miner;
pub mod retarget;
pub mod target;

use bc_block::{Block, Chain, ChainError, Consensus, Finality, GasError, GasSchedule};
use bc_hash::Hash;

pub use bc_block::chain::Reorg;
pub use miner::{mine, mine_from, pow_hash, seal_of};
pub use target::{Target, MAX_TARGET};

/// Where the target for the next block comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Difficulty {
    /// A constant. Every test here uses this, because the property under
    /// test — that heaviest-chain fork choice reorganises correctly — does
    /// not depend on the difficulty moving.
    Fixed(Target),
    /// The control loop in [`retarget`].
    ///
    /// **Unavailable until the `G0` hole is filled.** `retarget::next_target`
    /// is `todo!()`, so selecting this and crossing a window boundary
    /// panics. That is deliberate: a silently-working fallback is how an
    /// episode's subject quietly stops being anyone's job.
    Controlled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowError {
    Chain(ChainError),
    /// The seal is missing, malformed, or does not satisfy the target.
    InsufficientWork,
    /// The block breaks the gas schedule — over the limit, or ordinary
    /// traffic eating into the dispute reserve (episode 10).
    Gas(GasError),
}

impl From<GasError> for PowError {
    fn from(e: GasError) -> PowError {
        PowError::Gas(e)
    }
}

impl From<ChainError> for PowError {
    fn from(e: ChainError) -> PowError {
        PowError::Chain(e)
    }
}

impl std::fmt::Display for PowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for PowError {}

/// The folklore. Six blocks, for no reason that survives contact with a
/// deadline — it is roughly where Nakamoto's table crosses 0.1% for an
/// attacker with 10% of hashrate, and it has been repeated ever since as
/// though it were a property of the protocol rather than of that table.
pub const CONVENTIONAL_CONFIRMATIONS: u64 = 6;

pub struct ProofOfWork {
    chain: Chain,
    difficulty: Difficulty,
    gas: GasSchedule,
    /// Reported by [`ProofOfWork::last_reorg`] so a caller can see what a
    /// submission did to history without diffing the chain themselves.
    last_reorg: Reorg,
}

impl ProofOfWork {
    /// Start from a genesis block. It is accepted without work, because
    /// there is no parent whose difficulty would say how much to require.
    pub fn new(genesis: Block, difficulty: Difficulty) -> ProofOfWork {
        ProofOfWork {
            chain: Chain::new(genesis),
            difficulty,
            gas: GasSchedule::default(),
            last_reorg: Reorg::default(),
        }
    }

    /// Run with a different gas schedule — in particular
    /// [`GasSchedule::unprotected`], which is the chain as it was before
    /// episode 10 and which the censorship demonstration needs in order to
    /// show what the reserve is worth.
    pub fn with_gas(mut self, gas: GasSchedule) -> ProofOfWork {
        self.gas = gas;
        self
    }

    pub fn gas(&self) -> GasSchedule {
        self.gas
    }

    pub fn chain(&self) -> &Chain {
        &self.chain
    }

    pub fn last_reorg(&self) -> &Reorg {
        &self.last_reorg
    }

    /// Total accumulated work behind the current head.
    pub fn total_work(&self) -> u128 {
        self.chain.score()
    }

    /// What target a block building on `parent` must satisfy.
    pub fn required_target(&self, parent: &Hash) -> Target {
        match self.difficulty {
            Difficulty::Fixed(t) => t,
            Difficulty::Controlled => self.controlled_target(parent),
        }
    }

    /// The control loop, driven from the chain.
    ///
    /// Retargets only on window boundaries; between them the target is
    /// whatever the last boundary chose. Walks the parent's own ancestry
    /// rather than the canonical chain, because a block on a fork must be
    /// validated against the difficulty *its* chain implies — otherwise a
    /// fork could be judged by rules it never played under.
    fn controlled_target(&self, parent: &Hash) -> Target {
        let Some(p) = self.chain.block(parent) else {
            return MAX_TARGET;
        };
        let next_height = p.header.height + 1;
        let ancestry = self.chain.ancestry(parent);
        let current = self.target_in_force(&ancestry);
        if !next_height.is_multiple_of(retarget::WINDOW)
            || ancestry.len() <= retarget::WINDOW as usize
        {
            return current;
        }
        let newest = p.header.timestamp_ms;
        let oldest = self
            .chain
            .block(&ancestry[retarget::WINDOW as usize - 1])
            .map(|b| b.header.timestamp_ms)
            .unwrap_or(newest);
        retarget::next_target(current, newest.saturating_sub(oldest))
    }

    /// The target the most recent block on this ancestry was mined against,
    /// recovered from the work the chain recorded for it.
    fn target_in_force(&self, ancestry: &[Hash]) -> Target {
        match self.difficulty {
            Difficulty::Fixed(t) => t,
            Difficulty::Controlled => ancestry
                .first()
                .and_then(|h| self.block_target(h))
                .unwrap_or(MAX_TARGET),
        }
    }

    /// The target a stored block was actually mined against: the smallest
    /// one its own seal satisfies is not recoverable, so the work recorded
    /// in the chain is used instead.
    fn block_target(&self, hash: &Hash) -> Option<Target> {
        let block = self.chain.block(hash)?;
        let score = self.chain.score_of(hash)?;
        let parent_score = self.chain.score_of(&block.header.parent_hash).unwrap_or(0);
        let work = score.saturating_sub(parent_score).max(1);
        Some(Target(u128::MAX / work))
    }

    /// The six-confirmations rule, as folklore rather than as a guarantee.
    ///
    /// Separate from [`Consensus::is_final`], which answers `false`. This
    /// method exists so a caller can make the mistake everyone makes, in
    /// one clearly-named place, and so the demo can point at the call site.
    pub fn deep_enough_by_convention(&self, hash: &Hash) -> bool {
        self.chain
            .confirmations(hash)
            .is_some_and(|c| c >= CONVENTIONAL_CONFIRMATIONS)
    }
}

impl Consensus for ProofOfWork {
    type Error = PowError;

    fn finality(&self) -> Finality {
        Finality::Probabilistic
    }

    fn head(&self) -> Hash {
        self.chain.head()
    }

    fn height(&self) -> u64 {
        self.chain.height()
    }

    fn submit(&mut self, block: Block) -> Result<(), PowError> {
        let target = self.required_target(&block.header.parent_hash);
        if !miner::seal_is_valid(&block, target) {
            return Err(PowError::InsufficientWork);
        }
        self.gas.admits(&block)?;
        let parent_score = self
            .chain
            .score_of(&block.header.parent_hash)
            .ok_or(ChainError::UnknownParent)?;
        // Cumulative work, never block count. A chain of ten easy blocks
        // must lose to three hard ones, or an attacker just lowers their own
        // difficulty and calls it a longer chain.
        let score = parent_score.saturating_add(target.work());
        self.last_reorg = self.chain.insert(block, score)?;
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

    /// Never. See the module documentation — this is the answer, not a
    /// placeholder, and `bc-bft` exists because of it.
    fn is_final(&self, _hash: &Hash) -> bool {
        false
    }
}
