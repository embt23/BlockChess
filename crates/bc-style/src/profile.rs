//! Stages 5–6 — the profile, and the medal it commits to.

use crate::basis::Basis;
use crate::linalg::distance;
use bc_hash::{tagged_parts, Hash};

/// Quantisation step for medal commitment.
///
/// This constant is **the bit budget of a personality**. It decides how many
/// distinct medals exist, and therefore how hard one is to forge by playing
/// like someone else. It is currently a guess; `METAPLAN` O3 is the measurement
/// that should replace it. See `spec/10-personality.md` stage 6.
pub const STEP: f64 = 0.05;

/// Where a player stands, and how widely they range.
#[derive(Clone, Debug)]
pub struct Profile {
    pub player: String,
    pub coords: Vec<f64>,
    /// Mean distance of the player's games from their own centroid.
    ///
    /// Kept because it is meaningful on its own: two players with the same
    /// centroid, one who plays the same way every game and one who plays wildly
    /// differently, are not the same player. Glicko's `RD` is this quantity for
    /// the strength axis alone.
    pub dispersion: f64,
    pub games: usize,
}

impl Profile {
    /// The centroid of a player's game points.
    pub fn from_points(player: &str, points: &[Vec<f64>]) -> Option<Profile> {
        if points.is_empty() {
            return None;
        }
        let k = points[0].len();
        let n = points.len() as f64;
        let mut coords = vec![0.0; k];
        for p in points {
            for (i, c) in coords.iter_mut().enumerate() {
                *c += p[i];
            }
        }
        for c in coords.iter_mut() {
            *c /= n;
        }
        let dispersion = points.iter().map(|p| distance(p, &coords)).sum::<f64>() / n;
        Some(Profile {
            player: player.to_string(),
            coords,
            dispersion,
            games: points.len(),
        })
    }

    /// Coordinates on the commitment grid, saturating at ±127.
    pub fn quantised(&self) -> Vec<i8> {
        self.coords
            .iter()
            .map(|c| (c / STEP).round().clamp(-127.0, 127.0) as i8)
            .collect()
    }

    /// The medal: a commitment to *this* position under *that* lens.
    ///
    /// `corpus_root` is inside the hash, so the same player under a different
    /// lens has a different medal — by construction (`METAPLAN` N10). A medal
    /// without its corpus is not an identity, it is half of one.
    pub fn medal(&self, basis: &Basis) -> Hash {
        let q: Vec<u8> = self.quantised().iter().map(|&v| v as u8).collect();
        tagged_parts(
            "BC/style/medal/v1",
            &[&basis.corpus_root, &[basis.k as u8], &q],
        )
    }
}
