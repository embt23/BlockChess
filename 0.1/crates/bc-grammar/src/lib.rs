//! Finding the repeated patterns in a corpus, by compressing it.
//!
//! `papers/08-layers.md` claims that searching for the shortest encoding of a
//! corpus is the same search as searching for its theory. This crate is the
//! machine that tests the claim.
//!
//! The algorithm is **Re-Pair** (Larsson & Moffat, 1999), and the whole of it
//! fits in a sentence:
//!
//! > Find the pair of adjacent symbols that occurs most often, give it a name,
//! > replace every occurrence with that name, and repeat.
//!
//! Run it on English and it invents symbols for `th`, then `the`, then
//! ` the `. Run it on a corpus of chess games and — if the claim holds — it
//! invents symbols for opening moves, then opening lines, then whole systems.
//!
//! Nothing in this crate knows anything about chess. It is given sequences of
//! opaque integers. That is the point: if it finds openings, it found them in
//! the statistics, not in anything we told it.
//!
//! ## What it is not
//!
//! Re-Pair is not the best compressor available and is not meant to be.
//! `papers/08-layers.md` §6 predicts on the record that it will not beat a
//! learned policy model on bits. Its virtue is that its model is a **list you
//! can read**: every symbol it invents is an explicit sequence, countable and
//! nameable and arguable-about. A neural network compressing the same corpus
//! to fewer bits hands you a weights file.

mod repair;

pub use repair::{induce, Grammar, Rule, Sym};

/// How to stop. Re-Pair will happily keep going until every pair is unique,
/// long past the point where the symbols mean anything.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// Stop when the best remaining pair occurs fewer times than this.
    pub min_occurrences: u32,
    /// Stop after inventing this many symbols, whatever the counts say.
    pub max_rules: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            min_occurrences: 2,
            max_rules: 100_000,
        }
    }
}
