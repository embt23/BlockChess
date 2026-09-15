//! D23's model check, run at three configurations.
//!
//! The model itself is [`bc_conformance::dispute_model`] — it is library
//! code because it drives the real [`Dispute`] and is worth reading, not
//! only running. This file is the part that says which corners of the state
//! space to explore and how big each one came out.

use bc_adjudicator::dilation::{FLOOR_BLOCKS, MIN_MOVE_BLOCKS};
use bc_conformance::dispute_model::DisputeModel;
use stateright::{Checker, Model};

fn check(fen: &str, clock_ms: u32, max_plies: u16) -> usize {
    let checker = DisputeModel::new(fen, clock_ms, max_plies)
        .checker()
        .spawn_bfs()
        .join();
    checker.assert_properties();
    checker.unique_state_count()
}

/// The flagging case: both players near zero clock, so budgets are
/// [`FLOOR_BLOCKS`] and every deadline is tight. This is where dilation
/// arithmetic is load-bearing.
#[test]
fn the_dispute_machine_is_sound_when_both_clocks_are_nearly_out() {
    assert_eq!(FLOOR_BLOCKS, 32, "the model's size assumptions track this");
    assert_eq!(MIN_MOVE_BLOCKS, 8);
    let n = check("7k/5Q2/6K1/8/8/8/8/8 w - - 0 1", 1, 4);
    println!("flagging endgame: {n} states");
    assert!(n > 100, "only {n} states — the model stopped exploring");
}

/// A position where the side to move is in check, so the legal moves are few
/// and a mate claim is nearly true. Claims and refutations dominate here.
#[test]
fn the_dispute_machine_is_sound_under_a_live_mate_claim() {
    let n = check("R6k/8/8/8/8/8/8/6K1 b - - 0 1", 1, 6);
    println!("mate claim: {n} states");
    assert!(n > 50, "only {n} states");
}

/// A larger clock, so budgets exceed Δ and the per-move window is Δ rather
/// than what is left of the budget. The two branches of `arm` are different
/// code paths and both need covering.
#[test]
fn the_dispute_machine_is_sound_when_the_budget_exceeds_the_window() {
    let n = check("7k/5Q2/6K1/8/8/8/8/8 w - - 0 1", 60_000, 4);
    println!("budget above Δ: {n} states");
    assert!(n > 100, "only {n} states");
}
