//! `blockchess` — the whole project, as one command.
//!
//! No argument-parsing dependency on purpose. The surface is six subcommands
//! and hand-rolling it keeps the dependency list at zero, which is worth more
//! here than the convenience.

mod board;
mod measure;
mod pack;
mod play;

use std::process::ExitCode;

const USAGE: &str = "\
blockchess — a public chess database, in progress

USAGE
    blockchess <command> [args]

LOOKING AT CHESS
    play [FEN]              play a game in the terminal, both sides
    show <FEN>              draw one position and list its legal moves
    perft <depth> [FEN]     count leaf nodes — the correctness oracle
    divide <depth> [FEN]    perft split by first move, for finding bugs

MEASURING IT
    measure <file.pgn>      the real numbers: bits/ply under each encoding
                            --limit N   stop after N games
                            --csv PATH  also write per-ply data

STORING IT
    pack <file.pgn> <out.bcg>     compress a PGN file
    unpack <in.bcg> [out.pgn]     decompress it again
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        print!("{USAGE}");
        return ExitCode::SUCCESS;
    }

    let rest = &args[1..];
    let result = match args[0].as_str() {
        "play" => play::run(rest),
        "show" => board::show(rest),
        "perft" => measure::perft(rest, false),
        "divide" => measure::perft(rest, true),
        "measure" => measure::run(rest),
        "pack" => pack::pack(rest),
        "unpack" => pack::unpack(rest),
        "-h" | "--help" | "help" => {
            print!("{USAGE}");
            Ok(())
        }
        other => Err(format!("unknown command '{other}'\n\n{USAGE}")),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Parse a FEN from the remaining arguments, or use the starting position.
pub fn position_from(args: &[String]) -> Result<bc_chess::Position, String> {
    if args.is_empty() {
        return Ok(bc_chess::Position::startpos());
    }
    let fen = args.join(" ");
    bc_chess::Position::from_fen(&fen).map_err(|e| format!("{e}"))
}
