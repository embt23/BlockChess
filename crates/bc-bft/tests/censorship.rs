//! Episode 10's censorship bound: the `G0` specification.
//!
//! These live outside `src/` because `censorship.rs` was over the `G2`
//! line with them in it. Every one drives the public API — there is
//! nothing here a user of the crate could not write.
//!
//! Most are `#[ignore]`d: they specify `max_byzantine_run` and
//! `window_is_safe`, which are episode 10's filmed subject and Evan's to
//! type. See `docs/g0-holes.md`.

use bc_bft::censorship::{assess, max_byzantine_run, smallest_window, window_is_safe};
use bc_bft::ValidatorSet;
use bc_sig::{SigningKey, VerifyingKey};
const G0: &str = "G0: episode 10's subject — Evan types the censorship bound";

fn set(n: u8) -> ValidatorSet {
    let keys: Vec<VerifyingKey> = (0..n)
        .map(|i| SigningKey::from_seed(&[i + 1; 32]).verifying_key())
        .collect();
    ValidatorSet::uniform(&keys)
}

#[test]
#[ignore = "G0: episode 10's subject — Evan types the censorship bound"]
fn the_worst_run_is_the_byzantine_bound() {
    for n in 1..=40u8 {
        assert_eq!(
            max_byzantine_run(&set(n)),
            set(n).byzantine_bound() as u32,
            "{G0}: n={n}"
        );
    }
    assert_eq!(max_byzantine_run(&set(4)), 1);
    assert_eq!(max_byzantine_run(&set(7)), 2);
    assert_eq!(max_byzantine_run(&set(10)), 3);
}

/// A single validator has no run to bound. One hostile validator is not
/// a rotation problem, it is the whole chain.
#[test]
#[ignore = "G0: episode 10's subject — Evan types the censorship bound"]
fn a_single_validator_has_no_byzantine_run() {
    assert_eq!(max_byzantine_run(&set(1)), 0, "{G0}");
}

/// There must always be someone honest left to reach.
#[test]
#[ignore = "G0: episode 10's subject — Evan types the censorship bound"]
fn the_run_never_swallows_the_whole_set() {
    for n in 1..=60u8 {
        assert!(
            max_byzantine_run(&set(n)) < n as u32,
            "{G0}: n={n} has no honest proposer"
        );
    }
}

/// **The off-by-one.** A window of exactly `f` blocks can be spanned by
/// `f` consecutive Byzantine proposers. If only one of these tests
/// passes, it should be this one.
#[test]
#[ignore = "G0: episode 10's subject — Evan types the censorship bound"]
fn a_window_equal_to_the_run_is_not_safe() {
    for n in [4u8, 7, 10, 22, 40] {
        let s = set(n);
        let f = s.byzantine_bound() as u32;
        assert!(!window_is_safe(&s, f), "{G0}: n={n}, window==f=={f}");
        assert!(window_is_safe(&s, f + 1), "{G0}: n={n}, window==f+1");
    }
}

#[test]
#[ignore = "G0: episode 10's subject — Evan types the censorship bound"]
fn a_zero_window_is_never_safe() {
    for n in 1..=20u8 {
        assert!(!window_is_safe(&set(n), 0), "{G0}: n={n}");
    }
}

#[test]
#[ignore = "G0: episode 10's subject — Evan types the censorship bound"]
fn safety_is_monotone_in_the_window() {
    let s = set(10);
    let mut seen_safe = false;
    for w in 0..40 {
        let safe = window_is_safe(&s, w);
        if seen_safe {
            assert!(safe, "{G0}: w={w} unsafe after a safe smaller window");
        }
        seen_safe |= safe;
    }
    assert!(seen_safe, "{G0}: nothing was ever safe");
}

/// The result the episode is for. With an 8-block floor, this scheme
/// secures a set only up to the size at which `f` reaches 8 — and Δ,
/// however generous, does not move that line.
#[test]
#[ignore = "G0: episode 10's subject — Evan types the censorship bound"]
fn the_floor_and_not_delta_is_what_bounds_the_validator_set() {
    const FLOOR: u32 = 8;
    let largest_safe = (1..=60u8)
        .filter(|n| window_is_safe(&set(*n), FLOOR))
        .max()
        .expect("some set is securable");
    println!("with an {FLOOR}-block floor, sets up to {largest_safe} are secured");
    assert!(largest_safe >= 4, "{G0}: even four validators fail");

    // …and a generous Δ does not rescue a set past that line, because
    // the window shrinks to the floor regardless.
    let too_big = set(largest_safe + 1);
    for delta in [64u32, 256, 2_048] {
        let a = assess(&too_big, delta, FLOOR);
        assert_eq!(
            a.smallest_window, FLOOR,
            "{G0}: Δ={delta} changed the floor"
        );
        assert!(!a.safe, "{G0}: Δ={delta} appeared to rescue an unsafe set");
    }
}

// ---- not the hole: expected to pass ----

#[test]
fn the_smallest_window_is_the_floor_whatever_delta_is() {
    for delta in [8u32, 64, 128, 256, 512, 2_048] {
        assert_eq!(
            smallest_window(delta, 8),
            8,
            "Δ={delta} should not move the floor"
        );
    }
}

/// Terms whose Δ is below the move cost are incoherent, and the answer is
/// Δ rather than a panic: the window can never exceed what the channel
/// signed, so the floor is capped by Δ too. `Dispute::arm` does the same
/// thing, and the two having to agree is the point.
#[test]
fn a_delta_below_the_move_cost_becomes_the_binding_quantity() {
    assert_eq!(smallest_window(3, 8), 3);
    assert_eq!(smallest_window(8, 8), 8);
    assert_eq!(smallest_window(0, 8), 0);
}

/// And the real table never produces one, so the case above is a guard
/// rather than a live constraint.
#[test]
fn every_real_time_control_has_a_delta_above_the_move_cost() {
    // The five classes from D22, smallest first. Literals, so a change to
    // the table trips this rather than silently widening the assumption.
    for delta in [64u32, 128, 256, 512, 2_048] {
        assert!(delta >= 8, "Δ={delta} is below MIN_MOVE_BLOCKS");
        assert_eq!(smallest_window(delta, 8), 8);
    }
}
