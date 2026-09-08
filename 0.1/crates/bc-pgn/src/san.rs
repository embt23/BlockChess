//! Standard Algebraic Notation, in both directions.
//!
//! SAN is *ambiguous by design*: `Nf3` names a destination and a piece and
//! leaves you to work out which knight, which you can only do by generating
//! the legal moves. That is the whole argument in `papers/02-encodings.md`
//! against SAN as storage — it costs 40 bits per ply *and* still needs the
//! rules engine to read.
//!
//! Resolving it is therefore easy in exactly one way: generate every legal
//! move, render each one back to SAN, and keep the one that matches. That is
//! quadratic in the branching factor and it is obviously correct. This crate
//! does the faster thing — parse the fields and filter — but the round-trip
//! property `parse_san(pos, &to_san(pos, m)) == m` is tested over every legal
//! move of every position in the test corpus, which catches any disagreement
//! between the two directions.

use bc_chess::types::{file_of, rank_of, FLAG_CASTLE, FLAG_EP, FLAG_PROMO};
use bc_chess::{Move, Piece, Position, Square};

#[derive(Debug, PartialEq, Eq)]
pub struct SanError(pub String);

impl std::fmt::Display for SanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for SanError {}

/// Add the position to an error, for messages a human can act on.
pub fn san_error_context(pos: &Position, san: &str, e: &SanError) -> String {
    format!("{} in position {}: {}", san, pos.to_fen(), e.0)
}

/// Resolve one SAN token against a position.
pub fn parse_san(pos: &Position, san: &str) -> Result<Move, SanError> {
    let legal = pos.generate_legal();

    // Strip decoration. Check and mate markers carry no information we do not
    // already have, and `!?` and friends are commentary.
    let core = san.trim_end_matches(['+', '#', '!', '?']);
    if core.is_empty() {
        return Err(SanError("empty move".into()));
    }

    // Castling. `0-0` with zeros is common in European sources.
    let normalised = core.replace('0', "O");
    if normalised == "O-O" || normalised == "O-O-O" {
        let kingside = normalised == "O-O";
        for &m in &legal {
            if m.flag() == FLAG_CASTLE {
                let is_kingside = m.to() > m.from();
                if is_kingside == kingside {
                    return Ok(m);
                }
            }
        }
        return Err(SanError(format!("{core} is not available")));
    }

    let f = Fields::parse(core)?;

    let mut matches = Vec::new();
    for &m in &legal {
        if matches_fields(pos, m, &f) {
            matches.push(m);
        }
    }

    match matches.len() {
        1 => Ok(matches[0]),
        0 => Err(SanError(format!("no legal move matches {core}"))),
        n => Err(SanError(format!(
            "{core} is ambiguous between {n} legal moves"
        ))),
    }
}

/// The fields a SAN token can carry, all optional except the destination.
struct Fields {
    piece: Piece,
    dest: Square,
    from_file: Option<u8>,
    from_rank: Option<u8>,
    promo: Option<Piece>,
    capture: bool,
}

impl Fields {
    fn parse(core: &str) -> Result<Fields, SanError> {
        let chars: Vec<char> = core.chars().collect();
        let mut i = 0;

        let piece = match chars[0] {
            'N' => {
                i = 1;
                Piece::Knight
            }
            'B' => {
                i = 1;
                Piece::Bishop
            }
            'R' => {
                i = 1;
                Piece::Rook
            }
            'Q' => {
                i = 1;
                Piece::Queen
            }
            'K' => {
                i = 1;
                Piece::King
            }
            _ => Piece::Pawn,
        };

        // Promotion suffix, `=Q` or a bare trailing `Q`.
        let mut promo = None;
        let mut end = chars.len();
        if end >= 2 {
            if let Some(p) = piece_letter(chars[end - 1]) {
                if chars[end - 2] == '=' {
                    promo = Some(p);
                    end -= 2;
                } else if piece == Piece::Pawn {
                    promo = Some(p);
                    end -= 1;
                }
            }
        }

        // The destination is the last file+rank pair before any suffix.
        if end < 2 {
            return Err(SanError(format!("{core} is too short to name a square")));
        }
        let dest = square_from(chars[end - 2], chars[end - 1])
            .ok_or_else(|| SanError(format!("{core} does not end in a square")))?;

        // Whatever sits between the piece letter and the destination is the
        // disambiguator, plus possibly an `x`.
        let mut from_file = None;
        let mut from_rank = None;
        let mut capture = false;
        for &c in &chars[i..end - 2] {
            match c {
                'x' | ':' => capture = true,
                'a'..='h' => from_file = Some(c as u8 - b'a'),
                '1'..='8' => from_rank = Some(c as u8 - b'1'),
                '-' => {}
                _ => return Err(SanError(format!("{core} has an unexpected '{c}'"))),
            }
        }
        i = 0;
        let _ = i;

        Ok(Fields {
            piece,
            dest,
            from_file,
            from_rank,
            promo,
            capture,
        })
    }
}

fn matches_fields(pos: &Position, m: Move, f: &Fields) -> bool {
    if m.to() != f.dest {
        return false;
    }
    let Some((_, moved)) = pos.piece_at(m.from()) else {
        return false;
    };
    if moved != f.piece {
        return false;
    }
    if let Some(file) = f.from_file {
        if file_of(m.from()) != file {
            return false;
        }
    }
    if let Some(rank) = f.from_rank {
        if rank_of(m.from()) != rank {
            return false;
        }
    }
    match (f.promo, m.flag() == FLAG_PROMO) {
        (Some(p), true) => {
            if m.promo() != p {
                return false;
            }
        }
        // A promotion that named no piece is malformed, but plenty of files
        // contain it and the intent is always a queen.
        (None, true) => {
            if m.promo() != Piece::Queen {
                return false;
            }
        }
        (Some(_), false) => return false,
        (None, false) => {}
    }
    // `x` is a hint, not a constraint: some files omit it. Only reject when
    // the token claims a capture that did not happen.
    if f.capture && pos.piece_at(m.to()).is_none() && m.flag() != FLAG_EP {
        return false;
    }
    true
}

/// Render a move as SAN, disambiguating exactly as much as necessary.
pub fn to_san(pos: &Position, m: Move) -> String {
    if m.flag() == FLAG_CASTLE {
        let s = if m.to() > m.from() { "O-O" } else { "O-O-O" };
        return decorate(pos, m, s.to_string());
    }

    let Some((_, piece)) = pos.piece_at(m.from()) else {
        return m.to_uci();
    };
    let capture = pos.piece_at(m.to()).is_some() || m.flag() == FLAG_EP;
    let mut s = String::new();

    if piece == Piece::Pawn {
        if capture {
            s.push((b'a' + file_of(m.from())) as char);
        }
    } else {
        s.push(piece.ch().to_ascii_uppercase());
        s.push_str(&disambiguator(pos, m, piece));
    }

    if capture {
        s.push('x');
    }
    s.push((b'a' + file_of(m.to())) as char);
    s.push((b'1' + rank_of(m.to())) as char);

    if m.flag() == FLAG_PROMO {
        s.push('=');
        s.push(m.promo().ch().to_ascii_uppercase());
    }
    decorate(pos, m, s)
}

/// The shortest prefix that separates this move from its rivals: file if that
/// suffices, else rank, else both. This is the rule SAN actually specifies,
/// and getting it wrong produces tokens that round-trip in our own code and
/// nowhere else.
fn disambiguator(pos: &Position, m: Move, piece: Piece) -> String {
    let rivals: Vec<Move> = pos
        .generate_legal()
        .as_slice()
        .iter()
        .copied()
        .filter(|&o| {
            o.to() == m.to()
                && o.from() != m.from()
                && pos.piece_at(o.from()).map(|(_, p)| p) == Some(piece)
        })
        .collect();

    if rivals.is_empty() {
        return String::new();
    }
    if rivals
        .iter()
        .all(|o| file_of(o.from()) != file_of(m.from()))
    {
        return ((b'a' + file_of(m.from())) as char).to_string();
    }
    if rivals
        .iter()
        .all(|o| rank_of(o.from()) != rank_of(m.from()))
    {
        return ((b'1' + rank_of(m.from())) as char).to_string();
    }
    format!(
        "{}{}",
        (b'a' + file_of(m.from())) as char,
        (b'1' + rank_of(m.from())) as char
    )
}

fn decorate(pos: &Position, m: Move, mut s: String) -> String {
    let after = pos.make_move(m);
    if after.in_check(after.side) {
        let escapes = after.generate_legal();
        s.push(if escapes.is_empty() { '#' } else { '+' });
    }
    s
}

fn piece_letter(c: char) -> Option<Piece> {
    match c {
        'N' => Some(Piece::Knight),
        'B' => Some(Piece::Bishop),
        'R' => Some(Piece::Rook),
        'Q' => Some(Piece::Queen),
        _ => None,
    }
}

fn square_from(file: char, rank: char) -> Option<Square> {
    if !('a'..='h').contains(&file) || !('1'..='8').contains(&rank) {
        return None;
    }
    Some((rank as u8 - b'1') * 8 + (file as u8 - b'a'))
}
