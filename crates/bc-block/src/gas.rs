//! Episode 10 — reserved blockspace.
//!
//! The attack this defeats, or half-defeats: *"I'll censor your dispute."*
//!
//! ## What the reserve is, and what it is not
//!
//! Every block reserves a fraction of its gas limit that **only** the
//! dispute family may consume (`spec/02`). The rule is one comparison:
//!
//! ```text
//!     non-dispute gas  ≤  limit − reserve
//! ```
//!
//! Note which side is capped. The reserve is a **floor for disputes, not a
//! ceiling**: a block may be entirely dispute traffic, and on a busy
//! dispute day it should be. Capping disputes at 25% would be the same bug
//! the reserve exists to fix, wearing the opposite sign.
//!
//! ## Be precise about what this buys
//!
//! It stops disputes being **squeezed out**. A proposer who fills a block
//! with ordinary traffic cannot crowd you out, because ordinary traffic
//! hits its cap while your space is still free.
//!
//! It does **not** stop a proposer deliberately omitting you. A block
//! containing nothing at all satisfies this rule perfectly. `spec/02` is
//! honest about that — *"they must explicitly omit them, which is
//! detectable"* — and detectable is not prevented.
//!
//! So the reserve is exactly one half of the guarantee:
//!
//! ```text
//!     reserve   →  when an honest proposer arrives, there is ROOM
//!     Δ > f     →  an honest proposer ARRIVES IN TIME
//! ```
//!
//! The second half is [`bc-bft`'s censorship bound][1], and neither half is
//! worth anything alone: room with no honest turn is censorship anyway, and
//! an honest turn with no room is a squeeze-out anyway.
//!
//! [1]: https://github.com/embt23/BlockChess/blob/main/crates/bc-bft/src/censorship.rs

use crate::block::Block;

/// Basis points in the whole.
pub const BPS: u64 = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GasError {
    /// The block's transactions declare more gas than the limit allows.
    OverLimit { used: u64, limit: u64 },
    /// Ordinary traffic has eaten into the space reserved for disputes.
    CrowdsOutDisputes { non_dispute: u64, cap: u64 },
}

impl core::fmt::Display for GasError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for GasError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GasSchedule {
    pub limit: u64,
    /// Fraction of the limit only the dispute family may use, in basis
    /// points. `spec/02` proposes 25%.
    pub dispute_reserve_bps: u16,
}

impl Default for GasSchedule {
    fn default() -> GasSchedule {
        GasSchedule {
            limit: 30_000_000,
            dispute_reserve_bps: 2_500,
        }
    }
}

impl GasSchedule {
    /// A schedule with no reserve — what the chain looked like before this
    /// episode, kept so the demonstration can run both and compare.
    pub fn unprotected(limit: u64) -> GasSchedule {
        GasSchedule {
            limit,
            dispute_reserve_bps: 0,
        }
    }

    /// Gas no ordinary transaction may touch.
    pub fn reserve(&self) -> u64 {
        self.limit * self.dispute_reserve_bps.min(BPS as u16) as u64 / BPS
    }

    /// The most gas non-dispute traffic may claim in one block.
    pub fn non_dispute_cap(&self) -> u64 {
        self.limit - self.reserve()
    }

    /// Room still available to a dispute transaction, given what ordinary
    /// traffic has already taken.
    ///
    /// Never less than the reserve, however full the block is — which is
    /// the whole point, and the one line a test should be pointed at.
    pub fn room_for_disputes(&self, non_dispute_gas: u64) -> u64 {
        self.limit
            .saturating_sub(non_dispute_gas.min(self.non_dispute_cap()))
    }

    /// Is this block valid under the schedule?
    pub fn admits(&self, block: &Block) -> Result<(), GasError> {
        let used = block.gas_used();
        if used > self.limit {
            return Err(GasError::OverLimit {
                used,
                limit: self.limit,
            });
        }
        let non_dispute = block.non_dispute_gas();
        let cap = self.non_dispute_cap();
        if non_dispute > cap {
            return Err(GasError::CrowdsOutDisputes { non_dispute, cap });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::header::{BlockHeader, NO_PARENT};
    use crate::tx::{Payload, Tx};

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

    fn block(txs: Vec<Tx>) -> Block {
        Block {
            header: BlockHeader {
                version: 1,
                height: 1,
                parent_hash: NO_PARENT,
                state_root: [0u8; 32],
                tx_root: Block::tx_root(&txs),
                timestamp_ms: 0,
                proposer: [0u8; 32],
            },
            txs,
            seal: vec![],
        }
    }

    fn schedule() -> GasSchedule {
        GasSchedule {
            limit: 1_000,
            dispute_reserve_bps: 2_500,
        }
    }

    #[test]
    fn the_reserve_is_the_fraction_the_spec_says() {
        let s = schedule();
        assert_eq!(s.reserve(), 250);
        assert_eq!(s.non_dispute_cap(), 750);
    }

    #[test]
    fn ordinary_traffic_stops_at_the_cap() {
        let s = schedule();
        assert!(s
            .admits(&block(vec![tx(Payload::Transfer, 750, 0)]))
            .is_ok());
        assert_eq!(
            s.admits(&block(vec![tx(Payload::Transfer, 751, 0)])),
            Err(GasError::CrowdsOutDisputes {
                non_dispute: 751,
                cap: 750
            })
        );
    }

    /// The reserve is a floor for disputes, not a ceiling. A block that is
    /// entirely dispute traffic is valid, and on a busy dispute day it is
    /// what a block should be.
    #[test]
    fn disputes_may_use_the_whole_block() {
        let s = schedule();
        assert!(s
            .admits(&block(vec![tx(Payload::DisputeMove, 1_000, 0)]))
            .is_ok());
    }

    #[test]
    fn nothing_may_exceed_the_limit() {
        let s = schedule();
        assert_eq!(
            s.admits(&block(vec![tx(Payload::DisputeMove, 1_001, 0)])),
            Err(GasError::OverLimit {
                used: 1_001,
                limit: 1_000
            })
        );
    }

    /// The property the whole episode leans on: however much ordinary
    /// traffic a proposer stuffs in, there is always room left for a
    /// dispute.
    #[test]
    fn there_is_always_room_for_a_dispute() {
        let s = schedule();
        for flood in (0..3_000).step_by(7) {
            assert!(
                s.room_for_disputes(flood) >= s.reserve(),
                "a flood of {flood} left only {}",
                s.room_for_disputes(flood)
            );
        }
    }

    /// …and with no reserve, it does not. This is the pre-episode-10 chain,
    /// kept runnable so the comparison is a demonstration rather than an
    /// assertion.
    #[test]
    fn without_a_reserve_a_flood_leaves_nothing() {
        let s = GasSchedule::unprotected(1_000);
        assert_eq!(s.reserve(), 0);
        assert_eq!(s.room_for_disputes(1_000), 0);
        assert!(s
            .admits(&block(vec![tx(Payload::Transfer, 1_000, 0)]))
            .is_ok());
    }

    #[test]
    fn a_mixed_block_is_judged_only_on_its_ordinary_traffic() {
        let s = schedule();
        let b = block(vec![
            tx(Payload::Transfer, 700, 0),
            tx(Payload::DisputeMove, 300, 1),
        ]);
        assert_eq!(b.gas_used(), 1_000);
        assert_eq!(b.non_dispute_gas(), 700);
        assert!(s.admits(&b).is_ok());
    }

    /// `CloseGame` is the cooperative path and is deliberately ordinary
    /// traffic: both players signed it, nobody is under a deadline, and
    /// delaying it robs nobody.
    #[test]
    fn the_cooperative_close_does_not_get_privileged_space() {
        let s = schedule();
        assert_eq!(
            s.admits(&block(vec![tx(Payload::CloseGame, 800, 0)])),
            Err(GasError::CrowdsOutDisputes {
                non_dispute: 800,
                cap: 750
            })
        );
    }

    #[test]
    fn a_full_reserve_leaves_no_room_for_anything_else() {
        let s = GasSchedule {
            limit: 1_000,
            dispute_reserve_bps: 10_000,
        };
        assert_eq!(s.non_dispute_cap(), 0);
        assert!(s.admits(&block(vec![tx(Payload::Transfer, 1, 0)])).is_err());
        assert!(s
            .admits(&block(vec![tx(Payload::DisputeMove, 1_000, 0)]))
            .is_ok());
    }
}
