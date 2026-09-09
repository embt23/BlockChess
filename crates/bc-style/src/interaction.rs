//! Episode 18 — a player alone is a fiction.
//!
//! A player on their own is `p(move | position)`. A game is
//! `p(move | position, opponent)`. The gap between them is the **interaction
//! term** (`METAPLAN` N12), and it is the claim that your medal is not purely
//! yours: who you played shaped who you became.
//!
//! # Why the synthetic corpus is the right place to test this
//!
//! On real games the claim is unfalsifiable in the obvious way. If Kasparov's
//! profile differs against Karpov, you cannot tell whether the opponent pulled
//! him or whether he simply changed between those years. The confound is total.
//!
//! Here it is not, because the archetypes in [`crate::synth`] have **fixed
//! policies**. A `Style` is a constant weight vector; it does not learn, tire,
//! prepare, or have a bad afternoon. So the ground truth is exactly
//! *"personality does not change"* — and any measured shift by opponent must be
//! interaction, since there is nothing else left for it to be.
//!
//! That makes this a controlled experiment rather than an observation, which is
//! the same reason `synth` exists at all.
//!
//! # The test
//!
//! For one player, compare how far their per-opponent centroids sit from their
//! overall centroid (**between**) against how much their individual games
//! scatter within one opponent (**within**). A ratio near 1 means the opponent
//! label carries nothing.
//!
//! Since the null distribution of that ratio is not something to guess at, it
//! is obtained by **permutation**: shuffle the opponent labels among that
//! player's own games and recompute. The p-value is the share of shuffles that
//! reach the observed ratio. No distributional assumption, and it stays valid
//! at these sample sizes.

use crate::linalg::distance;
use crate::synth::Rng;
use crate::{GamePoint, Lab};

/// How much one player's measured style depends on who they are facing.
#[derive(Clone, Debug)]
pub struct PlayerInteraction {
    pub player: String,
    /// Mean distance from per-opponent centroids to the overall centroid.
    pub between: f64,
    /// Mean distance from individual games to their own opponent's centroid.
    pub within: f64,
    pub ratio: f64,
    /// Share of label shuffles reaching the observed ratio.
    pub p_value: f64,
    pub games: usize,
    pub opponents: usize,
}

/// The systematic drag one opponent exerts on everybody who faces them.
#[derive(Clone, Debug)]
pub struct Pull {
    pub opponent: String,
    /// Mean over all players of `centroid(player vs opponent) − centroid(player)`.
    pub vector: Vec<f64>,
    pub magnitude: f64,
    pub players: usize,
}

fn opponent_of(lab: &Lab, p: &GamePoint) -> Option<String> {
    let g = lab.corpus.games.get(p.game)?;
    Some(if p.player == g.white {
        g.black.clone()
    } else {
        g.white.clone()
    })
}

fn centroid(points: &[&Vec<f64>]) -> Vec<f64> {
    let k = points.first().map(|p| p.len()).unwrap_or(0);
    let mut c = vec![0.0; k];
    for p in points {
        for (i, v) in c.iter_mut().enumerate() {
            *v += p[i];
        }
    }
    for v in c.iter_mut() {
        *v /= points.len() as f64;
    }
    c
}

/// Bucket a player's games by opponent label.
///
/// A free function rather than a closure: the borrow of the coordinate vectors
/// outlives the borrow of the label slice, and elision inside a closure cannot
/// express that.
fn group_by<'a>(
    labels: &[usize],
    tagged: &[(String, &'a Vec<f64>)],
    groups: usize,
) -> Vec<Vec<&'a Vec<f64>>> {
    let mut g: Vec<Vec<&'a Vec<f64>>> = vec![Vec::new(); groups];
    for (i, l) in labels.iter().enumerate() {
        g[*l].push(tagged[i].1);
    }
    g
}

/// Between- and within-opponent spread for one grouping of a player's games.
fn spread(groups: &[Vec<&Vec<f64>>]) -> (f64, f64) {
    let all: Vec<&Vec<f64>> = groups.iter().flatten().copied().collect();
    if all.is_empty() {
        return (0.0, 0.0);
    }
    let overall = centroid(&all);

    let mut between = 0.0;
    let mut within = 0.0;
    let mut within_n = 0usize;
    let mut between_n = 0usize;
    for g in groups {
        if g.is_empty() {
            continue;
        }
        let c = centroid(g);
        between += distance(&c, &overall);
        between_n += 1;
        for p in g {
            within += distance(p, &c);
            within_n += 1;
        }
    }
    (
        if between_n > 0 {
            between / between_n as f64
        } else {
            0.0
        },
        if within_n > 0 {
            within / within_n as f64
        } else {
            0.0
        },
    )
}

/// Measure the interaction term for every player, with a permutation test.
///
/// `permutations` shuffles per player. 200 is enough to resolve p to 0.005,
/// which is finer than any claim worth making from four constructed players.
pub fn analyse(lab: &Lab, permutations: usize, seed: u64) -> Vec<PlayerInteraction> {
    let mut rng = Rng::new(seed);
    let mut out = Vec::new();

    for name in lab.corpus.players() {
        // This player's games, tagged by who they faced.
        let mut tagged: Vec<(String, &Vec<f64>)> = Vec::new();
        for p in &lab.points {
            if p.player == name {
                if let Some(o) = opponent_of(lab, p) {
                    tagged.push((o, &p.coords));
                }
            }
        }
        if tagged.len() < 4 {
            continue;
        }

        let mut names: Vec<String> = Vec::new();
        for (o, _) in &tagged {
            if !names.contains(o) {
                names.push(o.clone());
            }
        }
        if names.len() < 2 {
            continue;
        }

        let labels: Vec<usize> = tagged
            .iter()
            .map(|(o, _)| names.iter().position(|n| n == o).unwrap())
            .collect();
        let (between, within) = spread(&group_by(&labels, &tagged, names.len()));
        let ratio = if within > 0.0 { between / within } else { 0.0 };

        // Null: the opponent label tells you nothing. Shuffle it and see how
        // often chance alone produces a separation this large.
        let mut hits = 0usize;
        let mut shuffled = labels.clone();
        for _ in 0..permutations {
            for i in (1..shuffled.len()).rev() {
                let j = (rng.next_u64() as usize) % (i + 1);
                shuffled.swap(i, j);
            }
            let (b, w) = spread(&group_by(&shuffled, &tagged, names.len()));
            let r = if w > 0.0 { b / w } else { 0.0 };
            if r >= ratio {
                hits += 1;
            }
        }

        out.push(PlayerInteraction {
            player: name,
            between,
            within,
            ratio,
            // Add-one: with zero hits the honest statement is "below 1/(N+1)",
            // never "exactly zero" — no finite number of shuffles shows that.
            p_value: (hits + 1) as f64 / (permutations + 1) as f64,
            games: tagged.len(),
            opponents: names.len(),
        });
    }
    out
}

/// Which opponents systematically drag everyone in the same direction.
///
/// For each opponent, average `centroid(player vs opponent) − centroid(player)`
/// over every player who faced them. A large magnitude means facing them makes
/// people play measurably unlike themselves — the clearest single expression of
/// chess as an ecosystem with interacting parts rather than a set of solo acts.
pub fn pulls(lab: &Lab) -> Vec<Pull> {
    let players = lab.corpus.players();
    let mut out = Vec::new();

    for opp in &players {
        let mut sum: Vec<f64> = Vec::new();
        let mut n = 0usize;
        for p in &players {
            if p == opp {
                continue;
            }
            let mine: Vec<&Vec<f64>> = lab
                .points
                .iter()
                .filter(|x| &x.player == p)
                .map(|x| &x.coords)
                .collect();
            let versus: Vec<&Vec<f64>> = lab
                .points
                .iter()
                .filter(|x| &x.player == p && opponent_of(lab, x).as_ref() == Some(opp))
                .map(|x| &x.coords)
                .collect();
            if mine.is_empty() || versus.is_empty() {
                continue;
            }
            let (base, against) = (centroid(&mine), centroid(&versus));
            if sum.is_empty() {
                sum = vec![0.0; base.len()];
            }
            for (i, v) in sum.iter_mut().enumerate() {
                *v += against[i] - base[i];
            }
            n += 1;
        }
        if n == 0 {
            continue;
        }
        for v in sum.iter_mut() {
            *v /= n as f64;
        }
        let magnitude = sum.iter().map(|v| v * v).sum::<f64>().sqrt();
        out.push(Pull {
            opponent: opp.clone(),
            vector: sum,
            magnitude,
            players: n,
        });
    }
    out.sort_by(|a, b| b.magnitude.partial_cmp(&a.magnitude).unwrap());
    out
}
