//! Stage 7 — the chain of medals.
//!
//! A link is earned by **changing**, not by playing (`METAPLAN` N4). Play a
//! thousand games without changing how you play and you earn exactly one link;
//! genuinely become a different player and you earn another. That is what makes
//! the chain a medal rather than a logbook, and it is the single design choice
//! that makes a long chain mean something.
//!
//! The head commits to every profile you have ever held, which is
//! `spec/04-channel.md`'s argument one level up: *a chain of hashes turns a
//! statement about one thing into a statement about everything that led to it.*

use crate::basis::Basis;
use crate::linalg::distance;
use crate::profile::Profile;
use bc_hash::{tagged_parts, Hash};

/// How far a player must move in personality space to earn a link.
pub const DRIFT: f64 = 0.75;

#[derive(Clone, Debug)]
pub struct Link {
    pub index: u32,
    pub medal: Hash,
    pub head: Hash,
    pub coords: Vec<f64>,
    pub games: usize,
    /// Distance travelled since the previous link. Zero for the first.
    pub moved: f64,
}

/// A player's identity: the narrative of who they have been.
#[derive(Clone, Debug)]
pub struct MedalChain {
    pub player: String,
    pub links: Vec<Link>,
}

impl MedalChain {
    pub fn new(player: &str) -> MedalChain {
        MedalChain {
            player: player.to_string(),
            links: Vec::new(),
        }
    }

    pub fn head(&self) -> Hash {
        self.links.last().map(|l| l.head).unwrap_or([0u8; 32])
    }

    /// Offer a profile to the chain. A link is appended only if the player has
    /// moved further than [`DRIFT`] since the last one.
    ///
    /// Returns whether a link was minted.
    pub fn offer(&mut self, profile: &Profile, basis: &Basis) -> bool {
        let moved = match self.links.last() {
            None => 0.0,
            Some(prev) => {
                let d = distance(&profile.coords, &prev.coords);
                if d <= DRIFT {
                    return false;
                }
                d
            }
        };

        let medal = profile.medal(basis);
        let index = self.links.len() as u32;
        let head = tagged_parts(
            "BC/style/link/v1",
            &[
                &self.head(),
                &medal,
                &(profile.games as u32).to_le_bytes(),
                &basis.corpus_root,
            ],
        );
        self.links.push(Link {
            index,
            medal,
            head,
            coords: profile.coords.clone(),
            games: profile.games,
            moved,
        });
        true
    }

    /// Total distance travelled through personality space — the length of the
    /// journey, as opposed to `distance(first, last)`, which is only how far
    /// from home you ended up.
    pub fn path_length(&self) -> f64 {
        self.links.iter().map(|l| l.moved).sum()
    }
}
