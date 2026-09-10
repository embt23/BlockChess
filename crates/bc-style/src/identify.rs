//! Episodes 10 and 12 — is a medal forgeable, and can you escape one?
//!
//! Two attacks, one measurement.
//!
//! - **"I'll play like you and steal your medal."** A medal is a commitment to a
//!   quantised position, so forging one means landing in the same grid cell.
//!   The question is how far apart real players sit *in cells* — that margin is
//!   the medal's security parameter, and nothing in the design fixes it. It is
//!   an empirical property of chess.
//! - **"I'll just start a fresh account."** If identity is derived from play,
//!   a new name converges back to the same fingerprint after some number of
//!   games. That number is the whole content of the claim that you cannot hide.
//!
//! # The protocol, and why it is stricter than it looks
//!
//! Attribution accuracy is trivially inflated by letting any part of the
//! pipeline see the games it is later scored on. This module therefore splits
//! the corpus **before the lens is fitted**:
//!
//! ```text
//!   train games ──▶ standardise, basis, player centroids
//!   test games  ──▶ projected under the train basis, then attributed
//! ```
//!
//! So the axes, the column statistics and the centroids are all functions of
//! the training half alone, and a test game is scored exactly as a genuinely
//! new game would be. [`Lab::nearest_neighbour_accuracy`](crate::Lab) does not
//! do this — it compares every point against every other, including other games
//! by the same player, which is a leave-one-out estimate on data the basis
//! already saw. It is a useful smoke test and a bad measurement.

use crate::basis::Basis;
use crate::features;
use crate::linalg::distance;
use crate::profile::STEP;
use crate::synth::Rng;
use crate::{Corpus, Lab};
use bc_chess::types::Color;

/// A player's known position, learned from the training half.
#[derive(Clone, Debug)]
pub struct Centroid {
    pub player: String,
    pub coords: Vec<f64>,
    pub games: usize,
}

/// One player's games held out for scoring.
#[derive(Clone, Debug)]
pub struct HeldOut {
    pub player: String,
    pub points: Vec<Vec<f64>>,
}

/// A corpus split so the lens never sees what it is scored on.
pub struct Split {
    pub lab: Lab,
    pub centroids: Vec<Centroid>,
    pub held: Vec<HeldOut>,
}

/// Deterministically split games, fit on the first half, hold out the second.
///
/// Games alternate rather than being shuffled: the synthetic corpus is a round
/// robin, so alternating keeps every pairing represented on both sides. A
/// random split would sometimes put all of one match-up in training.
pub fn split(corpus: &Corpus, k: usize) -> Option<Split> {
    let mut train = Corpus::new();
    let mut test = Vec::new();
    for (i, g) in corpus.games.iter().enumerate() {
        if i % 2 == 0 {
            train.push(g.clone());
        } else {
            test.push(g.clone());
        }
    }
    let lab = Lab::fit(train, k)?;

    let mut centroids = Vec::new();
    for name in lab.corpus.players() {
        if let Some(p) = lab.profile(&name) {
            centroids.push(Centroid {
                player: name,
                coords: p.coords,
                games: p.games,
            });
        }
    }

    // Project the held-out games under the *training* basis.
    let mut held: Vec<HeldOut> = centroids
        .iter()
        .map(|c| HeldOut {
            player: c.player.clone(),
            points: Vec::new(),
        })
        .collect();
    for g in &test {
        for side in [Color::White, Color::Black] {
            let who = if side == Color::White {
                &g.white
            } else {
                &g.black
            };
            let Some(f) = features::extract(&g.start, &g.moves, side) else {
                continue;
            };
            let coords = lab.basis.project(&f);
            if let Some(slot) = held.iter_mut().find(|h| &h.player == who) {
                slot.points.push(coords);
            }
        }
    }
    Some(Split {
        lab,
        centroids,
        held,
    })
}

/// Nearest centroid, with the margin to the runner-up.
///
/// The margin matters as much as the guess: a correct attribution that only
/// just beat second place is not evidence a personality is distinctive.
pub fn attribute(centroids: &[Centroid], coords: &[f64]) -> Option<(String, f64)> {
    if centroids.len() < 2 {
        return None;
    }
    let mut best = (f64::INFINITY, String::new());
    let mut second = f64::INFINITY;
    for c in centroids {
        let d = distance(coords, &c.coords);
        if d < best.0 {
            second = best.0;
            best = (d, c.player.clone());
        } else if d < second {
            second = d;
        }
    }
    Some((best.1, second - best.0))
}

/// One point on the convergence curve.
#[derive(Clone, Debug)]
pub struct Convergence {
    /// How many of a player's games the attacker's observer has seen.
    pub games: usize,
    pub accuracy: f64,
    pub trials: usize,
}

/// How many games does it take to re-identify someone?
///
/// For each `n` up to `max_games`, repeatedly draw `n` held-out games from one
/// player, average them into a provisional profile, and attribute it. The curve
/// this traces is the answer to *"can I start a fresh account?"* — the number of
/// games before the new name is provably the old one.
pub fn convergence(split: &Split, max_games: usize, trials: usize, seed: u64) -> Vec<Convergence> {
    let mut rng = Rng::new(seed);
    let mut out = Vec::new();

    for n in 1..=max_games {
        let (mut hits, mut total) = (0usize, 0usize);
        for _ in 0..trials {
            for h in &split.held {
                if h.points.len() < n {
                    continue;
                }
                // Sample n distinct games by partial Fisher-Yates over indices.
                let mut idx: Vec<usize> = (0..h.points.len()).collect();
                for i in 0..n {
                    let j = i + (rng.next_u64() as usize) % (idx.len() - i);
                    idx.swap(i, j);
                }
                let k = h.points[0].len();
                let mut mean = vec![0.0; k];
                for &i in &idx[..n] {
                    for (d, m) in mean.iter_mut().enumerate() {
                        *m += h.points[i][d];
                    }
                }
                for m in mean.iter_mut() {
                    *m /= n as f64;
                }
                if let Some((guess, _)) = attribute(&split.centroids, &mean) {
                    total += 1;
                    if guess == h.player {
                        hits += 1;
                    }
                }
            }
        }
        out.push(Convergence {
            games: n,
            accuracy: if total > 0 {
                hits as f64 / total as f64
            } else {
                0.0
            },
            trials: total,
        });
    }
    out
}

/// Rows are truth, columns are guess, in `centroids` order.
pub fn confusion(split: &Split) -> Vec<Vec<usize>> {
    let n = split.centroids.len();
    let mut m = vec![vec![0usize; n]; n];
    for (row, h) in split.held.iter().enumerate() {
        for p in &h.points {
            if let Some((guess, _)) = attribute(&split.centroids, p) {
                if let Some(col) = split.centroids.iter().position(|c| c.player == guess) {
                    m[row][col] += 1;
                }
            }
        }
    }
    m
}

/// The closest pair of players, and how many quantisation cells separate them.
///
/// This is the medal's forgery margin. Two players whose centroids fall in the
/// same cell mint **the same medal**, and no amount of cryptography helps:
/// the collision happened before the hash. A margin of one cell is not
/// security, it is a rounding accident.
pub fn closest_pair(centroids: &[Centroid]) -> Option<(String, String, f64, f64)> {
    let mut best: Option<(String, String, f64)> = None;
    for (i, a) in centroids.iter().enumerate() {
        for b in &centroids[i + 1..] {
            let d = distance(&a.coords, &b.coords);
            if best.as_ref().is_none_or(|(_, _, bd)| d < *bd) {
                best = Some((a.player.clone(), b.player.clone(), d));
            }
        }
    }
    best.map(|(a, b, d)| (a, b, d, d / STEP))
}

/// How much of personality space the corpus actually occupies, per axis, in
/// quantisation cells.
///
/// The number of distinct medals is bounded by the product of these. It is an
/// **upper** bound on distinctiveness and a weak one: it counts cells the cloud
/// spans, not cells anybody occupies, and says nothing about how the population
/// is distributed inside them. Reported because a bound that is honest about
/// being loose beats a precise number that is wrong.
pub fn occupied_cells(basis: &Basis, points: &[Vec<f64>]) -> Vec<f64> {
    (0..basis.k)
        .map(|axis| {
            let vals: Vec<f64> = points.iter().map(|p| p[axis]).collect();
            let lo = vals.iter().cloned().fold(f64::INFINITY, f64::min);
            let hi = vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            if lo.is_finite() && hi.is_finite() {
                (hi - lo) / STEP
            } else {
                0.0
            }
        })
        .collect()
}

/// How strongly one discovered axis tracks the players' ratings.
#[derive(Clone, Debug)]
pub struct AxisStrength {
    pub axis: usize,
    /// Pearson correlation between position on this axis and rating.
    pub r: f64,
    /// Game-sides that carried a rating.
    pub rated: usize,
}

/// Is the basis just rating in disguise?
///
/// This is the first question a careful reader should ask, and the one that can
/// quietly sink the whole idea. Strength is a real, large, already-named axis of
/// chess. If the "personality" axes turn out to track it, then the medal is an
/// Elo rating with extra steps and every claim about *style* is a claim about
/// *skill* wearing a different word.
///
/// So: correlate each axis against the rating recorded with the game. A high
/// `|r|` on an axis means that axis is measuring strength. A low `|r|`
/// everywhere is the result the project needs and has not yet earned — the
/// synthetic archetypes carry no ratings, so this returns nothing until it is
/// pointed at real games.
///
/// Note what it cannot tell you: a *low* correlation does not prove the axes
/// measure personality, only that they do not measure the one confounder we can
/// name. Ruling out the obvious rival is the least a measurement can do, not
/// the most.
pub fn strength_leakage(lab: &Lab) -> Vec<AxisStrength> {
    let mut pairs: Vec<(Vec<f64>, f64)> = Vec::new();
    for p in &lab.points {
        let Some(g) = lab.corpus.games.get(p.game) else {
            continue;
        };
        let elo = if p.player == g.white {
            g.white_elo
        } else {
            g.black_elo
        };
        if let Some(e) = elo {
            pairs.push((p.coords.clone(), e as f64));
        }
    }
    if pairs.len() < 3 {
        return Vec::new();
    }

    let n = pairs.len() as f64;
    let mean_y = pairs.iter().map(|(_, y)| y).sum::<f64>() / n;
    let sd_y = (pairs.iter().map(|(_, y)| (y - mean_y).powi(2)).sum::<f64>() / n).sqrt();

    (0..lab.basis.k)
        .map(|axis| {
            let mean_x = pairs.iter().map(|(x, _)| x[axis]).sum::<f64>() / n;
            let sd_x = (pairs
                .iter()
                .map(|(x, _)| (x[axis] - mean_x).powi(2))
                .sum::<f64>()
                / n)
                .sqrt();
            let cov = pairs
                .iter()
                .map(|(x, y)| (x[axis] - mean_x) * (y - mean_y))
                .sum::<f64>()
                / n;
            AxisStrength {
                axis,
                r: if sd_x > 0.0 && sd_y > 0.0 {
                    cov / (sd_x * sd_y)
                } else {
                    0.0
                },
                rated: pairs.len(),
            }
        })
        .collect()
}
