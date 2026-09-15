//! ╔════════════════════════════════════════════════════════════════════╗
//! ║  G0 — THIS FILE IS EPISODE 10'S SUBJECT. EVAN TYPES IT.            ║
//! ║                                                                     ║
//! ║  `max_byzantine_run` and `window_is_safe` are `todo!()`.           ║
//! ║  Everything else is finished: the gas reserve as a block-validity  ║
//! ║  rule in `bc-block`, enforced by both engines, the censoring       ║
//! ║  proposer, and the demonstration.                                   ║
//! ║                                                                     ║
//! ║      cargo test -p bc-bft -- --ignored                              ║
//! ╚════════════════════════════════════════════════════════════════════╝
//!
//! ## The two halves
//!
//! A deadline is only fair if the chain can promise you a turn:
//!
//! ```text
//!     the reserve  →  when an honest proposer arrives, there is ROOM
//!     this file    →  an honest proposer ARRIVES IN TIME
//! ```
//!
//! Neither is worth anything alone. Room with no honest turn is censorship
//! anyway; an honest turn with no room is a squeeze-out anyway. `bc-block`'s
//! `gas` module is the first half and says the same thing from its side.
//!
//! ## The argument
//!
//! Proposers rotate round-robin over the validator set, sorted by key. In
//! any `w` consecutive blocks you therefore see `min(w, n)` distinct
//! proposers. At most `f` of them are Byzantine — and an adversary who
//! chooses their own keys can make those `f` land **consecutively**, so the
//! worst case is a run of `f`, not `f` scattered.
//!
//! An honest proposer is reached inside the window iff the window is longer
//! than the longest run the adversary can arrange.
//!
//! ## The part that is not obvious, and is the episode
//!
//! The window is **not Δ**.
//!
//! `Dispute::arm` gives the responder `min(Δ, budget)` blocks, floored at
//! `MIN_MOVE_BLOCKS`. As a budget runs down that window shrinks, and it
//! shrinks all the way to the floor. So a channel negotiated with a
//! generous Δ of 2048 can still, late in a dispute, hand someone an
//! **8-block** window — and 8 blocks is what has to beat `f`, not 2048.
//!
//! Δ is therefore almost irrelevant to censorship safety. The binding
//! quantity is the floor, which is a protocol constant rather than
//! something a channel negotiates. `docs/build-log.md` §18 is how that
//! floor came to exist at all.
//!
//! The consequence is sharp and is worth stating on camera: with
//! `MIN_MOVE_BLOCKS = 8`, this scheme secures a validator set only up to
//! the size at which `f` reaches 8. Past that, a dispute's last few moves
//! are censorable however long Δ was. Raising the floor, weighting the
//! rotation, or a forced-inclusion queue are the ways out, and none of
//! them is built.
//!
//! ## Why plain integers rather than `GameTerms`
//!
//! Consensus does not know what a chess game is. The caller reads Δ and the
//! floor out of the signed terms and passes numbers; keeping `bc-adjudicator`
//! out of `bc-bft`'s dependency list is the same instinct as `D24`.

use crate::validators::ValidatorSet;

/// ── G0 HOLE ─────────────────────────────────────────────────────────────
///
/// The longest run of consecutive Byzantine proposers the adversary can
/// arrange in the rotation.
///
/// Contract:
///
/// 1. It is the Byzantine stake bound — the adversary controls at most that
///    much, and with chosen keys can place it consecutively.
/// 2. A set of one has a run of 0: a single validator that is Byzantine is
///    not a *bound* being exceeded, it is the whole chain being hostile,
///    which no rotation argument addresses.
/// 3. It never reaches the size of the set, or there would be no honest
///    proposer to reach at all.
pub fn max_byzantine_run(set: &ValidatorSet) -> u32 {
    todo!(
        "G0 — episode 10. Evan writes the censorship bound. \
         n={} total_stake={}",
        set.len(),
        set.total_stake()
    )
}

/// ── G0 HOLE ─────────────────────────────────────────────────────────────
///
/// Is a response window of `window_blocks` long enough that an honest
/// proposer is guaranteed a turn inside it?
///
/// Contract:
///
/// 1. Strictly longer than the worst run is safe; equal is **not**. A
///    window of exactly `f` can be spanned by `f` Byzantine proposers, and
///    the off-by-one here is the whole difference between a guarantee and
///    a coin flip.
/// 2. A zero window is never safe.
/// 3. Safety is monotone in the window: if `w` is safe, so is anything
///    longer.
/// 4. It does not depend on Δ. `window_blocks` is what `Dispute::arm`
///    actually handed out, which late in a dispute is the floor — see the
///    module docs.
pub fn window_is_safe(set: &ValidatorSet, window_blocks: u32) -> bool {
    todo!(
        "G0 — episode 10. Evan writes the safety test. \
         window={window_blocks} run={:?}",
        set.len()
    )
}

// ---- not the hole: plumbing, and the numbers the hole is applied to ----

/// The smallest response window a dispute under these terms can ever
/// produce.
///
/// `Dispute::arm` hands the responder
///
/// ```text
///     window = max( min(Δ, budget), MIN_MOVE_BLOCKS )
/// ```
///
/// and a budget runs down to nothing. Minimising over every budget a live
/// dispute can hold — that is, over `budget ≥ 1`, because a budget of zero
/// is a flag rather than a window:
///
/// ```text
///     inf  = max( min(Δ, 1), MIN_MOVE_BLOCKS )
///          = MIN_MOVE_BLOCKS
/// ```
///
/// **Δ cancels.** That is not a simplification, it is the result: a channel
/// that negotiated a 2048-block challenge window is, in its last few moves,
/// defended by exactly the same 8 blocks as one that negotiated 64. Δ is
/// the *maximum* window and censorship safety is a question about the
/// *minimum*.
///
/// `delta_blocks` stays in the signature so a reader watches it being
/// discarded — and it is only *almost* discarded: the floor cannot exceed
/// Δ itself, which matters for nothing a real time control produces and
/// keeps the function total.
pub fn smallest_window(delta_blocks: u32, min_move_blocks: u32) -> u32 {
    // `arm` floors the window at the move cost but never hands out more
    // than Δ, so the floor it actually applies is `min(Δ, move cost)`.
    // For every real time control Δ is far larger (the smallest class is
    // 64 blocks against a cost of 8), so this is the move cost — and Δ
    // cancels. The `min` is here so the function is total rather than
    // correct-only-for-sane-input.
    delta_blocks.min(min_move_blocks)
}

/// Everything a caller needs to decide whether to accept a channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Assessment {
    /// Worst run of Byzantine proposers.
    pub byzantine_run: u32,
    /// The window the channel can shrink to.
    pub smallest_window: u32,
    pub safe: bool,
}

/// Assess a channel's terms against a live validator set.
///
/// Composes the hole with the plumbing, so it panics until the hole is
/// filled. That is deliberate: a version that quietly returned `safe: true`
/// would be a censorship check that does nothing, which is worse than none.
pub fn assess(set: &ValidatorSet, delta_blocks: u32, min_move_blocks: u32) -> Assessment {
    let smallest_window = smallest_window(delta_blocks, min_move_blocks);
    Assessment {
        byzantine_run: max_byzantine_run(set),
        smallest_window,
        safe: window_is_safe(set, smallest_window),
    }
}
