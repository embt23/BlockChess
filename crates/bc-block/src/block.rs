//! A block: a header, the transactions it commits to, and the engine's seal.
//!
//! The seal is opaque here — a PoW nonce, a BFT commit certificate, or
//! nothing at all — because what closes a block is exactly the part the two
//! episodes disagree about. See [`crate::consensus`].

use crate::header::BlockHeader;
use crate::tx::Tx;
use bc_hash::Hash;

/// Whatever the consensus engine needs beside the header to call a block
/// valid. Bytes here, structure in the engine.
pub type Seal = Vec<u8>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub header: BlockHeader,
    pub txs: Vec<Tx>,
    pub seal: Seal,
}

impl Block {
    /// The RFC 6962 Merkle root over the transaction ids, in block order.
    ///
    /// Order is part of the commitment and that is deliberate: two blocks
    /// with the same transactions in a different order execute differently
    /// against an account model, so they must not share a root.
    pub fn tx_root(txs: &[Tx]) -> Hash {
        let ids: Vec<Hash> = txs.iter().map(|t| t.hash()).collect();
        bc_merkle::tree::root_owned(&ids)
    }

    /// Does the header commit to the body it arrived with?
    ///
    /// The one check every engine must perform and neither may skip. A
    /// header whose `tx_root` does not match its transactions is a header
    /// that says nothing about what executing this block would do.
    pub fn body_matches_header(&self) -> bool {
        self.header.tx_root == Block::tx_root(&self.txs)
    }

    pub fn hash(&self) -> Hash {
        self.header.hash()
    }

    /// Total gas the block's transactions are allowed to consume.
    pub fn gas_used(&self) -> u64 {
        self.txs.iter().map(|t| t.gas_limit as u64).sum()
    }

    /// Gas claimed by the dispute family, which is what the reserve protects.
    pub fn dispute_gas(&self) -> u64 {
        self.txs
            .iter()
            .filter(|t| t.is_dispute())
            .map(|t| t.gas_limit as u64)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tx::Payload;

    fn tx(nonce: u64, payload: Payload) -> Tx {
        Tx {
            version: 1,
            nonce,
            sender: [1u8; 32],
            fee: 1,
            gas_limit: 100,
            payload,
            body: vec![nonce as u8],
            signature: [0u8; 64],
        }
    }

    fn block(txs: Vec<Tx>) -> Block {
        Block {
            header: BlockHeader {
                version: 1,
                height: 1,
                parent_hash: [0u8; 32],
                state_root: [0u8; 32],
                tx_root: Block::tx_root(&txs),
                timestamp_ms: 0,
                proposer: [0u8; 32],
            },
            txs,
            seal: vec![],
        }
    }

    #[test]
    fn a_block_commits_to_its_own_body() {
        let b = block(vec![tx(0, Payload::Transfer), tx(1, Payload::DisputeMove)]);
        assert!(b.body_matches_header());
    }

    #[test]
    fn swapping_a_transaction_breaks_the_commitment() {
        let mut b = block(vec![tx(0, Payload::Transfer), tx(1, Payload::Transfer)]);
        b.txs[1] = tx(2, Payload::Transfer);
        assert!(!b.body_matches_header());
    }

    /// Order is part of the commitment, because order is part of the result.
    #[test]
    fn reordering_transactions_changes_the_root() {
        let a = vec![tx(0, Payload::Transfer), tx(1, Payload::Transfer)];
        let b = vec![tx(1, Payload::Transfer), tx(0, Payload::Transfer)];
        assert_ne!(Block::tx_root(&a), Block::tx_root(&b));
    }

    #[test]
    fn an_empty_block_has_the_empty_root() {
        assert_eq!(Block::tx_root(&[]), bc_merkle::tree::empty_root());
        assert!(block(vec![]).body_matches_header());
    }

    #[test]
    fn dispute_gas_counts_only_the_dispute_family() {
        let b = block(vec![
            tx(0, Payload::Transfer),
            tx(1, Payload::DisputeMove),
            tx(2, Payload::CloseGame),
            tx(3, Payload::DisputeRefute),
        ]);
        assert_eq!(b.gas_used(), 400);
        assert_eq!(b.dispute_gas(), 200);
    }
}
