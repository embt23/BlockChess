//! The block header: 146 bytes, fixed, in the order `spec/02` gives them.
//!
//! A header is the unit a light client downloads and the unit a fork choice
//! rule compares, so its encoding is consensus-critical in a way the body is
//! not. It is fixed-width on purpose: there is no length prefix to disagree
//! about and no variable field whose encoding two implementations could
//! round-trip differently.
//!
//! ## What is deliberately *not* in here
//!
//! **Signatures.** The commit certificate — the set of validator precommits
//! that finalises a block under BFT — travels beside the header, never
//! inside it, because a header cannot contain signatures over itself.
//!
//! **The nonce.** Proof-of-work needs one and BFT does not, so it lives in
//! [`crate::consensus::Seal`], which is the part of a block each engine
//! defines for itself. Putting a PoW nonce in the shared header would make
//! every BFT block carry eight bytes of nothing, and would quietly encode
//! the assumption that there is only ever one way to close a block.

use bc_hash::{tagged, Hash};

/// 2 version + 8 height + 32 parent + 32 state_root + 32 tx_root
/// + 8 timestamp + 32 proposer.
pub const HEADER_LEN: usize = 146;

/// The first block's parent. Not a real hash of anything.
pub const NO_PARENT: Hash = [0u8; 32];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockHeader {
    pub version: u16,
    pub height: u64,
    pub parent_hash: Hash,
    /// The SMT root **after** this block's transactions are applied.
    pub state_root: Hash,
    /// RFC 6962 Merkle root of the transaction list.
    pub tx_root: Hash,
    /// Advisory only. Nothing security-critical may be measured in it —
    /// that is `P4`, and it is why challenge windows count blocks.
    pub timestamp_ms: u64,
    /// Whoever produced this block: a miner under PoW, a validator under BFT.
    pub proposer: Hash,
}

impl BlockHeader {
    pub fn encode(&self) -> [u8; HEADER_LEN] {
        let mut b = [0u8; HEADER_LEN];
        b[0..2].copy_from_slice(&self.version.to_le_bytes());
        b[2..10].copy_from_slice(&self.height.to_le_bytes());
        b[10..42].copy_from_slice(&self.parent_hash);
        b[42..74].copy_from_slice(&self.state_root);
        b[74..106].copy_from_slice(&self.tx_root);
        b[106..114].copy_from_slice(&self.timestamp_ms.to_le_bytes());
        b[114..146].copy_from_slice(&self.proposer);
        b
    }

    pub fn decode(b: &[u8]) -> Option<BlockHeader> {
        if b.len() != HEADER_LEN {
            return None;
        }
        let h = |from: usize| -> Hash {
            let mut out = [0u8; 32];
            out.copy_from_slice(&b[from..from + 32]);
            out
        };
        Some(BlockHeader {
            version: u16::from_le_bytes([b[0], b[1]]),
            height: u64::from_le_bytes(b[2..10].try_into().ok()?),
            parent_hash: h(10),
            state_root: h(42),
            tx_root: h(74),
            timestamp_ms: u64::from_le_bytes(b[106..114].try_into().ok()?),
            proposer: h(114),
        })
    }

    /// The block's identity.
    ///
    /// Domain-tagged like every other hash in the project, so a header can
    /// never be confused with a position, a state or a transaction even if
    /// the bytes were to collide.
    pub fn hash(&self) -> Hash {
        tagged("BC/header/v1", &self.encode())
    }

    pub fn is_genesis(&self) -> bool {
        self.height == 0 && self.parent_hash == NO_PARENT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> BlockHeader {
        BlockHeader {
            version: 1,
            height: 4_242,
            parent_hash: [7u8; 32],
            state_root: [9u8; 32],
            tx_root: [11u8; 32],
            timestamp_ms: 1_700_000_000_000,
            proposer: [13u8; 32],
        }
    }

    #[test]
    fn the_header_is_exactly_the_length_the_spec_says() {
        assert_eq!(HEADER_LEN, 146);
        assert_eq!(sample().encode().len(), 146);
    }

    #[test]
    fn a_header_round_trips() {
        let h = sample();
        assert_eq!(BlockHeader::decode(&h.encode()), Some(h));
    }

    #[test]
    fn decoding_rejects_anything_that_is_not_exactly_the_right_length() {
        let enc = sample().encode();
        assert_eq!(BlockHeader::decode(&enc[..145]), None);
        let mut long = enc.to_vec();
        long.push(0);
        assert_eq!(BlockHeader::decode(&long), None);
    }

    /// Every field has to reach the bytes. A field the encoder forgets is a
    /// field two validators can disagree about while both verifying the same
    /// hash.
    #[test]
    fn every_field_changes_the_hash() {
        let base = sample();
        let mut variants = alloc_variants(base);
        variants.dedup();
        assert_eq!(variants.len(), 7, "a field is missing from the encoding");
    }

    fn alloc_variants(base: BlockHeader) -> Vec<Hash> {
        let mut out = Vec::new();
        let mut v = base;
        v.version ^= 1;
        out.push(v.hash());
        let mut v = base;
        v.height ^= 1;
        out.push(v.hash());
        let mut v = base;
        v.parent_hash[0] ^= 1;
        out.push(v.hash());
        let mut v = base;
        v.state_root[0] ^= 1;
        out.push(v.hash());
        let mut v = base;
        v.tx_root[0] ^= 1;
        out.push(v.hash());
        let mut v = base;
        v.timestamp_ms ^= 1;
        out.push(v.hash());
        let mut v = base;
        v.proposer[0] ^= 1;
        out.push(v.hash());
        out.sort();
        out
    }

    /// The header hash is domain-tagged, so the same 146 bytes hashed as
    /// anything else gives a different answer.
    #[test]
    fn the_header_hash_is_domain_separated() {
        let h = sample();
        assert_ne!(h.hash(), bc_hash::sha256(&h.encode()));
    }
}
