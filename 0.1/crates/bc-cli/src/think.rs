//! `blockchess think` — experiment X11. Do bits track seconds?
//!
//! `papers/10-players.md` §1, from Chase & Simon by way of `09-lineage.md` §5:
//! a master's advantage is *recognition*, and recognition is fast. So a player
//! moves quickly when the position is already in their vocabulary and slowly
//! when it is not, and **time spent is where their own compression failed**.
//!
//! Our encoder prices every move in bits. The clock says how long the human
//! took. This asks whether the two land in the same places.
//!
//! # Two different notions of surprise, and only one is the right one
//!
//! - **`log2(legal moves)`** — how *branchy* the position is. Free to compute
//!   and the wrong measure: a forced mate has almost no branching and enormous
//!   insight, so it scores near zero. Reported as `branch` below, mostly to
//!   show it is not the answer.
//! - **`−log2 P(move | position)` from the corpus** — how *unexpected* the
//!   move was, given what everyone else played there. This is the one the
//!   hypothesis is about, and it needs `--corpus`.
//!
//! # The confounds, and what is done about each
//!
//! `measure/EXPERIMENTS.md` X11 names four. Each would manufacture the
//! correlation on its own, so each is handled rather than hoped about:
//!
//! - **Time pressure.** Under a low clock everything is fast regardless of
//!   recognition. Plies below `--min-clock` seconds are dropped.
//! - **Memorised theory.** An opening move can be instant because it was
//!   learned, not understood — cheap in bits *and* fast in seconds for a
//!   reason that is not the hypothesis. Reported split by phase, and the
//!   middlegame is where the claim lives.
//! - **Fast means bad.** Blunders are quick, so some correlation exists
//!   through move quality alone. Not controlled here; it needs an engine
//!   evaluation and is recorded as the open hole.
//! - **Pooling time controls.** Bullet and classical are different regimes.
//!   Split by base time.

use std::collections::HashMap;

use bc_index::{Index, IndexedGame};

use crate::study::load;

/// One observation: what the machine spent, and what the human spent.
struct Obs {
    branch_bits: f64,
    corpus_bits: Option<f64>,
    seconds: f64,
    ply: usize,
    base: u32,
}

/// Pearson correlation. Returns `None` when either side has no spread, which
/// is not an error — it means the sample cannot answer the question.
fn correlation(xs: &[f64], ys: &[f64]) -> Option<f64> {
    let n = xs.len() as f64;
    if n < 3.0 {
        return None;
    }
    let mx = xs.iter().sum::<f64>() / n;
    let my = ys.iter().sum::<f64>() / n;
    let mut sxy = 0.0;
    let mut sxx = 0.0;
    let mut syy = 0.0;
    for (x, y) in xs.iter().zip(ys) {
        sxy += (x - mx) * (y - my);
        sxx += (x - mx).powi(2);
        syy += (y - my).powi(2);
    }
    if sxx <= 0.0 || syy <= 0.0 {
        return None;
    }
    Some(sxy / (sxx * syy).sqrt())
}

fn median(v: &mut [f64]) -> f64 {
    if v.is_empty() {
        return f64::NAN;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

pub fn run(args: &[String]) -> Result<(), String> {
    let mut path = None;
    let mut limit = usize::MAX;
    let mut corpus = None;
    let mut min_clock = 30u32;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--limit" => {
                i += 1;
                limit = args
                    .get(i)
                    .ok_or("--limit needs a number")?
                    .parse()
                    .map_err(|_| "--limit needs a number")?;
            }
            "--corpus" => {
                i += 1;
                corpus = Some(args.get(i).ok_or("--corpus needs a path")?.clone());
            }
            "--min-clock" => {
                i += 1;
                min_clock = args
                    .get(i)
                    .ok_or("--min-clock needs seconds")?
                    .parse()
                    .map_err(|_| "--min-clock needs seconds")?;
            }
            other => path = Some(other.to_string()),
        }
        i += 1;
    }
    let path = path.ok_or("usage: blockchess think <file.pgn> [--limit N] [--corpus big.pgn]")?;

    let games = load(&path, limit)?;

    // Optional corpus model: what did everyone else play here?
    let index = match &corpus {
        Some(p) => {
            eprint!("  building corpus model… ");
            let cg = load(p, usize::MAX)?;
            let idx = Index::build(
                cg.iter().map(|g| IndexedGame {
                    moves: &g.moves,
                    start: g.start,
                    result: &g.result,
                }),
                40,
                0..0,
            );
            eprintln!("{} games", idx.games);
            Some(idx)
        }
        None => None,
    };

    let mut obs: Vec<Obs> = Vec::new();
    let mut with_clocks = 0usize;
    let mut dropped_pressure = 0usize;

    for g in &games {
        let times = g.think_times();
        if times.iter().all(|t| t.is_none()) {
            continue;
        }
        with_clocks += 1;
        let base = g.time_control.map(|(b, _)| b).unwrap_or(0);

        let mut pos = g.start;
        let mut node = index.as_ref().map(|_| bc_index::Trie::ROOT);

        for (ply, &m) in g.moves.iter().enumerate() {
            let legal = bc_codec::order::canonical_moves(&pos);

            // Corpus surprise: -log2 of this move's share of continuations at
            // this node. Only defined while the game is still inside the book.
            let corpus_bits = match (&index, node) {
                (Some(idx), Some(n)) => {
                    let conts = idx.trie.continuations(n);
                    let total: u32 = conts.iter().map(|(_, _, s)| s.total()).sum();
                    let mine = conts
                        .iter()
                        .find(|(mv, _, _)| *mv == m)
                        .map(|(_, _, s)| s.total())
                        .unwrap_or(0);
                    if total > 0 && mine > 0 {
                        Some(-((mine as f64 / total as f64).log2()))
                    } else {
                        None
                    }
                }
                _ => None,
            };
            if let (Some(idx), Some(n)) = (&index, node) {
                node = idx.trie.child(n, m);
            }

            if let Some(secs) = times[ply] {
                // Time pressure: the clock left for the player who just moved.
                let remaining = g.clocks.get(ply).copied().flatten().unwrap_or(u32::MAX);
                if remaining < min_clock {
                    dropped_pressure += 1;
                } else {
                    obs.push(Obs {
                        branch_bits: (legal.len().max(1) as f64).log2(),
                        corpus_bits,
                        seconds: secs,
                        ply,
                        base,
                    });
                }
            }
            pos = pos.make_move(m);
        }
    }

    if obs.is_empty() {
        return Err(format!(
            "{path}: no usable clock data. Lichess exports carry [%clk ...]; \
             many other sources do not."
        ));
    }

    println!();
    println!("  CORPUS  {path}");
    println!("  {:<32}{:>10}", "games", games.len());
    println!("  {:<32}{:>10}", "games with clocks", with_clocks);
    println!("  {:<32}{:>10}", "usable plies", obs.len());
    println!(
        "  {:<32}{:>10}   under {min_clock}s left",
        "dropped, time pressure", dropped_pressure
    );

    let secs: Vec<f64> = obs.iter().map(|o| o.seconds).collect();
    let branch: Vec<f64> = obs.iter().map(|o| o.branch_bits).collect();
    let mut secs_sorted = secs.clone();
    println!(
        "  {:<32}{:>10.1}s   median {:.1}s",
        "mean think time",
        secs.iter().sum::<f64>() / secs.len() as f64,
        median(&mut secs_sorted)
    );

    println!();
    println!("  DO BITS TRACK SECONDS?");
    match correlation(&branch, &secs) {
        Some(r) => println!("  {:<32}{:>+10.4}", "branchiness vs seconds", r),
        None => println!("  {:<32}{:>10}", "branchiness vs seconds", "no spread"),
    }

    let paired: Vec<(f64, f64)> = obs
        .iter()
        .filter_map(|o| o.corpus_bits.map(|b| (b, o.seconds)))
        .collect();
    if paired.is_empty() {
        println!(
            "  {:<32}{:>10}",
            "corpus surprise vs seconds", "no --corpus"
        );
        println!();
        println!("  Without --corpus this only measures branchiness, which is the");
        println!("  wrong quantity: a forced mate is barely branchy and hugely");
        println!("  insightful. Pass --corpus to price moves against what other");
        println!("  people actually played. That is the hypothesis.");
    } else {
        let xs: Vec<f64> = paired.iter().map(|p| p.0).collect();
        let ys: Vec<f64> = paired.iter().map(|p| p.1).collect();
        match correlation(&xs, &ys) {
            Some(r) => println!(
                "  {:<32}{:>+10.4}   over {} plies still in book",
                "corpus surprise vs seconds",
                r,
                paired.len()
            ),
            None => println!("  {:<32}{:>10}", "corpus surprise vs seconds", "no spread"),
        }
    }

    // --- by phase: the middlegame is where the claim lives -----------------
    println!();
    println!("  BY PHASE   (opening moves can be fast because they were memorised,");
    println!("  which is cheap in bits AND fast in seconds for the wrong reason)");
    println!(
        "  {:<14}{:>9}{:>12}{:>14}",
        "plies", "n", "median s", "r(branch,s)"
    );
    let mut buckets: HashMap<usize, Vec<&Obs>> = HashMap::new();
    for o in &obs {
        buckets.entry((o.ply / 20).min(4)).or_default().push(o);
    }
    let mut keys: Vec<usize> = buckets.keys().copied().collect();
    keys.sort();
    for k in keys {
        let v = &buckets[&k];
        let mut s: Vec<f64> = v.iter().map(|o| o.seconds).collect();
        let b: Vec<f64> = v.iter().map(|o| o.branch_bits).collect();
        let r = correlation(&b, &s.clone());
        let label = if k == 4 {
            "80+".to_string()
        } else {
            format!("{}-{}", k * 20, k * 20 + 19)
        };
        println!(
            "  {:<14}{:>9}{:>12.1}{:>14}",
            label,
            v.len(),
            median(&mut s),
            r.map(|x| format!("{x:+.4}")).unwrap_or("-".into())
        );
    }

    // --- by time control ---------------------------------------------------
    let mut tc: HashMap<u32, Vec<&Obs>> = HashMap::new();
    for o in &obs {
        tc.entry(o.base).or_default().push(o);
    }
    if tc.len() > 1 {
        println!();
        println!("  BY TIME CONTROL   (bullet and classical are different regimes)");
        println!(
            "  {:<14}{:>9}{:>12}{:>14}",
            "base secs", "n", "median s", "r(branch,s)"
        );
        let mut keys: Vec<u32> = tc.keys().copied().collect();
        keys.sort();
        for k in keys.iter().take(8) {
            let v = &tc[k];
            if v.len() < 50 {
                continue;
            }
            let mut s: Vec<f64> = v.iter().map(|o| o.seconds).collect();
            let b: Vec<f64> = v.iter().map(|o| o.branch_bits).collect();
            let r = correlation(&b, &s.clone());
            println!(
                "  {:<14}{:>9}{:>12.1}{:>14}",
                k,
                v.len(),
                median(&mut s),
                r.map(|x| format!("{x:+.4}")).unwrap_or("-".into())
            );
        }
    }

    println!();
    println!("  HOW TO READ THIS");
    println!("  A correlation near zero means the machine's surprise and the human's");
    println!("  hesitation are unrelated, and papers/10-players.md §1 is wrong — which");
    println!("  is worth knowing in a week rather than a year.");
    println!();
    println!("  One confound is NOT handled and it inflates any positive result:");
    println!("  blunders are fast, so move quality alone produces some correlation.");
    println!("  Separating it needs an engine evaluation as a covariate, which this");
    println!("  project does not have. Treat a positive r as an upper bound.");
    println!();
    Ok(())
}
