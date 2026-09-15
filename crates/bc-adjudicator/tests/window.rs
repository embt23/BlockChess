//! A deadline you cannot meet is not a deadline.
//!
//! `blocks_consumed` charges at least `MIN_MOVE_BLOCKS` for any move. `arm`
//! used to hand out `min(Δ, budget)` with no floor, so the two disagreed:
//! a player with 5 blocks of budget was given 5 blocks to make a move they
//! would be billed 8 for. `docs/build-log.md` §18.

use bc_adjudicator::dilation::{budget_blocks, MIN_MOVE_BLOCKS, TAU_MS};
use bc_adjudicator::dispute::Dispute;
use bc_adjudicator::state::{GameState, Status};
use bc_adjudicator::terms::GameTerms;
use bc_chess::{Color, Position};

/// A position with legal moves for both sides, so a dispute can run.
const FEN: &str = "7k/5Q2/6K1/8/8/8/8/8 w - - 0 1";

fn dispute(clock_ms: u32) -> Dispute {
    let pos = Position::from_fen(FEN).expect("test FEN");
    let terms = GameTerms::for_clock([0u8; 32], clock_ms.max(1), 0);
    let state = GameState {
        channel_id: [0u8; 32],
        ply: 0,
        prev_hash: [0u8; 32],
        mv: 0,
        pos_hash: [0u8; 32],
        clock_w_ms: clock_ms,
        clock_b_ms: clock_ms,
        status: Status::Ongoing,
    };
    Dispute::open(&state, pos, &terms, Color::White, 1_000).expect("open")
}

fn window(d: &Dispute) -> u64 {
    d.deadline_block - d.turn_started
}

/// Play a dispute out and report the smallest nonzero window it produced.
fn smallest_window(clock_ms: u32) -> u64 {
    let mut d = dispute(clock_ms);
    let mut h = 1_000u64;
    let mut smallest = u64::MAX;
    for _ in 0..80 {
        let w = window(&d);
        if w == 0 {
            break;
        }
        smallest = smallest.min(w);
        let side = d.side_to_move();
        let mv = d.pos.generate_legal().as_slice()[0];
        h += 1;
        if d.apply_move(side, mv, h, None).is_err() {
            break;
        }
    }
    smallest
}

/// The two clocks that produced the worst cases before the fix. 288 ms gives
/// a budget of 41, which steps 41 → 33 → 25 → 17 → 9 → **1**.
#[test]
fn the_window_never_falls_below_what_a_move_is_charged() {
    for clock in [7, 288, 400, 1_000] {
        let w = smallest_window(clock);
        assert!(
            w >= MIN_MOVE_BLOCKS,
            "clock {clock} ms (budget {}) produced a {w}-block window, \
             and a move is charged {MIN_MOVE_BLOCKS}",
            budget_blocks(clock, TAU_MS)
        );
    }
}

/// Swept, because the bad values are a residue class and picking four by
/// hand is how the original was missed.
#[test]
fn no_clock_produces_an_unmeetable_window() {
    let mut worst = (u64::MAX, 0u32);
    for clock in (0..6_000u32).step_by(7) {
        let w = smallest_window(clock);
        if w < worst.0 {
            worst = (w, clock);
        }
    }
    assert!(
        worst.0 >= MIN_MOVE_BLOCKS,
        "clock {} ms produced a {}-block window",
        worst.1,
        worst.0
    );
}

/// The floor must not hand a flagging player extra moves.
///
/// Stated on the move *count* rather than on budget spent, because the
/// final deduction saturates at zero — a player charged 8 with 5 left
/// loses 5, not 8, so summing deductions understates what was charged and
/// would make a weaker claim than intended.
///
/// A side with budget `b` gets at most `ceil(b / MIN_MOVE_BLOCKS)` moves,
/// and that bound does not move when the window floor is raised. This is
/// the clause that keeps the fix from being a way to buy time back.
#[test]
fn the_floor_gives_no_extra_moves_to_a_flagging_player() {
    let mut d = dispute(288);
    let budget = d.budget_of(Color::White);
    let allowed = budget.div_ceil(MIN_MOVE_BLOCKS as u32);

    let mut h = 1_000u64;
    let mut moves = [0u32, 0];
    while window(&d) > 0 && moves.iter().sum::<u32>() < 80 {
        let side = d.side_to_move();
        let mv = d.pos.generate_legal().as_slice()[0];
        h += 1;
        if d.apply_move(side, mv, h, None).is_err() {
            break;
        }
        moves[side as usize] += 1;
    }
    for (i, got) in moves.iter().enumerate() {
        assert!(
            *got <= allowed,
            "side {i} got {got} on-chain moves from a budget of {budget}, \
             which allows at most {allowed}"
        );
    }
    assert!(moves[0] > 0 && moves[1] > 0, "both sides should have moved");
}

/// Zero budget still means zero window: the next block settles it, and
/// that is a loss on time rather than a deadline anyone was cheated by.
#[test]
fn an_exhausted_budget_still_flags() {
    let mut d = dispute(288);
    let mut h = 1_000u64;
    for _ in 0..80 {
        if window(&d) == 0 {
            break;
        }
        let side = d.side_to_move();
        let mv = d.pos.generate_legal().as_slice()[0];
        h += 1;
        if d.apply_move(side, mv, h, None).is_err() {
            break;
        }
    }
    let side = d.side_to_move();
    assert_eq!(d.budget_of(side), 0, "the run should exhaust a budget");
    assert_eq!(window(&d), 0);
    assert!(d.verdict(d.deadline_block + 1).is_ok(), "it settles");
}
