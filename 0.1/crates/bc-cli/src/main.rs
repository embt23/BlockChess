//! `blockchess` — the whole project, as one command.
//!
//! No argument-parsing dependency on purpose. The surface is six subcommands
//! and hand-rolling it keeps the dependency list at zero, which is worth more
//! here than the convenience.

mod annotate;
mod board;
mod chunk;
mod grammar;
mod measure;
mod pack;
mod play;
mod study;
mod think;

use std::process::ExitCode;

const USAGE: &str = "\
blockchess — a public chess database, in progress

USAGE
    blockchess <command> [args]

LOOKING AT CHESS
    play [FEN]              play a game in the terminal, both sides
                            --save PATH  write the game, and any notes you
                                         typed, when you quit
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

STUDYING IT
    index <file.pgn>              build the index, report its shape
    book <file.pgn> [moves...]    the opening book: what is played here
    find <file.pgn> <FEN>         every game that reached a position,
                                  by any move order or mirror image
    grammar <file.pgn>            let a compressor that knows no chess find
                                  the repeated patterns, then check them
                                  against human opening theory  (X7)

    chunks <file.pgn>             groups of squares that occur together far
                                  more than chance — the spatial version of
                                  chunking theory  (X9)
                            --from-ply N  skip the opening (default 20)
    think <file.pgn> [--corpus big.pgn]
                                  do bits track seconds? experiment X11 —
                                  needs a source with clocks, e.g. lichess
                                  --min-clock N  drop plies under N sec left

YOUR OWN PERCEPTION
    annotate <game.pgn> <notes.txt> [--corpus big.pgn]
                                  put what you noticed next to what the
                                  machine saw at the same moment
    export <game.pgn> [--notes notes.txt] > game.json
                                  the game as JSON, one record per ply, for
                                  a graphics layer to draw

  Every command takes --limit N to stop after N games.
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
        "index" => study::index(rest),
        "book" => study::book(rest),
        "find" => study::find(rest),
        "grammar" => grammar::run(rest),
        "annotate" => annotate::run(rest),
        "export" => annotate::export(rest),
        "think" => think::run(rest),
        "chunks" => chunk::run(rest),
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
