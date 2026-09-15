//! Choosing what goes in a block.
//!
//! The gas reserve is usually described as a validity rule, and `bc-block`
//! implements it as one. But a validity rule that nothing exercises is
//! decoration: a proposer who never fills a block never trips it. The
//! reserve earns its keep here, in the **builder**, where it is the thing
//! that stops ordinary traffic from consuming the space a dispute needs.
//!
//! Two proposers, differing in one line:
//!
//! - [`Proposer::Honest`] takes transactions in mempool order, subject to
//!   the schedule.
//! - [`Proposer::Censoring`] does the same, and silently drops everything
//!   matching a predicate.
//!
//! The censoring one produces **perfectly valid blocks**. That is the point
//! and it is why the reserve is only half of episode 10: a block containing
//! nothing at all satisfies every rule in `bc-block::gas`.

use bc_block::gas::GasSchedule;
use bc_block::Tx;

/// How a proposer picks transactions.
#[derive(Clone, Copy)]
pub enum Proposer {
    Honest,
    /// Drops every transaction the predicate accepts.
    Censoring(fn(&Tx) -> bool),
}

impl Proposer {
    /// Censor dispute transactions — the attack episode 10 is about.
    pub fn censoring_disputes() -> Proposer {
        Proposer::Censoring(|t| t.is_dispute())
    }

    fn drops(&self, tx: &Tx) -> bool {
        match self {
            Proposer::Honest => false,
            Proposer::Censoring(p) => p(tx),
        }
    }
}

/// Fill a block from the mempool, in order, respecting the schedule.
///
/// In mempool order rather than by fee, because fee-ordering would make
/// the demonstration about fee markets — a flood that outbids you is a
/// different attack with a different answer. Taking transactions in the
/// order they arrived isolates the one thing under test: whether ordinary
/// traffic can consume the space reserved for disputes.
///
/// A transaction that does not fit is **skipped, not abandoned** — the loop
/// continues, so a dispute arriving after a flood still gets its chance at
/// the reserve. A builder that stopped at the first non-fitting transaction
/// would defeat the reserve while satisfying it, which is worth knowing is
/// possible.
pub fn select(mempool: &[Tx], gas: &GasSchedule, proposer: Proposer) -> Vec<Tx> {
    let mut out = Vec::new();
    let (mut total, mut ordinary) = (0u64, 0u64);
    for tx in mempool {
        if proposer.drops(tx) {
            continue;
        }
        let g = tx.gas_limit as u64;
        if total + g > gas.limit {
            continue;
        }
        if !tx.is_dispute() && ordinary + g > gas.non_dispute_cap() {
            continue;
        }
        if !tx.is_dispute() {
            ordinary += g;
        }
        total += g;
        out.push(tx.clone());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use bc_block::Payload;

    fn tx(payload: Payload, gas: u32, n: u64) -> Tx {
        Tx {
            version: 1,
            nonce: n,
            sender: [1u8; 32],
            fee: 1,
            gas_limit: gas,
            payload,
            body: vec![n as u8],
            signature: [0u8; 64],
        }
    }

    /// A flood of ordinary traffic, then one dispute arriving last — the
    /// worst ordering for the victim.
    fn flooded_mempool() -> Vec<Tx> {
        let mut m: Vec<Tx> = (0..40).map(|i| tx(Payload::Transfer, 50_000, i)).collect();
        m.push(tx(Payload::DisputeMove, 50_000, 999));
        m
    }

    fn contains_dispute(txs: &[Tx]) -> bool {
        txs.iter().any(|t| t.is_dispute())
    }

    /// Without a reserve, a flood crowds the dispute out. This is the
    /// pre-episode-10 chain, and the victim loses without anyone having to
    /// censor anything.
    #[test]
    fn without_a_reserve_a_flood_crowds_the_dispute_out() {
        let gas = GasSchedule::unprotected(1_000_000);
        let picked = select(&flooded_mempool(), &gas, Proposer::Honest);
        assert_eq!(picked.len(), 20, "the flood filled the block");
        assert!(!contains_dispute(&picked), "the dispute should not fit");
    }

    /// With one, the same flood stops at the cap and the dispute gets in.
    #[test]
    fn a_reserve_keeps_room_through_the_same_flood() {
        let gas = GasSchedule {
            limit: 1_000_000,
            dispute_reserve_bps: 2_500,
        };
        let picked = select(&flooded_mempool(), &gas, Proposer::Honest);
        assert!(
            contains_dispute(&picked),
            "the reserve should have held room"
        );
        let ordinary: u64 = picked
            .iter()
            .filter(|t| !t.is_dispute())
            .map(|t| t.gas_limit as u64)
            .sum();
        assert!(ordinary <= gas.non_dispute_cap());
    }

    /// The reserve does nothing against a proposer who simply leaves you
    /// out, and the block it builds is valid. This is the half episode 10
    /// does not close with blockspace.
    #[test]
    fn a_reserve_does_nothing_against_deliberate_omission() {
        let gas = GasSchedule {
            limit: 1_000_000,
            dispute_reserve_bps: 2_500,
        };
        let picked = select(&flooded_mempool(), &gas, Proposer::censoring_disputes());
        assert!(!contains_dispute(&picked));
        // …and what it produced is a perfectly legal block.
        let ordinary: u64 = picked.iter().map(|t| t.gas_limit as u64).sum();
        assert!(ordinary <= gas.non_dispute_cap());
    }

    /// Dispute traffic may take the whole block when that is what is
    /// waiting. The reserve is a floor, not a quota.
    #[test]
    fn disputes_alone_may_fill_a_block() {
        let gas = GasSchedule {
            limit: 1_000_000,
            dispute_reserve_bps: 2_500,
        };
        let m: Vec<Tx> = (0..40)
            .map(|i| tx(Payload::DisputeMove, 50_000, i))
            .collect();
        let picked = select(&m, &gas, Proposer::Honest);
        assert_eq!(picked.len(), 20, "disputes should use the whole limit");
    }

    /// Skipping rather than stopping is what lets a late dispute reach the
    /// reserve at all.
    #[test]
    fn a_transaction_that_does_not_fit_is_skipped_not_final() {
        let gas = GasSchedule {
            limit: 100,
            dispute_reserve_bps: 2_500,
        };
        // Cap on ordinary traffic is 75 of the 100 limit.
        let m = vec![
            tx(Payload::Transfer, 70, 0), // fits
            tx(Payload::Transfer, 70, 1), // would take ordinary to 140: skipped
            tx(Payload::Transfer, 5, 2),  // still fits, because we kept going
        ];
        let picked = select(&m, &gas, Proposer::Honest);
        assert_eq!(picked.len(), 2, "the builder stopped instead of skipping");
        assert_eq!(picked[1].nonce, 2);
    }
}
