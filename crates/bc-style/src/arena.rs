//! The arena — a corpus on disk that grows.
//!
//! One directory, and it only ever grows:
//!
//! ```text
//!   arena/
//!     games/0001-alice.pgn     a submission, never modified after landing
//!     games/0002-bob.pgn
//!     MANIFEST                 one line per submission, append-only
//!     index.html               the page, regenerated from the whole corpus
//! ```
//!
//! # Why a manifest and not just a directory listing
//!
//! A lens is only fair if everyone derives the same one, and that needs the
//! same games **in the same order** — directory listings are ordered by
//! whatever the filesystem feels like. The manifest fixes the order, records
//! the digest of each file, and carries a hash chain so that changing or
//! reordering anything already in it breaks every line after.
//!
//! That is episode 01's primitive doing its original job one level up: a chain
//! of hashes turns a statement about one submission into a statement about
//! every submission before it.
//!
//! # Built to grow
//!
//! Adding games is an **append**: one file lands, one line joins the manifest,
//! the chain advances. Nothing already on disk is rewritten, and the corpus is
//! reconstructible by replaying the manifest in order. Refitting the lens is
//! still a whole-corpus operation — the basis is a property of all the games,
//! so it has to be — but the *data* path is incremental, which is what a
//! version that updates while you play would need.

use crate::corpus::{Corpus, GameRecord};
use crate::pgn;
use bc_hash::{hex, tagged_parts, Hash};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// One landed submission.
#[derive(Clone, Debug)]
pub struct Entry {
    pub index: u32,
    pub file: String,
    /// Digest of the file's bytes as it landed.
    pub digest: Hash,
    pub games: usize,
    /// Manifest chain head after this line.
    pub head: Hash,
}

/// What one `add` did.
#[derive(Clone, Debug)]
pub struct Added {
    pub file: String,
    pub accepted: usize,
    pub rejected: usize,
    pub players: Vec<String>,
    pub head: Hash,
}

/// What `verify` found.
#[derive(Clone, Debug)]
pub struct Verified {
    pub entries: usize,
    pub games: usize,
    pub head: Hash,
    pub corpus_root: Hash,
    /// Empty when everything checks out.
    pub problems: Vec<String>,
}

pub struct Arena {
    pub dir: PathBuf,
}

impl Arena {
    pub fn open(dir: impl AsRef<Path>) -> io::Result<Arena> {
        let dir = dir.as_ref().to_path_buf();
        fs::create_dir_all(dir.join("games"))?;
        let manifest = dir.join("MANIFEST");
        if !manifest.exists() {
            fs::write(&manifest, "# bc-style arena v1\n")?;
        }
        Ok(Arena { dir })
    }

    fn manifest_path(&self) -> PathBuf {
        self.dir.join("MANIFEST")
    }

    /// Replay the manifest. Order here is the corpus order, and therefore part
    /// of what every medal commits to.
    pub fn entries(&self) -> io::Result<Vec<Entry>> {
        let text = fs::read_to_string(self.manifest_path())?;
        let mut out = Vec::new();
        for line in text.lines() {
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }
            let f: Vec<&str> = line.split('\t').collect();
            if f.len() != 5 {
                continue;
            }
            out.push(Entry {
                index: f[0].parse().unwrap_or(0),
                file: f[1].to_string(),
                digest: parse_hex(f[2]),
                games: f[3].parse().unwrap_or(0),
                head: parse_hex(f[4]),
            });
        }
        Ok(out)
    }

    /// The chain head, or zeroes for an empty arena.
    pub fn head(&self) -> io::Result<Hash> {
        Ok(self.entries()?.last().map(|e| e.head).unwrap_or([0u8; 32]))
    }

    /// Validate a PGN file and land it.
    ///
    /// Nothing enters that `bc-chess` cannot replay, so the corpus cannot hold
    /// a game that was never legal. Rejected games are reported rather than
    /// silently dropped: a corpus that quietly loses games is a lens fitted to
    /// a subset nobody chose.
    pub fn add(&self, source: impl AsRef<Path>) -> io::Result<Added> {
        let text = fs::read_to_string(source.as_ref())?;
        let (games, rejected) = pgn::parse(&text);
        if games.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "no replayable games in that file",
            ));
        }

        let mut players: Vec<String> = Vec::new();
        for g in &games {
            for n in [&g.white, &g.black] {
                if !n.is_empty() && !players.contains(n) {
                    players.push(n.clone());
                }
            }
        }

        let entries = self.entries()?;
        let index = entries.len() as u32 + 1;
        let stem = source
            .as_ref()
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("games");
        let file = format!("{index:04}-{}.pgn", sanitise(stem));
        fs::write(self.dir.join("games").join(&file), &text)?;

        let digest = tagged_parts("BC/arena/file/v1", &[text.as_bytes()]);
        let prev = entries.last().map(|e| e.head).unwrap_or([0u8; 32]);
        let head = tagged_parts(
            "BC/arena/entry/v1",
            &[
                &prev,
                &index.to_le_bytes(),
                file.as_bytes(),
                &digest,
                &(games.len() as u32).to_le_bytes(),
            ],
        );

        let line = format!(
            "{index}\t{file}\t{}\t{}\t{}\n",
            hex(&digest),
            games.len(),
            hex(&head)
        );
        let mut manifest = fs::read_to_string(self.manifest_path())?;
        manifest.push_str(&line);
        fs::write(self.manifest_path(), manifest)?;

        Ok(Added {
            file,
            accepted: games.len(),
            rejected,
            players,
            head,
        })
    }

    /// Rebuild the corpus by replaying the manifest in order.
    pub fn corpus(&self) -> io::Result<Corpus> {
        let mut c = Corpus::new();
        for e in self.entries()? {
            let text = fs::read_to_string(self.dir.join("games").join(&e.file))?;
            let (games, _) = pgn::parse(&text);
            for g in games {
                c.push(g);
            }
        }
        Ok(c)
    }

    /// Check that nothing already landed has been changed.
    ///
    /// Three separate claims: every file still hashes to what the manifest
    /// recorded, the chain still links, and the corpus still holds the number
    /// of games the manifest says it does.
    pub fn verify(&self) -> io::Result<Verified> {
        let entries = self.entries()?;
        let mut problems = Vec::new();
        let mut prev = [0u8; 32];
        let mut claimed = 0usize;
        let mut corpus = Corpus::new();

        for e in &entries {
            // One read, used for both the digest check and the rebuild. A
            // second read would also be a second chance for the file to change
            // underneath us between the two.
            match fs::read_to_string(self.dir.join("games").join(&e.file)) {
                Ok(text) => {
                    if tagged_parts("BC/arena/file/v1", &[text.as_bytes()]) != e.digest {
                        problems.push(format!("{} has been modified since it landed", e.file));
                    }
                    let (games, _) = pgn::parse(&text);
                    for g in games {
                        corpus.push(g);
                    }
                }
                // Reported, not returned: a missing file is exactly what this
                // function exists to find, so failing here would mean the check
                // could never report its own subject.
                Err(_) => problems.push(format!("{} is missing", e.file)),
            }

            let expect = tagged_parts(
                "BC/arena/entry/v1",
                &[
                    &prev,
                    &e.index.to_le_bytes(),
                    e.file.as_bytes(),
                    &e.digest,
                    &(e.games as u32).to_le_bytes(),
                ],
            );
            if expect != e.head {
                problems.push(format!("manifest line {} does not link", e.index));
            }
            prev = e.head;
            claimed += e.games;
        }

        if corpus.len() != claimed && problems.is_empty() {
            problems.push(format!(
                "manifest claims {claimed} games, the files replay {}",
                corpus.len()
            ));
        }

        Ok(Verified {
            entries: entries.len(),
            games: corpus.len(),
            head: prev,
            corpus_root: corpus.root(),
            problems,
        })
    }

    /// Every distinct player, with how many games each has.
    pub fn roster(&self) -> io::Result<Vec<(String, usize)>> {
        let corpus = self.corpus()?;
        let mut out: Vec<(String, usize)> = Vec::new();
        for g in &corpus.games {
            for n in [&g.white, &g.black] {
                match out.iter_mut().find(|(p, _)| p == n) {
                    Some((_, c)) => *c += 1,
                    None => out.push((n.clone(), 1)),
                }
            }
        }
        out.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        Ok(out)
    }
}

/// Keep submitted names from escaping the games directory.
fn sanitise(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .take(40)
        .collect();
    if cleaned.is_empty() {
        "games".to_string()
    } else {
        cleaned
    }
}

fn parse_hex(s: &str) -> Hash {
    let mut out = [0u8; 32];
    let b = s.as_bytes();
    for (i, slot) in out.iter_mut().enumerate() {
        if b.len() >= 2 * i + 2 {
            *slot = u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap_or(0);
        }
    }
    out
}

/// A game record is only useful here if it replays; this keeps the type in the
/// public surface so callers can hold one without importing `corpus`.
pub type Game = GameRecord;
