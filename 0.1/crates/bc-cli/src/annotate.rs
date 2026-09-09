//! `blockchess annotate` — put a person's perception next to the machine's.
//!
//! Evan played a game against himself, wrote down what he perceived for each
//! side, and drew shapes for how he saw the board. This command joins those
//! notes to what the corpus and the codec say about the same moments.
//!
//! ## Why the drawn shapes matter more than they look
//!
//! `papers/09-lineage.md` §5: Chase & Simon established that a master
//! remembers *chunks* — familiar groupings — rather than pieces. They could
//! only infer where one chunk ended and the next began **indirectly**, by
//! timing the pauses as a subject rebuilt a position from memory.
//!
//! A drawn shape is that boundary, stated directly. Somebody circling four
//! squares and calling them one thing is reporting a chunk. That is a better
//! measurement than the pauses, and it is why the notes format has a `shape`
//! line at all.
//!
//! ## The hypothesis this is built to test
//!
//! Notes should cluster where the machine spends many bits. A move that was
//! predictable from a hundred thousand other games is one a player glides
//! past; a surprising one is where they stop and write something down. If
//! notes fall uniformly regardless of bits, that is a real negative result and
//! `papers/10-players.md` §1 is in trouble.
//!
//! Playing both sides is a gift here, and not one that was planned for. The
//! same position gets perceived twice, once as each side's problem. Where the
//! two sets of notes *disagree* about a position is where perception is about
//! whose problem it is rather than about the board.

use std::collections::HashMap;

use bc_chess::{Move, Position, Square};
use bc_index::{Index, IndexedGame};
use bc_pgn::to_san;

use crate::board::draw;
use crate::study::{load, load_eco};

/// One line of somebody's notes.
#[derive(Debug, Clone)]
pub struct Note {
    /// Ply index, 0-based: ply 0 is before White's first move.
    pub ply: usize,
    /// `w`, `b`, or `.` for a note that is not from either side's view.
    pub side: char,
    pub text: String,
    /// Squares, when the line was a drawn shape rather than a remark.
    pub shape: Vec<Square>,
}

/// Parse a notes file.
///
/// ```text
/// # anything after a hash is ignored
/// 14w  the knight move felt forced
/// 14b  I didn't see that coming
/// 20   shape: e4 d5 c6 b7   the pawn chain
/// 27w  shape: f7 g8 h7      his king is drafty
/// ```
///
/// The position may be written as a ply number (`27`) or the way a person
/// actually thinks about it (`14w` = White's 14th move). Both are accepted
/// because asking somebody to convert move numbers to plies by hand is how
/// notes stop getting written.
pub fn parse_notes(text: &str) -> (Vec<Note>, Vec<String>) {
    let mut notes = Vec::new();
    let mut errors = Vec::new();

    for (n, raw) in text.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let Some((anchor, rest)) = line.split_once(char::is_whitespace) else {
            errors.push(format!("line {}: no text after the position", n + 1));
            continue;
        };

        let (ply, side) = match parse_anchor(anchor) {
            Some(v) => v,
            None => {
                errors.push(format!(
                    "line {}: '{anchor}' is not a ply or a move like 14w",
                    n + 1
                ));
                continue;
            }
        };

        let rest = rest.trim();
        let (shape, body) = match rest.strip_prefix("shape:") {
            Some(after) => {
                let mut squares = Vec::new();
                let mut words = Vec::new();
                for tok in after.split_whitespace() {
                    match square_from_name(tok) {
                        // Squares come first; the first non-square ends the
                        // list and everything after it is the label.
                        Some(s) if words.is_empty() => squares.push(s),
                        _ => words.push(tok),
                    }
                }
                (squares, words.join(" "))
            }
            None => (Vec::new(), rest.to_string()),
        };

        notes.push(Note {
            ply,
            side,
            text: body,
            shape,
        });
    }
    notes.sort_by_key(|n| n.ply);
    (notes, errors)
}

fn parse_anchor(s: &str) -> Option<(usize, char)> {
    let lower = s.to_ascii_lowercase();
    let (digits, side) = match lower.chars().last() {
        Some(c @ ('w' | 'b')) => (&lower[..lower.len() - 1], c),
        _ => (lower.as_str(), '.'),
    };
    let n: usize = digits.parse().ok()?;
    if side == '.' {
        // A bare number is a ply.
        Some((n, '.'))
    } else {
        // "14w" is White's 14th move, which is ply 26; "14b" is ply 27.
        let ply = (n.checked_sub(1)? * 2) + usize::from(side == 'b');
        Some((ply, side))
    }
}

fn square_from_name(s: &str) -> Option<Square> {
    let b = s.as_bytes();
    if b.len() != 2 {
        return None;
    }
    let f = b[0].to_ascii_lowercase();
    let r = b[1];
    if !(b'a'..=b'h').contains(&f) || !(b'1'..=b'8').contains(&r) {
        return None;
    }
    Some((r - b'1') * 8 + (f - b'a'))
}

pub fn run(args: &[String]) -> Result<(), String> {
    let mut game_path = None;
    let mut notes_path = None;
    let mut corpus = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--corpus" => {
                i += 1;
                corpus = Some(args.get(i).ok_or("--corpus needs a path")?.clone());
            }
            other if game_path.is_none() => game_path = Some(other.to_string()),
            other if notes_path.is_none() => notes_path = Some(other.to_string()),
            other => return Err(format!("unexpected argument '{other}'")),
        }
        i += 1;
    }
    let game_path =
        game_path.ok_or("usage: blockchess annotate <game.pgn> <notes.txt> [--corpus big.pgn]")?;
    let notes_path =
        notes_path.ok_or("usage: blockchess annotate <game.pgn> <notes.txt> [--corpus big.pgn]")?;

    let games = load(&game_path, usize::MAX)?;
    let game = games.first().ok_or("no game in that file")?;
    let notes_text =
        std::fs::read_to_string(&notes_path).map_err(|e| format!("{notes_path}: {e}"))?;
    let (notes, note_errors) = parse_notes(&notes_text);
    for e in &note_errors {
        eprintln!("  note: {e}");
    }
    if notes.is_empty() {
        return Err(format!("{notes_path}: no usable notes"));
    }

    // Replay the game, recording what the machine sees at every ply.
    let mut positions = vec![game.start];
    let mut bits = Vec::new();
    let mut pos = game.start;
    for &m in &game.moves {
        let n = bc_codec::order::canonical_moves(&pos).len();
        bits.push((n as f64).log2());
        pos = pos.make_move(m);
        positions.push(pos);
    }

    // Optional: what a corpus of other people's games says about the same
    // positions. Without it we still have the board and the bits.
    let index = match &corpus {
        Some(path) => {
            eprint!("  loading corpus… ");
            let corpus_games = load(path, usize::MAX)?;
            let idx = Index::build(
                corpus_games.iter().map(|g| IndexedGame {
                    moves: &g.moves,
                    start: g.start,
                    result: &g.result,
                }),
                60,
                0..60,
            );
            eprintln!("{} games", idx.games);
            Some(idx)
        }
        None => None,
    };
    let eco = load_eco()?;

    let mean_bits = bits.iter().sum::<f64>() / bits.len().max(1) as f64;

    println!();
    println!("  GAME    {game_path}");
    println!("  NOTES   {notes_path}");
    println!("  {:<26}{:>10}", "plies", game.moves.len());
    println!("  {:<26}{:>10}", "notes", notes.len());
    println!("  {:<26}{:>10.3}", "mean bits/ply this game", mean_bits);
    if let Some(e) = eco.classify(&game.moves) {
        println!("  {:<26}{} {}", "opening", e.code, e.name);
    }

    // ---- the join -------------------------------------------------------
    println!();
    println!("  WHERE YOU LOOKED, AND WHAT THE MACHINE SAW");
    println!();
    println!(
        "  {:<6}{:<5}{:>7}{:>8}{:>10}  your note",
        "ply", "side", "bits", "vs mean", "in corpus"
    );

    let mut noted_bits = Vec::new();
    for note in &notes {
        if note.ply > game.moves.len() {
            eprintln!("  note: ply {} is past the end of the game", note.ply);
            continue;
        }
        // The bits spent *arriving* at this moment: the move just played.
        let b = if note.ply == 0 {
            f64::NAN
        } else {
            bits[note.ply - 1]
        };
        if b.is_finite() {
            noted_bits.push(b);
        }
        let delta = if b.is_finite() {
            format!("{:+.2}", b - mean_bits)
        } else {
            "-".into()
        };
        let reach = match &index {
            Some(idx) => match idx.positions.lookup(&positions[note.ply]) {
                Some(e) => format!("{}", e.postings.len()),
                None => "novel".to_string(),
            },
            None => "-".to_string(),
        };
        let label = if note.shape.is_empty() {
            note.text.clone()
        } else {
            format!(
                "[shape {}] {}",
                note.shape
                    .iter()
                    .map(|&s| bc_codec::position::square_name(s))
                    .collect::<Vec<_>>()
                    .join(" "),
                note.text
            )
        };
        println!(
            "  {:<6}{:<5}{:>7}{:>8}{:>10}  {}",
            note.ply,
            note.side,
            if b.is_finite() {
                format!("{b:.2}")
            } else {
                "-".into()
            },
            delta,
            reach,
            label
        );
    }

    // ---- the result -----------------------------------------------------
    if !noted_bits.is_empty() {
        let noted_mean = noted_bits.iter().sum::<f64>() / noted_bits.len() as f64;
        println!();
        println!("  DID YOU LOOK WHERE THE MACHINE WAS SURPRISED?");
        println!("  {:<30}{:>8.3}", "mean bits, all plies", mean_bits);
        println!("  {:<30}{:>8.3}", "mean bits, plies you noted", noted_mean);
        println!(
            "  {:<30}{:>+8.3}   {}",
            "difference",
            noted_mean - mean_bits,
            if noted_mean > mean_bits {
                "you noted the harder moments"
            } else {
                "you noted the ordinary ones"
            }
        );
        println!();
        println!("  One game is an anecdote, not a result — with a handful of notes");
        println!("  this difference is well inside noise. It becomes a measurement");
        println!("  at a few hundred notes across many games. papers/10-players.md.");
        println!();
        println!("  And read `bits` carefully: without --corpus it is log2(legal");
        println!("  moves), which measures how BRANCHY a position is, not how hard.");
        println!("  A forced mate has almost no branching and enormous insight, so");
        println!("  it scores LOW here. Pass --corpus to price moves against what");
        println!("  other people actually played, which is the surprise that");
        println!("  papers/10-players.md §1 is really about.");
    }

    // ---- both sides -----------------------------------------------------
    let mut by_ply: HashMap<usize, Vec<&Note>> = HashMap::new();
    for n in &notes {
        by_ply.entry(n.ply).or_default().push(n);
    }
    let both: Vec<_> = by_ply
        .iter()
        .filter(|(_, v)| v.iter().any(|n| n.side == 'w') && v.iter().any(|n| n.side == 'b'))
        .collect();
    if !both.is_empty() {
        println!();
        println!("  THE SAME POSITION, PERCEIVED TWICE");
        println!("  You played both sides, so these plies have two readings. Where");
        println!("  they disagree, perception was about whose problem it was rather");
        println!("  than about the board.");
        println!();
        let mut plies: Vec<usize> = both.iter().map(|(&p, _)| p).collect();
        plies.sort();
        for p in plies {
            println!("  ply {p}");
            for n in &by_ply[&p] {
                println!("    {}  {}", n.side, n.text);
            }
        }
    }

    // ---- the shapes -----------------------------------------------------
    let shapes: Vec<&Note> = notes.iter().filter(|n| !n.shape.is_empty()).collect();
    if !shapes.is_empty() {
        println!();
        println!("  YOUR CHUNKS");
        println!();
        for n in &shapes {
            let occupied = n
                .shape
                .iter()
                .filter(|&&s| positions[n.ply].piece_at(s).is_some())
                .count();
            println!(
                "  ply {:<5} {} squares, {} occupied   {}",
                n.ply,
                n.shape.len(),
                occupied,
                n.text
            );
        }
        println!();
        println!("  These are chunk boundaries, stated directly. Chase & Simon had");
        println!("  to infer them from pauses during recall; you drew them. Comparing");
        println!("  them against machine-found groupings needs the position grammar,");
        println!("  which is stage 5 — say \"launch 5\".");
    }

    // ---- one board ------------------------------------------------------
    if let Some(first) = notes.first() {
        println!();
        println!("  ply {} — your first note", first.ply);
        println!();
        let last = if first.ply == 0 {
            None
        } else {
            Some(game.moves[first.ply - 1])
        };
        print!("{}", draw(&positions[first.ply], last));
        if first.ply > 0 {
            println!(
                "\n  last move  {}",
                to_san(&positions[first.ply - 1], game.moves[first.ply - 1])
            );
        }
        println!("  your note  {}", first.text);
    }
    println!();
    Ok(())
}

/// Write a game out as PGN, so `play` can save what was played.
pub fn to_pgn(start: &Position, moves: &[Move], tags: &[(&str, &str)]) -> String {
    let mut out = String::new();
    for (k, v) in tags {
        out.push_str(&format!("[{k} \"{v}\"]\n"));
    }
    if *start != Position::startpos() {
        out.push_str(&format!("[FEN \"{}\"]\n[SetUp \"1\"]\n", start.to_fen()));
    }
    out.push('\n');
    let mut pos = *start;
    for (i, &m) in moves.iter().enumerate() {
        if i % 2 == 0 {
            out.push_str(&format!("{}. ", i / 2 + 1));
        }
        out.push_str(&to_san(&pos, m));
        out.push(if (i + 1) % 10 == 0 { '\n' } else { ' ' });
        pos = pos.make_move(m);
    }
    out.push_str("*\n");
    out
}
