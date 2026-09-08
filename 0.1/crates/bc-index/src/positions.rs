//! The position index: games keyed by where they were, not how they got there.
//!
//! Answers Q2, and Q2 is the one that makes the database worth using. Two
//! games that reach the same position by different move orders are studying
//! the same thing, and a tool that files them separately is making the reader
//! do the transposition in their head.
//!
//! The key is `bc_codec::position::canonical_key` — the packed position, with
//! the halfmove clock excluded and the surviving symmetry group quotiented
//! out. So a line and its file-mirror image land in the same bucket, which
//! `papers/01-position-space.md` §6 measured as about a 4× reduction in
//! distinct keys in the regime most real positions live in.

use std::collections::HashMap;

use bc_chess::Position;
use bc_codec::position::canonical_key;

use crate::Score;

/// Where a game stood at a given moment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Posting {
    pub game: u32,
    pub ply: u16,
}

#[derive(Debug, Default)]
pub struct Entry {
    pub postings: Vec<Posting>,
    pub score: Score,
}

#[derive(Debug, Default)]
pub struct PositionIndex {
    pub map: HashMap<Vec<u8>, Entry>,
}

impl PositionIndex {
    pub fn new() -> PositionIndex {
        PositionIndex::default()
    }

    pub fn insert(&mut self, pos: &Position, game: u32, ply: u16, result: &str) {
        let (key, _) = canonical_key(pos);
        let e = self.map.entry(key).or_default();
        e.postings.push(Posting { game, ply });
        e.score.record(result);
    }

    /// Q2: which games reached here, by any move order or mirror.
    pub fn lookup(&self, pos: &Position) -> Option<&Entry> {
        self.map.get(&canonical_key(pos).0)
    }

    /// Q4: has this position ever occurred?
    pub fn contains(&self, pos: &Position) -> bool {
        self.map.contains_key(&canonical_key(pos).0)
    }

    pub fn distinct(&self) -> usize {
        self.map.len()
    }

    /// How many keys were reached by exactly one game. `papers/05-index.md`
    /// §2 predicts this is the overwhelming majority and proposes tiering on
    /// it; this is the measurement that decides whether that is worth doing.
    pub fn singletons(&self) -> usize {
        self.map.values().filter(|e| e.postings.len() == 1).count()
    }

    pub fn total_postings(&self) -> usize {
        self.map.values().map(|e| e.postings.len()).sum()
    }
}
