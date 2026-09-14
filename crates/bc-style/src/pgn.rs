//! Reading a PGN corpus: headers, movetext, and the players behind them.
//!
//! `bc_chess::san` turns movetext into moves. This turns a file into games,
//! which is the boring half and the half with the encoding problems in it.
//!
//! Real corpora are dirty — `rozim/ChessData` says so on the tin — so every
//! fallible step here drops the game rather than failing the run. A corpus of
//! 138,000 games where 200 are malformed should produce a measurement, not a
//! stack trace. [`Corpus::skipped`] counts what was dropped so that "we
//! silently discarded 90% of the data" cannot hide.

use bc_chess::{san::parse_movetext, Move, Position};
use std::collections::BTreeMap;

/// One game, reduced to what the estimator needs.
#[derive(Debug, Clone)]
pub struct Game {
    pub white: String,
    pub black: String,
    pub moves: Vec<Move>,
}

#[derive(Debug, Default)]
pub struct Corpus {
    pub games: Vec<Game>,
    /// Games seen but not used, by reason. Kept visible on purpose.
    pub skipped: BTreeMap<&'static str, usize>,
}

impl Corpus {
    fn skip(&mut self, why: &'static str) {
        *self.skipped.entry(why).or_insert(0) += 1;
    }

    /// Games per player, both colours pooled.
    pub fn by_player(&self) -> BTreeMap<&str, Vec<usize>> {
        let mut m: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
        for (i, g) in self.games.iter().enumerate() {
            m.entry(g.white.as_str()).or_default().push(i);
            m.entry(g.black.as_str()).or_default().push(i);
        }
        m
    }
}

/// A header line `[Tag "value"]`, or `None`.
fn header(line: &str) -> Option<(&str, &str)> {
    let rest = line.strip_prefix('[')?.strip_suffix(']')?;
    let (tag, val) = rest.split_once(' ')?;
    Some((tag, val.trim().strip_prefix('"')?.strip_suffix('"')?))
}

/// Parse a PGN file's worth of text.
///
/// `start` is the position every game begins from. Games carrying a `FEN`
/// header start somewhere else and are skipped: a style model wants ordinary
/// games, and a handful of odds games are not worth the branch.
pub fn parse(text: &str, start: &Position, out: &mut Corpus) {
    let (mut white, mut black) = (String::new(), String::new());
    let mut movetext = String::new();
    let mut setup = false;
    let mut in_moves = false;

    // A blank line after movetext ends a game; PGN's own rule is a blank line
    // after the headers and another after the moves, which real files honour
    // inconsistently. Keying off "a header line after movetext" is sturdier.
    let flush = |white: &mut String,
                 black: &mut String,
                 movetext: &mut String,
                 setup: &mut bool,
                 out: &mut Corpus| {
        if movetext.trim().is_empty() {
            return;
        }
        let done = std::mem::take(movetext);
        let (w, b) = (std::mem::take(white), std::mem::take(black));
        let had_setup = std::mem::replace(setup, false);

        if had_setup {
            out.skip("non-standard start position");
        } else if w.is_empty() || b.is_empty() {
            out.skip("missing player name");
        } else {
            match parse_movetext(start, &done) {
                Some(moves) if moves.len() >= 10 => out.games.push(Game {
                    white: w,
                    black: b,
                    moves,
                }),
                Some(_) => out.skip("too short"),
                None => out.skip("unparseable movetext"),
            }
        }
    };

    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            if in_moves {
                flush(&mut white, &mut black, &mut movetext, &mut setup, out);
                in_moves = false;
            }
            match header(line) {
                Some(("White", v)) => white = v.to_string(),
                Some(("Black", v)) => black = v.to_string(),
                Some(("FEN", _)) | Some(("SetUp", "1")) => setup = true,
                _ => {}
            }
        } else if !line.is_empty() {
            in_moves = true;
            movetext.push(' ');
            movetext.push_str(line);
        }
    }
    flush(&mut white, &mut black, &mut movetext, &mut setup, out);
}

#[cfg(test)]
mod tests {
    use super::*;

    const TWO: &str = r#"
[Event "A"]
[White "Carlsen, M"]
[Black "Nakamura, Hi"]
[Result "1-0"]

1. e4 e5 2. Nf3 Nc6 3. Bb5 a6 4. Ba4 Nf6 5. O-O Be7 1-0

[Event "B"]
[White "Nakamura, Hi"]
[Black "Carlsen, M"]
[Result "0-1"]

1. d4 Nf6 2. c4 e6 3. Nc3 Bb4 4. e3 O-O 5. Bd3 d5 0-1
"#;

    #[test]
    fn reads_players_and_moves() {
        let mut c = Corpus::default();
        parse(TWO, &Position::startpos(), &mut c);
        assert_eq!(c.games.len(), 2, "{:?}", c.skipped);
        assert_eq!(c.games[0].white, "Carlsen, M");
        assert_eq!(c.games[0].moves.len(), 10);

        // Both players are indexed across both colours.
        let by = c.by_player();
        assert_eq!(by["Carlsen, M"].len(), 2);
        assert_eq!(by["Nakamura, Hi"].len(), 2);
    }

    #[test]
    fn dirty_games_are_dropped_and_counted_not_fatal() {
        let dirty =
            format!("{TWO}\n[Event \"C\"]\n[White \"X\"]\n[Black \"Y\"]\n\n1. e4 e5 2. Qq9 *\n");
        let mut c = Corpus::default();
        parse(&dirty, &Position::startpos(), &mut c);
        assert_eq!(c.games.len(), 2);
        assert_eq!(c.skipped.values().sum::<usize>(), 1);
    }

    #[test]
    fn games_from_a_set_up_position_are_skipped() {
        let odds = "[White \"X\"]\n[Black \"Y\"]\n[FEN \"8/8/4k3/8/8/4K3/8/8 w - - 0 1\"]\n\n1. Ke4 Kd6 2. Kd4 Ke6 3. Ke4 Kd6 4. Kd4 Ke6 5. Ke4 Kd6 *\n";
        let mut c = Corpus::default();
        parse(odds, &Position::startpos(), &mut c);
        assert!(c.games.is_empty());
        assert_eq!(c.skipped["non-standard start position"], 1);
    }
}
