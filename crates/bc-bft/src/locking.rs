//! ╔════════════════════════════════════════════════════════════════════╗
//! ║  G0 — THIS FILE IS EPISODE 06'S SUBJECT. EVAN TYPES IT.            ║
//! ║                                                                     ║
//! ║  The two functions below are `todo!()`. Everything else in         ║
//! ║  `bc-bft` is finished and tested: votes, signatures, the stake-    ║
//! ║  weighted quorum arithmetic, the tally, equivocation detection,    ║
//! ║  the round machine, the commit certificate, and the `Consensus`    ║
//! ║  implementation.                                                    ║
//! ║                                                                     ║
//! ║  Run the specification with                                         ║
//! ║      cargo test -p bc-bft -- --ignored                              ║
//! ╚════════════════════════════════════════════════════════════════════╝
//!
//! ## Why the hole is here and not somewhere else
//!
//! **A single round of Tendermint needs no locking at all.** Propose,
//! collect a ⅔ prevote quorum, collect a ⅔ precommit quorum, commit — that
//! is already safe, by quorum intersection, as long as it *completes*.
//! `bc-bft` implements all of it and a four-validator network commits
//! blocks with this file untouched.
//!
//! Locking exists for one reason: **rounds can fail.** A proposer crashes,
//! a partition heals halfway through, a timeout fires — and now round 1
//! must choose a value without being able to see what round 0 decided.
//! Every line in this file is there because of that, which makes it
//! precisely the content of the episode: FLP says the failure cannot be
//! ruled out, partial synchrony says you survive it with timeouts, and
//! *this* is what you must remember across one to stay safe.
//!
//! So the boundary is not arbitrary. It is the difference between the
//! protocol that works and the protocol that survives.
//!
//! ## The rule, in words
//!
//! A validator holds a **lock**: a `(round, block)` pair recorded when it
//! last precommitted a block rather than nil. Having precommitted, it has
//! told the network that block may already have been committed by someone
//! who saw a quorum it did not. So:
//!
//! - It must not prevote a *different* block in a later round, or two
//!   quorums could form for two blocks and the overlap argument dies.
//! - It must be able to *release* the lock, or a single crashed proposer
//!   freezes the chain forever — which is safety bought with liveness, and
//!   a chain that never progresses is not a chain.
//!
//! Tendermint releases it on evidence: a proposal that carries a **proof
//! of lock change** — a ⅔ prevote quorum from a round strictly later than
//! the one the validator locked in. That quorum could not have formed
//! unless more than ⅓ of honest stake had already moved on, which means
//! the locked value cannot have been committed.
//!
//! ## What is already decided, so it is not in the hole
//!
//! - [`Lock`] and [`Proposal`], their shapes and what they carry.
//! - That the proposer re-proposes its own locked value ([`Node`] does it).
//! - That a released lock is *replaced*, never merely cleared.
//!
//! [`Node`]: crate::node::Node

use bc_hash::Hash;

/// What a validator is bound to, from precommitting it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lock {
    /// The round in which the precommit was cast.
    pub round: u32,
    pub block: Hash,
}

/// A proposal, as the locking rules need to see it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Proposal {
    pub block: Hash,
    /// The round whose ⅔ prevote quorum justifies this proposal, if the
    /// proposer had one. `None` for a freshly built block.
    ///
    /// This is Tendermint's `validRound`, and it is the proof of lock
    /// change: it says "more than two thirds of stake had already prevoted
    /// this in round `r`".
    pub valid_round: Option<u32>,
}

/// ── G0 HOLE ─────────────────────────────────────────────────────────────
///
/// What may this validator prevote in `round`?
///
/// Returns `Some(hash)` to prevote a block, `None` to prevote nil. Nil is a
/// real answer and often the right one — see `vote.rs`.
///
/// Contract, enforced by the tests below:
///
/// 1. Unlocked, with a proposal → prevote it.
/// 2. Unlocked, no proposal → nil.
/// 3. Locked on `b`, proposal is `b` → prevote `b`.
/// 4. Locked on `b`, proposal is `c ≠ b` with no proof of lock change →
///    **nil**. Never `c`. This is the safety rule; everything else is
///    liveness.
/// 5. Locked on `b` at round `r`, proposal is `c` with
///    `valid_round = Some(v)` where `v > r` → prevote `c`. The lock is
///    released by evidence, and only by evidence.
/// 6. A `valid_round` at or below the locked round proves nothing that was
///    not already known, so it does not release the lock.
/// 7. A `valid_round` from the future (`v >= round`) is not evidence, it
///    is a claim about a round that has not happened.
pub fn prevote_for(lock: Option<Lock>, proposal: Option<Proposal>, round: u32) -> Option<Hash> {
    todo!(
        "G0 — episode 06. Evan writes the locking rule. \
         lock={lock:?} proposal={proposal:?} round={round}"
    )
}

/// ── G0 HOLE ─────────────────────────────────────────────────────────────
///
/// Update the lock after a ⅔ **prevote** quorum was seen for `block` in
/// `round`, immediately before precommitting it.
///
/// Contract:
///
/// 1. A quorum for a block always (re)locks on it at the current round —
///    including when already locked on the same block, because the round
///    must move forward or rule 6 above never releases anything.
/// 2. A quorum for **nil** releases nothing and locks nothing. A validator
///    that dropped its lock on a nil quorum could prevote a conflicting
///    block next round, which is the safety rule with extra steps.
/// 3. The returned lock is never at a round earlier than the one passed in.
pub fn lock_on_quorum(lock: Option<Lock>, block: Option<Hash>, round: u32) -> Option<Lock> {
    todo!(
        "G0 — episode 06. Evan writes the lock update. \
         lock={lock:?} block={block:?} round={round}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const G0: &str = "G0: episode 06's subject — Evan types the locking rules";
    const B: Hash = [0xbb; 32];
    const C: Hash = [0xcc; 32];

    #[test]
    #[ignore = "G0: episode 06's subject — Evan types the locking rules"]
    fn an_unlocked_validator_prevotes_whatever_was_proposed() {
        let p = Proposal {
            block: B,
            valid_round: None,
        };
        assert_eq!(prevote_for(None, Some(p), 0), Some(B));
    }

    #[test]
    #[ignore = "G0: episode 06's subject — Evan types the locking rules"]
    fn an_unlocked_validator_with_no_proposal_prevotes_nil() {
        assert_eq!(prevote_for(None, None, 3), None);
    }

    #[test]
    #[ignore = "G0: episode 06's subject — Evan types the locking rules"]
    fn a_locked_validator_prevotes_its_own_locked_value_again() {
        let lock = Lock { round: 1, block: B };
        let p = Proposal {
            block: B,
            valid_round: None,
        };
        assert_eq!(prevote_for(Some(lock), Some(p), 4), Some(B));
    }

    /// **The safety rule.** Everything else in this file is liveness; if
    /// only one of these tests passes, it should be this one.
    #[test]
    #[ignore = "G0: episode 06's subject — Evan types the locking rules"]
    fn a_locked_validator_never_prevotes_a_conflicting_block_without_proof() {
        let lock = Lock { round: 1, block: B };
        let p = Proposal {
            block: C,
            valid_round: None,
        };
        assert_eq!(prevote_for(Some(lock), Some(p), 5), None, "{G0}");
    }

    /// …and it must be able to change its mind on evidence, or one crashed
    /// proposer stops the chain forever.
    #[test]
    #[ignore = "G0: episode 06's subject — Evan types the locking rules"]
    fn a_later_prevote_quorum_releases_the_lock() {
        let lock = Lock { round: 1, block: B };
        let p = Proposal {
            block: C,
            valid_round: Some(3),
        };
        assert_eq!(prevote_for(Some(lock), Some(p), 5), Some(C), "{G0}");
    }

    /// Evidence from before the lock is not evidence. This is the test a
    /// plausible implementation fails: `valid_round.is_some()` is not the
    /// condition, `valid_round > lock.round` is.
    #[test]
    #[ignore = "G0: episode 06's subject — Evan types the locking rules"]
    fn a_prevote_quorum_from_before_the_lock_releases_nothing() {
        let lock = Lock { round: 4, block: B };
        for v in [0, 1, 2, 3, 4] {
            let p = Proposal {
                block: C,
                valid_round: Some(v),
            };
            assert_eq!(
                prevote_for(Some(lock), Some(p), 6),
                None,
                "{G0}: valid_round {v} is not later than the lock at 4"
            );
        }
    }

    /// Nor is a claim about a round that has not happened yet.
    #[test]
    #[ignore = "G0: episode 06's subject — Evan types the locking rules"]
    fn a_valid_round_from_the_future_is_not_evidence() {
        let lock = Lock { round: 1, block: B };
        for v in [5, 6, 99] {
            let p = Proposal {
                block: C,
                valid_round: Some(v),
            };
            assert_eq!(prevote_for(Some(lock), Some(p), 5), None, "{G0}: vr={v}");
        }
    }

    #[test]
    #[ignore = "G0: episode 06's subject — Evan types the locking rules"]
    fn a_prevote_quorum_for_a_block_locks_on_it_at_the_current_round() {
        assert_eq!(
            lock_on_quorum(None, Some(B), 2),
            Some(Lock { round: 2, block: B })
        );
    }

    /// Re-locking on the same block must still move the round forward, or
    /// the release rule can never fire and the chain stalls.
    #[test]
    #[ignore = "G0: episode 06's subject — Evan types the locking rules"]
    fn relocking_on_the_same_block_advances_the_locked_round() {
        let lock = Lock { round: 1, block: B };
        assert_eq!(
            lock_on_quorum(Some(lock), Some(B), 7),
            Some(Lock { round: 7, block: B }),
            "{G0}"
        );
    }

    /// A nil quorum says "nobody agreed", not "you are free".
    #[test]
    #[ignore = "G0: episode 06's subject — Evan types the locking rules"]
    fn a_nil_quorum_does_not_release_a_lock() {
        let lock = Lock { round: 1, block: B };
        assert_eq!(lock_on_quorum(Some(lock), None, 9), Some(lock), "{G0}");
        assert_eq!(lock_on_quorum(None, None, 9), None);
    }

    #[test]
    #[ignore = "G0: episode 06's subject — Evan types the locking rules"]
    fn a_lock_never_moves_backwards() {
        let lock = Lock { round: 8, block: B };
        for r in 0..8 {
            if let Some(next) = lock_on_quorum(Some(lock), Some(C), r) {
                assert!(next.round >= r, "{G0}: lock went to round {}", next.round);
            }
        }
    }
}
