//! `style` — the lab, on the command line.
//!
//!   style demo [rounds]      fit a lens to synthetic players with known styles
//!   style pgn <file> [k]     fit a lens to a PGN export
//!   style export [rounds]    the same, as JSON on stdout
//!   style viz [rounds]       a self-contained HTML page of the same, on stdout
//!   style identify [rounds]  held-out attribution: is a medal forgeable, and
//!                            how many games until a fresh account is unmasked

use bc_hash::hex;
use bc_style::{identify, report, synth, Corpus, Lab};

const USAGE: &str = "\
usage: style <command> [source] [k]

  commands   demo · identify · export · viz
  source     a .pgn file, or a number of synthetic round-robin rounds
  k          axes to keep (default 4)

  style demo                     the constructed players, as a report
  style identify                 held-out attribution and the forgery margin
  style viz games.pgn > out.html a page from your own games";

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("demo");

    let quiet = matches!(cmd, "export" | "viz");

    if !matches!(cmd, "demo" | "pgn" | "export" | "viz" | "identify") {
        eprintln!("{USAGE}");
        std::process::exit(2);
    }

    // Every command takes the same source: a PGN path, or a synthetic round
    // count. They are told apart by whether the argument parses as a number, so
    // `style viz 10` and `style viz games.pgn` both do the obvious thing.
    let source = args.get(2).map(|s| s.as_str());
    let corpus = match source {
        Some(arg) if arg.parse::<usize>().is_err() => load_pgn(arg, quiet),
        _ => {
            if cmd == "pgn" {
                eprintln!("usage: style pgn <file> [k]");
                std::process::exit(2);
            }
            let rounds: usize = source.and_then(|s| s.parse().ok()).unwrap_or(10);
            synthetic(rounds, quiet)
        }
    };

    if corpus.is_empty() {
        eprintln!("empty corpus — nothing to fit");
        std::process::exit(1);
    }

    let k: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(4);

    if cmd == "identify" {
        report_identify(&corpus, k);
        return;
    }

    let Some(lab) = Lab::fit(corpus, k) else {
        eprintln!("no usable games in corpus");
        std::process::exit(1);
    };

    match cmd {
        "export" => {
            println!("{}", report::build_json(&lab));
            return;
        }
        "viz" => {
            print!("{}", report::render_page(&lab));
            return;
        }
        _ => {}
    }

    println!("corpus root  {}", hex(&lab.basis.corpus_root));
    println!("samples      {} game-sides", lab.basis.samples);

    if let Some(w) = lab.basis.adequacy().warning() {
        println!("\n  ┌─────────────────────────────────────────────────────────────");
        println!("  │  NOT ENOUGH GAMES");
        for line in wrap(&w, 58) {
            println!("  │  {line}");
        }
        println!("  │  Everything below is printed so you can see the shape of it.");
        println!("  │  None of it is evidence about anybody.");
        println!("  └─────────────────────────────────────────────────────────────");
    }

    println!("\nthe axes, as this corpus expresses them");
    println!("(discovered, then named by the features loading hardest at each pole)");
    for i in 0..lab.basis.k {
        let (pos, neg) = lab.basis.poles(i);
        println!(
            "\n  axis {i}   {:>5.1}% of variance      {neg}  ←→  {pos}",
            lab.basis.explained(i) * 100.0
        );
        for (name, w) in lab.basis.loadings(i, 4) {
            println!("            {w:+.3}  {name}");
        }
    }

    println!("\nplayers");
    for name in lab.corpus.players() {
        let Some(p) = lab.profile(&name) else {
            continue;
        };
        let chain = lab.chain_for(&name);
        let coords: Vec<String> = p.coords.iter().map(|c| format!("{c:+.2}")).collect();
        println!("\n  {name}");
        println!("    position     [{}]", coords.join("  "));
        println!(
            "    dispersion   {:.2}  over {} games",
            p.dispersion, p.games
        );
        println!("    medal        {}", hex(&p.medal(&lab.basis)));
        println!(
            "    chain        {} links, path length {:.2}",
            chain.links.len(),
            chain.path_length()
        );
        println!("    head         {}", hex(&chain.head()));
    }

    let (within, between) = lab.separation();
    let acc = lab.nearest_neighbour_accuracy();
    let n = lab.corpus.players().len().max(1);
    println!("\nseparation");
    println!("  mean distance, same player       {within:.3}");
    println!("  mean distance, different players {between:.3}");
    println!(
        "  ratio                            {:.2}×",
        if within > 0.0 { between / within } else { 0.0 }
    );
    println!(
        "  nearest-neighbour accuracy       {:.1}%   (chance {:.1}%)",
        acc * 100.0,
        100.0 / n as f64
    );
    println!(
        "\n{}",
        if between > within * 1.2 {
            "Games cluster by who played them. The compression carries identity."
        } else {
            "Games do NOT separate by player. The points carry no identity — \
             any medal minted from this lens is noise."
        }
    );
}

/// Episodes 10 and 12, on the command line.
fn report_identify(corpus: &Corpus, k: usize) {
    let Some(sp) = identify::split(corpus, k) else {
        eprintln!("corpus too small to split");
        std::process::exit(1);
    };
    let held_total: usize = sp.held.iter().map(|h| h.points.len()).sum();
    let n = sp.centroids.len();

    println!("held-out attribution");
    println!("  the lens, the statistics and the centroids come from the training");
    println!("  half only. Test games are scored exactly as new games would be.\n");
    println!("  training game-sides   {}", sp.lab.points.len());
    println!("  held-out game-sides   {held_total}");
    println!("  players               {n}");

    let m = identify::confusion(&sp);
    let correct: usize = (0..n).map(|i| m[i][i]).sum();
    let total: usize = m.iter().flatten().sum();
    let chance = 100.0 / n as f64;
    println!(
        "\n  single-game accuracy  {:.1}%   (chance {chance:.1}%)",
        if total > 0 {
            correct as f64 / total as f64 * 100.0
        } else {
            0.0
        }
    );

    println!("\nconfusion — rows are truth, columns are the guess");
    print!("  {:<14}", "");
    for c in &sp.centroids {
        print!("{:>12}", short(&c.player));
    }
    println!();
    for (i, c) in sp.centroids.iter().enumerate() {
        print!("  {:<14}", short(&c.player));
        for cell in &m[i] {
            print!("{cell:>12}");
        }
        println!();
    }

    println!("\n\"I'll just start a fresh account\"");
    println!("  games observed before the new name is provably the old one:\n");
    let curve = identify::convergence(&sp, 12, 60, 0xC0FFEE);
    for c in &curve {
        let bar = "█".repeat((c.accuracy * 40.0).round() as usize);
        println!(
            "   {:>3} games  {:>6.1}%  {bar}",
            c.games,
            c.accuracy * 100.0
        );
    }
    if let Some(first) = curve.iter().find(|c| c.accuracy >= 0.95) {
        println!(
            "\n  → {} games is enough to identify a player 95% of the time.",
            first.games
        );
    } else {
        println!("\n  → 95% is not reached within 12 games.");
    }

    println!("\n\"I'll play like you and steal your medal\"");
    if let Some((a, b, d, cells)) = identify::closest_pair(&sp.centroids) {
        println!("  closest pair          {a} ↔ {b}");
        println!("  distance              {d:.3}");
        println!(
            "  in quantisation cells {cells:.0}   (STEP = {})",
            bc_style::profile::STEP
        );
        println!(
            "\n  {} ",
            if cells < 2.0 {
                "COLLISION RISK — these two mint the same or adjacent medals."
            } else {
                "No collision: the closest two players are many cells apart."
            }
        );
    }

    let all: Vec<Vec<f64>> = sp.lab.points.iter().map(|p| p.coords.clone()).collect();
    let cells = identify::occupied_cells(&sp.lab.basis, &all);
    let bits: f64 = cells.iter().map(|c| c.max(1.0).log2()).sum();
    println!(
        "\n  cells spanned per axis {}",
        cells
            .iter()
            .map(|c| format!("{c:.0}"))
            .collect::<Vec<_>>()
            .join(" · ")
    );
    println!("  upper bound            {bits:.1} bits of personality");
    println!("\n  That bound is loose and deliberately so: it counts the cells the");
    println!("  cloud spans, not the cells anyone occupies, and four constructed");
    println!("  players cannot speak for the human population. It is a method,");
    println!("  waiting for a real corpus — see METAPLAN O3.");
}

fn short(s: &str) -> String {
    if s.chars().count() > 11 {
        format!("{}…", s.chars().take(10).collect::<String>())
    } else {
        s.to_string()
    }
}

/// Wrap a warning to a column, on spaces.
fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > width {
            out.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        out.push(line);
    }
    out
}

/// A synthetic round robin between the constructed archetypes.
fn synthetic(rounds: usize, quiet: bool) -> Corpus {
    let games = synth::round_robin(rounds, 0x5EED_1234, 120);
    if !quiet {
        println!(
            "synthetic corpus: {} games between {} constructed personalities\n",
            games.len(),
            synth::ARCHETYPES.len()
        );
    }
    let mut c = Corpus::new();
    for g in games {
        c.push(g);
    }
    c
}

/// Games from a PGN file. Rejected games are counted, never silently dropped:
/// a corpus that quietly loses games produces a lens fitted to a subset nobody
/// chose.
fn load_pgn(path: &str, quiet: bool) -> Corpus {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("cannot read {path}: {e}");
            std::process::exit(1);
        }
    };
    let (games, skipped) = bc_style::pgn::parse(&text);
    if !quiet {
        println!("ingested {} games ({skipped} rejected)\n", games.len());
    } else if skipped > 0 {
        eprintln!("{skipped} games rejected from {path}");
    }
    let mut c = Corpus::new();
    for g in games {
        c.push(g);
    }
    c
}
