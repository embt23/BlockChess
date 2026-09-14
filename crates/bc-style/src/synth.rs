//! Synthetic players with a **known** divergence.
//!
//! Why this exists: a model fitted on a player's own games will always score
//! those games better than a population model will — whether or not that player
//! has any personality at all. Overfitting alone manufactures a positive
//! result. So a measurement of `D(π_you ‖ π_pop)` on real data means nothing
//! until the estimator has been shown to
//!
//!   1. recover a divergence that was put there deliberately, and
//!   2. report ~0 for a player who genuinely has none.
//!
//! Here the true policies are known by construction, so the true divergence can
//! be computed exactly and compared against what the estimator reports. This is
//! `G1` — verify against an oracle you did not write — applied to a measurement
//! rather than to a codec. The oracle is arithmetic.

use crate::features::*;
use crate::model::{policy, samples_from_game, Lcg, Sample, Weights};
use bc_chess::types::{MoveList, Piece};
use bc_chess::{Move, Position};

/// A base policy that produces recognisably chess-like games: prefers winning
/// material, mildly prefers the centre, mildly likes checks, and dislikes
/// walking the king around in the opening.
///
/// It does not need to play *well*. It needs to induce a plausible distribution
/// of positions, so the estimator is calibrated on the kind of data it will
/// actually meet.
pub fn plausible_base() -> Weights {
    let mut w = vec![0.0f32; N_FEATURES];

    // Capturing is good, in proportion to what you take.
    for (i, v) in [1.0f32, 3.0, 3.2, 5.0, 9.0, 0.0].iter().enumerate() {
        w[F_VICTIM + 1 + i] = 0.45 * v;
    }
    w[F_CHECK] = 0.35;
    w[F_PROMO + 3] = 2.0; // queening
    w[F_CASTLE] = 0.6;
    w[F_CASTLE + 1] = 0.4;

    // Centralisation: score each destination by how central it is.
    for sq in 0..64usize {
        let (f, r) = ((sq % 8) as f32, (sq / 8) as f32);
        let central = -((f - 3.5).abs() + (r - 3.5).abs()) * 0.06;
        for p in 0..6 {
            w[F_TO + p * 64 + sq] += central;
        }
        // Advancing is good for pawns, bad for the king early on.
        w[F_TO + Piece::Pawn.idx() * 64 + sq] += r * 0.05;
        w[F_TO + Piece::King.idx() * 64 + sq] -= r * 0.30;
    }
    // Get the minor pieces out.
    w[F_PHASE + Piece::Knight.idx()] = 0.5;
    w[F_PHASE + Piece::Bishop.idx()] = 0.5;
    w[F_PHASE + Piece::King.idx()] = -1.2;

    Weights(w)
}

/// A synthetic player: the base policy plus a known personality offset.
pub struct SynthPlayer {
    pub name: String,
    /// The true offset. Zero means "no personality" — the null control.
    pub delta: Weights,
    /// Cached `base + delta`.
    pub effective: Weights,
}

impl SynthPlayer {
    pub fn new(name: &str, base: &Weights, delta: Weights) -> SynthPlayer {
        SynthPlayer {
            name: name.into(),
            effective: base.plus(&delta),
            delta,
        }
    }

    /// A player whose personality is a random perturbation of a random subset of
    /// features. `sigma` scales how distinctive they are.
    pub fn random(
        name: &str,
        base: &Weights,
        sigma: f32,
        density: f32,
        rng: &mut Lcg,
    ) -> SynthPlayer {
        let mut d = vec![0.0f32; N_FEATURES];
        for slot in d.iter_mut() {
            if rng.unit() < density {
                *slot = rng.normal() * sigma;
            }
        }
        SynthPlayer::new(name, base, Weights(d))
    }

    /// A player with no personality at all: plays exactly the base policy.
    pub fn null(name: &str, base: &Weights) -> SynthPlayer {
        SynthPlayer::new(name, base, Weights::zeros())
    }
}

/// One generated game: who played, and the moves.
pub struct SynthGame {
    pub white: usize,
    pub black: usize,
    pub moves: Vec<Move>,
}

/// Sample a move from a player's true policy.
fn sample_move(pos: &Position, legal: &MoveList, w: &Weights, rng: &mut Lcg) -> Move {
    let feats = extract_all(pos, legal.as_slice());
    let p = policy(&feats, &w.0);
    let mut r = rng.unit();
    for (i, &prob) in p.iter().enumerate() {
        r -= prob;
        if r <= 0.0 {
            return legal.as_slice()[i];
        }
    }
    *legal.as_slice().last().unwrap()
}

/// Play one game between two players.
pub fn play(
    white: usize,
    black: usize,
    players: &[SynthPlayer],
    max_plies: usize,
    rng: &mut Lcg,
) -> SynthGame {
    let mut pos = Position::startpos();
    let mut moves = Vec::new();
    for _ in 0..max_plies {
        let legal = pos.generate_legal();
        if legal.is_empty() || pos.halfmove >= 100 {
            break;
        }
        let who = if moves.len() % 2 == 0 { white } else { black };
        let m = sample_move(&pos, &legal, &players[who].effective, rng);
        moves.push(m);
        pos = pos.make_move(m);
    }
    SynthGame {
        white,
        black,
        moves,
    }
}

/// Generate a corpus: every player faces random opponents, both colours.
pub fn corpus(
    players: &[SynthPlayer],
    games_per_player: usize,
    max_plies: usize,
    rng: &mut Lcg,
) -> Vec<SynthGame> {
    let n = players.len();
    let mut games = Vec::new();
    for i in 0..n {
        for g in 0..games_per_player {
            let mut j = (rng.next() % n as u64) as usize;
            if j == i {
                j = (j + 1) % n;
            }
            let (w, b) = if g % 2 == 0 { (i, j) } else { (j, i) };
            games.push(play(w, b, players, max_plies, rng));
        }
    }
    games
}

/// The **exact** divergence `D(π_player ‖ reference)`, in nats per move,
/// averaged over the positions this player actually faced.
///
/// This is the ground truth the estimator is checked against. It is computable
/// only because the player's policy is known in closed form.
///
/// `reference` must be the **fitted** population model, not the base policy the
/// synthetic world was built from. The estimator measures divergence from the
/// population it can actually observe, and that population is skewed by
/// whichever eccentric players are in it. Scoring against the base instead
/// makes even a personality-free player look divergent — which is a mistake in
/// the ground truth, not in the estimator.
pub fn true_divergence(
    games: &[SynthGame],
    player: usize,
    players: &[SynthPlayer],
    reference: &Weights,
) -> f64 {
    let mut total = 0.0f64;
    let mut count = 0usize;

    for g in games {
        if g.white != player && g.black != player {
            continue;
        }
        let mut pos = Position::startpos();
        for (ply, &m) in g.moves.iter().enumerate() {
            let mover = if ply % 2 == 0 { g.white } else { g.black };
            if mover == player {
                let legal = pos.generate_legal();
                let feats = extract_all(&pos, legal.as_slice());
                let p_you = policy(&feats, &players[player].effective.0);
                let p_pop = policy(&feats, &reference.0);
                for (a, b) in p_you.iter().zip(&p_pop) {
                    if *a > 1e-12 {
                        total += (*a as f64) * ((*a as f64).ln() - (*b as f64).max(1e-12).ln());
                    }
                }
                count += 1;
            }
            pos = pos.make_move(m);
        }
    }
    if count == 0 {
        f64::NAN
    } else {
        total / count as f64
    }
}

/// Every decision made by `player` across `games`.
///
/// Only the player's own moves: a policy generates the moves of the person who
/// holds it, not the positions they were handed.
pub fn samples_for(games: &[SynthGame], player: usize) -> Vec<Sample> {
    let start = Position::startpos();
    let mut out = Vec::new();
    for g in games {
        if g.white != player && g.black != player {
            continue;
        }
        let all = match samples_from_game(&start, &g.moves) {
            Some(s) => s,
            None => continue,
        };
        let first = usize::from(g.black == player);
        for (ply, s) in all.into_iter().enumerate() {
            if ply % 2 == first {
                out.push(s);
            }
        }
    }
    out
}
