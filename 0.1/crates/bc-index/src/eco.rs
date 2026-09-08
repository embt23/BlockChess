//! Opening names, so that what the machine finds can be checked against what
//! people already call things.
//!
//! This is the ground truth for `papers/08-layers.md` §3. The claim there is
//! that searching for the shortest encoding of a corpus is the same search as
//! searching for chess theory. The way to test a claim like that is to run the
//! compressor, take the sequences it decided were worth naming, and ask
//! whether humans had already named them.
//!
//! The dataset says what humans named. It is deliberately **not** given to the
//! grammar inducer, which sees only anonymous integers. If it were an input,
//! any agreement would be circular and worth nothing.

use std::collections::HashMap;

use bc_chess::{Move, Position};

#[derive(Debug, Clone)]
pub struct EcoEntry {
    pub code: String,
    pub name: String,
    /// The line, resolved into moves against the rules.
    pub moves: Vec<Move>,
}

#[derive(Debug, Default)]
pub struct Eco {
    pub entries: Vec<EcoEntry>,
    /// Move sequence to entry index. The lookup is exact: a line is named only
    /// if it matches an entry's moves precisely.
    by_line: HashMap<Vec<u16>, usize>,
}

impl Eco {
    /// Parse the vendored TSV. Rows that fail to resolve are skipped and
    /// counted rather than being fatal — the table is reference data, and one
    /// unparseable row should not take down an experiment.
    ///
    /// `resolve` turns a SAN token into a move; it is passed in so this crate
    /// does not have to depend on the PGN reader.
    pub fn parse(
        tsv: &str,
        mut resolve: impl FnMut(&Position, &str) -> Option<Move>,
    ) -> (Eco, usize) {
        let mut eco = Eco::default();
        let mut skipped = 0usize;

        for (i, line) in tsv.lines().enumerate() {
            if i == 0 && line.starts_with("eco\t") {
                continue;
            }
            let mut fields = line.split('\t');
            let (Some(code), Some(name), Some(pgn)) = (fields.next(), fields.next(), fields.next())
            else {
                continue;
            };

            let mut pos = Position::startpos();
            let mut moves = Vec::new();
            let mut ok = true;
            for tok in pgn.split_whitespace() {
                // Strip "12." and "12..." move numbers.
                let t = tok.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.');
                if t.is_empty() {
                    continue;
                }
                match resolve(&pos, t) {
                    Some(m) => {
                        moves.push(m);
                        pos = pos.make_move(m);
                    }
                    None => {
                        ok = false;
                        break;
                    }
                }
            }
            if !ok || moves.is_empty() {
                skipped += 1;
                continue;
            }

            let key: Vec<u16> = moves.iter().map(|m| m.0).collect();
            let idx = eco.entries.len();
            eco.entries.push(EcoEntry {
                code: code.to_string(),
                name: name.to_string(),
                moves,
            });
            // First name wins: the table is sorted, and later duplicates are
            // alternate spellings of the same line.
            eco.by_line.entry(key).or_insert(idx);
        }
        (eco, skipped)
    }

    /// Exact match: is this move sequence, from the start, a named opening?
    pub fn name_of(&self, moves: &[Move]) -> Option<&EcoEntry> {
        let key: Vec<u16> = moves.iter().map(|m| m.0).collect();
        self.by_line.get(&key).map(|&i| &self.entries[i])
    }

    /// The most specific named opening that is a *prefix* of this line.
    ///
    /// This is what a database does when you show it a game: 1.e4 c5 2.Nf3 d6
    /// is the Sicilian even if the exact 40-ply game is not in any table.
    pub fn classify(&self, moves: &[Move]) -> Option<&EcoEntry> {
        let mut best: Option<&EcoEntry> = None;
        for len in 1..=moves.len() {
            if let Some(e) = self.name_of(&moves[..len]) {
                best = Some(e);
            }
        }
        best
    }

    /// Does this move sequence occur *anywhere inside* a named opening, as a
    /// contiguous run?
    ///
    /// The looser question, and the honest one to ask of a grammar symbol.
    /// Re-Pair finds patterns wherever they repeat, not only at the start of a
    /// game, so a symbol may be a fragment of theory rather than a whole named
    /// line. Counting only exact matches would understate the agreement;
    /// counting this instead is the fair test, and it is reported separately
    /// so nobody can confuse the two.
    pub fn contains_run(&self, run: &[Move]) -> Option<&EcoEntry> {
        if run.is_empty() {
            return None;
        }
        self.entries.iter().find(|e| {
            e.moves
                .windows(run.len())
                .any(|w| w.iter().zip(run).all(|(a, b)| a == b))
        })
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
