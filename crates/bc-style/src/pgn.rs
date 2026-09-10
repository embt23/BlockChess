//! PGN ingest — so the lab can be pointed at real human games.
//!
//! SAN is resolved the direct way: parse the notation into constraints
//! (piece, destination, whatever disambiguation is present), then filter the
//! legal move list. That has a property worth having — **a game that does not
//! replay is rejected**, because the only moves it can resolve to are ones
//! `bc-chess` says are legal. Ingest is therefore also validation, which is
//! episode 03's attack ("that game is legal, trust me") pointed at the corpus.

use crate::corpus::GameRecord;
use bc_chess::types::{file_of, rank_of, Piece, FLAG_CASTLE, FLAG_EP, FLAG_PROMO};
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
    let mut tags = Tags::default();
    let mut movetext = String::new();
    let mut in_moves = false;

    for line in text.lines() {
        // A BOM on the first line would otherwise stop `[Event ...]` looking
        // like a tag, and the whole file would parse as one nameless game.
        let t = line.trim_start_matches('\u{feff}').trim();

        if t.starts_with('[') {
            if in_moves {
                flush(&mut tags, &mut movetext, &mut games, &mut skipped);
                in_moves = false;
            }
            tags.read(t);
        } else if !t.is_empty() {
            in_moves = true;
            // A `;` comment runs to end of line. Lines are joined below, so it
            // has to go now — afterwards there is no end of line left to run
            // to, and it would swallow the rest of the game.
            let t = match t.find(';') {
                Some(i) => t[..i].trim_end(),
                None => t,
            };
            if !t.is_empty() {
                movetext.push(' ');
                movetext.push_str(t);
            }
        }
    }
    flush(&mut tags, &mut movetext, &mut games, &mut skipped);
    (games, skipped)
}

/// The tags of the game currently being read.
#[derive(Default)]
struct Tags {
    white: String,
    black: String,
    white_elo: Option<u16>,
    black_elo: Option<u16>,
    variant: Option<String>,
    fen: Option<String>,
}

impl Tags {
    fn read(&mut self, line: &str) {
        if let Some(v) = tag_value(line, "White") {
            self.white = v;
        } else if let Some(v) = tag_value(line, "Black") {
            self.black = v;
        } else if let Some(v) = tag_value(line, "WhiteElo") {
            self.white_elo = v.parse().ok();
        } else if let Some(v) = tag_value(line, "BlackElo") {
            self.black_elo = v.parse().ok();
        } else if let Some(v) = tag_value(line, "Variant") {
            self.variant = Some(v);
        } else if let Some(v) = tag_value(line, "FEN") {
            self.fen = Some(v);
        }
    }
}

fn flush(tags: &mut Tags, movetext: &mut String, games: &mut Vec<GameRecord>, skipped: &mut usize) {
    if !movetext.trim().is_empty() {
        match build(tags, movetext) {
            Ok(g) => games.push(g),
            Err(_) => *skipped += 1,
        }
    }
    *tags = Tags::default();
    movetext.clear();
}

/// Read one tag, requiring the name to end where the key ends.
///
/// Without that requirement `[WhiteElo "1523"]` answers to `White`, and on any
/// real export every player is named by their rating rather than themselves.
/// See `docs/build-log.md` 07.
fn tag_value(line: &str, key: &str) -> Option<String> {
    let rest = line.strip_prefix('[')?.strip_prefix(key)?;
    if !rest.starts_with([' ', '\t']) {
        return None;
    }
    let start = rest.find('"')? + 1;
    let end = rest[start..].find('"')? + start;
    Some(rest[start..end].to_string())
}

fn build(tags: &Tags, movetext: &str) -> Result<GameRecord, PgnError> {
    // Chess960 and the rest are different games with different castling; this
    // engine would mis-replay them rather than fail loudly, which is worse.
    // "From Position" is ordinary chess from a supplied FEN, so it is allowed.
    if let Some(v) = &tags.variant {
        if !v.eq_ignore_ascii_case("standard") && !v.eq_ignore_ascii_case("from position") {
            return Err(PgnError(format!("unsupported variant {v}")));
        }
    }
    let start = match tags.fen.as_deref() {
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
        white: tags.white.clone(),
        black: tags.black.clone(),
        start,
        moves,
        white_elo: tags.white_elo,
        black_elo: tags.black_elo,
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

/// Render one move as SAN, in the position it is played in.
///
/// This exists for two reasons and the first is the important one: it gives the
/// *reader* an oracle. Rendering a known game and parsing it back must return
/// the same moves, so the parser can be tested against thousands of generated
/// games rather than against a handful of examples somebody typed by hand.
///
/// Disambiguation follows the standard rule — file if that separates the
/// candidates, else rank, else both — and it is computed from the legal move
/// list rather than from piece geometry, so a piece that is pinned and
/// therefore *cannot* legally move to the square correctly does not force a
/// disambiguator.
pub fn write_san(pos: &Position, m: Move) -> String {
    let mut s = String::new();

    if m.flag() == FLAG_CASTLE {
        s.push_str(if file_of(m.to()) == 6 { "O-O" } else { "O-O-O" });
    } else {
        let (_, piece) = match pos.piece_at(m.from()) {
            Some(p) => p,
            None => return String::new(),
        };
        let captures = m.flag() == FLAG_EP || pos.piece_at(m.to()).is_some();

        if piece == Piece::Pawn {
            if captures {
                s.push((b'a' + file_of(m.from())) as char);
                s.push('x');
            }
        } else {
            s.push(piece.ch().to_ascii_uppercase());

            // Which other pieces of this kind could also land there legally?
            let rivals: Vec<Move> = pos
                .generate_legal()
                .as_slice()
                .iter()
                .copied()
                .filter(|o| {
                    o.to() == m.to()
                        && o.from() != m.from()
                        && matches!(pos.piece_at(o.from()), Some((_, p)) if p == piece)
                })
                .collect();
            if !rivals.is_empty() {
                let same_file = rivals
                    .iter()
                    .any(|o| file_of(o.from()) == file_of(m.from()));
                let same_rank = rivals
                    .iter()
                    .any(|o| rank_of(o.from()) == rank_of(m.from()));
                if !same_file {
                    s.push((b'a' + file_of(m.from())) as char);
                } else if !same_rank {
                    s.push((b'1' + rank_of(m.from())) as char);
                } else {
                    s.push((b'a' + file_of(m.from())) as char);
                    s.push((b'1' + rank_of(m.from())) as char);
                }
            }
            if captures {
                s.push('x');
            }
        }
        s.push((b'a' + file_of(m.to())) as char);
        s.push((b'1' + rank_of(m.to())) as char);
        if m.flag() == FLAG_PROMO {
            s.push('=');
            s.push(m.promo().ch().to_ascii_uppercase());
        }
    }

    let after = pos.make_move(m);
    if after.in_check(after.side) {
        s.push(if after.generate_legal().is_empty() {
            '#'
        } else {
            '+'
        });
    }
    s
}

/// Render a game's movetext, numbered as PGN expects.
pub fn write_movetext(g: &GameRecord) -> String {
    let mut pos = g.start;
    let mut out = String::new();
    let mut n = pos.fullmove;
    for (i, &m) in g.moves.iter().enumerate() {
        if pos.side == bc_chess::types::Color::White {
            out.push_str(&format!("{n}. "));
        } else if i == 0 {
            out.push_str(&format!("{n}... "));
        }
        out.push_str(&write_san(&pos, m));
        out.push(' ');
        if pos.side == bc_chess::types::Color::Black {
            n += 1;
        }
        pos = pos.make_move(m);
    }
    out.trim_end().to_string()
}
