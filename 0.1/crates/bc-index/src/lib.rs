//! Making the corpus findable.
//!
//! `papers/05-index.md` lists four questions a chess database is asked, and
//! they need two different structures because two of them are different
//! questions that look the same:
//!
//! | | question | structure |
//! |---|---|---|
//! | Q1 | "show me games in this opening" | the **trie** — keyed by move order |
//! | Q2 | "show me games that reached this position, any move order" | the **position index** — keyed by canonical position |
//! | Q3 | "what is played here, and how does it score" | a fold over Q2 |
//! | Q4 | "has this ever occurred at all" | membership in Q2 |
//!
//! A tool that only answers Q1 will insist a Najdorf reached by transposition
//! is a different opening, which is wrong in the way that matters most to
//! somebody studying it.
//!
//! ## Nothing here is consensus state
//!
//! The index is *derived*: anyone can rebuild it from the games and get the
//! same answer, so it never needs to be agreed, only recomputed. That is what
//! makes it safe to be ambitious here — symmetry canonicalisation, lossy
//! sampling, learned models, anything — while the stored record stays austere.
//! Everything `papers/02-encodings.md` and `04-permanence.md` reject for being
//! unfit for consensus is welcome on this side of the line.

pub mod chunks;
pub mod eco;
pub mod positions;
pub mod trie;

pub use chunks::{Chunk, Counts};
pub use eco::{Eco, EcoEntry};

/// A square's name, for display.
pub fn square_name(s: bc_chess::Square) -> String {
    format!("{}{}", (b'a' + s % 8) as char, (b'1' + s / 8) as char)
}
pub use positions::PositionIndex;
pub use trie::{NodeId, Trie};

use bc_chess::{Move, Position};

/// Which side a game went to. Kept as a triple because "how does this line
/// score" is the question Q3 exists to answer, and a single win/loss field
/// cannot express a draw-heavy line.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Score {
    pub white: u32,
    pub black: u32,
    pub draws: u32,
    pub unknown: u32,
}

impl Score {
    pub fn total(&self) -> u32 {
        self.white + self.black + self.draws + self.unknown
    }

    /// White's score as a fraction, counting a draw as a half. `None` when
    /// nothing decisive has been recorded, rather than a misleading 0.5.
    pub fn white_percentage(&self) -> Option<f64> {
        let decided = self.white + self.black + self.draws;
        if decided == 0 {
            return None;
        }
        Some((self.white as f64 + 0.5 * self.draws as f64) / decided as f64)
    }

    pub fn record(&mut self, result: &str) {
        match result {
            "1-0" => self.white += 1,
            "0-1" => self.black += 1,
            "1/2-1/2" => self.draws += 1,
            _ => self.unknown += 1,
        }
    }
}

/// One game, as the index sees it.
pub struct IndexedGame<'a> {
    pub moves: &'a [Move],
    pub start: Position,
    pub result: &'a str,
}

/// Both structures, built in one pass over the corpus.
pub struct Index {
    pub trie: Trie,
    pub positions: PositionIndex,
    pub games: u32,
}

impl Index {
    /// Build from a corpus.
    ///
    /// `trie_depth` caps how deep the opening trie goes. Past about 30 plies
    /// almost every node has exactly one game in it, so the tail is all
    /// storage and no answers — `papers/05-index.md` §2 calls this out as the
    /// knob that makes the structure affordable.
    pub fn build<'a>(
        games: impl IntoIterator<Item = IndexedGame<'a>>,
        trie_depth: usize,
        position_plies: std::ops::Range<usize>,
    ) -> Index {
        let mut trie = Trie::new();
        let mut positions = PositionIndex::new();
        let mut count = 0u32;

        for game in games {
            let id = count;
            // Only games from the standard start belong in the opening book.
            // A game beginning from some other position has a move sequence
            // that means nothing when read from move one — it would graft a
            // foreign line onto the root and, worse, render as nonsense,
            // because those moves are not legal from the initial position.
            // Such games are still fully indexed by position below, which is
            // the structure that does not care how you got there.
            if game.start == Position::startpos() {
                trie.insert(game.moves, game.result, trie_depth);
            }

            let mut pos = game.start;
            for (ply, &m) in game.moves.iter().enumerate() {
                if position_plies.contains(&ply) {
                    positions.insert(&pos, id, ply as u16, game.result);
                }
                pos = pos.make_move(m);
            }
            // The final position too: "how do games in this line end up" is a
            // question about the last position, and leaving it out would make
            // every finished game invisible at its own conclusion.
            if position_plies.contains(&game.moves.len()) || game.moves.len() >= position_plies.end
            {
                positions.insert(&pos, id, game.moves.len() as u16, game.result);
            }
            count += 1;
        }

        Index {
            trie,
            positions,
            games: count,
        }
    }
}
