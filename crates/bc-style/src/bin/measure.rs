//! D20, run on humans.
//!
//! ```sh
//! cargo run --release -p bc-style --bin measure -- <corpus-dir>
//! ```
//!
//! The corpus is `rozim/ChessData`'s `mega2600_part_*.pgn`: 138,000 games,
//! every player 2600+. `π_pop` is therefore the **master** population, not
//! the general one, which makes this a *conservative* test — masters resemble
//! each other far more than a mixed-rating field does, so divergence measured
//! here should come out smaller than `spec/10` §7 imagines. Clearing the
//! threshold on this corpus is stronger evidence than clearing it on Lichess.
//!
//! Splits are by game and assigned before anything is fitted: `π_pop` never
//! sees a test game, and each player's regularisation is chosen on validation
//! games they are not scored on.

use bc_chess::Position;
use bc_style::model::{
    cross_entropy, fit_personality, samples_from_game, train, Sample, TrainCfg, Weights,
};
use bc_style::pgn::{self, Corpus};
use bc_style::verdict::{Estimate, Verdict};
use std::collections::BTreeMap;

/// Games a player needs before the calibration says their style is visible.
const MIN_GAMES: usize = 200;
/// How many players to measure by default. The rest still populate `π_pop`.
const N_PLAYERS: usize = 40;
/// Cap per player, so the heavy hitters do not dominate the population model.
const MAX_GAMES: usize = 400;
/// Games used to fit `π_pop`.
const POP_GAMES: usize = 6_000;

const LAMBDAS: [f32; 5] = [1e-1, 3e-2, 1e-2, 3e-3, 1e-3];

/// train / val / test, assigned per game so nothing leaks across a split.
fn split_of(i: usize) -> usize {
    match i % 5 {
        0..=2 => 0,
        3 => 1,
        _ => 2,
    }
}

/// A player's own decisions: the plies where it was their turn.
fn my_samples(g: &pgn::Game, me_is_white: bool, start: &Position) -> Vec<Sample> {
    let Some(all) = samples_from_game(start, &g.moves) else {
        return Vec::new();
    };
    all.into_iter()
        .enumerate()
        .filter(|(ply, _)| (ply % 2 == 0) == me_is_white)
        .map(|(_, s)| s)
        .collect()
}

fn main() {
    let dir = std::env::args()
        .nth(1)
        .expect("usage: measure <corpus-dir> [n_players]");
    let n_players: usize = std::env::args()
        .nth(2)
        .and_then(|a| a.parse().ok())
        .unwrap_or(N_PLAYERS);
    let start = Position::startpos();

    let mut corpus = Corpus::default();
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .expect("corpus directory")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "pgn"))
        .collect();
    files.sort();
    for f in &files {
        pgn::parse(
            &std::fs::read_to_string(f).unwrap_or_default(),
            &start,
            &mut corpus,
        );
    }
    println!("D20 — measuring D(pi_you || pi_pop) on humans\n");
    println!(
        "  corpus     {} games from {} files",
        corpus.games.len(),
        files.len()
    );
    println!("  skipped    {:?}", corpus.skipped);

    // --- who gets measured -------------------------------------------------
    let by_player = corpus.by_player();
    let mut ranked: Vec<(usize, &str)> = by_player
        .iter()
        .map(|(k, v)| (v.len(), *k))
        .filter(|(n, _)| *n >= MIN_GAMES)
        .collect();
    ranked.sort_unstable_by(|a, b| b.cmp(a));
    println!(
        "  players    {} with >={MIN_GAMES} games; measuring top {n_players}",
        ranked.len()
    );
    ranked.truncate(n_players);

    // --- the population model ----------------------------------------------
    // Every ply of a spread of training games, both sides. This is "how the
    // population plays", and it is the thing each player is measured against.
    let mut pop_samples = Vec::new();
    let stride = (corpus.games.len() / POP_GAMES).max(1);
    for (i, g) in corpus.games.iter().enumerate() {
        if i % stride != 0 || split_of(i) != 0 {
            continue;
        }
        if let Some(s) = samples_from_game(&start, &g.moves) {
            pop_samples.extend(s);
        }
    }
    println!("  pi_pop     {} decisions", pop_samples.len());
    let t = std::time::Instant::now();
    let pop = train(
        &pop_samples,
        &Weights::zeros(),
        &TrainCfg {
            epochs: 6,
            lr: 0.05,
            l2: 1e-5,
        },
    );
    println!(
        "             trained in {:.0}s\n",
        t.elapsed().as_secs_f32()
    );

    // --- per player ---------------------------------------------------------
    println!(
        "  {:<26} {:>6} {:>7} {:>9} {:>9} {:>9}",
        "player", "games", "test", "D_raw", "D_true", "lambda"
    );
    println!("  {}", "-".repeat(72));

    let mut deltas: BTreeMap<&str, Weights> = BTreeMap::new();
    let mut test_by_player: BTreeMap<&str, Vec<Sample>> = BTreeMap::new();
    let mut raws = Vec::new();

    for &(n_games, name) in &ranked {
        let idxs = &by_player[name];
        let (mut tr, mut va, mut te) = (Vec::new(), Vec::new(), Vec::new());
        for (k, &gi) in idxs.iter().take(MAX_GAMES).enumerate() {
            let g = &corpus.games[gi];
            let mine = my_samples(g, g.white == name, &start);
            match split_of(k) {
                0 => tr.extend(mine),
                1 => va.extend(mine),
                _ => te.extend(mine),
            }
        }
        if tr.len() < 500 || va.is_empty() || te.is_empty() {
            continue;
        }
        let fit = fit_personality(&tr, &va, &pop, &LAMBDAS, 6, 0.05);
        let raw = cross_entropy(&te, &pop.0) - cross_entropy(&te, &pop.plus(&fit.delta).0);
        let est = Estimate::from_raw(raw);
        println!(
            "  {:<26} {:>6} {:>7} {:>9.4} {:>9.4} {:>9}",
            name,
            n_games.min(MAX_GAMES),
            te.len(),
            raw,
            est.corrected,
            fit.lambda
                .map_or("none".to_string(), |l| format!("{l:.0e}"))
        );
        raws.push(raw);
        test_by_player.insert(name, te);
        deltas.insert(name, fit.delta);
    }

    // --- the number ----------------------------------------------------------
    raws.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mean: f64 = raws.iter().sum::<f64>() / raws.len() as f64;
    let median = raws[raws.len() / 2];
    let est = Estimate::from_raw(mean);

    println!("\n  players measured   {}", raws.len());
    println!(
        "  D_raw    mean {:.4}   median {:.4}   min {:.4}   max {:.4} nats/move",
        mean,
        median,
        raws[0],
        raws[raws.len() - 1]
    );
    println!(
        "  D_true   {:.4} nats/move   (recovery-corrected point estimate)",
        est.corrected
    );
    println!(
        "           [{:.4}, {:.4}]   (calibration recovery interval)",
        est.interval.0, est.interval.1
    );

    // --- identification ------------------------------------------------------
    // Each player's held-out decisions, scored under every candidate delta.
    let names: Vec<&str> = deltas.keys().copied().collect();
    let scored: Vec<(&str, Weights)> = names.iter().map(|n| (*n, pop.plus(&deltas[n]))).collect();
    let mut correct = 0;
    for name in &names {
        let te = &test_by_player[name];
        let best = scored
            .iter()
            .min_by(|a, b| {
                cross_entropy(te, &a.1 .0)
                    .partial_cmp(&cross_entropy(te, &b.1 .0))
                    .unwrap()
            })
            .unwrap();
        if best.0 == *name {
            correct += 1;
        }
    }
    let chance = 100.0 / names.len() as f64;
    println!(
        "  identification     {}/{} = {:.1}%   (chance {:.1}%)",
        correct,
        names.len(),
        100.0 * correct as f64 / names.len() as f64,
        chance
    );

    // --- the pre-registered verdict -------------------------------------------
    println!("\n  ================================================");
    println!("  VERDICT: {}", est.verdict());
    println!("  ================================================");
    println!(
        "  Cuts are on true D: holds >= {:.3}, dead < {:.3} (spec/09 D20).",
        bc_style::verdict::THESIS_HOLDS,
        bc_style::verdict::ACT_II_DEAD
    );
    if !est.unambiguous() {
        println!("  NOTE: the recovery interval spans a threshold — this verdict");
        println!("  depends on where in the calibrated range these players sit.");
    }
    if est.verdict() == Verdict::Dead {
        println!("  spec/10 Act II should be abandoned rather than rescued.");
    }
}
