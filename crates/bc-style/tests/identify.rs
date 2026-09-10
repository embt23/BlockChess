//! The oracle for episodes 10 and 12.
//!
//! Attribution accuracy is the easiest number in this project to inflate by
//! accident, so most of these tests are about the *protocol* rather than the
//! result: that the split is disjoint, that the lens never sees what it is
//! scored on, and that the honest number comes out below the leaky one.

use bc_style::identify::{self, Centroid};
use bc_style::{synth, Corpus, Lab};

fn corpus(rounds: usize) -> Corpus {
    let mut c = Corpus::new();
    for g in synth::round_robin(rounds, 0x5EED_1234, 120) {
        c.push(g);
    }
    c
}

fn split(rounds: usize) -> identify::Split {
    identify::split(&corpus(rounds), 4).expect("corpus should split")
}

#[test]
fn the_split_is_disjoint_and_covers_everyone() {
    let c = corpus(6);
    let sp = identify::split(&c, 4).unwrap();

    assert_eq!(
        sp.lab.corpus.len() + sp.held.iter().map(|h| h.points.len()).sum::<usize>() / 2,
        c.len(),
        "train games plus held-out games should account for the corpus"
    );
    assert_eq!(sp.centroids.len(), 4, "every player should get a centroid");
    for h in &sp.held {
        assert!(!h.points.is_empty(), "{} has no held-out games", h.player);
    }
}

/// The whole point of the protocol: the basis is fitted to training games only,
/// so a held-out game is projected by a lens that never saw it.
#[test]
fn the_lens_is_fitted_without_the_held_out_games() {
    let c = corpus(6);
    let sp = identify::split(&c, 4).unwrap();
    let whole = Lab::fit(c, 4).unwrap();

    assert_ne!(
        sp.lab.basis.corpus_root, whole.basis.corpus_root,
        "the training lens must not be the whole-corpus lens"
    );
    assert!(sp.lab.corpus.len() < whole.corpus.len());
}

#[test]
fn a_centroid_attributes_to_itself() {
    let sp = split(6);
    for c in &sp.centroids {
        let (guess, margin) = identify::attribute(&sp.centroids, &c.coords).unwrap();
        assert_eq!(guess, c.player);
        assert!(
            margin > 0.0,
            "{} has no margin over the runner-up",
            c.player
        );
    }
}

#[test]
fn attribution_needs_someone_to_choose_between() {
    let one = vec![Centroid {
        player: "solo".into(),
        coords: vec![0.0, 0.0, 0.0, 0.0],
        games: 1,
    }];
    assert!(identify::attribute(&one, &[0.0, 0.0, 0.0, 0.0]).is_none());
    assert!(identify::attribute(&[], &[0.0]).is_none());
}

#[test]
fn held_out_attribution_beats_chance() {
    let sp = split(8);
    let m = identify::confusion(&sp);
    let n = sp.centroids.len();
    let correct: usize = (0..n).map(|i| m[i][i]).sum();
    let total: usize = m.iter().flatten().sum();
    let acc = correct as f64 / total as f64;
    let chance = 1.0 / n as f64;
    assert!(
        acc > chance * 2.0,
        "held-out accuracy {acc:.3} is not clearly above chance {chance:.3}"
    );
}

/// The honesty check, and the reason this module exists.
///
/// `Lab::nearest_neighbour_accuracy` scores every point against every other,
/// including other games by the same player, on data the basis already saw.
/// A properly held-out estimate must come out **lower**. If it ever comes out
/// higher, the split is leaking and every number here is worthless.
#[test]
fn the_honest_estimate_is_not_better_than_the_leaky_one() {
    let c = corpus(8);
    let leaky = Lab::fit(c.clone(), 4).unwrap().nearest_neighbour_accuracy();

    let sp = identify::split(&c, 4).unwrap();
    let m = identify::confusion(&sp);
    let n = sp.centroids.len();
    let correct: usize = (0..n).map(|i| m[i][i]).sum();
    let total: usize = m.iter().flatten().sum();
    let honest = correct as f64 / total as f64;

    assert!(
        honest <= leaky + 0.02,
        "held-out {honest:.3} beat in-sample {leaky:.3} — the split is leaking"
    );
}

/// Episode 12: more games means less hiding. The curve must rise.
#[test]
fn watching_more_games_identifies_better() {
    let sp = split(8);
    let curve = identify::convergence(&sp, 8, 30, 0xC0FFEE);
    assert_eq!(curve.len(), 8);
    assert!(curve.iter().all(|c| c.trials > 0));

    let first = curve[0].accuracy;
    let last = curve[curve.len() - 1].accuracy;
    assert!(
        last > first,
        "accuracy did not improve with more games: {first:.3} → {last:.3}"
    );
    assert!(
        last > 0.9,
        "eight games should identify a constructed player: {last:.3}"
    );
}

#[test]
fn the_confusion_matrix_accounts_for_every_held_out_game() {
    let sp = split(6);
    let m = identify::confusion(&sp);
    for (i, h) in sp.held.iter().enumerate() {
        let row: usize = m[i].iter().sum();
        assert_eq!(row, h.points.len(), "row {i} lost games");
    }
}

/// Episode 10: two players in the same quantisation cell mint the same medal,
/// and the collision happens *before* the hash, where cryptography cannot help.
#[test]
fn the_closest_pair_is_many_cells_apart() {
    let sp = split(8);
    let (a, b, d, cells) = identify::closest_pair(&sp.centroids).unwrap();
    assert_ne!(a, b);
    assert!(d > 0.0);
    assert!(
        cells > 2.0,
        "{a} and {b} are only {cells:.1} cells apart — they would collide"
    );
}

#[test]
fn closest_pair_needs_a_pair() {
    assert!(identify::closest_pair(&[]).is_none());
    let one = vec![Centroid {
        player: "solo".into(),
        coords: vec![0.0],
        games: 1,
    }];
    assert!(identify::closest_pair(&one).is_none());
}

#[test]
fn occupied_cells_are_finite_and_per_axis() {
    let sp = split(6);
    let pts: Vec<Vec<f64>> = sp.lab.points.iter().map(|p| p.coords.clone()).collect();
    let cells = identify::occupied_cells(&sp.lab.basis, &pts);
    assert_eq!(cells.len(), sp.lab.basis.k);
    assert!(cells.iter().all(|c| c.is_finite() && *c > 0.0));
}

// --- Is the basis just rating in disguise? -----------------------------------

use bc_style::GameRecord;

/// Rebuild a corpus with ratings attached by a rule, so the detector can be
/// checked against a known answer.
fn rated(rounds: usize, mut elo_of: impl FnMut(&str) -> u16) -> Lab {
    let mut c = Corpus::new();
    for g in synth::round_robin(rounds, 0x5EED_1234, 120) {
        let (w, b) = (elo_of(&g.white), elo_of(&g.black));
        c.push(GameRecord {
            white_elo: Some(w),
            black_elo: Some(b),
            ..g
        });
    }
    Lab::fit(c, 4).expect("should fit")
}

/// Positive control: give each archetype its own rating and the axes that
/// separate them must correlate with it. If this does not fire, the detector
/// cannot detect anything and its silence elsewhere means nothing.
#[test]
fn strength_leakage_is_detected_when_it_is_there() {
    let lab = rated(8, |name| match name {
        "Tal" => 1200,
        "Petrosian" => 1600,
        "Capablanca" => 2000,
        _ => 2400,
    });
    let rows = bc_style::identify::strength_leakage(&lab);
    assert_eq!(rows.len(), lab.basis.k);
    assert!(rows[0].rated > 0);
    let worst = rows.iter().map(|r| r.r.abs()).fold(0.0, f64::max);
    assert!(
        worst > 0.3,
        "ratings tied to identity produced no correlation: {worst:.3}"
    );
}

/// Negative control: ratings unrelated to who is playing must not correlate.
#[test]
fn unrelated_ratings_do_not_correlate() {
    let mut n = 0u16;
    let lab = rated(8, move |_| {
        n = n.wrapping_add(37);
        1500 + (n % 400)
    });
    let rows = bc_style::identify::strength_leakage(&lab);
    let worst = rows.iter().map(|r| r.r.abs()).fold(0.0, f64::max);
    assert!(
        worst < 0.3,
        "noise ratings produced a correlation of {worst:.3}"
    );
}

/// Without ratings the honest answer is nothing, not zero. Reporting r = 0
/// would read as "checked, and the axes are clean".
#[test]
fn no_ratings_means_no_answer() {
    let mut c = Corpus::new();
    for g in synth::round_robin(4, 0x5EED_1234, 120) {
        c.push(g);
    }
    let lab = Lab::fit(c, 4).unwrap();
    assert!(bc_style::identify::strength_leakage(&lab).is_empty());
}
