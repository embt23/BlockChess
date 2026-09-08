//! Drawing a board in a terminal.
//!
//! Unicode pieces on a chequered background. The colours are ANSI 256 so that
//! it looks the same in most terminals, and the whole thing degrades to
//! letters if `NO_COLOR` is set, which some people need and which costs one
//! branch.

use bc_chess::types::{file_of, rank_of};
use bc_chess::{Color, Move, Piece, Position, Square};

const LIGHT: &str = "\x1b[48;5;180m";
const DARK: &str = "\x1b[48;5;137m";
const WHITE_PIECE: &str = "\x1b[38;5;255m";
const BLACK_PIECE: &str = "\x1b[38;5;232m";
const HIGHLIGHT: &str = "\x1b[48;5;108m";
const RESET: &str = "\x1b[0m";
const DIM: &str = "\x1b[2m";

fn plain() -> bool {
    std::env::var_os("NO_COLOR").is_some()
}

fn glyph(p: Piece) -> char {
    // One glyph per piece kind, coloured by side rather than using the hollow
    // outline set: outlines are unreadable on a light square in most fonts.
    match p {
        Piece::Pawn => '♟',
        Piece::Knight => '♞',
        Piece::Bishop => '♝',
        Piece::Rook => '♜',
        Piece::Queen => '♛',
        Piece::King => '♚',
    }
}

/// Draw the board from white's point of view, optionally highlighting the
/// squares of the move just played.
pub fn draw(pos: &Position, last: Option<Move>) -> String {
    let mut s = String::new();
    let touched = |sq: Square| match last {
        Some(m) => m.from() == sq || m.to() == sq,
        None => false,
    };

    for rank in (0..8).rev() {
        s.push_str(&format!("{DIM} {}{RESET} ", rank + 1));
        for file in 0..8 {
            let sq = rank * 8 + file;
            let piece = pos.piece_at(sq);
            if plain() {
                s.push(match piece {
                    Some((Color::White, p)) => p.ch().to_ascii_uppercase(),
                    Some((Color::Black, p)) => p.ch(),
                    None => '.',
                });
                s.push(' ');
                continue;
            }
            let bg = if touched(sq) {
                HIGHLIGHT
            } else if (rank + file) % 2 == 0 {
                DARK
            } else {
                LIGHT
            };
            match piece {
                Some((c, p)) => {
                    let fg = if c == Color::White {
                        WHITE_PIECE
                    } else {
                        BLACK_PIECE
                    };
                    s.push_str(&format!("{bg}{fg} {} {RESET}", glyph(p)));
                }
                None => s.push_str(&format!("{bg}   {RESET}")),
            }
        }
        s.push('\n');
    }
    s.push_str(&format!("{DIM}   "));
    for file in 0..8 {
        s.push_str(if plain() {
            match file {
                0 => "a ",
                1 => "b ",
                2 => "c ",
                3 => "d ",
                4 => "e ",
                5 => "f ",
                6 => "g ",
                _ => "h ",
            }
        } else {
            match file {
                0 => " a ",
                1 => " b ",
                2 => " c ",
                3 => " d ",
                4 => " e ",
                5 => " f ",
                6 => " g ",
                _ => " h ",
            }
        });
    }
    s.push_str(RESET);
    s.push('\n');
    s
}

/// `blockchess show <FEN>` — the board, plus everything the codec sees.
pub fn show(args: &[String]) -> Result<(), String> {
    let pos = crate::position_from(args)?;
    let legal = bc_codec::order::canonical_moves(&pos);

    println!();
    print!("{}", draw(&pos, None));
    println!();
    println!("  fen        {}", pos.to_fen());
    println!(
        "  to move    {}",
        if pos.side == Color::White {
            "white"
        } else {
            "black"
        }
    );
    println!(
        "  in check   {}",
        if pos.in_check(pos.side) { "yes" } else { "no" }
    );
    println!("  legal      {} moves", legal.len());
    if !legal.is_empty() {
        println!(
            "  E7 cost    {:.3} bits to name one of them",
            (legal.len() as f64).log2()
        );
    }
    println!();

    // The canonical order is the file format, so print it as such: index, the
    // move, and what a human would call it.
    println!("  {DIM}index  move   san      (the order is the format){RESET}");
    for (i, &m) in legal.iter().enumerate() {
        println!("  {i:>5}  {:<6} {}", m.to_uci(), bc_pgn::to_san(&pos, m));
    }
    println!();
    let _ = file_of;
    let _ = rank_of;
    Ok(())
}
