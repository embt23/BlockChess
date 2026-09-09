//! Reading PGN, the format every chess database in the world already speaks.
//!
//! This crate exists so the project can be pointed at real games. Every number
//! in `papers/` that is currently a placeholder or a literature value needs a
//! corpus behind it, and a corpus means PGN.
//!
//! Two jobs, and the second is the hard one:
//!
//! - **Split** a PGN file into games and tag pairs. Tedious, not difficult.
//! - **Resolve SAN.** `Nf3` does not say *which* knight. Working that out
//!   requires generating the legal moves and finding the one that matches,
//!   which is why this crate depends on `bc-chess` and why SAN is a poor
//!   storage format despite being the universal interchange one.
//!   See `papers/02-encodings.md`, E1.

mod san;
mod tags;

pub use san::{parse_san, san_error_context, to_san, SanError};

use bc_chess::{Move, Position};

/// One game, as read from a PGN file.
#[derive(Debug, Clone)]
pub struct Game {
    /// The seven-tag roster and anything else the file carried, in file order.
    pub tags: Vec<(String, String)>,
    /// Moves, resolved against the rules. Always playable from `start`.
    pub moves: Vec<Move>,
    /// Usually the standard opening position; a `FEN` tag overrides it.
    pub start: Position,
    /// `1-0`, `0-1`, `1/2-1/2`, or `*` for unfinished.
    pub result: String,
    /// The clock reading after each move, in seconds, where the file carried
    /// one. Parallel to `moves`.
    ///
    /// Lichess writes `{ [%clk 0:02:58] }` after every move: the mover's
    /// remaining time *after* making it. That makes per-move think time
    /// derivable, which is the whole input to `papers/10-players.md` §1 —
    /// time spent is where a player's own compression failed. Discarding
    /// comments threw this away, so it is kept now.
    pub clocks: Vec<Option<u32>>,
    /// Base seconds and increment, from the `TimeControl` tag (`"300+3"`).
    pub time_control: Option<(u32, u32)>,
    /// Set when the movetext held a token that is not a chess move and the
    /// game was cut short there: the ply reached, and the offending token.
    ///
    /// The usual culprit is a null move — `--`, or ChessBase's `Z0`. A null
    /// move is not a move, so a game containing one cannot be replayed
    /// faithfully; but throwing away forty good moves because the forty-first
    /// is a placeholder is worse. So the prefix is kept and the reason is
    /// recorded, and a caller that needs whole games can filter on it.
    pub truncated: Option<(usize, String)>,
}

impl Default for Game {
    fn default() -> Self {
        Game {
            tags: Vec::new(),
            moves: Vec::new(),
            start: Position::startpos(),
            result: String::new(),
            clocks: Vec::new(),
            time_control: None,
            truncated: None,
        }
    }
}

impl Game {
    pub fn tag(&self, key: &str) -> Option<&str> {
        self.tags
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str())
    }

    /// Seconds spent on each move, where the clocks allow it.
    ///
    /// A player's think time is what came off their own clock, plus the
    /// increment they were given back for moving:
    ///
    /// ```text
    /// think(n) = clock_before(n) - clock_after(n) + increment
    /// ```
    ///
    /// `clock_before` is that same player's previous reading, two plies
    /// earlier, or the base time for their first move. `None` where the file
    /// gave no clock, and also where the arithmetic comes out negative —
    /// which happens with clock adjustments and berserk, and is better dropped
    /// than believed.
    pub fn think_times(&self) -> Vec<Option<f64>> {
        let (base, inc) = self.time_control.unwrap_or((0, 0));
        let mut out = Vec::with_capacity(self.moves.len());
        for ply in 0..self.moves.len() {
            let after = self.clocks.get(ply).copied().flatten();
            let before = if ply >= 2 {
                self.clocks.get(ply - 2).copied().flatten()
            } else if base > 0 {
                Some(base)
            } else {
                None
            };
            out.push(match (before, after) {
                (Some(b), Some(a)) => {
                    let t = b as f64 - a as f64 + inc as f64;
                    if t >= 0.0 {
                        Some(t)
                    } else {
                        None
                    }
                }
                _ => None,
            });
        }
        out
    }

    /// Every position the game passed through, starting position included.
    /// `positions().len() == moves.len() + 1`.
    pub fn positions(&self) -> Vec<Position> {
        let mut pos = self.start;
        let mut out = vec![pos];
        for &m in &self.moves {
            pos = pos.make_move(m);
            out.push(pos);
        }
        out
    }
}

/// What went wrong, and in which game, so a 3 GB file reports usefully.
#[derive(Debug)]
pub struct PgnError {
    pub game_index: usize,
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for PgnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "game {} (line {}): {}",
            self.game_index + 1,
            self.line,
            self.message
        )
    }
}

impl std::error::Error for PgnError {}

/// Read every game in a PGN document.
///
/// Games that fail to parse are *skipped and reported*, not fatal. Real PGN
/// dumps contain damaged games, and a corpus tool that stops at the first one
/// is a corpus tool that never finishes.
pub fn parse_all(text: &str) -> (Vec<Game>, Vec<PgnError>) {
    // A UTF-8 byte order mark sits in front of the first `[Event`, which
    // stops it looking like a tag line and makes the whole file parse as
    // one anonymous game with no moves. Windows tooling writes these
    // routinely. Found by running against python-chess's `utf8-bom.pgn`.
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut games = Vec::new();
    let mut errors = Vec::new();
    for (i, (chunk, line)) in split_games(text).into_iter().enumerate() {
        match parse_game(&chunk) {
            Ok(g) => games.push(g),
            Err(message) => errors.push(PgnError {
                game_index: i,
                line,
                message,
            }),
        }
    }
    (games, errors)
}

/// Resolve a UCI token like `e2e4` or `e7e8q` against a position.
fn parse_uci(pos: &Position, text: &str) -> Option<Move> {
    let t = text.to_ascii_lowercase();
    if t.len() < 4 || t.len() > 5 {
        return None;
    }
    pos.generate_legal()
        .as_slice()
        .iter()
        .copied()
        .find(|m| m.to_uci() == t)
}

/// Split a document into per-game chunks with the line each one started on.
///
/// The rule: a tag line that follows movetext starts a new game. Blank lines
/// alone will not do, because plenty of files in the wild put blank lines in
/// the middle of movetext.
fn split_games(text: &str) -> Vec<(String, usize)> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut start_line = 1;
    let mut seen_moves = false;

    for (n, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        let is_tag = trimmed.starts_with('[');
        if is_tag && seen_moves {
            out.push((std::mem::take(&mut cur), start_line));
            start_line = n + 1;
            seen_moves = false;
        }
        if !is_tag && !trimmed.is_empty() {
            seen_moves = true;
        }
        cur.push_str(line);
        cur.push('\n');
    }
    if !cur.trim().is_empty() {
        out.push((cur, start_line));
    }
    out
}

fn parse_game(chunk: &str) -> Result<Game, String> {
    let (tags, movetext) = tags::split(chunk);

    let start = match tags.iter().find(|(k, _)| k.eq_ignore_ascii_case("FEN")) {
        Some((_, fen)) => Position::from_fen(fen).map_err(|e| format!("bad FEN tag: {e}"))?,
        None => Position::startpos(),
    };

    let mut game = Game {
        tags,
        start,
        ..Default::default()
    };

    game.time_control = game.tag("TimeControl").and_then(|tc| {
        let (b, i) = tc.split_once('+').unwrap_or((tc, "0"));
        Some((b.trim().parse().ok()?, i.trim().parse().unwrap_or(0)))
    });

    let mut pos = start;
    for token in tokenise(&movetext) {
        match token {
            Token::Result(r) => game.result = r,
            Token::Clock(secs) => {
                // The reading belongs to the move just played. Pad if a
                // producer omitted clocks on some moves but not others.
                while game.clocks.len() + 1 < game.moves.len() {
                    game.clocks.push(None);
                }
                if game.clocks.len() < game.moves.len() {
                    game.clocks.push(Some(secs));
                }
            }
            Token::Null(t) => {
                // Stop here and say why. See `Game::truncated`.
                game.truncated = Some((game.moves.len(), t));
                break;
            }
            Token::San(text) => {
                let m = match parse_san(&pos, &text) {
                    Ok(m) => m,
                    // Some producers — CCRL's archives among them — write
                    // movetext in UCI rather than SAN. It is unambiguous and
                    // cheap to accept, and refusing it would reject whole
                    // archives over a notation choice.
                    Err(e) => match parse_uci(&pos, &text) {
                        Some(m) => m,
                        None => return Err(format!("move {}: {}", game.moves.len() + 1, e)),
                    },
                };
                game.moves.push(m);
                pos = pos.make_move(m);
            }
        }
    }

    if game.result.is_empty() {
        game.result = game.tag("Result").unwrap_or("*").to_string();
    }
    Ok(game)
}

enum Token {
    San(String),
    Result(String),
    /// A null-move placeholder: `--`, or ChessBase's `Z0`. Not a chess move.
    Null(String),
    /// A `[%clk H:MM:SS]` reading, in seconds, from inside a comment.
    Clock(u32),
}

/// Strip everything that is not a move: comments, variations, move numbers,
/// numeric annotation glyphs, and the trailing result.
fn tokenise(movetext: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let bytes: Vec<char> = movetext.chars().collect();
    let mut i = 0;
    let mut depth = 0usize; // nesting depth of ( ) variations

    while i < bytes.len() {
        let c = bytes[i];
        match c {
            '{' => {
                let start = i;
                while i < bytes.len() && bytes[i] != '}' {
                    i += 1;
                }
                let comment: String = bytes[start..i.min(bytes.len())].iter().collect();
                if depth == 0 {
                    if let Some(secs) = parse_clk(&comment) {
                        out.push(Token::Clock(secs));
                    }
                }
                i += 1;
            }
            ';' => {
                while i < bytes.len() && bytes[i] != '\n' {
                    i += 1;
                }
            }
            '(' => {
                depth += 1;
                i += 1;
            }
            ')' => {
                depth = depth.saturating_sub(1);
                i += 1;
            }
            '$' => {
                i += 1;
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
            }
            c if c.is_whitespace() => i += 1,
            _ => {
                let start = i;
                while i < bytes.len() && !bytes[i].is_whitespace() && bytes[i] != '{' {
                    i += 1;
                }
                let word: String = bytes[start..i].iter().collect();
                // Variations are alternatives that were never played. Skipping
                // them loses real content -- see the note in the crate docs.
                if depth == 0 {
                    classify(&word, &mut out);
                }
            }
        }
    }
    out
}

/// Pull `H:MM:SS` out of a `[%clk ...]` tag. Returns seconds.
fn parse_clk(comment: &str) -> Option<u32> {
    let at = comment.find("%clk")?;
    let rest = comment[at + 4..].trim_start();
    let end = rest.find(']').unwrap_or(rest.len());
    let mut total: u32 = 0;
    let mut parts = 0;
    for field in rest[..end].trim().split(':') {
        // Seconds may be fractional in some producers; take the whole part.
        let whole = field.split('.').next()?;
        total = total
            .checked_mul(60)?
            .checked_add(whole.trim().parse().ok()?)?;
        parts += 1;
    }
    if parts == 0 {
        None
    } else {
        Some(total)
    }
}

fn classify(word: &str, out: &mut Vec<Token>) {
    const RESULTS: [&str; 4] = ["1-0", "0-1", "1/2-1/2", "*"];
    const NULLS: [&str; 3] = ["--", "Z0", "@@@@"];
    if RESULTS.contains(&word) {
        out.push(Token::Result(word.to_string()));
        return;
    }
    if NULLS.contains(&word) {
        out.push(Token::Null(word.to_string()));
        return;
    }
    // "12." or "12..." prefixes a move, and may or may not be glued to it.
    let rest = word.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.');
    if rest.is_empty() {
        return;
    }
    if RESULTS.contains(&rest) {
        out.push(Token::Result(rest.to_string()));
        return;
    }
    if NULLS.contains(&rest) {
        out.push(Token::Null(rest.to_string()));
        return;
    }
    out.push(Token::San(rest.to_string()));
}
