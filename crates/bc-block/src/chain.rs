//! A block store that knows the difference between *seen* and *canonical*.
//!
//! Both engines need somewhere to keep blocks, an index of which ones are
//! currently on the best chain, and a way to say what changed when the best
//! chain moves. Only the last of those is interesting, and it is interesting
//! because it is the thing episode 06 exists to argue about.
//!
//! [`Chain::insert`] returns a [`Reorg`]. Its `reverted` field is the list of
//! blocks that **were** canonical and are not any more — which is to say, the
//! list of transactions that were confirmed and now are not. Under
//! proof-of-work that list is routinely non-empty. Under BFT it must be empty
//! below the finalised height or the protocol has failed.
//!
//! Making it a return value rather than an internal detail is the point: a
//! reorg that nothing reports is a reorg nobody can write a test about.
//!
//! ## Fork choice lives outside
//!
//! `insert` takes a `score` and picks the highest. What the score *means* is
//! the engine's business — cumulative work for `bc-pow`, height for
//! `bc-bft` — because that choice is precisely what distinguishes them.

use crate::block::Block;
use bc_hash::Hash;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainError {
    /// No block with this `parent_hash` is known.
    UnknownParent,
    /// A block with this hash is already stored.
    Duplicate,
    /// `height` is not `parent.height + 1`.
    HeightNotSequential,
    /// The header's `tx_root` does not commit to the transactions supplied.
    BodyDoesNotMatchHeader,
}

impl std::fmt::Display for ChainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ChainError {}

/// What changed when the best chain moved.
///
/// `reverted` is newest-first and `applied` is oldest-first, so replaying a
/// reorg is "undo each of `reverted` in order, then do each of `applied`".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Reorg {
    pub reverted: Vec<Hash>,
    pub applied: Vec<Hash>,
}

impl Reorg {
    /// A reorg that un-included something. The dangerous kind.
    pub fn is_rollback(&self) -> bool {
        !self.reverted.is_empty()
    }

    /// How deep it cut.
    pub fn depth(&self) -> usize {
        self.reverted.len()
    }
}

struct Entry {
    block: Block,
    score: u128,
}

pub struct Chain {
    blocks: BTreeMap<Hash, Entry>,
    /// height → hash, for the chain currently believed in. Rebuilt on reorg.
    canonical: BTreeMap<u64, Hash>,
    head: Hash,
    genesis: Hash,
}

impl Chain {
    /// Start from a genesis block, which is canonical by definition and has
    /// no parent to check.
    pub fn new(genesis: Block) -> Chain {
        let hash = genesis.hash();
        let height = genesis.header.height;
        let mut blocks = BTreeMap::new();
        blocks.insert(
            hash,
            Entry {
                block: genesis,
                score: 0,
            },
        );
        let mut canonical = BTreeMap::new();
        canonical.insert(height, hash);
        Chain {
            blocks,
            canonical,
            head: hash,
            genesis: hash,
        }
    }

    pub fn head(&self) -> Hash {
        self.head
    }

    pub fn genesis(&self) -> Hash {
        self.genesis
    }

    pub fn height(&self) -> u64 {
        self.blocks[&self.head].block.header.height
    }

    pub fn score(&self) -> u128 {
        self.blocks[&self.head].score
    }

    pub fn block(&self, hash: &Hash) -> Option<&Block> {
        self.blocks.get(hash).map(|e| &e.block)
    }

    pub fn score_of(&self, hash: &Hash) -> Option<u128> {
        self.blocks.get(hash).map(|e| e.score)
    }

    pub fn contains(&self, hash: &Hash) -> bool {
        self.blocks.contains_key(hash)
    }

    pub fn at_height(&self, height: u64) -> Option<Hash> {
        self.canonical.get(&height).copied()
    }

    /// Is this block on the chain we currently believe in?
    pub fn is_canonical(&self, hash: &Hash) -> bool {
        match self.blocks.get(hash) {
            Some(e) => self.canonical.get(&e.block.header.height) == Some(hash),
            None => false,
        }
    }

    /// How many blocks sit on top of this one, if it is canonical.
    pub fn confirmations(&self, hash: &Hash) -> Option<u64> {
        if !self.is_canonical(hash) {
            return None;
        }
        let h = self.blocks.get(hash)?.block.header.height;
        Some(self.height() - h + 1)
    }

    /// Store a block and, if it now has the best score, move the head.
    ///
    /// `score` is the engine's fork-choice value: cumulative work under PoW,
    /// height under BFT. Ties keep the incumbent, which is the conventional
    /// rule and matters — "first seen wins" denies a miner who arrives late
    /// with equal work the right to reorganise anyone.
    pub fn insert(&mut self, block: Block, score: u128) -> Result<Reorg, ChainError> {
        let hash = block.hash();
        if self.blocks.contains_key(&hash) {
            return Err(ChainError::Duplicate);
        }
        if !block.body_matches_header() {
            return Err(ChainError::BodyDoesNotMatchHeader);
        }
        let parent = self
            .blocks
            .get(&block.header.parent_hash)
            .ok_or(ChainError::UnknownParent)?;
        if block.header.height != parent.block.header.height + 1 {
            return Err(ChainError::HeightNotSequential);
        }

        self.blocks.insert(hash, Entry { block, score });
        if score > self.score() {
            Ok(self.move_head_to(hash))
        } else {
            Ok(Reorg::default())
        }
    }

    /// Walk back from `hash` to the genesis, newest first.
    pub fn ancestry(&self, hash: &Hash) -> Vec<Hash> {
        let mut out = Vec::new();
        let mut cur = *hash;
        while let Some(e) = self.blocks.get(&cur) {
            out.push(cur);
            if cur == self.genesis {
                break;
            }
            cur = e.block.header.parent_hash;
        }
        out
    }

    /// Re-point the head, and report exactly which blocks left the canonical
    /// chain and which joined it.
    fn move_head_to(&mut self, new_head: Hash) -> Reorg {
        let old: Vec<Hash> = self.ancestry(&self.head);
        let new: Vec<Hash> = self.ancestry(&new_head);

        // The fork point is the deepest block both chains contain.
        let shared: std::collections::BTreeSet<Hash> = new.iter().copied().collect();
        let reverted: Vec<Hash> = old
            .iter()
            .take_while(|h| !shared.contains(*h))
            .copied()
            .collect();
        let reverted_set: std::collections::BTreeSet<Hash> = old.iter().copied().collect();
        let mut applied: Vec<Hash> = new
            .iter()
            .take_while(|h| !reverted_set.contains(*h))
            .copied()
            .collect();
        applied.reverse();

        self.head = new_head;
        self.canonical.clear();
        for h in new.iter().rev() {
            let height = self.blocks[h].block.header.height;
            self.canonical.insert(height, *h);
        }
        Reorg { reverted, applied }
    }
}
