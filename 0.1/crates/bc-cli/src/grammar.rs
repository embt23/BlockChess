//! `blockchess grammar` — experiment X7.
//!
//! `papers/08-layers.md` §3 makes a claim big enough to be worth trying to
//! break:
//!
//! > Searching for the shortest encoding of the corpus is the same search as
//! > searching for chess theory.
//!
//! This command is the test. It feeds a corpus of games to a general-purpose
//! compressor that knows nothing about chess — `bc-grammar`, which sees only
//! anonymous integers — and then asks the opening-name database whether the
//! sequences it decided to name are ones humans had already named.
//!
//! Nothing about chess is given to the inducer. The names are looked up
//! *afterwards*, and only for reporting. If they were an input, agreement
//! would be circular and worth nothing.
//!
//! ## How to read the output, and how it could be wrong
//!
//! The headline number is what fraction of the top discovered symbols land
//! inside a named opening. Two ways that number could flatter itself, both
//! reported separately so they can be judged:
//!
//! - **Exact vs contained.** A symbol that *is* a named line from move one is
//!   a much stronger hit than one that merely appears somewhere inside a named
//!   line. Both are counted, apart.
//! - **Short symbols are cheap.** `1.e4` occurs in thousands of named
//!   openings, so a two-ply symbol matching something is nearly free. The
//!   report breaks results down by symbol length, and the interesting rows are
//!   the long ones.

use std::collections::HashMap;

use bc_chess::{Move, Position};
use bc_grammar::{induce, Limits, Sym};
use bc_pgn::to_san;

use crate::study::{load, load_eco};

pub fn run(args: &[String]) -> Result<(), String> {
    let mut path = None;
    let mut limit = usize::MAX;
    let mut min_occ = 0u32;
    let mut max_rules = 20_000usize;
    let mut show = 40usize;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--limit" => {
                i += 1;
                limit = num(args.get(i), "--limit")?;
            }
            "--min" => {
                i += 1;
                min_occ = num(args.get(i), "--min")? as u32;
            }
            "--rules" => {
                i += 1;
                max_rules = num(args.get(i), "--rules")?;
            }
            "--show" => {
                i += 1;
                show = num(args.get(i), "--show")?;
            }
            other => path = Some(other.to_string()),
        }
        i += 1;
    }
    let path = path.ok_or(
        "usage: blockchess grammar <file.pgn> [--limit N] [--min K] [--rules R] [--show S]",
    )?;

    let games = load(&path, limit)?;
    let eco = load_eco()?;

    // Games become sequences of move ids. A move's 16-bit packed form is the
    // symbol: the same move played in different games is the same symbol,
    // which is what lets a repeated line be seen as a repetition at all.
    let seqs: Vec<Vec<Sym>> = games
        .iter()
        .filter(|g| g.start == Position::startpos())
        .map(|g| g.moves.iter().map(|m| m.0 as Sym).collect())
        .collect();
    let skipped = games.len() - seqs.len();
    let total_plies: usize = seqs.iter().map(|s| s.len()).sum();
    if total_plies == 0 {
        return Err("no games starting from the initial position".into());
    }

    // A pattern seen twice in a big corpus is noise. Scale the threshold with
    // the corpus unless the caller pinned it.
    if min_occ == 0 {
        min_occ = ((seqs.len() as f64 * 0.002).round() as u32).max(3);
    }

    println!();
    println!("  CORPUS  {path}");
    println!("  {:<30}{:>10}", "games", seqs.len());
    if skipped > 0 {
        println!("  {:<30}{:>10}   (non-standard start)", "skipped", skipped);
    }
    println!("  {:<30}{:>10}", "plies", total_plies);
    println!("  {:<30}{:>10}", "min occurrences to name", min_occ);
    println!();
    print!("  inducing grammar… ");
    let t = std::time::Instant::now();
    let g = induce(
        &seqs,
        Limits {
            min_occurrences: min_occ,
            max_rules,
        },
    );
    println!("{:.2}s", t.elapsed().as_secs_f64());

    let after: usize = g.sequences.iter().map(|s| s.len()).sum();
    println!();
    println!("  COMPRESSION");
    println!("  {:<30}{:>10}", "symbols before", total_plies);
    println!("  {:<30}{:>10}", "symbols after", after);
    println!("  {:<30}{:>10}", "rules invented", g.rules.len());
    println!(
        "  {:<30}{:>10}   sequences + 2 per rule",
        "total with grammar",
        g.total_size()
    );
    println!(
        "  {:<30}{:>9.2}x",
        "net reduction",
        total_plies as f64 / g.total_size().max(1) as f64
    );

    // Rank by how much each symbol earned: (length - 1) saved symbols per
    // substitution. A long rare symbol and a short common one can be worth the
    // same, and this is the number that says so.
    let mut ranked: Vec<(u32, usize, Vec<Move>)> = g
        .rules
        .iter()
        .map(|r| {
            let expanded: Vec<Move> = g
                .expand(r.lhs)
                .into_iter()
                .map(|s| Move(s as u16))
                .collect();
            (r.occurrences, expanded.len(), expanded)
        })
        .collect();
    ranked.sort_by(|a, b| {
        let ea = (a.1.saturating_sub(1)) as u64 * a.0 as u64;
        let eb = (b.1.saturating_sub(1)) as u64 * b.0 as u64;
        eb.cmp(&ea).then(b.1.cmp(&a.1))
    });

    // Cross-check against the names, over the whole grammar.
    let mut exact = 0usize;
    let mut contained = 0usize;
    let mut by_len: HashMap<usize, (usize, usize)> = HashMap::new();
    for (_, len, moves) in &ranked {
        let e = eco.name_of(moves).is_some();
        let c = e || eco.contains_run(moves).is_some();
        if e {
            exact += 1;
        }
        if c {
            contained += 1;
        }
        let slot = by_len.entry(*len).or_default();
        slot.0 += 1;
        if c {
            slot.1 += 1;
        }
    }

    println!();
    println!("  AGREEMENT WITH HUMAN OPENING THEORY");
    println!("  {:<30}{:>10}", "opening names loaded", eco.len());
    println!(
        "  {:<30}{:>10}   {:.1}% of symbols",
        "exactly a named line",
        exact,
        100.0 * exact as f64 / ranked.len().max(1) as f64
    );
    println!(
        "  {:<30}{:>10}   {:.1}% of symbols",
        "inside a named line",
        contained,
        100.0 * contained as f64 / ranked.len().max(1) as f64
    );
    println!();
    println!("  by symbol length (long ones are the interesting rows):");
    println!(
        "  {:<10}{:>10}{:>12}{:>10}",
        "plies", "symbols", "in theory", "share"
    );
    let mut lens: Vec<usize> = by_len.keys().copied().collect();
    lens.sort();
    for l in lens {
        let (n, hit) = by_len[&l];
        println!(
            "  {:<10}{:>10}{:>12}{:>9.0}%",
            l,
            n,
            hit,
            100.0 * hit as f64 / n as f64
        );
    }

    println!();
    println!("  WHAT IT NAMED   (top {show} by symbols saved)");
    println!();
    let start = Position::startpos();
    for (occ, len, moves) in ranked.iter().take(show) {
        let san = line_of(&start, moves);
        let label = match eco.name_of(moves) {
            Some(e) => format!("= {} {}", e.code, e.name),
            None => match eco.contains_run(moves) {
                Some(e) => format!("~ in {} {}", e.code, e.name),
                None => String::new(),
            },
        };
        println!("  {:>6}x  {:>2}ply  {}", occ, len, san);
        if !label.is_empty() {
            println!("               {label}");
        }
    }

    println!();
    println!("  '=' the symbol is exactly a named opening line");
    println!("  '~' it occurs inside one, but is not a whole named line");
    println!("  blank: the compressor thinks this repeats and nobody has named it.");
    println!("  Those are the rows worth looking at by hand.");
    println!();
    Ok(())
}

/// Render a run of moves as SAN. A grammar symbol found mid-game will not be
/// legal from the starting position, so this stops and marks the point rather
/// than inventing notation — an honest `…` beats a plausible lie.
fn line_of(start: &Position, moves: &[Move]) -> String {
    let mut pos = *start;
    let mut out = Vec::new();
    for (i, &m) in moves.iter().enumerate() {
        if !bc_codec::order::canonical_moves(&pos).contains(&m) {
            out.push(format!("…+{}", moves.len() - i));
            break;
        }
        if i % 2 == 0 {
            out.push(format!("{}.", i / 2 + 1));
        }
        out.push(to_san(&pos, m));
        pos = pos.make_move(m);
    }
    out.join(" ")
}

fn num(s: Option<&String>, flag: &str) -> Result<usize, String> {
    s.ok_or(format!("{flag} needs a number"))?
        .parse()
        .map_err(|_| format!("{flag} needs a number"))
}
