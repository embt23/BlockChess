//! Episodes 05–09 — the compression.
//!
//! The attack this defeats: *"you can't measure how I play."*
//!
//! A player is a function from positions to moves. This crate compresses that
//! function into a point, commits to the point as a **medal**, and chains the
//! medals so the record of who you were becomes the record of who you became.
//! See `spec/10-personality.md`.
//!
//! The pipeline, one module per stage:
//!
//! ```text
//!   PGN ──▶ features ──▶ basis ──▶ profile ──▶ chain
//!   pgn     features     linalg    profile     chain
//!   synth                basis
//! ```
//!
//! Nothing here authenticates anybody. A medal says *this play came from that
//! personality* (attribution, statistical); Ed25519 says *this person is at the
//! keyboard* (authentication, cryptographic). The old spec conflated them.

pub mod basis;
pub mod chain;
pub mod corpus;
pub mod features;
pub mod identify;
pub mod linalg;
pub mod pgn;
pub mod profile;
pub mod report;
pub mod synth;

pub use basis::Basis;
pub use chain::MedalChain;
pub use corpus::{Corpus, GameRecord};
pub use profile::Profile;

use bc_chess::types::Color;
use linalg::distance;

/// One game, from one side's point of view, placed in personality space.
#[derive(Clone, Debug)]
pub struct GamePoint {
    pub game: usize,
    pub player: String,
    pub side: Color,
    pub coords: Vec<f64>,
}

/// A fitted lens plus every point seen under it.
pub struct Lab {
    pub corpus: Corpus,
    pub basis: Basis,
    pub points: Vec<GamePoint>,
}

/// Games in a rolling profile window.
///
/// A *cumulative* mean would be nearly immovable after a hundred games, so a
/// player who genuinely changed would never earn a second link. A window keeps
/// the profile a statement about who someone is *now*, which is what the chain
/// is supposed to be a history of.
pub const WINDOW: usize = 10;

impl Lab {
    /// Extract, fit, and project in one pass.
    pub fn fit(corpus: Corpus, k: usize) -> Option<Lab> {
        let mut rows = Vec::new();
        let mut meta = Vec::new();
        for (i, g) in corpus.games.iter().enumerate() {
            for side in [Color::White, Color::Black] {
                if let Some(f) = features::extract(&g.start, &g.moves, side) {
                    rows.push(f);
                    let player = if side == Color::White {
                        g.white.clone()
                    } else {
                        g.black.clone()
                    };
                    meta.push((i, player, side));
                }
            }
        }
        if rows.is_empty() {
            return None;
        }

        let basis = Basis::fit(&rows, k, corpus.root());
        let points = rows
            .iter()
            .zip(meta)
            .map(|(f, (game, player, side))| GamePoint {
                game,
                player,
                side,
                coords: basis.project(f),
            })
            .collect();

        Some(Lab {
            corpus,
            basis,
            points,
        })
    }

    pub fn points_for(&self, player: &str) -> Vec<&GamePoint> {
        self.points.iter().filter(|p| p.player == player).collect()
    }

    /// A player's overall position: the centroid of every game they played.
    pub fn profile(&self, player: &str) -> Option<Profile> {
        let pts: Vec<Vec<f64>> = self
            .points_for(player)
            .into_iter()
            .map(|p| p.coords.clone())
            .collect();
        Profile::from_points(player, &pts)
    }

    /// Walk a player's games in order, minting a link whenever they have moved
    /// far enough. This is the narrative of their growth.
    pub fn chain_for(&self, player: &str) -> MedalChain {
        let pts = self.points_for(player);
        let mut chain = MedalChain::new(player);
        for end in 1..=pts.len() {
            let start = end.saturating_sub(WINDOW);
            let window: Vec<Vec<f64>> = pts[start..end].iter().map(|p| p.coords.clone()).collect();
            if let Some(p) = Profile::from_points(player, &window) {
                chain.offer(&p, &self.basis);
            }
        }
        chain
    }

    /// Mean distance between two games by the same player, and between two
    /// games by different players.
    ///
    /// This is the pipeline's honesty check. If personality is real and the
    /// compression captured it, *within* must be clearly smaller than
    /// *between*. If the two are equal, the points carry no information about
    /// who played the game and every medal minted from them is noise.
    pub fn separation(&self) -> (f64, f64) {
        let (mut win, mut wn, mut bet, mut bn) = (0.0, 0usize, 0.0, 0usize);
        for (i, a) in self.points.iter().enumerate() {
            for b in &self.points[i + 1..] {
                let d = distance(&a.coords, &b.coords);
                if a.player == b.player {
                    win += d;
                    wn += 1;
                } else {
                    bet += d;
                    bn += 1;
                }
            }
        }
        (
            if wn > 0 { win / wn as f64 } else { 0.0 },
            if bn > 0 { bet / bn as f64 } else { 0.0 },
        )
    }

    /// Share of games whose nearest other point belongs to the same player.
    ///
    /// A blunt but honest clustering score: chance level is roughly
    /// `1/(number of players)`.
    pub fn nearest_neighbour_accuracy(&self) -> f64 {
        if self.points.len() < 2 {
            return 0.0;
        }
        let mut hits = 0usize;
        for (i, a) in self.points.iter().enumerate() {
            let mut best = f64::INFINITY;
            let mut best_player = "";
            for (j, b) in self.points.iter().enumerate() {
                if i == j {
                    continue;
                }
                let d = distance(&a.coords, &b.coords);
                if d < best {
                    best = d;
                    best_player = &b.player;
                }
            }
            if best_player == a.player {
                hits += 1;
            }
        }
        hits as f64 / self.points.len() as f64
    }
}
