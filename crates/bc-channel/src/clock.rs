//! Clock accounting, and why there is no trusted time source.
//!
//! The mover debits their own clock and asserts the result inside the state
//! they sign. The opponent decides whether to accept it. That is the whole
//! mechanism: **the clock is enforced by refusal to countersign.**
//!
//! It works because refusing costs the honest player nothing they did not
//! already have. If your opponent asserts a clock you do not believe, you stop
//! signing and go to chain — which is the same thing you do if they simply
//! stop replying. The protocol needs no new liveness assumption to enforce
//! time, which is the test `spec/05` applies to every mechanism in it.
//!
//! The second half of this module is the *other* time base. When a game goes
//! on-chain the millisecond clock becomes unplayable — nobody moves in 400 ms
//! through a mempool — so [`budget_blocks`] changes the unit while holding the
//! ratio. See the note on it, and `spec/05-adjudication.md`.
//!
//! The asymmetry in [`plausible`] is deliberate. A mover who claims to have
//! used *more* time than you observed is giving you time; there is no reason
//! to object, and objecting would turn network jitter into a dispute. Only an
//! under-claim — time the mover spent but does not want debited — is refused.

/// How much under-claim to tolerate before refusing to countersign.
///
/// Absorbs network latency and honest clock skew. Too small and flaky wifi
/// produces disputes; too large and a player can steal `grace` milliseconds
/// per move, which over 40 moves is 12 seconds.
pub const DEFAULT_GRACE_MS: u32 = 300;

/// What the mover's clock reads after spending `elapsed_ms`.
///
/// `None` means the flag fell: the move took longer than the mover had.
/// Increment is added after the move, per Fischer.
pub fn debit(remaining_ms: u32, elapsed_ms: u32, increment_ms: u32) -> Option<u32> {
    if elapsed_ms > remaining_ms {
        return None;
    }
    Some((remaining_ms - elapsed_ms).saturating_add(increment_ms))
}

/// Would a receiver accept the mover's asserted clock?
///
/// `observed_elapsed_ms` is what the receiver measured locally, which is an
/// over-estimate of the mover's true think time by roughly one network
/// latency — hence the grace.
pub fn plausible(
    prev_remaining_ms: u32,
    asserted_ms: u32,
    observed_elapsed_ms: u32,
    increment_ms: u32,
    grace_ms: u32,
) -> bool {
    let ceiling = prev_remaining_ms as i64 + increment_ms as i64;
    let claimed_elapsed = ceiling - asserted_ms as i64;
    // Claiming a clock above the ceiling is claiming time that was never
    // there: it is minting, one ply at a time.
    if claimed_elapsed < 0 {
        return false;
    }
    claimed_elapsed + grace_ms as i64 >= observed_elapsed_ms as i64
}

// ---------------------------------------------------------------------------
// Clock dilation — the on-chain time base
// ---------------------------------------------------------------------------

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

    #[test]
    fn fischer_increment_is_added_after_the_move() {
        assert_eq!(debit(60_000, 5_000, 2_000), Some(57_000));
    }

    #[test]
    fn the_flag_falls_when_the_move_outlasts_the_clock() {
        assert_eq!(debit(1_000, 1_001, 5_000), None);
        // Exactly on the buzzer is not a flag, and the increment still lands.
        assert_eq!(debit(1_000, 1_000, 5_000), Some(5_000));
    }

    #[test]
    fn honest_jitter_is_accepted_and_theft_is_not() {
        // Claimed 5s, receiver saw 5.2s: inside the grace.
        assert!(plausible(60_000, 57_000, 5_200, 2_000, DEFAULT_GRACE_MS));
        // Claimed 5s, receiver saw 9s: four seconds is not jitter.
        assert!(!plausible(60_000, 57_000, 9_000, 2_000, DEFAULT_GRACE_MS));
        // Claiming to have used *more* than observed is the opponent's loss
        // and nobody else's business.
        assert!(plausible(60_000, 50_000, 1_000, 2_000, DEFAULT_GRACE_MS));
    }

    #[test]
    fn a_clock_cannot_grow_past_its_ceiling() {
        // 60s remaining plus a 2s increment is 62s; 62.001s is minted time.
        assert!(plausible(60_000, 62_000, 0, 2_000, DEFAULT_GRACE_MS));
        assert!(!plausible(60_000, 62_001, 0, 2_000, DEFAULT_GRACE_MS));
    }

    /// The worked examples in `spec/05-adjudication.md`, at two-second blocks.
    #[test]
    fn dilation_matches_the_spec_table() {
        for (clock_ms, blocks) in [
            (180_000u32, 3_632u32), // 3+2 blitz, fresh
            (30_000, 632),
            (5_000, 132),
            (400, 40),
        ] {
            assert_eq!(budget_blocks(clock_ms, TAU_MS), blocks, "{clock_ms} ms");
        }
    }

    #[test]
    fn the_ratio_survives_the_change_of_base() {
        // The whole point: the player who was losing on time is still losing
        // on time, by roughly the same factor, in the new unit.
        let rich = budget_blocks(180_000, TAU_MS);
        let poor = budget_blocks(400, TAU_MS);
        assert!(rich > poor * 80, "{rich} vs {poor}");
    }

    #[test]
    fn even_a_flagging_player_can_physically_move() {
        // Zero on the clock still buys the floor, because a deadline nobody
        // can meet is not a deadline, it is a forfeit with extra steps.
        assert_eq!(budget_blocks(0, TAU_MS), FLOOR_BLOCKS);
        assert!(budget_blocks(1, TAU_MS) >= FLOOR_BLOCKS);
    }

    #[test]
    fn budgets_are_capped() {
        // Otherwise a correspondence time control puts an unbounded worst
        // case on every validator.
        assert_eq!(budget_blocks(u32::MAX, TAU_MS), MAX_BUDGET);
        assert_eq!(budget_blocks(MAX_BUDGET * TAU_MS, TAU_MS), MAX_BUDGET);
    }

    #[test]
    fn stalling_cannot_buy_time_back() {
        // A player who burns their clock down and *then* forces a dispute
        // arrives with a smaller budget than if they had not. Dilation is
        // monotonic, so there is no clock value where stalling pays.
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
        assert_eq!(blocks_consumed(100, 103), MIN_MOVE_BLOCKS);
        assert_eq!(blocks_consumed(100, 140), 40);
        // A clock that went backwards is a reorg, not a negative duration.
        assert_eq!(blocks_consumed(140, 100), MIN_MOVE_BLOCKS);
    }
}
