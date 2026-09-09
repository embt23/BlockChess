//! Stages 2–4 — standardise, discover the axes, name the poles.
//!
//! The axes are **discovered, not chosen** (`METAPLAN` N6): factorise the
//! corpus and let the natural directions fall out. Each is a signed
//! eigenvector, so each is a *tension* with two meaningful poles rather than a
//! part that you have more or less of (`METAPLAN` N7) — being at the negative
//! end of `sharp ↔ prophylactic` is not the absence of a style, it is the other
//! style.

use crate::features::{D, FEATURE_NAMES};
use crate::linalg::{column_stats, covariance, standardise, symmetric_eigen, Matrix};
use bc_hash::Hash;

/// Samples per dimension below which loadings are unstable.
///
/// Rank is the hard limit (see [`Adequacy`]); this is the practical one. Ten
/// observations per estimated parameter is the usual rule of thumb for factor
/// analysis, and below it the axes move noticeably when you add a game.
pub const SAMPLES_PER_DIM: usize = 10;

/// Whether a corpus can actually support the lens fitted to it.
///
/// A covariance estimated from `n` points has rank at most `n − 1`, so a basis
/// asked for more axes than that returns eigenvectors whose eigenvalues are
/// zero to within rounding — directions the data never spoke about, ordered by
/// floating-point noise. They look exactly like real axes. They have poles,
/// loadings, a share of variance, and they will happily mint a medal.
///
/// This is the same failure as `docs/build-log.md` 02 and 05: a value that is
/// authoritative in form and empty in content. The only defence is to compute
/// the bound and say so.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Adequacy {
    pub samples: usize,
    /// Axes the data can support at all: `min(samples − 1, D)`.
    pub rank: usize,
    /// Axes actually kept.
    pub requested: usize,
    /// Samples needed for [`SAMPLES_PER_DIM`] per dimension.
    pub wanted_samples: usize,
}

impl Adequacy {
    /// Some kept axes are beyond the rank of the data and mean nothing.
    pub fn over_rank(&self) -> bool {
        self.requested > self.rank
    }
    /// Enough axes, but not enough games to place them stably.
    pub fn thin(&self) -> bool {
        self.samples < self.wanted_samples
    }
    pub fn trustworthy(&self) -> bool {
        !self.over_rank() && !self.thin()
    }
    /// One line, or `None` when the corpus is adequate.
    pub fn warning(&self) -> Option<String> {
        if self.over_rank() {
            Some(format!(
                "{} game-sides support at most {} axes, but {} were kept — \
                 axes {}..{} are numerical noise and every medal minted here is meaningless",
                self.samples,
                self.rank,
                self.requested,
                self.rank,
                self.requested - 1
            ))
        } else if self.thin() {
            Some(format!(
                "{} game-sides for {} features — the loadings are unstable below \
                 about {} and will move as games are added",
                self.samples, D, self.wanted_samples
            ))
        } else {
            None
        }
    }
}

/// A fitted lens. Everything needed to place a game in personality space, and
/// nothing that depends on any particular player.
#[derive(Clone, Debug)]
pub struct Basis {
    pub corpus_root: Hash,
    pub k: usize,
    pub mean: Vec<f64>,
    pub dev: Vec<f64>,
    /// `D × k`; column `i` is axis `i`.
    pub vectors: Matrix,
    pub values: Vec<f64>,
    pub samples: usize,
}

impl Basis {
    /// Fit a lens to a corpus of per-side feature vectors.
    pub fn fit(rows: &[[f64; D]], k: usize, corpus_root: Hash) -> Basis {
        assert!((1..=D).contains(&k), "k must be in 1..={D}");
        assert!(!rows.is_empty(), "cannot fit a basis to an empty corpus");

        let m = Matrix::from_rows(&rows.iter().map(|r| r.to_vec()).collect::<Vec<_>>());
        let (mean, dev) = column_stats(&m);
        let z = standardise(&m, &mean, &dev);
        let (values, vectors) = symmetric_eigen(&covariance(&z));

        let mut trimmed = Matrix::zeros(D, k);
        for r in 0..D {
            for c in 0..k {
                trimmed.set(r, c, vectors.get(r, c));
            }
        }
        Basis {
            corpus_root,
            k,
            mean,
            dev,
            vectors: trimmed,
            values: values[..k].to_vec(),
            samples: rows.len(),
        }
    }

    /// Place one game's feature vector in personality space.
    ///
    /// Uses the lens's own `μ` and `σ`, never the sample's — a point is only
    /// meaningful relative to the corpus it was projected under.
    pub fn project(&self, f: &[f64; D]) -> Vec<f64> {
        (0..self.k)
            .map(|c| {
                (0..D)
                    .map(|d| {
                        let z = if self.dev[d] == 0.0 {
                            0.0
                        } else {
                            (f[d] - self.mean[d]) / self.dev[d]
                        };
                        z * self.vectors.get(d, c)
                    })
                    .sum()
            })
            .collect()
    }

    /// Whether this lens rests on enough games to mean anything.
    ///
    /// Always check before quoting a loading, a pole or a medal. The struct is
    /// cheap and the failure it catches is silent.
    pub fn adequacy(&self) -> Adequacy {
        Adequacy {
            samples: self.samples,
            rank: self.samples.saturating_sub(1).min(D),
            requested: self.k,
            wanted_samples: D * SAMPLES_PER_DIM,
        }
    }

    /// Fraction of total variance carried by axis `i`.
    pub fn explained(&self, i: usize) -> f64 {
        let total: f64 = self.values.iter().sum();
        if total <= 0.0 {
            0.0
        } else {
            self.values[i] / total
        }
    }

    /// The two poles of axis `i`, as `(positive_name, negative_name)`.
    ///
    /// An axis is named by its extreme loadings: the feature pulling hardest in
    /// each direction. A human then relabels the pair — `capture_rate ↔
    /// opp_mobility` becomes `tactical ↔ prophylactic`. Naming a pole after a
    /// *player* requires that player's games in the corpus, so it is a property
    /// of the corpus and not of this code.
    pub fn poles(&self, i: usize) -> (&'static str, &'static str) {
        let col = self.vectors.col(i);
        let mut hi = 0usize;
        let mut lo = 0usize;
        for (d, &v) in col.iter().enumerate() {
            if v > col[hi] {
                hi = d;
            }
            if v < col[lo] {
                lo = d;
            }
        }
        (FEATURE_NAMES[hi], FEATURE_NAMES[lo])
    }

    /// The `n` features loading most strongly on axis `i`, strongest first.
    pub fn loadings(&self, i: usize, n: usize) -> Vec<(&'static str, f64)> {
        let mut v: Vec<(&'static str, f64)> = self
            .vectors
            .col(i)
            .into_iter()
            .enumerate()
            .map(|(d, w)| (FEATURE_NAMES[d], w))
            .collect();
        v.sort_by(|a, b| b.1.abs().partial_cmp(&a.1.abs()).unwrap());
        v.truncate(n);
        v
    }
}
