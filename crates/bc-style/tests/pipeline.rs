//! The oracle for episodes 05, 08 and 09.
//!
//! The eigensolver can be checked against known spectra. The *pipeline* cannot
//! — there is no published answer to "what is this player's style". So the
//! oracle is constructed instead: players whose personalities are defined in
//! advance, which the pipeline must be able to sort back out. If four styles
//! built to differ do not separate, nothing downstream means anything.

use bc_chess::types::Color;
use bc_style::features::{self, D};
use bc_style::{chain::MedalChain, pgn, synth, Basis, Corpus, Lab, Profile};

fn synthetic_lab(rounds: usize) -> Lab {
    let mut c = Corpus::new();
    for g in synth::round_robin(rounds, 0x5EED_1234, 120) {
        c.push(g);
    }
    Lab::fit(c, 4).expect("corpus should fit")
}

/// The central claim: games cluster by who played them.
#[test]
fn constructed_personalities_separate() {
    let lab = synthetic_lab(8);
    let (within, between) = lab.separation();
    assert!(
        between > within * 1.2,
        "no separation: within {within:.3}, between {between:.3}"
    );

    let acc = lab.nearest_neighbour_accuracy();
    let chance = 1.0 / synth::ARCHETYPES.len() as f64;
    assert!(
        acc > chance * 2.0,
        "nearest-neighbour accuracy {acc:.3} is not clearly above chance {chance:.3}"
    );
}

/// The most distinctive archetype must not land on top of the others.
/// Petrosian is the only one rewarded for *restricting* the opponent, which is
/// a property no other feature in the set is a proxy for.
#[test]
fn the_outlier_archetype_is_an_outlier() {
    let lab = synthetic_lab(8);
    let p = lab.profile("Petrosian").unwrap();
    for other in ["Tal", "Capablanca", "Morphy"] {
        let q = lab.profile(other).unwrap();
        let d = bc_style::linalg::distance(&p.coords, &q.coords);
        assert!(
            d > 1.0,
            "Petrosian sits on top of {other} (distance {d:.2})"
        );
    }
}

/// A medal is a commitment, so it must be a pure function of the inputs.
/// Any nondeterminism here — an unstable eigenvector sign, a hash map iteration
/// order — would make the same player mint different medals on different runs.
#[test]
fn medals_are_deterministic() {
    let a = synthetic_lab(4);
    let b = synthetic_lab(4);
    assert_eq!(a.basis.corpus_root, b.basis.corpus_root);
    for name in a.corpus.players() {
        let pa = a.profile(&name).unwrap();
        let pb = b.profile(&name).unwrap();
        assert_eq!(
            pa.medal(&a.basis),
            pb.medal(&b.basis),
            "{name} minted two different medals"
        );
    }
}

/// A medal is a position *under a stated lens*. The same coordinates under a
/// different corpus must be a different medal, or an identity could be quoted
/// out of the context that gives it meaning (`METAPLAN` N10).
#[test]
fn the_medal_is_bound_to_its_corpus() {
    let lab = synthetic_lab(4);
    let profile = lab.profile("Tal").unwrap();

    let mut other = lab.basis.clone();
    other.corpus_root[0] ^= 0x01;

    assert_ne!(
        profile.medal(&lab.basis),
        profile.medal(&other),
        "medal ignored the corpus it was minted under"
    );
}

/// A link is earned by changing, not by playing (`METAPLAN` N4).
#[test]
fn standing_still_earns_exactly_one_link() {
    let lab = synthetic_lab(4);
    let profile = lab.profile("Tal").unwrap();
    let mut chain = MedalChain::new("Tal");

    assert!(chain.offer(&profile, &lab.basis), "first link should mint");
    for _ in 0..100 {
        assert!(
            !chain.offer(&profile, &lab.basis),
            "an unchanged player minted a second link"
        );
    }
    assert_eq!(chain.links.len(), 1);
    assert_eq!(chain.path_length(), 0.0);
}

/// Moving far enough mints another link, and the head changes with it — the
/// head commits to the whole history, so it cannot repeat.
#[test]
fn moving_mints_a_link_and_advances_the_head() {
    let lab = synthetic_lab(4);
    let mut chain = MedalChain::new("drifter");

    let here = Profile::from_points("drifter", &[vec![0.0, 0.0, 0.0, 0.0]]).unwrap();
    let far = Profile::from_points("drifter", &[vec![9.0, 0.0, 0.0, 0.0]]).unwrap();

    assert!(chain.offer(&here, &lab.basis));
    let first_head = chain.head();
    assert!(chain.offer(&far, &lab.basis));

    assert_eq!(chain.links.len(), 2);
    assert_ne!(chain.head(), first_head);
    assert!(chain.path_length() > 8.0);
}

/// Ingest is also validation: a game only parses if every move is legal,
/// which is episode 03's attack pointed at the corpus.
#[test]
fn pgn_ingest_replays_a_real_game() {
    let text = "[White \"A\"]\n[Black \"B\"]\n\n\
                1. e4 e5 2. Bc4 Nc6 3. Qh5 Nf6 4. Qxf7# 1-0\n";
    let (games, skipped) = pgn::parse(text);
    assert_eq!(skipped, 0);
    assert_eq!(games.len(), 1);
    assert_eq!(games[0].moves.len(), 7);
    assert_eq!(games[0].white, "A");

    // The final move is a queen capture on f7.
    let last = games[0].moves[6];
    assert_eq!(last.to_uci(), "h5f7");
}

#[test]
fn pgn_handles_castling_comments_and_move_numbers() {
    let text = "[White \"A\"]\n[Black \"B\"]\n\n\
                1. e4 {a comment} e5 2. Nf3 Nc6 3. Bc4 Bc5 4. O-O $1 Nf6 *\n";
    let (games, skipped) = pgn::parse(text);
    assert_eq!(skipped, 0);
    assert_eq!(games.len(), 1);
    assert_eq!(games[0].moves.len(), 8);
    assert_eq!(
        games[0].moves[6].to_uci(),
        "e1g1",
        "castling did not resolve"
    );
}

/// An illegal game must be rejected rather than silently truncated — a corpus
/// that accepts fiction produces a lens fitted to fiction.
#[test]
fn pgn_rejects_an_illegal_game() {
    let text = "[White \"A\"]\n[Black \"B\"]\n\n1. e4 e5 2. Qh9 *\n";
    let (games, skipped) = pgn::parse(text);
    assert_eq!(games.len(), 0);
    assert_eq!(skipped, 1);
}

/// A side that never moved is not a personality. Averaging over an empty set
/// would place them at the origin, which is a real location in personality
/// space and would therefore be a lie.
#[test]
fn a_side_that_never_moved_has_no_features() {
    let start = bc_chess::Position::startpos();
    assert!(features::extract(&start, &[], Color::White).is_none());
}

#[test]
fn features_are_finite_and_named() {
    assert_eq!(features::FEATURE_NAMES.len(), D);
    let lab = synthetic_lab(2);
    for g in &lab.corpus.games {
        for side in [Color::White, Color::Black] {
            if let Some(f) = features::extract(&g.start, &g.moves, side) {
                assert!(f.iter().all(|v| v.is_finite()), "non-finite feature: {f:?}");
            }
        }
    }
}

/// Projecting a game must use the lens's own statistics, never the sample's.
/// If it used the sample's, every single game would project to the origin.
#[test]
fn projection_uses_the_lens_statistics() {
    let lab = synthetic_lab(4);
    let g = &lab.corpus.games[0];
    let f = features::extract(&g.start, &g.moves, Color::White).unwrap();
    let coords = lab.basis.project(&f);
    assert_eq!(coords.len(), lab.basis.k);
    assert!(coords.iter().any(|c| c.abs() > 1e-9), "projected to origin");
}

/// The basis must not depend on how many axes are kept: axis 0 under k=2 is
/// axis 0 under k=6. Otherwise medals would silently disagree between tools
/// that chose different k.
#[test]
fn axes_are_stable_across_k() {
    let mut c = Corpus::new();
    for g in synth::round_robin(4, 0x5EED_1234, 120) {
        c.push(g);
    }
    let root = c.root();

    let mut rows = Vec::new();
    for g in &c.games {
        for side in [Color::White, Color::Black] {
            if let Some(f) = features::extract(&g.start, &g.moves, side) {
                rows.push(f);
            }
        }
    }
    let small = Basis::fit(&rows, 2, root);
    let large = Basis::fit(&rows, 6, root);
    for d in 0..D {
        for c in 0..2 {
            assert!(
                (small.vectors.get(d, c) - large.vectors.get(d, c)).abs() < 1e-9,
                "axis {c} moved when k changed"
            );
        }
    }
}
