//! Standard Algebraic Notation — the move format every chess corpus uses.
//!
//! Sits beside `uci`, the other notation reader: UCI (`e2e4`) is what the
//! protocol and our own game files speak, SAN (`Nf3`) is what published games
//! and PGN archives speak. Both are notation over the same rules, so both live
//! here rather than wherever they happened to be needed first.
//!
//! Parsing SAN generatively (render each legal move to SAN, compare strings) is
//! fiddly because of disambiguation rules. Parsing it as a *filter* is not: SAN
//! states a piece, a destination, and as much of the origin as was needed to be
//! unambiguous, so decode those constraints and intersect them with the legal
//! move list.
//!
//! This makes a PGN parse **self-validating**. A misparse picks the wrong move,
//! which makes the *next* move illegal, so an error surfaces within a ply or
//! two instead of silently corrupting the corpus. A whole game that parses to
//! the end is almost certainly parsed correctly.

use crate::types::{Piece, FLAG_CASTLE, FLAG_PROMO};
use crate::{Move, Position};

/// Parse one SAN token in `pos`, returning the legal move it names.
pub fn parse_san(pos: &Position, token: &str) -> Option<Move> {
    // Strip check/mate marks and annotations: Qxe7+ Rd8# Nf3! e4?!
    let t: String = token
        .chars()
        .filter(|c| !matches!(c, '+' | '#' | '!' | '?'))
        .collect();
    if t.is_empty() {
        return None;
    }

    let legal = pos.generate_legal();

    // Castling. Accept both letter-O and digit-zero spellings.
    let norm = t.replace('0', "O");
    if norm == "O-O" || norm == "O-O-O" {
        let kingside = norm == "O-O";
        return legal
            .as_slice()
            .iter()
            .copied()
            .find(|m| m.flag() == FLAG_CASTLE && (m.to() % 8 > 4) == kingside);
    }

    let b = t.as_bytes();
    let (piece, rest) = match b[0] {
        b'K' => (Piece::King, &t[1..]),
        b'Q' => (Piece::Queen, &t[1..]),
        b'R' => (Piece::Rook, &t[1..]),
        b'B' => (Piece::Bishop, &t[1..]),
        b'N' => (Piece::Knight, &t[1..]),
        _ => (Piece::Pawn, &t[..]),
    };

    // Promotion suffix: "=Q", or bare "Q" as some writers emit.
    let (body, promo) = match rest.find('=') {
        Some(i) => (&rest[..i], rest[i + 1..].chars().next()),
        None => (rest, None),
    };
    let body = body.replace('x', "");
    if body.len() < 2 {
        return None;
    }

    // The destination is always the final two characters.
    let dst = &body[body.len() - 2..];
    let db = dst.as_bytes();
    if !(b'a'..=b'h').contains(&db[0]) || !(b'1'..=b'8').contains(&db[1]) {
        return None;
    }
    let to = (db[1] - b'1') * 8 + (db[0] - b'a');

    // Whatever is left over disambiguates the origin: a file, a rank, or both.
    let hint = &body[..body.len() - 2];
    let want_file = hint
        .bytes()
        .find(|c| (b'a'..=b'h').contains(c))
        .map(|c| c - b'a');
    let want_rank = hint
        .bytes()
        .find(|c| (b'1'..=b'8').contains(c))
        .map(|c| c - b'1');

    let mut found = None;
    for &m in legal.as_slice() {
        if m.to() != to {
            continue;
        }
        if pos.piece_at(m.from()).map(|(_, p)| p) != Some(piece) {
            continue;
        }
        if let Some(f) = want_file {
            if m.from() % 8 != f {
                continue;
            }
        }
        if let Some(r) = want_rank {
            if m.from() / 8 != r {
                continue;
            }
        }
        if let Some(pc) = promo {
            if m.flag() != FLAG_PROMO {
                continue;
            }
            let want = match pc.to_ascii_uppercase() {
                'Q' => Piece::Queen,
                'R' => Piece::Rook,
                'B' => Piece::Bishop,
                'N' => Piece::Knight,
                _ => return None,
            };
            if m.promo() != want {
                continue;
            }
        } else if m.flag() == FLAG_PROMO {
            continue;
        }
        // Ambiguity here means the notation was under-specified or we mis-read
        // it. Either way, refusing is safer than guessing.
        if found.is_some() {
            return None;
        }
        found = Some(m);
    }
    found
}

/// Parse a whole movetext body into moves, replaying from `start`.
///
/// Returns `None` at the first token that does not name a legal move.
pub fn parse_movetext(start: &Position, movetext: &str) -> Option<Vec<Move>> {
    let mut pos = *start;
    let mut out = Vec::new();
    for tok in clean_movetext(movetext).split_whitespace() {
        if matches!(tok, "1-0" | "0-1" | "1/2-1/2" | "*") {
            continue;
        }
        // Strip a leading move number. Writers emit "1. e4" and "1.e4"
        // interchangeably, and "1..." before a Black move — so the number must
        // be *removed* from the token, not used to discard it. Discarding the
        // whole token silently drops every move written without a space.
        let digits = tok.chars().take_while(|c| c.is_ascii_digit()).count();
        let tok = if digits > 0 && tok[digits..].starts_with('.') {
            let dots = tok[digits..].chars().take_while(|&c| c == '.').count();
            &tok[digits + dots..]
        } else {
            tok
        };
        if tok.is_empty() {
            continue;
        }
        let m = parse_san(&pos, tok)?;
        pos = pos.make_move(m);
        out.push(m);
    }
    Some(out)
}

/// Strip PGN comments `{...}`, variations `(...)`, and NAGs `$n`.
fn clean_movetext(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut brace = 0i32;
    let mut paren = 0i32;
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' => brace += 1,
            '}' => brace -= 1,
            '(' => paren += 1,
            ')' => paren -= 1,
            ';' => {
                // Rest-of-line comment.
                for c2 in chars.by_ref() {
                    if c2 == '\n' {
                        break;
                    }
                }
            }
            '$' => {
                while chars.peek().is_some_and(|c2| c2.is_ascii_digit()) {
                    chars.next();
                }
            }
            _ if brace <= 0 && paren <= 0 => out.push(c),
            _ => {}
        }
    }
    out
}
