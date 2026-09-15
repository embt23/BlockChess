//! ╔════════════════════════════════════════════════════════════════════╗
//! ║  G0 — THIS FILE IS EPISODE 05'S SUBJECT. EVAN TYPES IT.            ║
//! ║                                                                     ║
//! ║  `next_target` is `todo!()` on purpose. It is not missing and it   ║
//! ║  is not a stub to be helpfully filled in later; it is the piece    ║
//! ║  of code the episode is *about*, and `spec/09` D26 says the person ║
//! ║  filming types those. Everything around it is finished: the        ║
//! ║  target arithmetic, the miner, the fork choice, the reorg          ║
//! ║  machinery, and the tests below.                                    ║
//! ║                                                                     ║
//! ║  The tests are the specification. They are `#[ignore]`d so the     ║
//! ║  suite stays green while the hole is open — run them with          ║
//! ║      cargo test -p bc-pow -- --ignored                              ║
//! ║  and the ignore attributes come off in the same commit as the      ║
//! ║  implementation.                                                    ║
//! ╚════════════════════════════════════════════════════════════════════╝
//!
//! ## What the episode is about
//!
//! Bitcoin's retarget is a **proportional controller with gain 1**, sampling
//! every 2016 blocks, on a plant with enormous delay. Written as control
//! theory rather than as folklore:
//!
//! ```text
//!     error      = observed_ms / expected_ms
//!     new_target = prev_target × error        (clamped to ×4 and ÷4)
//! ```
//!
//! It is marginally stable and it oscillates under a hashrate step, which is
//! not a flaw anyone introduced — it is what a P-controller with gain 1 and
//! a one-sample-period delay does, and the stability analysis is the same
//! one you would run on any feedback loop. EIP-1559's base fee is the same
//! shape with a much shorter sampling period and an exponential response,
//! which is why it tracks and Bitcoin's hunts.
//!
//! The clamps are the interesting part and they are not tuning. Without
//! them a single absurd timestamp — and timestamps are miner-supplied, so
//! "absurd" is a choice an attacker makes — moves the difficulty
//! arbitrarily far in one step. The clamp converts a consensus-critical
//! parameter from *attacker-controlled* to *attacker-influenced, bounded*.
//!
//! ## What is already decided, so it is not in the hole
//!
//! - [`Target`] arithmetic, including the overflow behaviour of `scale`,
//!   which is the thing that actually bites (`target.rs`).
//! - The window: [`WINDOW`] blocks, and what `observed_ms` means over it.
//! - The clamp bounds, [`MAX_STEP_UP`] and [`MAX_STEP_DOWN`].
//!
//! So the hole is the control law and nothing else.

use crate::target::Target;

/// Blocks per retarget. Short compared to Bitcoin's 2016 because this chain
/// exists to be watched: at two-second blocks, a retarget every 32 blocks is
/// about a minute, which is a timescale a person can sit through on camera
/// and see the loop respond.
pub const WINDOW: u64 = 32;

/// Target block time. `spec/02`: this is the sampling rate of the whole
/// system, and everything measured in blocks inherits its resolution here.
pub const BLOCK_TIME_MS: u64 = 2_000;

/// The window's expected duration, if the hashrate were exactly matched.
pub const EXPECTED_WINDOW_MS: u64 = WINDOW * BLOCK_TIME_MS;

/// Difficulty may not fall by more than this factor in one step (the target
/// may not *rise* by more than it).
pub const MAX_STEP_UP: u64 = 4;
/// …and may not rise by more than this factor in one step.
pub const MAX_STEP_DOWN: u64 = 4;

/// ── G0 HOLE ─────────────────────────────────────────────────────────────
///
/// Given the target that produced the last [`WINDOW`] blocks and how long
/// they actually took, choose the next target.
///
/// Contract, which the tests below enforce:
///
/// 1. `observed == EXPECTED_WINDOW_MS` leaves the target unchanged.
/// 2. Slower than expected (`observed` larger) makes the target **larger**,
///    i.e. mining easier. Faster makes it smaller.
/// 3. The result is clamped to `[prev/MAX_STEP_DOWN, prev*MAX_STEP_UP]`.
/// 4. The result is never zero (unmineable) and never above [`MAX_TARGET`].
/// 5. It is a pure function of its arguments: same inputs, same bytes, on
///    every validator, forever.
///
/// [`Target::scale`] already handles the overflow and clamping-to-range
/// parts of (3) and (4); what it does not do is the control law.
pub fn next_target(prev: Target, observed_ms: u64) -> Target {
    todo!(
        "G0 — episode 05. Evan writes the control loop. \
         prev={prev:?} observed_ms={observed_ms} expected={EXPECTED_WINDOW_MS}"
    )
}

/// Clamp a ratio to the permitted step. Plumbing, so it is written.
///
/// Separated from the control law because the clamp is a decision already
/// made (above) while the law is the episode. A `next_target` that uses this
/// and one that clamps inline are both fine.
pub fn clamp_ratio(observed_ms: u64) -> (u64, u64) {
    let lo = EXPECTED_WINDOW_MS / MAX_STEP_DOWN;
    let hi = EXPECTED_WINDOW_MS.saturating_mul(MAX_STEP_UP);
    (observed_ms.clamp(lo, hi), EXPECTED_WINDOW_MS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::target::MAX_TARGET;

    const G0: &str = "G0: episode 05's subject — Evan types `next_target` on camera";

    /// Mining at exactly the target rate must not move the difficulty. A
    /// loop that drifts at zero error is not a controller, it is a leak.
    #[test]
    #[ignore = "G0: episode 05's subject — Evan types `next_target` on camera"]
    fn a_perfectly_paced_window_leaves_the_target_alone() {
        let t = Target(1 << 100);
        assert_eq!(next_target(t, EXPECTED_WINDOW_MS), t);
    }

    /// Sign convention. Getting this backwards produces a chain whose
    /// difficulty runs away from equilibrium in whichever direction it was
    /// first pushed, and it is a one-character mistake.
    #[test]
    #[ignore = "G0: episode 05's subject — Evan types `next_target` on camera"]
    fn slow_blocks_make_mining_easier_and_fast_blocks_make_it_harder() {
        let t = Target(1 << 100);
        assert!(
            next_target(t, EXPECTED_WINDOW_MS * 2) > t,
            "{G0}: slow blocks must raise the target"
        );
        assert!(
            next_target(t, EXPECTED_WINDOW_MS / 2) < t,
            "{G0}: fast blocks must lower the target"
        );
    }

    /// Monotone in the observed time, everywhere. A non-monotone controller
    /// has a region where mining slower makes it harder, which is an
    /// incentive nobody intended.
    #[test]
    #[ignore = "G0: episode 05's subject — Evan types `next_target` on camera"]
    fn the_target_is_monotone_in_the_observed_window() {
        let t = Target(1 << 100);
        let mut last = Target(0);
        for k in 1..=64u64 {
            let next = next_target(t, EXPECTED_WINDOW_MS * k / 8);
            assert!(next >= last, "not monotone at k={k}: {next:?} < {last:?}");
            last = next;
        }
    }

    /// Timestamps are miner-supplied. Without the clamp, one absurd value
    /// moves difficulty arbitrarily far in a single step, which turns a
    /// consensus parameter into something an attacker writes.
    #[test]
    #[ignore = "G0: episode 05's subject — Evan types `next_target` on camera"]
    fn a_single_absurd_timestamp_cannot_move_the_difficulty_far() {
        let t = Target(1 << 100);
        let up = next_target(t, u64::MAX);
        let down = next_target(t, 0);
        assert!(up.0 <= t.0.saturating_mul(MAX_STEP_UP as u128), "{G0}");
        assert!(down.0 >= t.0 / MAX_STEP_DOWN as u128, "{G0}");
    }

    /// Both ends of the range are absorbing states if they are reachable:
    /// a zero target can never be mined and the chain stops forever.
    #[test]
    #[ignore = "G0: episode 05's subject — Evan types `next_target` on camera"]
    fn the_target_never_leaves_the_mineable_range() {
        for start in [Target(1), Target(1 << 64), MAX_TARGET] {
            for observed in [0, 1, EXPECTED_WINDOW_MS, u64::MAX] {
                let next = next_target(start, observed);
                assert!(next.0 >= 1, "{G0}: target hit zero");
                assert!(next <= MAX_TARGET, "{G0}: target above maximum");
            }
        }
    }

    /// The closed-loop test, and the one the episode is really about.
    ///
    /// Run the controller against a simulated miner, step the hashrate by
    /// 4× partway through, and require that the observed block time comes
    /// back to target and *stays* there. A P-controller with gain 1 passes
    /// this with visible ringing, which is the point — the test asserts
    /// convergence, not elegance, and the ringing is what gets discussed.
    #[test]
    #[ignore = "G0: episode 05's subject — Evan types `next_target` on camera"]
    fn a_hashrate_step_settles_instead_of_running_away() {
        let mut target = Target(u128::MAX / 1_000);
        let mut hashrate: u128 = 1_000;

        let mut recent = Vec::new();
        for window in 0..60 {
            if window == 20 {
                hashrate *= 4;
            }
            // Blocks come at expected_work / hashrate seconds apiece.
            let observed_ms = (target.work() * WINDOW as u128 / hashrate).max(1) as u64;
            recent.push(observed_ms);
            target = next_target(target, observed_ms);
        }

        // The last ten windows must sit within 2× of target in both
        // directions. Ringing is allowed; divergence is not.
        for (i, ms) in recent.iter().rev().take(10).enumerate() {
            assert!(
                *ms >= EXPECTED_WINDOW_MS / 2 && *ms <= EXPECTED_WINDOW_MS * 2,
                "{G0}: window {i} from the end took {ms} ms, target {EXPECTED_WINDOW_MS}"
            );
        }
    }

    // ---- not the hole: this part is plumbing and is expected to pass ----

    #[test]
    fn the_clamp_bounds_the_ratio_in_both_directions() {
        assert_eq!(clamp_ratio(EXPECTED_WINDOW_MS).0, EXPECTED_WINDOW_MS);
        assert_eq!(clamp_ratio(0).0, EXPECTED_WINDOW_MS / MAX_STEP_DOWN);
        assert_eq!(clamp_ratio(u64::MAX).0, EXPECTED_WINDOW_MS * MAX_STEP_UP);
    }

    #[test]
    fn the_window_is_something_a_person_can_sit_through() {
        assert_eq!(EXPECTED_WINDOW_MS, 64_000);
    }
}
