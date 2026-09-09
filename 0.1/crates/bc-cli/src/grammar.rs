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

    // Counting symbols flatters the result and it took a real corpus to make
    // that obvious. Induction *grows the alphabet* — every rule adds a symbol
    // — so each surviving symbol costs more bits than the ones it replaced.
    // A 1.14x reduction in symbol count was only 1.07x in bits.
    let distinct_terminals = {
        let mut set = std::collections::HashSet::new();
        for s in &seqs {
            for &x in s {
                set.insert(x);
            }
        }
        set.len().max(2)
    };
    let alpha_before = (distinct_terminals as f64).log2();
    let alpha_after = ((distinct_terminals + g.rules.len()) as f64).log2();
    let bits_before = total_plies as f64 * alpha_before;
    let bits_after = (after + 2 * g.rules.len()) as f64 * alpha_after;

    println!();
    println!("  COMPRESSION   (in bits — counting symbols overstates it)");
    println!("  {:<30}{:>10}", "symbols before", total_plies);
    println!("  {:<30}{:>10}", "symbols after", after);
    println!("  {:<30}{:>10}", "rules invented", g.rules.len());
    println!(
        "  {:<30}{:>10}   {} -> {} symbols wide",
        "alphabet grew",
        distinct_terminals + g.rules.len(),
        distinct_terminals,
        distinct_terminals + g.rules.len()
    );
    println!(
        "  {:<30}{:>9.3}x   symbol count only",
        "apparent reduction",
        total_plies as f64 / g.total_size().max(1) as f64
    );
    println!(
        "  {:<30}{:>9.3}x   <- the honest one",
        "net reduction in bits",
        bits_before / bits_after.max(1.0)
    );
    println!(
        "  {:<30}{:>10.2}   vs E7's ~4.6 on this corpus",
        "bits/ply after grammar",
        bits_after / total_plies as f64
    );
    println!();
    println!("  Re-Pair is a poor compressor of chess and that is expected —");
    println!("  papers/08-layers.md §6. What it produces that a better one does");
    println!("  not is a list you can read. That list is below.");

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
    println!();
    println!("  Caveat that the first run of this command did not state: a symbol");
    println!("  starting mid-game is being scored against a table of *openings*,");
    println!("  which is the wrong answer key and inflates 'inside a named line'.");
    println!("  The list below marks which is which.");
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
    // ---------------------------------------------------------------------
    // The control. Without it the agreement figures are unfalsifiable.
    //
    // A two-ply run has thousands of chances to appear somewhere inside 3,810
    // opening lines, so "48.7% occur inside a named line" could easily be
    // coincidence rather than discovery. The question that matters is not
    // "are the compressor's symbols named?" but "are they named MORE OFTEN
    // than an equally common chess sequence that the compressor did not
    // pick?"
    //
    // So: for each length the grammar produced, take that many real game
    // prefixes of the same length straight from the corpus, and score them
    // the same way. Real openings people actually played, chosen by nothing.
    // If the compressor's rate is no better, it found frequency, not theory.
    // ---------------------------------------------------------------------
    let mut null_by_len: HashMap<usize, (usize, usize, usize)> = HashMap::new();
    {
        let mut cursor = 0usize;
        let mut lens: Vec<usize> = by_len.keys().copied().collect();
        lens.sort();
        for l in lens {
            let want = by_len[&l].0.max(50);
            let mut n = 0;
            let mut exact_n = 0;
            let mut contained_n = 0;
            let mut tried = 0;
            while n < want && tried < seqs.len() {
                let s = &seqs[cursor % seqs.len()];
                cursor += 7; // stride, so we don't sample one region
                tried += 1;
                if s.len() < l {
                    continue;
                }
                let run: Vec<Move> = s[..l].iter().map(|&x| Move(x as u16)).collect();
                if eco.name_of(&run).is_some() {
                    exact_n += 1;
                    contained_n += 1;
                } else if eco.contains_run(&run).is_some() {
                    contained_n += 1;
                }
                n += 1;
            }
            null_by_len.insert(l, (n, exact_n, contained_n));
        }
    }

    println!();
    println!("  by symbol length, against a control of real game prefixes:");
    println!(
        "  {:<7}{:>9}{:>10}{:>12}{:>10}",
        "plies", "symbols", "in theory", "control", "lift"
    );
    let mut lens: Vec<usize> = by_len.keys().copied().collect();
    lens.sort();
    for l in lens {
        let (n, hit) = by_len[&l];
        let share = 100.0 * hit as f64 / n as f64;
        let (cn, _, chit) = null_by_len.get(&l).copied().unwrap_or((0, 0, 0));
        let cshare = if cn > 0 {
            100.0 * chit as f64 / cn as f64
        } else {
            f64::NAN
        };
        let lift = if cshare > 0.0 {
            share / cshare
        } else {
            f64::NAN
        };
        // n is small at the long lengths, so say so rather than letting a
        // 100% built on one symbol look like evidence.
        let weak = if n < 10 { " (n small)" } else { "" };
        println!(
            "  {:<7}{:>9}{:>11.0}%{:>11.0}%{:>9.2}x{}",
            l, n, share, cshare, lift, weak
        );
    }
    println!();
    println!("  control = the same number of real game prefixes of that length,");
    println!("  scored identically. lift above 1.0 means the compressor picked");
    println!("  sequences that are named more often than the openings people");
    println!("  actually play. At 1.0 it found frequency, not theory.");

    // ---------------------------------------------------------------------
    // Where does each symbol actually occur?
    //
    // Re-Pair finds repetition wherever it lives, so a symbol need not start
    // at move one. Until now those were printed as "…+2" — unreadable, and
    // silently scored against a table of *openings*, which is the wrong answer
    // key for a middlegame motif and inflated the agreement figures.
    //
    // So find a real occurrence: which game, at which ply. That gives a
    // position to render the moves from, and it separates the two kinds of
    // symbol, which need to be judged differently.
    // ---------------------------------------------------------------------
    let located: Vec<Option<Located>> = ranked
        .iter()
        .take(show.max(60))
        .map(|(_, _, moves)| locate(&seqs, moves))
        .collect();

    println!();
    println!("  WHAT IT NAMED   (top {show} by symbols saved)");
    println!();
    let start = Position::startpos();
    for (i, (occ, len, moves)) in ranked.iter().take(show).enumerate() {
        let here = located.get(i).and_then(|o| o.as_ref());
        let from_start = here.map(|l| l.ply == 0).unwrap_or(false);
        let san = match here {
            Some(l) if l.ply > 0 => {
                // Render from the position the pattern actually starts in.
                let mut pos = Position::startpos();
                for &m in &seqs[l.game][..l.ply] {
                    pos = pos.make_move(Move(m as u16));
                }
                line_of_at(&pos, moves, l.ply)
            }
            _ => line_of(&start, moves),
        };
        // An opening table cannot judge a middlegame motif. Only ask it about
        // patterns that begin at move one.
        let label = if !from_start {
            match here {
                Some(l) => format!("· middlegame, typically ply {}", l.median_ply),
                None => "· not located".to_string(),
            }
        } else {
            match eco.name_of(moves) {
                Some(e) => format!("= {} {}", e.code, e.name),
                None => match eco.contains_run(moves) {
                    Some(e) => format!("~ in {} {}", e.code, e.name),
                    None => String::new(),
                },
            }
        };
        println!("  {:>6}x  {:>2}ply  {}", occ, len, san);
        if !label.is_empty() {
            println!("               {label}");
        }
    }

    // ---------------------------------------------------------------------
    // The middlegame patterns. This is the part with no answer key.
    // ---------------------------------------------------------------------
    let mid: Vec<MidPattern> = ranked
        .iter()
        .enumerate()
        .filter_map(|(i, r)| {
            located
                .get(i)
                .and_then(|o| o.as_ref())
                .filter(|l| l.ply > 0 && r.1 >= 3)
                .map(|l| (i, r, l))
        })
        .take(6)
        .collect();

    if !mid.is_empty() {
        println!();
        println!("  PATTERNS NOBODY NAMED");
        println!();
        println!("  These do not start at move one, so no opening table can judge");
        println!("  them — chess systematically named its openings and left the");
        println!("  middlegame to prose. They recur anyway. Boards are the position");
        println!("  the pattern begins in.");

        for (_, (occ, len, moves), l) in &mid {
            let mut pos = Position::startpos();
            for &m in &seqs[l.game][..l.ply] {
                pos = pos.make_move(Move(m as u16));
            }
            let last = if l.ply > 0 {
                Some(Move(seqs[l.game][l.ply - 1] as u16))
            } else {
                None
            };
            println!();
            println!(
                "  {occ}x   {len} plies   starts around ply {}   seen in {}+ games",
                l.median_ply, l.seen
            );
            println!("  {}", line_of_at(&pos, moves, l.ply));
            println!();
            print!("{}", crate::board::draw(&pos, last));
        }
        println!();
        println!("  Whether any of these is a real idea or an artefact of how club");
        println!("  players trade pieces is exactly what a person has to look at.");
        println!("  The machine can only say that they repeat.");
    }

    println!();
    println!("  '=' the symbol is exactly a named opening line");
    println!("  '~' it occurs inside one, but is not a whole named line");
    println!("  blank: the compressor thinks this repeats and nobody has named it.");
    println!("  Those are the rows worth looking at by hand.");
    println!();
    Ok(())
}

/// A discovered symbol that does not start at move one: its rank, the symbol
/// itself, and where it was found.
type MidPattern<'a> = (usize, &'a (u32, usize, Vec<Move>), &'a Located);

/// Where a symbol really lives in the corpus.
pub struct Located {
    pub game: usize,
    pub ply: usize,
    /// Median starting ply across the occurrences sampled — one occurrence
    /// could be a fluke, so the typical depth is the number worth printing.
    pub median_ply: usize,
    pub seen: usize,
}

/// Find where a run of moves occurs, and how deep into games it usually is.
///
/// Bounded on purpose: it stops after `CAP` sightings. The median over a
/// hundred occurrences is as good as the median over ten thousand, and the
/// unbounded version turns an interactive command into a batch job.
fn locate(seqs: &[Vec<Sym>], run: &[Move]) -> Option<Located> {
    const CAP: usize = 200;
    let needle: Vec<Sym> = run.iter().map(|m| m.0 as Sym).collect();
    if needle.is_empty() {
        return None;
    }
    let mut first: Option<(usize, usize)> = None;
    let mut plies: Vec<usize> = Vec::new();

    for (gi, s) in seqs.iter().enumerate() {
        if s.len() < needle.len() {
            continue;
        }
        for start in 0..=(s.len() - needle.len()) {
            if s[start..start + needle.len()] == needle[..] {
                if first.is_none() {
                    first = Some((gi, start));
                }
                plies.push(start);
                break; // one sighting per game is plenty for a median
            }
        }
        if plies.len() >= CAP {
            break;
        }
    }

    let (game, ply) = first?;
    plies.sort_unstable();
    let median_ply = plies[plies.len() / 2];
    Some(Located {
        game,
        ply,
        median_ply,
        seen: plies.len(),
    })
}

/// Render a run of moves as SAN. A grammar symbol found mid-game will not be
/// legal from the starting position, so this stops and marks the point rather
/// than inventing notation — an honest `…` beats a plausible lie.
/// `at_ply` is where the run starts in a real game, so the move numbers are the
/// ones a person would see on a scoresheet. A run beginning on Black's move
/// opens with `12...` rather than pretending to be move one — getting this
/// wrong makes a middlegame motif unreadable, which was the whole complaint.
fn line_of_at(start: &Position, moves: &[Move], at_ply: usize) -> String {
    let mut pos = *start;
    let mut out = Vec::new();
    for (i, &m) in moves.iter().enumerate() {
        if !bc_codec::order::canonical_moves(&pos).contains(&m) {
            out.push(format!("…+{}", moves.len() - i));
            break;
        }
        let ply = at_ply + i;
        if ply.is_multiple_of(2) {
            out.push(format!("{}.", ply / 2 + 1));
        } else if i == 0 {
            out.push(format!("{}...", ply / 2 + 1));
        }
        out.push(to_san(&pos, m));
        pos = pos.make_move(m);
    }
    out.join(" ")
}

fn line_of(start: &Position, moves: &[Move]) -> String {
    line_of_at(start, moves, 0)
}

fn num(s: Option<&String>, flag: &str) -> Result<usize, String> {
    s.ok_or(format!("{flag} needs a number"))?
        .parse()
        .map_err(|_| format!("{flag} needs a number"))
}
