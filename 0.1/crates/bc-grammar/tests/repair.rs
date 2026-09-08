//! Does the inducer actually find what is there?
//!
//! Two kinds of test. The first kind checks the algorithm is correct — the
//! grammar expands back to the input, nothing is lost, the accounting adds up.
//! The second kind is a **planted-pattern harness**: build a corpus out of
//! known repeated sequences and check that the inducer recovers them.
//!
//! The planted tests validate the *machine*, not the claim in
//! `papers/08-layers.md`. That claim is about real chess games and can only be
//! tested on a real corpus, because a synthetic corpus can only ever return
//! the patterns that were put into it. Keeping that distinction sharp matters:
//! a planted test that "rediscovers openings" would be circular, and reporting
//! it as a result would be worthless.

use bc_grammar::{induce, Limits, Sym};

fn expand_all(g: &bc_grammar::Grammar) -> Vec<Vec<Sym>> {
    g.sequences
        .iter()
        .map(|s| s.iter().flat_map(|&x| g.expand(x)).collect())
        .collect()
}

#[test]
fn the_grammar_expands_back_to_the_input() {
    // The only property that must never break: substitution is lossless.
    let seqs: Vec<Vec<Sym>> = vec![
        vec![1, 2, 3, 1, 2, 3, 1, 2, 4],
        vec![1, 2, 3, 5, 5, 5, 1, 2, 3],
        vec![7],
        vec![],
        vec![1, 1, 1, 1, 1, 1],
    ];
    let g = induce(&seqs, Limits::default());
    assert_eq!(expand_all(&g), seqs, "substitution lost data");
}

#[test]
fn overlapping_occurrences_are_not_double_consumed() {
    // `aaa…` is the classic Re-Pair trap: the pair (a,a) overlaps itself, and
    // an implementation that trusts its occurrence list will consume the same
    // symbol twice and silently corrupt the output.
    for len in 1..40usize {
        let seqs = vec![vec![9u32; len]];
        let g = induce(&seqs, Limits::default());
        assert_eq!(expand_all(&g), seqs, "run of length {len} was corrupted");
    }
}

#[test]
fn patterns_are_never_found_across_a_sequence_boundary() {
    // The end of one game and the start of the next are not a pattern. If the
    // boundary leaks, the grammar invents symbols that mean nothing.
    let seqs: Vec<Vec<Sym>> = (0..50).map(|_| vec![1, 2]).collect();
    let g = induce(&seqs, Limits::default());
    assert_eq!(expand_all(&g), seqs);
    for r in &g.rules {
        let e = g.expand(r.lhs);
        assert!(
            e.len() <= 2,
            "rule spans a boundary: {:?} expands to {:?}",
            r,
            e
        );
    }
}

#[test]
fn a_planted_pattern_is_recovered_exactly() {
    // Validates the machine, not the chess claim. 200 sequences that all begin
    // with the same eight symbols, then diverge into noise.
    let planted: Vec<Sym> = vec![10, 11, 12, 13, 14, 15, 16, 17];
    let mut seqs = Vec::new();
    let mut x: u32 = 12345;
    for _ in 0..200 {
        let mut s = planted.clone();
        for _ in 0..20 {
            x = x.wrapping_mul(1664525).wrapping_add(1013904223);
            s.push(1000 + (x >> 16) % 400);
        }
        seqs.push(s);
    }

    // A threshold of 10 is what makes this test say something. At the default
    // of 2, two of the 200 random tails coinciding on one symbol is likely,
    // and a 9-symbol rule forms that is a real repetition but not a planted
    // one. Requiring ten occurrences puts coincidence out of reach and leaves
    // only what was actually put there.
    let g = induce(
        &seqs,
        Limits {
            min_occurrences: 10,
            max_rules: 100_000,
        },
    );
    assert_eq!(expand_all(&g), seqs);

    // Some invented symbol must expand to exactly the planted sequence.
    let found = g.rules.iter().any(|r| g.expand(r.lhs) == planted);
    assert!(
        found,
        "the planted 8-symbol pattern was not recovered as a single symbol"
    );

    // And nothing longer, because nothing longer repeats ten times.
    let longest = g.rules.iter().map(|r| g.expanded_len(r.lhs)).max().unwrap();
    assert_eq!(longest, planted.len(), "found something longer than exists");
}

#[test]
fn two_planted_patterns_at_different_frequencies_are_both_found() {
    // Closer to the real case: a common line and a rarer one. Both should get
    // symbols, and the common one should be substituted more often.
    let common: Vec<Sym> = vec![20, 21, 22, 23, 24, 25];
    let rare: Vec<Sym> = vec![30, 31, 32, 33, 34, 35];
    let mut seqs = Vec::new();
    let mut x: u32 = 999;
    for i in 0..300 {
        let mut s = if i % 5 == 0 {
            rare.clone()
        } else {
            common.clone()
        };
        for _ in 0..15 {
            x = x.wrapping_mul(1664525).wrapping_add(1013904223);
            s.push(2000 + (x >> 16) % 500);
        }
        seqs.push(s);
    }

    let g = induce(&seqs, Limits::default());
    assert_eq!(expand_all(&g), seqs);

    let find = |want: &[Sym]| g.rules.iter().find(|r| g.expand(r.lhs) == want).copied();
    let c = find(&common).expect("common pattern not recovered");
    let r = find(&rare).expect("rare pattern not recovered");
    assert!(
        c.occurrences > r.occurrences,
        "common {} should be substituted more than rare {}",
        c.occurrences,
        r.occurrences
    );
}

#[test]
fn random_input_yields_almost_no_grammar() {
    // The control. If the inducer invents structure in noise, then finding
    // structure in chess would mean nothing. With a large alphabet and short
    // sequences there is essentially nothing to find, and the rule count
    // should be tiny next to the corpus size.
    let mut x: u32 = 4242;
    let seqs: Vec<Vec<Sym>> = (0..200)
        .map(|_| {
            (0..30)
                .map(|_| {
                    x = x.wrapping_mul(1664525).wrapping_add(1013904223);
                    (x >> 8) % 20_000
                })
                .collect()
        })
        .collect();
    let total: usize = seqs.iter().map(|s| s.len()).sum();
    let g = induce(&seqs, Limits::default());
    assert_eq!(expand_all(&g), seqs);
    assert!(
        g.rules.len() * 20 < total,
        "found {} rules in {} random symbols — the inducer is hallucinating structure",
        g.rules.len(),
        total
    );
}

#[test]
fn limits_are_respected() {
    let seqs: Vec<Vec<Sym>> = (0..100).map(|_| vec![1, 2, 3, 4, 5, 6, 7, 8]).collect();
    let g = induce(
        &seqs,
        Limits {
            min_occurrences: 2,
            max_rules: 3,
        },
    );
    assert_eq!(g.rules.len(), 3);
    assert_eq!(
        expand_all(&g),
        seqs,
        "truncated grammar still must be lossless"
    );

    // A high threshold should stop it finding anything.
    let g = induce(
        &seqs,
        Limits {
            min_occurrences: 1000,
            max_rules: 100,
        },
    );
    assert!(g.rules.is_empty());
}

#[test]
fn compression_actually_happens() {
    // 500 copies of the same 40-symbol sequence should collapse to almost
    // nothing: one symbol per sequence, plus the grammar to build it.
    let seq: Vec<Sym> = (0..40).collect();
    let seqs: Vec<Vec<Sym>> = (0..500).map(|_| seq.clone()).collect();
    let before: usize = seqs.iter().map(|s| s.len()).sum();
    let g = induce(&seqs, Limits::default());
    assert_eq!(expand_all(&g), seqs);
    assert!(
        g.total_size() * 10 < before,
        "expected a big win: {} -> {} (incl. grammar)",
        before,
        g.total_size()
    );
}
