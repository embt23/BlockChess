//! The commit certificate: why anyone else should believe a block is final.
//!
//! `spec/02` puts it beside the header rather than inside it, because a
//! header cannot contain signatures over itself. Here it rides in
//! [`bc_block::Block::seal`], which is the slot each engine fills with
//! whatever closes a block — a nonce under proof-of-work, this under BFT.
//!
//! A certificate is self-verifying against a validator set: anyone holding
//! the set can check that more than ⅔ of stake precommitted this exact
//! block at this exact height and round. That is what makes finality
//! *transferable* — a light client does not have to have been present.

use crate::validators::ValidatorSet;
use crate::vote::{Step, Vote};
use bc_hash::Hash;
use bc_sig::{Signature, VerifyingKey};

/// 8 height + 4 round + 1 step + 1 has_block + 32 block + 32 validator + 64 sig.
const VOTE_LEN: usize = 142;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commit {
    pub round: u32,
    pub votes: Vec<Vote>,
}

impl Commit {
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(6 + self.votes.len() * VOTE_LEN);
        out.extend_from_slice(&self.round.to_le_bytes());
        out.extend_from_slice(&(self.votes.len() as u16).to_le_bytes());
        for v in &self.votes {
            out.extend_from_slice(&v.height.to_le_bytes());
            out.extend_from_slice(&v.round.to_le_bytes());
            out.push(v.step as u8);
            out.push(u8::from(v.block.is_some()));
            out.extend_from_slice(&v.block.unwrap_or([0u8; 32]));
            out.extend_from_slice(&v.validator.0);
            out.extend_from_slice(&v.signature.0);
        }
        out
    }

    pub fn decode(b: &[u8]) -> Option<Commit> {
        if b.len() < 6 {
            return None;
        }
        let round = u32::from_le_bytes(b[0..4].try_into().ok()?);
        let n = u16::from_le_bytes(b[4..6].try_into().ok()?) as usize;
        if b.len() != 6 + n * VOTE_LEN {
            return None;
        }
        let mut votes = Vec::with_capacity(n);
        for i in 0..n {
            let o = 6 + i * VOTE_LEN;
            let step = match b[o + 12] {
                1 => Step::Prevote,
                2 => Step::Precommit,
                _ => return None,
            };
            let mut block = [0u8; 32];
            block.copy_from_slice(&b[o + 14..o + 46]);
            let mut validator = [0u8; 32];
            validator.copy_from_slice(&b[o + 46..o + 78]);
            let mut sig = [0u8; 64];
            sig.copy_from_slice(&b[o + 78..o + 142]);
            votes.push(Vote {
                height: u64::from_le_bytes(b[o..o + 8].try_into().ok()?),
                round: u32::from_le_bytes(b[o + 8..o + 12].try_into().ok()?),
                step,
                block: match b[o + 13] {
                    0 => None,
                    1 => Some(block),
                    _ => return None,
                },
                validator: VerifyingKey(validator),
                signature: Signature(sig),
            });
        }
        Some(Commit { round, votes })
    }

    /// Does this certificate actually finalise `block` at `height`?
    ///
    /// Every condition here is load-bearing and each one has been the
    /// subject of a real CVE in some chain or other:
    ///
    /// - every vote is a **precommit**, not a prevote (a prevote quorum
    ///   does not finalise anything);
    /// - every vote is for **this** block, at **this** height and round;
    /// - every signature verifies;
    /// - every signer is in the validator set;
    /// - **no signer appears twice**, or one validator with one key funds
    ///   an entire quorum;
    /// - the stake totals above ⅔.
    pub fn verifies(&self, set: &ValidatorSet, height: u64, block: &Hash) -> bool {
        let mut seen: Vec<[u8; 32]> = Vec::with_capacity(self.votes.len());
        let mut stake = 0u128;
        for v in &self.votes {
            if v.step != Step::Precommit
                || v.height != height
                || v.round != self.round
                || v.block != Some(*block)
                || !set.contains(&v.validator)
                || !v.is_valid()
            {
                return false;
            }
            if seen.contains(&v.validator.0) {
                return false;
            }
            seen.push(v.validator.0);
            stake += set.stake_of(&v.validator);
        }
        stake >= set.quorum()
    }
}
