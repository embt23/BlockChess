//! Command-line perft driver.
//!
//!   cargo run --release --bin perft -- <depth> ["fen"]
//!   cargo run --release --bin perft -- divide <depth> ["fen"]
//!
//! `divide` is the debugging tool. When your total is wrong, compare the
//! per-move breakdown against a known-good engine (Stockfish's `go perft N`
//! prints the same format), find the one move whose count differs, make that
//! move, and repeat. Each round cuts the search space by a factor of ~35, so a
//! bug at depth 6 is located in about six steps.

use bc_chess::{divide, perft, Position};
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (mode, rest) = match args.split_first() {
        Some((m, r)) if m == "divide" => ("divide", r.to_vec()),
        _ => ("perft", args.clone()),
    };

    let depth: u32 = rest.first().and_then(|d| d.parse().ok()).unwrap_or(5);
    let fen = rest
        .get(1)
        .cloned()
        .unwrap_or_else(|| Position::START_FEN.to_string());

    let pos = match Position::from_fen(&fen) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    println!("{pos}\n");

    let t = Instant::now();
    let total = if mode == "divide" {
        let rows = divide(&pos, depth);
        for (m, n) in &rows {
            println!("{}: {n}", m.to_uci());
        }
        println!();
        rows.iter().map(|(_, n)| n).sum()
    } else {
        perft(&pos, depth)
    };
    let dt = t.elapsed();

    println!("depth {depth}: {total} nodes in {:.3}s", dt.as_secs_f64());
    if dt.as_secs_f64() > 0.0 {
        println!("{:.1} Mnps", total as f64 / dt.as_secs_f64() / 1e6);
    }
}
