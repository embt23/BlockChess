//! Finding chunks: groups of squares that keep turning up together.
//!
//! `papers/09-lineage.md` §5 records a limitation that this file exists to
//! remove. Chase & Simon's chunks are **spatial** — a castled king with its
//! pawns, a familiar structure — and `bc-grammar` induces over **move
//! sequences**, which is a temporal object. Related, but not the same thing,
//! and the paper says the psychology claim should not be made until the
//! spatial version works.
//!
//! # What a chunk is here
//!
//! Treat a position as a set of facts: "white king on g1", "white pawn on f2".
//! There are 768 possible facts (12 kinds × 64 squares). A **chunk** is a set
//! of facts that occurs together far more often than if the facts were
//! independent.
//!
//! Independence is the whole point. "White king on g1" is common and "white
//! pawn on f2" is common, so seeing both is common too — that says nothing.
//! The question is whether they occur together *more* than the product of
//! their separate rates. The measure is **lift**:
//!
//! ```text
//! lift(S) = P(all of S together) / Π P(each fact alone)
//! ```
//!
//! computed in logs, because the product underflows at four or five facts.
//! Lift of 1 is exactly chance. Lift of 50 means the group appears fifty times
//! more often than independent pieces landing that way would explain.
//!
//! # Why this is not circular
//!
//! Nothing here is told what a pawn shield is, that castling exists, or that
//! g1 is where a king goes. It counts co-occurrences of anonymous
//! (piece, square) facts. If it returns the castled kingside, it found it.
//!
//! # The honest limits
//!
//! **Exact squares, no slots.** A chunk here is a specific set of squares.
//! Gobet & Simon's templates have *variable* slots — "this structure, with
//! that square either occupied or not" — and nothing here can express one. So
//! the same expectation as `papers/11-results.md` §3c applies: this recovers
//! rigid configurations and misses flexible schemas.
//!
//! **Frequency is not meaning.** The opening position is the most frequent
//! configuration in any corpus and is not an idea. Sampling starts past the
//! opening for that reason, and even then a high-lift group may be a
//! consequence of the rules rather than a concept a player holds.

use std::collections::HashMap;

use bc_chess::bitboard::Squares;
use bc_chess::{Color, Piece, Position, Square};

/// A (colour, piece, square) fact, packed into `0..768`.
pub type Fact = u16;

pub fn fact(c: Color, p: Piece, s: Square) -> Fact {
    ((c.idx() * 6 + p.idx()) as u16) * 64 + s as u16
}

pub fn unfact(f: Fact) -> (Color, Piece, Square) {
    let sq = (f % 64) as Square;
    let kind = (f / 64) as usize;
    let c = if kind < 6 { Color::White } else { Color::Black };
    (c, Piece::from_idx(kind % 6), sq)
}

pub fn fact_name(f: Fact) -> String {
    let (c, p, s) = unfact(f);
    format!(
        "{}{}{}",
        if c == Color::White { "" } else { "…" },
        p.ch().to_ascii_uppercase(),
        crate::square_name(s)
    )
}

/// The facts true of a position.
pub fn facts_of(pos: &Position) -> Vec<Fact> {
    let mut out = Vec::with_capacity(32);
    for sq in Squares(pos.occupied()) {
        if let Some((c, p)) = pos.piece_at(sq) {
            out.push(fact(c, p, sq));
        }
    }
    out
}

const N_FACTS: usize = 768;

/// Co-occurrence counts over a sample of positions.
pub struct Counts {
    pub positions: u64,
    pub single: Vec<u64>,
    pub pair: HashMap<(Fact, Fact), u32>,
    /// Every sampled position's fact set, kept so a candidate group's support
    /// can be counted exactly rather than estimated from pairs.
    pub sample: Vec<Vec<Fact>>,
}

impl Default for Counts {
    fn default() -> Self {
        Counts {
            positions: 0,
            single: vec![0; N_FACTS],
            pair: HashMap::new(),
            sample: Vec::new(),
        }
    }
}

impl Counts {
    /// Add a position. `keep` decides whether its fact set is retained for
    /// exact support counting later — retaining every one is what costs
    /// memory, so the caller can sample.
    pub fn observe(&mut self, pos: &Position, keep: bool) {
        let f = facts_of(pos);
        self.positions += 1;
        for &a in &f {
            self.single[a as usize] += 1;
        }
        for i in 0..f.len() {
            for j in (i + 1)..f.len() {
                let k = if f[i] < f[j] {
                    (f[i], f[j])
                } else {
                    (f[j], f[i])
                };
                *self.pair.entry(k).or_insert(0) += 1;
            }
        }
        if keep {
            self.sample.push(f);
        }
    }

    fn p_single(&self, f: Fact) -> f64 {
        self.single[f as usize] as f64 / self.positions.max(1) as f64
    }

    /// How many retained positions contain every fact in `set`.
    fn support(&self, set: &[Fact]) -> u64 {
        self.sample
            .iter()
            .filter(|fs| set.iter().all(|f| fs.contains(f)))
            .count() as u64
    }

    /// `log2` of the lift of a group: how much more often it occurs than
    /// independent facts landing that way would explain.
    fn log_lift(&self, set: &[Fact], support: u64) -> f64 {
        if support == 0 || self.sample.is_empty() {
            return f64::NEG_INFINITY;
        }
        let p_joint = support as f64 / self.sample.len() as f64;
        let indep: f64 = set
            .iter()
            .map(|&f| self.p_single(f).max(1e-12).log2())
            .sum();
        p_joint.log2() - indep
    }
}

/// A discovered chunk.
#[derive(Debug, Clone)]
pub struct Chunk {
    pub facts: Vec<Fact>,
    pub support: u64,
    pub log_lift: f64,
}

impl Chunk {
    pub fn lift(&self) -> f64 {
        self.log_lift.exp2()
    }
}

/// Grow chunks greedily from the most-lifted pairs.
///
/// Seeds are pairs that clear `min_support` and have the highest lift; each is
/// then extended, one fact at a time, by whichever addition maximises lift
/// while keeping support above the floor. Greedy rather than exhaustive
/// because the exhaustive version is the frequent-itemset problem and does not
/// finish — and because a greedy chain is legible, which is the point of
/// preferring this to a neural model in the first place.
pub fn discover(counts: &Counts, min_support: u64, max_size: usize, want: usize) -> Vec<Chunk> {
    // Seed pairs, best lift first.
    let mut seeds: Vec<(f64, (Fact, Fact), u64)> = counts
        .pair
        .iter()
        .filter(|(_, &c)| c as u64 >= min_support)
        .filter_map(|(&(a, b), &c)| {
            let sup = c as u64;
            let p_joint = sup as f64 / counts.positions.max(1) as f64;
            let lift = p_joint.log2()
                - counts.p_single(a).max(1e-12).log2()
                - counts.p_single(b).max(1e-12).log2();
            if lift.is_finite() {
                Some((lift, (a, b), sup))
            } else {
                None
            }
        })
        .collect();
    seeds.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap());

    let mut out: Vec<Chunk> = Vec::new();
    let mut seen: Vec<Vec<Fact>> = Vec::new();

    for (_, (a, b), _) in seeds.into_iter().take(want * 40) {
        let mut set = vec![a, b];
        let mut sup = counts.support(&set);
        if sup < min_support {
            continue;
        }
        let mut lift = counts.log_lift(&set, sup);

        // Extend while it helps. Candidates come from facts that co-occur with
        // everything already in the set, which keeps the search small.
        while set.len() < max_size {
            let mut best: Option<(f64, Fact, u64)> = None;
            let candidates: Vec<Fact> = (0..N_FACTS as Fact)
                .filter(|f| !set.contains(f) && counts.single[*f as usize] >= min_support)
                .collect();
            for c in candidates {
                let mut trial = set.clone();
                trial.push(c);
                let s = counts.support(&trial);
                if s < min_support {
                    continue;
                }
                let l = counts.log_lift(&trial, s);
                if l > lift && best.map(|(bl, _, _)| l > bl).unwrap_or(true) {
                    best = Some((l, c, s));
                }
            }
            match best {
                Some((l, c, s)) => {
                    set.push(c);
                    lift = l;
                    sup = s;
                }
                None => break,
            }
        }

        set.sort_unstable();
        // Drop a group that is a subset of one already found — the greedy walk
        // reaches the same structure from several seed pairs.
        if seen.iter().any(|prev| set.iter().all(|f| prev.contains(f))) {
            continue;
        }
        seen.push(set.clone());
        out.push(Chunk {
            facts: set,
            support: sup,
            log_lift: lift,
        });
        if out.len() >= want {
            break;
        }
    }

    out.sort_by(|a, b| b.log_lift.partial_cmp(&a.log_lift).unwrap());
    out
}
