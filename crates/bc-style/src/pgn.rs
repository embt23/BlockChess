//! PGN ingest — so the lab can be pointed at real human games.
//!
//! SAN is resolved the direct way: parse the notation into constraints
//! (piece, destination, whatever disambiguation is present), then filter the
//! legal move list. That has a property worth having — **a game that does not
//! replay is rejected**, because the only moves it can resolve to are ones
//! `bc-chess` says are legal. Ingest is therefore also validation, which is
//! episode 03's attack ("that game is legal, trust me") pointed at the corpus.

use crate::corpus::GameRecord;
use bc_chess::types::{file_of, rank_of, Piece, FLAG_CASTLE, FLAG_PROMO};
use bc_chess::{Move, Position};

#[derive(Debug)]
pub struct PgnError(pub String);

impl std::fmt::Display for PgnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "bad PGN: {}", self.0)
    }
}
impl std::error::Error for PgnError {}

/// Resolve one SAN token against a position.
pub fn resolve_san(pos: &Position, token: &str) -> Result<Move, PgnError> {
    let san: String = token
        .chars()
        .filter(|c| !matches!(c, '+' | '#' | '!' | '?'))
        .collect();
    let legal = pos.generate_legal();

    // Castling. Both the letter and digit spellings appear in the wild.
    let normalised = san.replace('0', "O");
    if normalised == "O-O" || normalised == "O-O-O" {
        let kingside = normalised == "O-O";
        for &m in legal.as_slice() {
            if m.flag() == FLAG_CASTLE && (file_of(m.to()) == 6) == kingside {
                return Ok(m);
            }
        }
        return Err(PgnError(format!("no legal castle for {token}")));
    }

    let mut body = san.as_str();
    let mut promo: Option<Piece> = None;
    if let Some(eq) = body.find('=') {
        promo = Some(match body.as_bytes().get(eq + 1) {
            Some(b'N') => Piece::Knight,
            Some(b'B') => Piece::Bishop,
            Some(b'R') => Piece::Rook,
            Some(b'Q') => Piece::Queen,
            _ => return Err(PgnError(format!("bad promotion in {token}"))),
        });
        body = &body[..eq];
    }

    let first = body
        .chars()
        .next()
        .ok_or_else(|| PgnError("empty".into()))?;
    let piece = match first {
        'N' => Piece::Knight,
        'B' => Piece::Bishop,
        'R' => Piece::Rook,
        'Q' => Piece::Queen,
        'K' => Piece::King,
        _ => Piece::Pawn,
    };
    if piece != Piece::Pawn {
        body = &body[1..];
    }

    if body.len() < 2 {
        return Err(PgnError(format!("no destination in {token}")));
    }
    let dest = &body[body.len() - 2..];
    let db = dest.as_bytes();
    if !(b'a'..=b'h').contains(&db[0]) || !(b'1'..=b'8').contains(&db[1]) {
        return Err(PgnError(format!("bad destination in {token}")));
    }
    let to = (db[1] - b'1') * 8 + (db[0] - b'a');

    // Whatever is left is disambiguation, possibly with an 'x'.
    let mut hint_file = None;
    let mut hint_rank = None;
    for c in body[..body.len() - 2].chars() {
        match c {
            'a'..='h' => hint_file = Some(c as u8 - b'a'),
            '1'..='8' => hint_rank = Some(c as u8 - b'1'),
            'x' => {}
            _ => return Err(PgnError(format!("unexpected '{c}' in {token}"))),
        }
    }

    let mut found = None;
    for &m in legal.as_slice() {
        if m.to() != to {
            continue;
        }
        match pos.piece_at(m.from()) {
            Some((_, p)) if p == piece => {}
            _ => continue,
        }
        if let Some(want) = promo {
            if m.flag() != FLAG_PROMO || m.promo() != want {
                continue;
            }
        } else if m.flag() == FLAG_PROMO {
            continue;
        }
        if hint_file.is_some_and(|f| file_of(m.from()) != f) {
            continue;
        }
        if hint_rank.is_some_and(|r| rank_of(m.from()) != r) {
            continue;
        }
        if found.is_some() {
            return Err(PgnError(format!("ambiguous move {token}")));
        }
        found = Some(m);
    }
    found.ok_or_else(|| PgnError(format!("no legal move matches {token}")))
}

/// Parse a PGN file, which may contain many games.
///
/// Games that fail to replay are skipped rather than aborting the file; the
/// count of skipped games is returned alongside, because silently dropping data
/// would corrupt a corpus and every medal minted under it.
pub fn parse(text: &str) -> (Vec<GameRecord>, usize) {
    let mut games = Vec::new();
    let mut skipped = 0usize;
    let mut white = String::new();
    let mut black = String::new();
    let mut fen: Option<String> = None;
    let mut movetext = String::new();
    let mut in_moves = false;

    let flush = |white: &mut String,
                 black: &mut String,
                 fen: &mut Option<String>,
                 movetext: &mut String,
                 games: &mut Vec<GameRecord>,
                 skipped: &mut usize| {
        if !movetext.trim().is_empty() {
            match build(white, black, fen.as_deref(), movetext) {
                Ok(g) => games.push(g),
                Err(_) => *skipped += 1,
            }
        }
        white.clear();
        black.clear();
        *fen = None;
        movetext.clear();
    };

    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            if in_moves {
                flush(
                    &mut white,
                    &mut black,
                    &mut fen,
                    &mut movetext,
                    &mut games,
                    &mut skipped,
                );
                in_moves = false;
            }
            if let Some(v) = tag_value(t, "White") {
                white = v;
            } else if let Some(v) = tag_value(t, "Black") {
                black = v;
            } else if let Some(v) = tag_value(t, "FEN") {
                fen = Some(v);
            }
        } else if !t.is_empty() {
            in_moves = true;
            movetext.push(' ');
            movetext.push_str(t);
        }
    }
    flush(
        &mut white,
        &mut black,
        &mut fen,
        &mut movetext,
        &mut games,
        &mut skipped,
    );
    (games, skipped)
}

fn tag_value(line: &str, key: &str) -> Option<String> {
    let rest = line.strip_prefix('[')?.strip_prefix(key)?;
    let start = rest.find('"')? + 1;
    let end = rest[start..].find('"')? + start;
    Some(rest[start..end].to_string())
}

fn build(
    white: &str,
    black: &str,
    fen: Option<&str>,
    movetext: &str,
) -> Result<GameRecord, PgnError> {
    let start = match fen {
        Some(f) => Position::from_fen(f).map_err(|e| PgnError(e.to_string()))?,
        None => Position::startpos(),
    };
    let mut pos = start;
    let mut moves = Vec::new();

    for token in tokenise(movetext) {
        let m = resolve_san(&pos, &token)?;
        pos = pos.make_move(m);
        moves.push(m);
    }
    if moves.is_empty() {
        return Err(PgnError("no moves".into()));
    }
    Ok(GameRecord {
        white: white.to_string(),
        black: black.to_string(),
        start,
        moves,
    })
}

/// Strip comments, variations, NAGs, move numbers and results; keep SAN.
fn tokenise(movetext: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = movetext.chars().peekable();
    let mut cur = String::new();

    let push = |cur: &mut String, out: &mut Vec<String>| {
        if !cur.is_empty() {
            let t = cur.clone();
            cur.clear();
            let is_result = matches!(t.as_str(), "1-0" | "0-1" | "1/2-1/2" | "*");
            let is_number = t.chars().all(|c| c.is_ascii_digit() || c == '.');
            if !is_result && !is_number {
                out.push(t);
            }
        }
    };

    while let Some(c) = chars.next() {
        match c {
            '{' => {
                push(&mut cur, &mut out);
                for c in chars.by_ref() {
                    if c == '}' {
                        break;
                    }
                }
            }
            '(' => {
                push(&mut cur, &mut out);
                let mut depth = 1i32;
                for c in chars.by_ref() {
                    match c {
                        '(' => depth += 1,
                        ')' => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                }
            }
            '$' => {
                push(&mut cur, &mut out);
                while chars.peek().is_some_and(|c| c.is_ascii_digit()) {
                    chars.next();
                }
            }
            c if c.is_whitespace() => push(&mut cur, &mut out),
            _ => cur.push(c),
        }
    }
    push(&mut cur, &mut out);

    // A trailing move number like "42." attaches to nothing; drop leading
    // digits+dots from any token that still carries them (e.g. "1.e4").
    out.into_iter()
        .map(|t| {
            t.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.')
                .to_string()
        })
        .filter(|t| !t.is_empty())
        .collect()
}
