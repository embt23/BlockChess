//! A log-linear (maximum-entropy) policy over legal moves.
//!
//! ```text
//!   P(m | pos) =  exp(w · f(pos, m))  /  Σ    exp(w · f(pos, m'))
//!                                        m' legal
//! ```
//!
//! The normalisation runs over the **legal moves of this position only**, which
//! is what makes this a chess model rather than a generic classifier: the rules
//! supply the support, the weights supply the preference. `bc-chess` is the
//! feature extractor, and it is the same move generator `perft` validated.
//!
//! Personality is expressed as an additive offset:
//!
//! ```text
//!   π_pop  = softmax( w          · f )
//!   π_you  = softmax( (w + δ_you) · f )
//! ```
//!
//! `δ` is regularised toward zero, so a player with few games stays close to
//! the population and only pulls away where the evidence is real. That makes
//! `δ` literally "how you differ", which is the quantity `spec/10` is about.

use crate::features::{extract_all, Feats, N_FEATURES};
use bc_chess::{Move, Position};

/// Weights over the feature space.
#[derive(Clone, Debug)]
pub struct Weights(pub Vec<f32>);

impl Default for Weights {
    fn default() -> Self {
        Weights(vec![0.0; N_FEATURES])
    }
}

impl Weights {
    pub fn zeros() -> Weights {
        Weights::default()
    }

    /// Sum of two weight vectors — used to form `w_pop + δ`.
    pub fn plus(&self, other: &Weights) -> Weights {
        Weights(self.0.iter().zip(&other.0).map(|(a, b)| a + b).collect())
    }

    /// Euclidean norm. A proxy for "how much personality".
    pub fn norm(&self) -> f32 {
        self.0.iter().map(|x| x * x).sum::<f32>().sqrt()
    }
}

/// Softmax over the supplied moves. Returns probabilities in the same order.
pub fn policy(feats: &[Feats], w: &[f32]) -> Vec<f32> {
    let mut s: Vec<f32> = feats.iter().map(|f| f.score(w)).collect();
    // Subtract the max before exponentiating, or a large score overflows to inf
    // and the whole distribution becomes NaN.
    let max = s.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let mut total = 0.0f32;
    for x in s.iter_mut() {
        *x = (*x - max).exp();
        total += *x;
    }
    for x in s.iter_mut() {
        *x /= total;
    }
    s
}

/// One observed decision: a position, its legal moves, and which was played.
pub struct Sample {
    pub feats: Vec<Feats>,
    pub chosen: usize,
}

/// Turn a game — a start position and the moves played — into decisions.
///
/// Returns `None` if any move is not legal in its position, which is the check
/// that makes a PGN parse self-validating: a misparse produces an illegal move
/// within a ply or two.
pub fn samples_from_game(start: &Position, moves: &[Move]) -> Option<Vec<Sample>> {
    let mut pos = *start;
    let mut out = Vec::with_capacity(moves.len());
    for &m in moves {
        let legal = pos.generate_legal();
        let idx = legal.as_slice().iter().position(|&x| x == m)?;
        out.push(Sample {
            feats: extract_all(&pos, legal.as_slice()),
            chosen: idx,
        });
        pos = pos.make_move(m);
    }
    Some(out)
}

/// Mean negative log-likelihood, in **nats per move**.
///
/// This is the cross-entropy of the player's true behaviour under the model.
/// Every number this crate reports is a difference of two of these.
pub fn cross_entropy(samples: &[Sample], w: &[f32]) -> f64 {
    if samples.is_empty() {
        return f64::NAN;
    }
    let mut total = 0.0f64;
    for s in samples {
        let p = policy(&s.feats, w);
        total -= (p[s.chosen].max(1e-12) as f64).ln();
    }
    total / samples.len() as f64
}

/// Training configuration.
pub struct TrainCfg {
    pub epochs: usize,
    pub lr: f32,
    /// L2 pull toward the starting point. For `δ` this is a pull toward zero,
    /// i.e. toward "no personality" — so weak evidence yields no claim.
    pub l2: f32,
}

impl Default for TrainCfg {
    fn default() -> Self {
        TrainCfg {
            epochs: 8,
            lr: 0.05,
            l2: 1e-4,
        }
    }
}

/// Fit weights by stochastic gradient ascent on the log-likelihood.
///
/// The gradient of a log-linear model is the cleanest in machine learning:
///
/// ```text
///   ∇ log P(m* | pos)  =  f(m*)  −  E    [ f(m) ]
///                                   m~P
/// ```
///
/// observed features minus expected features. At the optimum the model's
/// expected feature counts match the empirical ones — which is exactly the
/// maximum-entropy condition, and the reason this family is called that.
pub fn train(samples: &[Sample], init: &Weights, cfg: &TrainCfg) -> Weights {
    let mut w = init.0.clone();
    let base = init.0.clone();
    let mut order: Vec<usize> = (0..samples.len()).collect();
    let mut rng = Lcg::new(0x5EED_1234_9E37_79B9);

    for epoch in 0..cfg.epochs {
        // Shuffle each epoch: consecutive samples come from the same game and
        // are highly correlated, which makes unshuffled SGD oscillate.
        for i in (1..order.len()).rev() {
            order.swap(i, (rng.next() % (i as u64 + 1)) as usize);
        }
        // Decay the step size so later epochs refine rather than thrash.
        let lr = cfg.lr / (1.0 + epoch as f32);

        for &si in &order {
            let s = &samples[si];
            let p = policy(&s.feats, &w);

            // − E[f]
            for (mi, f) in s.feats.iter().enumerate() {
                let pm = p[mi];
                if pm > 1e-6 {
                    for &fi in f.as_slice() {
                        w[fi as usize] -= lr * pm;
                    }
                }
            }
            // + f(observed)
            for &fi in s.feats[s.chosen].as_slice() {
                w[fi as usize] += lr;
            }
            // Pull back toward the starting point, over *every* weight — not
            // just the ones this sample touched. Shrinking only touched
            // features leaves rarely-chosen ones un-regularised, and those are
            // exactly the ones that overfit.
            if cfg.l2 > 0.0 {
                let k = lr * cfg.l2;
                for (wi, bi) in w.iter_mut().zip(&base) {
                    *wi -= k * (*wi - *bi);
                }
            }
        }
    }
    Weights(w)
}

/// The result of fitting a personality, including the regularisation strength
/// the data actually supported.
pub struct Fit {
    pub delta: Weights,
    /// `None` means no personality was detectable — the population model, left
    /// alone, beat every adapted model on held-out data.
    pub lambda: Option<f32>,
    pub val_ce: f64,
}

/// Fit `δ`, choosing the regularisation strength on a validation split.
///
/// The candidate set always includes **no personality at all** (`δ = 0`). That
/// single detail is what makes the estimator honest: a player whose games do
/// not support a distinctive model gets `δ = 0` and measures exactly zero,
/// rather than a small positive number manufactured from noise.
///
/// Without it, the measurement is biased upward by construction, because a
/// model fitted on your games always fits your games better.
pub fn fit_personality(
    train_s: &[Sample],
    val: &[Sample],
    pop: &Weights,
    lambdas: &[f32],
    epochs: usize,
    lr: f32,
) -> Fit {
    let mut best = Fit {
        delta: Weights::zeros(),
        lambda: None,
        val_ce: cross_entropy(val, &pop.0),
    };
    for &l2 in lambdas {
        let d = train_delta(train_s, pop, &TrainCfg { epochs, lr, l2 });
        let ce = cross_entropy(val, &pop.plus(&d).0);
        if ce < best.val_ce {
            best = Fit {
                delta: d,
                lambda: Some(l2),
                val_ce: ce,
            };
        }
    }
    best
}

/// Fit a personality offset `δ` on top of a frozen population model.
pub fn train_delta(samples: &[Sample], pop: &Weights, cfg: &TrainCfg) -> Weights {
    let fitted = train(samples, pop, cfg);
    Weights(fitted.0.iter().zip(&pop.0).map(|(a, b)| a - b).collect())
}

/// Small deterministic PRNG, so every experiment reproduces exactly.
///
/// `next` is deliberately an inherent method rather than `Iterator::next`: an
/// infinite iterator of random numbers invites `.collect()`, and an experiment
/// that hangs is worse than one that reads slightly unusually.
pub struct Lcg(u64);

#[allow(clippy::should_implement_trait)]
impl Lcg {
    pub fn new(seed: u64) -> Lcg {
        Lcg(seed | 1)
    }
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn unit(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }
    /// Approximately standard normal, by the central limit theorem.
    pub fn normal(&mut self) -> f32 {
        (0..6).map(|_| self.unit()).sum::<f32>() - 3.0
    }
}
