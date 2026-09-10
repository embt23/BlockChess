//! The arena's claims, checked.
//!
//! Three of them matter: what lands cannot be changed without detection, the
//! order games arrive in is fixed and part of what medals commit to, and a
//! corpus rebuilt from disk is the same corpus.

use bc_style::arena::Arena;
use bc_style::pgn;
use bc_style::synth::{self, Rng};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

static N: AtomicU32 = AtomicU32::new(0);

fn scratch(tag: &str) -> PathBuf {
    let n = N.fetch_add(1, Ordering::SeqCst);
    let p = std::env::temp_dir().join(format!("bc-arena-{}-{tag}-{n}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

/// A PGN file of `n` games between two named people.
fn write_pgn(dir: &Path, name: &str, white: &str, black: &str, n: usize, seed: u64) -> PathBuf {
    let mut rng = Rng::new(seed);
    let mut text = String::new();
    for _ in 0..n {
        let g = synth::play(&synth::ARCHETYPES[0], &synth::ARCHETYPES[1], &mut rng, 120);
        text.push_str(&format!(
            "[White \"{white}\"]\n[Black \"{black}\"]\n[Result \"*\"]\n\n{} *\n\n",
            pgn::write_movetext(&g)
        ));
    }
    let p = dir.join(format!("{name}.pgn"));
    fs::write(&p, text).unwrap();
    p
}

#[test]
fn a_submission_lands_and_the_chain_advances() {
    let dir = scratch("land");
    let a = Arena::open(dir.join("arena")).unwrap();
    assert_eq!(a.head().unwrap(), [0u8; 32], "an empty arena has no head");

    let src = write_pgn(&dir, "alice", "alice", "bob", 3, 1);
    let added = a.add(src.as_path()).unwrap();
    assert_eq!(added.accepted, 3);
    assert_eq!(added.rejected, 0);
    assert!(added.players.contains(&"alice".to_string()));

    let entries = a.entries().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].index, 1);
    assert_ne!(a.head().unwrap(), [0u8; 32]);

    let second = write_pgn(&dir, "carol", "carol", "dave", 2, 2);
    let head_before = a.head().unwrap();
    a.add(&second).unwrap();
    assert_eq!(a.entries().unwrap().len(), 2);
    assert_ne!(a.head().unwrap(), head_before, "the head did not advance");
}

#[test]
fn the_corpus_replays_from_disk() {
    let dir = scratch("replay");
    let a = Arena::open(dir.join("arena")).unwrap();
    a.add(write_pgn(&dir, "one", "alice", "bob", 3, 1)).unwrap();
    a.add(write_pgn(&dir, "two", "carol", "dave", 2, 2))
        .unwrap();

    let c = a.corpus().unwrap();
    assert_eq!(c.len(), 5);
    let players = c.players();
    for who in ["alice", "bob", "carol", "dave"] {
        assert!(players.contains(&who.to_string()), "{who} is missing");
    }

    // Reopening must give the identical corpus — nothing lives only in memory.
    let reopened = Arena::open(dir.join("arena")).unwrap();
    assert_eq!(reopened.corpus().unwrap().root(), c.root());
}

#[test]
fn a_clean_arena_verifies() {
    let dir = scratch("clean");
    let a = Arena::open(dir.join("arena")).unwrap();
    a.add(write_pgn(&dir, "one", "alice", "bob", 3, 1)).unwrap();
    a.add(write_pgn(&dir, "two", "carol", "dave", 2, 2))
        .unwrap();

    let v = a.verify().unwrap();
    assert!(
        v.problems.is_empty(),
        "clean arena complained: {:?}",
        v.problems
    );
    assert_eq!(v.entries, 2);
    assert_eq!(v.games, 5);
}

/// The point of recording a digest: a game that landed cannot be quietly
/// rewritten afterwards, because the lens fitted to it would silently change
/// and every medal with it.
#[test]
fn editing_a_landed_file_is_detected() {
    let dir = scratch("tamper-file");
    let a = Arena::open(dir.join("arena")).unwrap();
    a.add(write_pgn(&dir, "one", "alice", "bob", 3, 1)).unwrap();

    let landed = a.dir.join("games").join(&a.entries().unwrap()[0].file);
    let text = fs::read_to_string(&landed).unwrap();
    fs::write(&landed, text.replace("alice", "mallory")).unwrap();

    let v = a.verify().unwrap();
    assert!(!v.problems.is_empty(), "an edited file passed verification");
    assert!(
        v.problems.iter().any(|p| p.contains("modified")),
        "wrong complaint: {:?}",
        v.problems
    );
}

/// And the point of chaining the manifest: you cannot rewrite a line either,
/// because every later line commits to it.
#[test]
fn editing_the_manifest_breaks_the_chain() {
    let dir = scratch("tamper-manifest");
    let a = Arena::open(dir.join("arena")).unwrap();
    a.add(write_pgn(&dir, "one", "alice", "bob", 3, 1)).unwrap();
    a.add(write_pgn(&dir, "two", "carol", "dave", 2, 2))
        .unwrap();

    let mp = a.dir.join("MANIFEST");
    let text = fs::read_to_string(&mp).unwrap();
    // Claim the first submission held more games than it did.
    let doctored = text.replacen("\t3\t", "\t9\t", 1);
    assert_ne!(
        doctored, text,
        "fixture did not actually change the manifest"
    );
    fs::write(&mp, doctored).unwrap();

    let v = a.verify().unwrap();
    assert!(
        v.problems.iter().any(|p| p.contains("does not link")),
        "a doctored manifest line passed: {:?}",
        v.problems
    );
}

#[test]
fn a_missing_file_is_detected() {
    let dir = scratch("missing");
    let a = Arena::open(dir.join("arena")).unwrap();
    a.add(write_pgn(&dir, "one", "alice", "bob", 3, 1)).unwrap();
    fs::remove_file(a.dir.join("games").join(&a.entries().unwrap()[0].file)).unwrap();

    let v = a.verify().unwrap();
    assert!(v.problems.iter().any(|p| p.contains("missing")));
}

/// Order is part of the lens. Two arenas fed the same games in different orders
/// are different corpora and must not pretend otherwise.
#[test]
fn submission_order_changes_the_corpus() {
    let dir = scratch("order");
    let one = write_pgn(&dir, "one", "alice", "bob", 2, 1);
    let two = write_pgn(&dir, "two", "carol", "dave", 2, 2);

    let a = Arena::open(dir.join("a")).unwrap();
    a.add(one.as_path()).unwrap();
    a.add(two.as_path()).unwrap();

    let b = Arena::open(dir.join("b")).unwrap();
    b.add(two.as_path()).unwrap();
    b.add(one.as_path()).unwrap();

    assert_ne!(
        a.corpus().unwrap().root(),
        b.corpus().unwrap().root(),
        "order made no difference to the corpus"
    );
}

/// Same games, same order, different directory — identical root. Without this
/// nobody else could ever reproduce a medal.
#[test]
fn the_same_submissions_in_the_same_order_reproduce() {
    let dir = scratch("repro");
    let one = write_pgn(&dir, "one", "alice", "bob", 2, 1);
    let two = write_pgn(&dir, "two", "carol", "dave", 2, 2);

    let a = Arena::open(dir.join("a")).unwrap();
    let b = Arena::open(dir.join("b")).unwrap();
    for arena in [&a, &b] {
        arena.add(one.as_path()).unwrap();
        arena.add(two.as_path()).unwrap();
    }
    assert_eq!(a.corpus().unwrap().root(), b.corpus().unwrap().root());
    assert_eq!(a.head().unwrap(), b.head().unwrap());
}

#[test]
fn a_file_with_no_replayable_games_is_refused() {
    let dir = scratch("junk");
    let a = Arena::open(dir.join("arena")).unwrap();
    let junk = dir.join("junk.pgn");
    fs::write(&junk, "[White \"a\"]\n[Black \"b\"]\n\n1. e4 e5 2. Qh9 *\n").unwrap();

    assert!(a.add(junk.as_path()).is_err(), "junk was accepted");
    assert_eq!(a.entries().unwrap().len(), 0, "a refused file still landed");
}

/// Submitted filenames must not be able to escape the games directory.
#[test]
fn a_hostile_filename_cannot_escape() {
    let dir = scratch("escape");
    let a = Arena::open(dir.join("arena")).unwrap();
    let nasty = dir.join("..-..-etc-passwd.pgn");
    let src = write_pgn(&dir, "ok", "alice", "bob", 1, 1);
    fs::copy(&src, &nasty).unwrap();

    a.add(nasty.as_path()).unwrap();
    let file = a.entries().unwrap()[0].file.clone();
    assert!(!file.contains(".."), "path traversal survived: {file}");
    assert!(a.dir.join("games").join(&file).exists());
}

#[test]
fn the_roster_counts_games_per_player() {
    let dir = scratch("roster");
    let a = Arena::open(dir.join("arena")).unwrap();
    a.add(write_pgn(&dir, "one", "alice", "bob", 4, 1)).unwrap();
    a.add(write_pgn(&dir, "two", "alice", "carol", 2, 2))
        .unwrap();

    let roster = a.roster().unwrap();
    let alice = roster.iter().find(|(n, _)| n == "alice").unwrap();
    assert_eq!(alice.1, 6, "alice played six games");
    assert_eq!(
        roster[0].0, "alice",
        "roster should lead with the most active"
    );
}
