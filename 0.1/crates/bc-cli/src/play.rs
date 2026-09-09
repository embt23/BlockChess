//! `blockchess play` — a board you can push pieces around on.
//!
//! Both sides are played by whoever is at the keyboard. There is no engine
//! here and there should not be one: the project is a database, not an
//! opponent. What this is for is *seeing* the thing the rest of the code
//! reasons about — the legal move list, its canonical order, and how many bits
//! each choice costs.

use std::io::{BufRead, Write};

use bc_chess::{Color, Position};

use crate::board::draw;

const HELP: &str = "\
  moves    e4, Nf3, O-O, e8=Q      or   e2e4, g1f3
  back     undo the last ply
  fen      print the position
  list     the legal moves in canonical order, with their indices
  bits     what the game costs so far under each encoding
  note     record what you are perceiving right now
  save     write the game (and any notes) to disk
  quit
";

pub fn run(args: &[String]) -> Result<(), String> {
    // --save <path> writes game.pgn and, if any were typed, game.notes.txt
    let mut save_to: Option<String> = None;
    let mut rest: Vec<String> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--save" => {
                i += 1;
                save_to = Some(args.get(i).ok_or("--save needs a path")?.clone());
            }
            other => rest.push(other.to_string()),
        }
        i += 1;
    }
    let args = &rest[..];
    let start = crate::position_from(args)?;
    // Notes typed during the game, as (ply, text).
    let mut notes: Vec<(usize, String)> = Vec::new();
    let mut pos = start;
    let mut history: Vec<bc_chess::Move> = Vec::new();

    println!("\n  a board. type 'help' for what you can do.\n");

    let stdin = std::io::stdin();
    let mut lines = stdin.lock().lines();

    loop {
        let last = history.last().copied();
        // Redraw from the position *before* the last move so the highlight
        // shows where the piece came from as well as where it went.
        println!();
        print!("{}", draw(&pos, last));

        let legal = bc_codec::order::canonical_moves(&pos);
        let status = if legal.is_empty() {
            if pos.in_check(pos.side) {
                "checkmate".to_string()
            } else {
                "stalemate".to_string()
            }
        } else {
            format!(
                "{} to move, {} legal, {:.2} bits",
                if pos.side == Color::White {
                    "white"
                } else {
                    "black"
                },
                legal.len(),
                (legal.len() as f64).log2()
            )
        };
        println!("\n  ply {}  —  {}\n", history.len(), status);

        print!("  > ");
        let _ = std::io::stdout().flush();
        let Some(line) = lines.next() else { break };
        let line = line.map_err(|e| e.to_string())?;
        let cmd = line.trim();

        match cmd {
            "" => continue,
            "quit" | "q" | "exit" => break,
            "help" | "?" => print!("{HELP}"),
            "fen" => println!("  {}", pos.to_fen()),
            "back" => {
                if history.pop().is_some() {
                    pos = start;
                    for &m in &history {
                        pos = pos.make_move(m);
                    }
                } else {
                    println!("  nothing to undo");
                }
            }
            "list" => {
                for (i, &m) in legal.iter().enumerate() {
                    println!("  {i:>3}  {:<6} {}", m.to_uci(), bc_pgn::to_san(&pos, m));
                }
            }
            "bits" => report_bits(&start, &history),
            "save" => match &save_to {
                Some(path) => save(path, &start, &history, &notes)?,
                None => println!("  start with --save <path> to enable saving"),
            },
            _ if cmd.starts_with("note ") || cmd == "note" => {
                let text = cmd.strip_prefix("note").unwrap_or("").trim();
                if text.is_empty() {
                    println!("  usage: note <what you are perceiving>");
                    println!("         note shape: e4 d5 c6   <what that grouping is>");
                } else {
                    notes.push((history.len(), text.to_string()));
                    println!("  recorded at ply {}", history.len());
                }
            }
            _ => match resolve(&pos, cmd) {
                Some(m) => {
                    history.push(m);
                    pos = pos.make_move(m);
                }
                None => println!("  '{cmd}' is not a legal move here — try 'list'"),
            },
        }
    }
    if let Some(path) = &save_to {
        save(path, &start, &history, &notes)?;
    }
    Ok(())
}

/// Write the game as PGN, and any notes alongside it in the format
/// `blockchess annotate` reads. Saving both together matters: a note is
/// anchored to a ply, and a ply only means something next to its game.
fn save(
    path: &str,
    start: &Position,
    moves: &[bc_chess::Move],
    notes: &[(usize, String)],
) -> Result<(), String> {
    let pgn = crate::annotate::to_pgn(
        start,
        moves,
        &[("Event", "Self-play"), ("White", "?"), ("Black", "?")],
    );
    std::fs::write(path, pgn).map_err(|e| format!("{path}: {e}"))?;
    println!("  wrote {} ({} plies)", path, moves.len());

    if !notes.is_empty() {
        let notes_path = path
            .strip_suffix(".pgn")
            .map(|b| format!("{b}.notes.txt"))
            .unwrap_or_else(|| format!("{path}.notes.txt"));
        let mut out = String::from("# ply  text — read by `blockchess annotate`\n");
        for (ply, text) in notes {
            out.push_str(&format!("{ply}  {text}\n"));
        }
        std::fs::write(&notes_path, out).map_err(|e| format!("{notes_path}: {e}"))?;
        println!("  wrote {} ({} notes)", notes_path, notes.len());
    }
    Ok(())
}

/// Accept either SAN or the plain from-to form, because people type both.
fn resolve(pos: &Position, text: &str) -> Option<bc_chess::Move> {
    if let Ok(m) = bc_pgn::parse_san(pos, text) {
        return Some(m);
    }
    bc_codec::order::canonical_moves(pos)
        .into_iter()
        .find(|m| m.to_uci() == text.to_ascii_lowercase())
}

fn report_bits(start: &Position, moves: &[bc_chess::Move]) {
    if moves.is_empty() {
        println!("  no moves yet");
        return;
    }
    let bits = bc_codec::index_bits(start, moves);
    let e3 = bc_codec::encode(bc_codec::Scheme::Fixed, start, moves).unwrap_or_default();
    let e7 = bc_codec::encode(bc_codec::Scheme::Index, start, moves).unwrap_or_default();
    println!("  plies            {}", moves.len());
    println!(
        "  E7 exact cost    {:.3} bits  ({:.3} bits/ply)",
        bits,
        bits / moves.len() as f64
    );
    println!("  E3 payload       {} bytes", e3.len());
    println!("  E7 payload       {} bytes", e7.len());
    println!(
        "  saving           {:.2}x",
        e3.len() as f64 / e7.len().max(1) as f64
    );
}
