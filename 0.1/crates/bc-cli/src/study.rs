//! `index`, `book` and `find` — the commands that make the corpus answerable.
//!
//! This is the point where the project stops being a compressor and starts
//! being the thing it exists to be: a database you can ask questions of.

use std::collections::HashMap;

use bc_chess::{Move, Position};
use bc_index::{Eco, Index, IndexedGame, Score};
use bc_pgn::{parse_all, to_san};

/// Read a PGN file and report what came out of it.
pub fn load(path: &str, limit: usize) -> Result<Vec<bc_pgn::Game>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    let (mut games, errors) = parse_all(&text);
    if games.is_empty() {
        return Err(format!("{path}: no games could be read"));
    }
    if !errors.is_empty() {
        eprintln!(
            "  note: skipped {} damaged game(s); first was {}",
            errors.len(),
            errors[0]
        );
    }
    games.truncate(limit);
    Ok(games)
}

/// Load the vendored opening names, resolving each line against the rules.
pub fn load_eco() -> Result<Eco, String> {
    // Compiled in so the binary works from any directory and so a result is
    // tied to an exact version of the ground truth.
    const TSV: &str = include_str!("../../../data/eco.tsv");
    let (eco, skipped) = Eco::parse(TSV, |pos, san| bc_pgn::parse_san(pos, san).ok());
    if skipped > 0 {
        eprintln!("  note: {skipped} opening rows did not resolve and were skipped");
    }
    Ok(eco)
}

fn build_index(games: &[bc_pgn::Game], depth: usize) -> Index {
    Index::build(
        games.iter().map(|g| IndexedGame {
            moves: &g.moves,
            start: g.start,
            result: &g.result,
        }),
        depth,
        0..40,
    )
}

fn pct(s: &Score) -> String {
    match s.white_percentage() {
        Some(p) => format!("{:.1}%", 100.0 * p),
        None => "  -  ".to_string(),
    }
}

fn line_san(start: &Position, moves: &[Move]) -> String {
    let mut pos = *start;
    let mut out = Vec::new();
    for (i, &m) in moves.iter().enumerate() {
        if i % 2 == 0 {
            out.push(format!("{}.", i / 2 + 1));
        }
        out.push(to_san(&pos, m));
        pos = pos.make_move(m);
    }
    out.join(" ")
}

/// `blockchess index <pgn>` — build both structures and report their shape.
pub fn index(args: &[String]) -> Result<(), String> {
    let (path, limit) = parse_common(args, "index")?;
    let games = load(&path, limit)?;
    let t = std::time::Instant::now();
    let idx = build_index(&games, 60);
    let elapsed = t.elapsed().as_secs_f64();

    let plies: usize = games.iter().map(|g| g.moves.len()).sum();
    println!();
    println!("  CORPUS  {path}");
    println!("  {:<28}{:>12}", "games", idx.games);
    println!("  {:<28}{:>12}", "plies", plies);
    println!("  {:<28}{:>12.2}s", "index built in", elapsed);
    println!();
    println!("  OPENING TRIE   (Q1: games by move order)");
    println!("  {:<28}{:>12}", "nodes", idx.trie.len());
    for min in [2u32, 5, 10, 50] {
        let n = idx.trie.frequent_lines(min).len();
        println!("  {:<28}{:>12}", format!("lines played >= {min} times"), n);
    }
    println!();
    println!("  POSITION INDEX   (Q2: games by position, any move order)");
    let d = idx.positions.distinct();
    let s = idx.positions.singletons();
    println!("  {:<28}{:>12}", "postings", idx.positions.total_postings());
    println!("  {:<28}{:>12}", "distinct positions", d);
    println!(
        "  {:<28}{:>12}   {:.1}%",
        "reached by one game only",
        s,
        100.0 * s as f64 / d.max(1) as f64
    );
    println!();
    println!("  papers/05-index.md §2 predicts singletons dominate and proposes");
    println!("  tiering them. The percentage above is the measurement that decides it.");
    println!();
    Ok(())
}

/// `blockchess book <pgn> [moves...]` — the opening book, as a tree.
pub fn book(args: &[String]) -> Result<(), String> {
    let mut path = None;
    let mut limit = usize::MAX;
    let mut line: Vec<String> = Vec::new();
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
            other if path.is_none() => path = Some(other.to_string()),
            other => line.push(other.to_string()),
        }
        i += 1;
    }
    let path = path.ok_or("usage: blockchess book <file.pgn> [moves...]")?;

    let games = load(&path, limit)?;
    let idx = build_index(&games, 60);
    let eco = load_eco()?;

    // Walk the requested line.
    let mut pos = Position::startpos();
    let mut moves = Vec::new();
    for tok in &line {
        let m = bc_pgn::parse_san(&pos, tok)
            .map_err(|e| format!("{tok}: {e}"))
            .or_else(|e| {
                bc_codec::order::canonical_moves(&pos)
                    .into_iter()
                    .find(|m| m.to_uci() == tok.to_ascii_lowercase())
                    .ok_or(e)
            })?;
        moves.push(m);
        pos = pos.make_move(m);
    }

    let Some(node) = idx.trie.walk(&moves) else {
        println!();
        println!("  {}", line_san(&Position::startpos(), &moves));
        println!();
        println!("  Nobody in this corpus has played that. It is a novelty here.");
        println!();
        return Ok(());
    };

    let score = idx.trie.node(node).score;
    println!();
    if moves.is_empty() {
        println!("  ALL GAMES");
    } else {
        println!("  {}", line_san(&Position::startpos(), &moves));
    }
    if let Some(e) = eco.classify(&moves) {
        println!("  {} {}", e.code, e.name);
    }
    println!("  {} games   white scores {}", score.total(), pct(&score));
    println!();

    let conts = idx.trie.continuations(node);
    if conts.is_empty() {
        println!("  No continuations recorded — every game in this line ended here.");
        println!();
        return Ok(());
    }

    println!(
        "  {:<8}{:>8}{:>9}  {:<10}  name",
        "move", "games", "white", "share"
    );
    let total: u32 = conts.iter().map(|(_, _, s)| s.total()).sum();
    for (m, _, s) in conts.iter().take(15) {
        let mut extended = moves.clone();
        extended.push(*m);
        let name = eco
            .name_of(&extended)
            .map(|e| format!("{} {}", e.code, e.name))
            .unwrap_or_default();
        let share = s.total() as f64 / total.max(1) as f64;
        let bar = "#".repeat((share * 10.0).round() as usize);
        println!(
            "  {:<8}{:>8}{:>9}  {:<10}  {}",
            to_san(&pos, *m),
            s.total(),
            pct(s),
            bar,
            name
        );
    }
    println!();
    Ok(())
}

/// `blockchess find <pgn> <FEN>` — every game that reached a position, by any
/// move order or mirror image.
pub fn find(args: &[String]) -> Result<(), String> {
    let mut path = None;
    let mut limit = usize::MAX;
    let mut fen: Vec<String> = Vec::new();
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
            other if path.is_none() => path = Some(other.to_string()),
            other => fen.push(other.to_string()),
        }
        i += 1;
    }
    let path = path.ok_or("usage: blockchess find <file.pgn> <FEN>")?;
    if fen.is_empty() {
        return Err("usage: blockchess find <file.pgn> <FEN>".into());
    }
    let target = Position::from_fen(&fen.join(" ")).map_err(|e| format!("{e}"))?;

    let games = load(&path, limit)?;
    let idx = build_index(&games, 60);

    println!();
    println!("  {}", target.to_fen());
    match idx.positions.lookup(&target) {
        None => {
            println!();
            println!("  Not reached by any game in this corpus.");
            println!();
            println!("  papers/05-index.md §5: proving *absence* is what a study database");
            println!("  most wants to say honestly, and it is why the index needs a");
            println!("  committed root before anyone should believe this answer from a");
            println!("  server they do not run.");
        }
        Some(entry) => {
            println!(
                "  reached by {} game(s)   white scores {}",
                entry.postings.len(),
                pct(&entry.score)
            );
            println!();
            println!("  {:<6}{:>6}  players", "game", "ply");
            for p in entry.postings.iter().take(20) {
                let g = &games[p.game as usize];
                let w = g.tag("White").unwrap_or("?");
                let b = g.tag("Black").unwrap_or("?");
                println!("  {:<6}{:>6}  {} vs {}", p.game, p.ply, w, b);
            }
            if entry.postings.len() > 20 {
                println!("  … and {} more", entry.postings.len() - 20);
            }
            // Transpositions are the whole reason this index exists, so say
            // when one actually happened.
            let mut plies: HashMap<u16, usize> = HashMap::new();
            for p in &entry.postings {
                *plies.entry(p.ply).or_default() += 1;
            }
            if plies.len() > 1 {
                let mut ks: Vec<_> = plies.keys().copied().collect();
                ks.sort();
                println!();
                println!(
                    "  Reached at {} different plies ({}) — these are transpositions,",
                    plies.len(),
                    ks.iter()
                        .map(|k| k.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                println!("  and a trie alone would have filed them as different openings.");
            }
        }
    }
    println!();
    Ok(())
}

fn parse_common(args: &[String], cmd: &str) -> Result<(String, usize), String> {
    let mut path = None;
    let mut limit = usize::MAX;
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
            other => path = Some(other.to_string()),
        }
        i += 1;
    }
    Ok((
        path.ok_or(format!("usage: blockchess {cmd} <file.pgn> [--limit N]"))?,
        limit,
    ))
}
