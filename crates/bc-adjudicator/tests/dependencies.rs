//! The dependency list is the specification (`D24`, `P5`).
//!
//! A crate boundary only enforces something if something checks it. Without
//! this test, `bc-adjudicator` stays consensus-clean exactly as long as
//! nobody is in a hurry — and the first `bc-sig` import would arrive with a
//! good reason attached, because they always do.

use std::collections::BTreeSet;
use std::path::Path;

/// Everything the state transition function is allowed to reach for.
///
/// `bc-chess` because the rules exist exactly once and are compiled for both
/// sides (`P5`). `bc-hash` because a state has a hash. Nothing else — in
/// particular **not `bc-sig`**: deciding whether a signature is good is the
/// escrow's job, and deciding what follows from it is this crate's.
const ALLOWED: [&str; 2] = ["bc-chess", "bc-hash"];

fn manifest() -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    std::fs::read_to_string(p).expect("own manifest")
}

#[test]
fn the_adjudicator_depends_only_on_the_rules_and_hashing() {
    let text = manifest();
    let deps = text
        .split("[dependencies]")
        .nth(1)
        .expect("a dependencies section");

    let found: BTreeSet<&str> = deps
        .lines()
        .take_while(|l| !l.trim_start().starts_with('['))
        .filter_map(|l| l.split_once(' ').map(|(name, _)| name.trim()))
        .filter(|n| !n.is_empty() && !n.starts_with('#'))
        .collect();

    let allowed: BTreeSet<&str> = ALLOWED.into_iter().collect();
    let extra: Vec<_> = found.difference(&allowed).collect();
    assert!(
        extra.is_empty(),
        "consensus code grew a dependency: {extra:?}.\n\
         If that is deliberate, change ALLOWED and say why in the commit — \
         but read D24 first: the point of this crate is that the list is short."
    );
    assert_eq!(found, allowed, "a dependency went missing");
}

#[test]
fn there_are_no_dev_dependencies_either() {
    // A dev-dependency cannot reach consensus code at runtime, but it can
    // quietly become the reason a type is `pub` that should not be.
    assert!(
        !manifest().contains("[dev-dependencies]"),
        "bc-adjudicator has dev-dependencies"
    );
}

/// The determinism lints, as an executable reminder of what they are for.
///
/// Consensus must be bit-identical on every validator. Floating point is the
/// classic way for that to fail quietly, and `HashMap` iteration order is the
/// other. Neither appears here, and the source is checked rather than the
/// intention.
#[test]
fn nothing_in_the_state_transition_uses_floats_or_hash_maps() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut offenders = Vec::new();
    let mut stack = vec![src];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap();
            for (n, line) in text.lines().enumerate() {
                let code = line.split("//").next().unwrap_or("");
                for bad in ["f32", "f64", "HashMap", "HashSet"] {
                    if code.contains(bad) {
                        offenders.push(format!("{}:{} {bad}", path.display(), n + 1));
                    }
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "non-deterministic constructs: {offenders:#?}"
    );
}
