//! Clock dilation: the on-chain time base.
//!
//! The client half of timekeeping — Fischer debits, and the grace that
//! decides whether to countersign — is policy and lives in `bc-channel`.
//! This half is consensus: every validator must compute the same budget from
//! the same clock, so it is here, with no floating point and no allocation.

/// Milliseconds of game clock per block of on-chain budget. `GameTerms`
/// carries the real value; this is the default.
pub const TAU_MS: u32 = 50;

/// Added to every budget, so that even a flagging player can physically move.
/// At two-second blocks this is about 64 seconds.
pub const FLOOR_BLOCKS: u32 = 32;

/// Cap on any one player's total on-chain budget. About three hours.
pub const MAX_BUDGET: u32 = 5_400;

/// Floor on what a single move consumes, so nobody is required to respond
/// faster than roughly sixteen seconds however little clock they have.
pub const MIN_MOVE_BLOCKS: u64 = 8;

/// A false claim of mate or stalemate costs half the remaining budget.
pub const FALSE_CLAIM_PENALTY_DIVISOR: u32 = 2;

/// Convert a game clock into a budget of blocks for the rest of the game.
///
/// The problem this solves is narrower than it looks. Keeping the millisecond
/// clock on-chain is impossible — a player with 400 ms cannot move through a
/// mempool. But *resetting* everyone to a generous on-chain clock hands a
/// player who is about to lose on time a free escape: stall, force the
/// dispute, get the time back. A certain loss becomes a fresh game.
///
/// So neither keeping nor discarding the clock works, and the answer is to
/// change the **time base** while holding the **ratio**. A player with 400 ms
/// gets 40 blocks — around 80 real seconds, enough to physically play — while
/// their opponent on 180 s holds 3632. They can still move; they are still
/// overwhelmingly losing on time; they will still run out first. The
/// strategic meaning of the clock survives and every deadline becomes
/// achievable.
///
/// This is a total budget for the rest of the game, not a per-move allowance.
pub fn budget_blocks(clock_ms: u32, tau_ms: u32) -> u32 {
    let tau = tau_ms.max(1) as u64;
    let scaled = (clock_ms as u64).div_ceil(tau) + FLOOR_BLOCKS as u64;
    scaled.min(MAX_BUDGET as u64) as u32
}

/// What one move costs a player's budget: the blocks they actually took, but
/// never fewer than [`MIN_MOVE_BLOCKS`].
///
/// Charging the floor even to an instant reply is deliberate. Without it, a
/// player with a large budget could force an opponent with a small one to
/// answer at mempool speed indefinitely.
pub fn blocks_consumed(started: u64, now: u64) -> u64 {
    now.saturating_sub(started).max(MIN_MOVE_BLOCKS)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The worked examples in `spec/05-adjudication.md`, at two-second blocks.
    #[test]
    fn dilation_matches_the_spec_table() {
        for (clock_ms, blocks) in [
            (180_000u32, 3_632u32),
            (30_000, 632),
            (5_000, 132),
            (400, 40),
        ] {
            assert_eq!(budget_blocks(clock_ms, TAU_MS), blocks, "{clock_ms} ms");
        }
    }

    #[test]
    fn the_ratio_survives_the_change_of_base() {
        let rich = budget_blocks(180_000, TAU_MS);
        let poor = budget_blocks(400, TAU_MS);
        assert!(rich > poor * 80, "{rich} vs {poor}");
    }

    #[test]
    fn even_a_flagging_player_can_physically_move() {
        assert_eq!(budget_blocks(0, TAU_MS), FLOOR_BLOCKS);
        assert!(budget_blocks(1, TAU_MS) >= FLOOR_BLOCKS);
    }

    #[test]
    fn budgets_are_capped() {
        assert_eq!(budget_blocks(u32::MAX, TAU_MS), MAX_BUDGET);
    }

    #[test]
    fn stalling_cannot_buy_time_back() {
        let mut last = 0;
        for ms in (0..20_000).step_by(97) {
            let b = budget_blocks(ms, TAU_MS);
            assert!(b >= last, "not monotonic at {ms} ms");
            last = b;
        }
    }

    #[test]
    fn a_move_never_costs_less_than_the_floor() {
        assert_eq!(blocks_consumed(100, 100), MIN_MOVE_BLOCKS);
        assert_eq!(blocks_consumed(100, 140), 40);
        assert_eq!(blocks_consumed(140, 100), MIN_MOVE_BLOCKS);
    }
}
