//! The corpus: an ordered, committed set of games.
//!
//! A basis is only meaningful if everyone derives the *same* one, and that
//! requires everyone reading the same games in the same order. So the corpus is
//! committed with the RFC 6962 list tree from `bc-merkle`, and its root goes
//! inside every medal minted under it (`METAPLAN` N9, N10).
//!
//! This is the whole answer to "why does this need a chain?" A mutable database
//! would let an operator quietly add or drop games, shift the axes, and
//! silently re-judge every player who ever earned a medal.

use bc_chess::{Move, Position};
use bc_hash::Hash;
use bc_merkle::tree;

/// One game, as everything downstream sees it.
#[derive(Clone, Debug)]
pub struct GameRecord {
    pub white: String,
    pub black: String,
    pub start: Position,
    pub moves: Vec<Move>,
}

impl GameRecord {
    /// The bytes this game is committed as.
    ///
    /// Player names are length-prefixed rather than delimited: with a delimiter,
    /// a player called `"a\0b"` could be made to collide with the pair
    /// `("a", "b")`, which is the same ambiguity that makes unprefixed Merkle
    /// leaves forgeable (`spec/02-chain.md`).
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(64 + self.moves.len() * 2);
        for name in [&self.white, &self.black] {
            out.extend_from_slice(&(name.len() as u32).to_le_bytes());
            out.extend_from_slice(name.as_bytes());
        }
        let fen = self.start.to_fen();
        out.extend_from_slice(&(fen.len() as u32).to_le_bytes());
        out.extend_from_slice(fen.as_bytes());
        out.extend_from_slice(&(self.moves.len() as u32).to_le_bytes());
        for m in &self.moves {
            out.extend_from_slice(&m.0.to_le_bytes());
        }
        out
    }
}

/// An ordered set of games, and the commitment to it.
#[derive(Clone, Debug, Default)]
pub struct Corpus {
    pub games: Vec<GameRecord>,
}

impl Corpus {
    pub fn new() -> Corpus {
        Corpus::default()
    }

    pub fn push(&mut self, g: GameRecord) {
        self.games.push(g);
    }

    pub fn len(&self) -> usize {
        self.games.len()
    }

    pub fn is_empty(&self) -> bool {
        self.games.is_empty()
    }

    /// The Merkle root over the games, in order. This is the lens's identity.
    pub fn root(&self) -> Hash {
        let leaves: Vec<Vec<u8>> = self.games.iter().map(|g| g.canonical_bytes()).collect();
        tree::root_owned(&leaves)
    }

    /// Every distinct player name, in first-appearance order.
    pub fn players(&self) -> Vec<String> {
        let mut seen: Vec<String> = Vec::new();
        for g in &self.games {
            for name in [&g.white, &g.black] {
                if !seen.iter().any(|s| s == name) {
                    seen.push(name.clone());
                }
            }
        }
        seen
    }
}
