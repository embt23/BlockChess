//! `blockchess measure` — the experiment the papers are waiting on.
//!
//! `papers/02-encodings.md` quotes `b ≈ 30` as a placeholder and says the
//! number that matters is `E[log2 b]` over real games. This command computes
//! it, and every storage figure downstream of it, from a PGN file.
//!
//! It is experiment **X1** in `measure/EXPERIMENTS.md`. When it has been run
//! on a real corpus, the placeholder rows in the papers get replaced with its
//! output and the source is this file rather than a citation.

use std::fmt::Write as _;

use bc_chess::Position;

pub fn perft(args: &[String], divide: bool) -> Result<(), String> {
    let depth: u32 = args
        .first()
        .ok_or("usage: blockchess perft <depth> [FEN]")?
        .parse()
        .map_err(|_| "depth must be a number")?;
    let pos = crate::position_from(&args[1..])?;

    let t = std::time::Instant::now();
    if divide {
        let results = bc_chess::divide(&pos, depth);
        let mut total = 0;
        for (m, n) in &results {
            println!("  {:<6} {:>14}", m.to_uci(), group(*n));
            total += n;
        }
        println!("  {:<6} {:>14}", "total", group(total));
    } else {
        let n = bc_chess::perft(&pos, depth);
        let secs = t.elapsed().as_secs_f64();
        println!("  perft({depth}) = {}", group(n));
        println!("  {:.2}s, {:.1} Mnps", secs, n as f64 / secs / 1_000_000.0);
        // The published counts are the oracle. Checking against them here
        // means anyone who builds this can verify the rules in one command.
        if pos == Position::startpos() {
            let expected: &[u64] = &[1, 20, 400, 8_902, 197_281, 4_865_609, 119_060_324];
            if let Some(&want) = expected.get(depth as usize) {
                println!(
                    "  published    {}   {}",
                    group(want),
                    if want == n {
                        "MATCH"
                    } else {
                        "*** MISMATCH ***"
                    }
                );
            }
        }
    }
    Ok(())
}

fn group(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

#[derive(Default)]
struct Stats {
    games: usize,
    plies: usize,
    /// Σ log2 b — the exact E7 payload size in bits, summed over all plies.
    sum_log2_b: f64,
    /// Σ ⌈log2 b⌉ — E6, the same scheme rounded up to whole bits.
    sum_ceil_log2_b: f64,
    /// Σ b — so that log2 E[b] can be compared with E[log2 b].
    sum_b: f64,
    max_b: usize,
    max_b_fen: String,
    /// Branching factor histogram, indexed by b.
    hist: Vec<u64>,
    /// Σ log2 b bucketed by ply/10, so the phase dependence is visible.
    phase: Vec<(f64, usize)>,
}

impl Stats {
    fn observe(&mut self, b: usize, ply: usize, pos: &Position) {
        self.plies += 1;
        self.sum_b += b as f64;
        let l = (b as f64).log2();
        self.sum_log2_b += l;
        self.sum_ceil_log2_b += l.ceil().max(0.0);
        if b > self.max_b {
            self.max_b = b;
            self.max_b_fen = pos.to_fen();
        }
        if self.hist.len() <= b {
            self.hist.resize(b + 1, 0);
        }
        self.hist[b] += 1;

        let bucket = ply / 10;
        if self.phase.len() <= bucket {
            self.phase.resize(bucket + 1, (0.0, 0));
        }
        self.phase[bucket].0 += l;
        self.phase[bucket].1 += 1;
    }
}

pub fn run(args: &[String]) -> Result<(), String> {
    let mut path = None;
    let mut limit = usize::MAX;
    let mut csv = None;
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
            "--csv" => {
                i += 1;
                csv = Some(args.get(i).ok_or("--csv needs a path")?.clone());
            }
            other => path = Some(other.to_string()),
        }
        i += 1;
    }
    let path = path.ok_or("usage: blockchess measure <file.pgn> [--limit N] [--csv PATH]")?;

    let text = std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
    let (games, errors) = bc_pgn::parse_all(&text);

    let mut st = Stats::default();
    let mut rows = String::from("game,ply,legal_moves,bits\n");
    let mut results: (usize, usize, usize, usize) = (0, 0, 0, 0); // 1-0, 0-1, draw, other

    for (gi, game) in games.iter().take(limit).enumerate() {
        let mut pos = game.start;
        for (ply, &m) in game.moves.iter().enumerate() {
            let b = bc_codec::order::canonical_moves(&pos).len();
            st.observe(b, ply, &pos);
            if csv.is_some() {
                let _ = writeln!(rows, "{gi},{ply},{b},{:.6}", (b as f64).log2());
            }
            pos = pos.make_move(m);
        }
        st.games += 1;
        match game.result.as_str() {
            "1-0" => results.0 += 1,
            "0-1" => results.1 += 1,
            "1/2-1/2" => results.2 += 1,
            _ => results.3 += 1,
        }
    }

    if st.plies == 0 {
        return Err(format!("{path}: no games with moves were read"));
    }

    if let Some(csv_path) = csv {
        std::fs::write(&csv_path, rows).map_err(|e| format!("{csv_path}: {e}"))?;
        println!("wrote {csv_path}");
    }

    report(&st, &games, &errors, results, &path);
    Ok(())
}

fn report(
    st: &Stats,
    games: &[bc_pgn::Game],
    errors: &[bc_pgn::PgnError],
    results: (usize, usize, usize, usize),
    path: &str,
) {
    let plies = st.plies as f64;
    let e_log2_b = st.sum_log2_b / plies;
    let e_b = st.sum_b / plies;
    let mean_plies = plies / st.games as f64;

    println!();
    println!("  CORPUS  {path}");
    println!("  {:<24}{:>14}", "games read", st.games);
    if !errors.is_empty() {
        println!(
            "  {:<24}{:>14}   (first: {})",
            "games skipped",
            errors.len(),
            errors[0]
        );
    }
    println!("  {:<24}{:>14}", "plies", st.plies);
    println!("  {:<24}{:>14.1}", "mean game length", mean_plies);
    println!(
        "  {:<24}{:>14}",
        "results W/B/D/?",
        format!("{}/{}/{}/{}", results.0, results.1, results.2, results.3)
    );
    let _ = games;

    println!();
    println!("  BRANCHING FACTOR");
    println!("  {:<24}{:>14.4}", "E[b]", e_b);
    println!("  {:<24}{:>14.4}", "log2 E[b]", e_b.log2());
    println!(
        "  {:<24}{:>14.4}  <- the number the papers needed",
        "E[log2 b]", e_log2_b
    );
    println!(
        "  {:<24}{:>14.4}   Jensen gap",
        "difference",
        e_b.log2() - e_log2_b
    );
    println!("  {:<24}{:>14}   {}", "max b", st.max_b, st.max_b_fen);

    println!();
    println!("  BITS PER PLY, AND WHAT A GAME COSTS");
    let schemes: [(&str, f64); 4] = [
        ("E1  PGN/SAN text", 40.0),
        ("E3  16-bit move", 16.0),
        ("E6  index, whole bits", st.sum_ceil_log2_b / plies),
        ("E7  index, mixed radix", e_log2_b),
    ];
    println!("  {:<24}{:>10}{:>14}", "", "bits/ply", "bytes/game");
    for (name, bits) in schemes {
        println!(
            "  {:<24}{:>10.3}{:>14.1}",
            name,
            bits,
            bits * mean_plies / 8.0
        );
    }

    println!();
    println!("  E7 SAVING OVER E3        {:>10.2}x", 16.0 / e_log2_b);
    println!(
        "  E7 SAVING OVER E6        {:>10.2}%   (what the rounding costs)",
        100.0 * (st.sum_ceil_log2_b - st.sum_log2_b) / st.sum_ceil_log2_b
    );

    println!();
    println!("  BY PHASE   (papers/03-corpus.md predicts this curve rises then flattens)");
    println!("  {:<12}{:>10}{:>12}", "plies", "E[log2 b]", "positions");
    for (i, &(sum, n)) in st.phase.iter().enumerate() {
        if n == 0 {
            continue;
        }
        let avg = sum / n as f64;
        let bar = "#".repeat((avg * 4.0).round() as usize);
        println!(
            "  {:<12}{:>10.3}{:>12}  {}",
            format!("{}-{}", i * 10, i * 10 + 9),
            avg,
            n,
            bar
        );
    }

    println!();
    println!("  THE ENVELOPE   (papers/04-permanence.md §3)");
    let payload = e_log2_b * mean_plies / 8.0;
    for (label, env) in [("two signatures per game", 200.0), ("batched by 100", 17.0)] {
        println!(
            "  {:<24}{:>10.1} B{:>10.1}% chess",
            label,
            payload + env,
            100.0 * payload / (payload + env)
        );
    }
    println!();
}
