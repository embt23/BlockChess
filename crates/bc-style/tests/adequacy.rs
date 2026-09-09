//! Guards against the quietest failure in the crate.
//!
//! A basis fitted to too few games returns axes that have poles, loadings, a
//! share of variance, and a medal — and mean nothing. Nothing about the output
//! looks different. `docs/build-log.md` 06 is the story; these are the tests.

use bc_style::basis::{Basis, SAMPLES_PER_DIM};
use bc_style::features::D;
use bc_style::{report, synth, Corpus, Lab};

fn rows(n: usize) -> Vec<[f64; D]> {
    // Deterministic but not degenerate: a constant column would be dropped by
    // standardisation and change the rank for a different reason.
    let mut rng = synth::Rng::new(0xA11CE);
    (0..n)
        .map(|_| {
            let mut r = [0.0f64; D];
            for v in r.iter_mut() {
                *v = rng.unit();
            }
            r
        })
        .collect()
}

/// A covariance estimated from `n` points has rank at most `n − 1`. Axes past
/// that are ordered by floating-point noise, not by anything the data said.
#[test]
fn asking_for_more_axes_than_the_data_supports_is_caught() {
    let b = Basis::fit(&rows(5), 10, [0u8; 32]);
    let a = b.adequacy();

    assert_eq!(a.rank, 4, "5 samples support at most 4 axes");
    assert!(a.over_rank());
    assert!(!a.trustworthy());
    let w = a.warning().expect("must warn");
    assert!(w.contains("at most 4 axes"), "{w}");
    assert!(w.contains("meaningless"), "{w}");
}

/// Enough axes, but not enough games to place them stably.
#[test]
fn a_thin_corpus_is_caught_separately() {
    let b = Basis::fit(&rows(40), 3, [0u8; 32]);
    let a = b.adequacy();

    assert!(!a.over_rank(), "40 samples easily support 3 axes");
    assert!(a.thin());
    assert!(!a.trustworthy());
    let w = a.warning().expect("must warn");
    assert!(w.contains("unstable"), "{w}");
}

#[test]
fn an_adequate_corpus_does_not_warn() {
    let b = Basis::fit(&rows(D * SAMPLES_PER_DIM + 10), 4, [0u8; 32]);
    let a = b.adequacy();

    assert!(a.trustworthy());
    assert_eq!(a.warning(), None);
}

/// The threshold is a property of the feature count, not a magic number.
#[test]
fn the_sample_target_follows_the_feature_count() {
    let b = Basis::fit(&rows(50), 2, [0u8; 32]);
    assert_eq!(b.adequacy().wanted_samples, D * SAMPLES_PER_DIM);
}

/// The demo has to clear its own bar. If the corpus every example uses is
/// itself too thin, every number in the README is unquotable.
#[test]
fn the_synthetic_demo_corpus_is_adequate() {
    let mut c = Corpus::new();
    for g in synth::round_robin(10, 0x5EED_1234, 120) {
        c.push(g);
    }
    let lab = Lab::fit(c, 4).unwrap();
    let a = lab.basis.adequacy();
    assert!(
        a.trustworthy(),
        "the demo corpus is not adequate: {:?}",
        a.warning()
    );
}

/// The caveat has to reach the page, which is the most authoritative-looking
/// artifact the crate produces and therefore the most dangerous one.
#[test]
fn the_warning_travels_into_the_json_and_the_page() {
    let mut c = Corpus::new();
    for g in synth::round_robin(1, 0x5EED_1234, 120) {
        c.push(g);
    }
    let lab = Lab::fit(c, 4).unwrap();
    assert!(!lab.basis.adequacy().trustworthy(), "12 games should warn");

    let json = report::build_json(&lab);
    assert!(json.contains("\"adequacy\""));
    assert!(json.contains("\"trustworthy\":false"));
    assert!(!json.contains("\"warning\":null"));

    let page = report::render_page(&lab);
    assert!(page.contains("\"trustworthy\":false"));
    assert!(
        page.contains("Not enough games"),
        "the page must carry the banner text"
    );
}

/// And an adequate corpus must not cry wolf, or the banner stops being read.
#[test]
fn an_adequate_corpus_ships_a_null_warning() {
    let mut c = Corpus::new();
    for g in synth::round_robin(10, 0x5EED_1234, 120) {
        c.push(g);
    }
    let lab = Lab::fit(c, 4).unwrap();
    let json = report::build_json(&lab);
    assert!(json.contains("\"trustworthy\":true"));
    assert!(json.contains("\"warning\":null"));
}
