//! Transactions, and the ratio that is the honest summary of the job.
//!
//! Ten payload variants (`spec/02`). **Two of them are the happy path** —
//! `OpenGame` and `CloseGame`, the two transactions a cooperative 40-move
//! blitz game costs the chain. The other eight exist only because people
//! misbehave.
//!
//! ## Why the payload is opaque bytes here
//!
//! A `DisputeMove` payload means something to `bc-adjudicator` and nothing
//! to a consensus engine: a miner orders transactions, it does not adjudicate
//! them. Keeping the body opaque at this layer means `bc-pow` and `bc-bft`
//! can be written, tested and reasoned about without linking the chess rules
//! at all — and it means the chain cannot accidentally grow an opinion about
//! a game, which is the whole architecture in one type.
//!
//! The kind tag is *not* opaque, because the chain does need one thing from
//! it: [`Payload::is_dispute`], which decides who may spend the reserved
//! dispute gas (`spec/02` §censorship-resistance 2). That is the minimum
//! knowledge required and it is deliberately the maximum.

use bc_hash::{tagged_parts, Hash};

/// What a transaction is asking the chain to do.
///
/// Discriminants are wire values and are append-only for the same reason
/// `RulesetRegistry` is (`D25`): renumbering one changes the meaning of
/// transactions already signed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Payload {
    Transfer = 0,
    OpenGame = 1,
    CloseGame = 2,
    DisputeOpen = 3,
    DisputeMove = 4,
    DisputeClaimTerminal = 5,
    DisputeRefute = 6,
    DisputeFinalize = 7,
    RegisterServer = 8,
    SlashServer = 9,
}

impl Payload {
    pub const ALL: [Payload; 10] = [
        Payload::Transfer,
        Payload::OpenGame,
        Payload::CloseGame,
        Payload::DisputeOpen,
        Payload::DisputeMove,
        Payload::DisputeClaimTerminal,
        Payload::DisputeRefute,
        Payload::DisputeFinalize,
        Payload::RegisterServer,
        Payload::SlashServer,
    ];

    pub fn from_u8(b: u8) -> Option<Payload> {
        Payload::ALL.into_iter().find(|p| *p as u8 == b)
    }

    /// May this transaction spend the block's reserved dispute gas?
    ///
    /// The reserve is what stops a proposer squeezing disputes out by
    /// stuffing a block with ordinary traffic. It only works if the
    /// membership test is narrow: `Transfer` must never qualify, or the
    /// reserve is just more general-purpose blockspace.
    ///
    /// `CloseGame` is excluded although it settles a game, because it is the
    /// *cooperative* path — both players signed it, so nobody is under a
    /// deadline and nobody can be robbed by delay.
    pub fn is_dispute(self) -> bool {
        matches!(
            self,
            Payload::DisputeOpen
                | Payload::DisputeMove
                | Payload::DisputeClaimTerminal
                | Payload::DisputeRefute
                | Payload::DisputeFinalize
        )
    }
}

/// A transaction, with its body left uninterpreted.
///
/// `body` is whatever the payload kind means — a `GameOffer` and two
/// signatures for `OpenGame`, a packed move for `DisputeMove`. The consensus
/// layer never looks inside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tx {
    pub version: u16,
    /// Replay protection. Must equal the sender account's nonce.
    pub nonce: u64,
    pub sender: Hash,
    pub fee: u128,
    pub gas_limit: u32,
    pub payload: Payload,
    pub body: Vec<u8>,
    pub signature: [u8; 64],
}

impl Tx {
    /// The bytes a sender signs: everything except the signature.
    ///
    /// Excluding the signature is not a convention, it is forced — a
    /// signature cannot be over itself. Including *everything else* is the
    /// part that is a choice, and it is the right one: any field left out
    /// is a field an attacker may rewrite in flight.
    pub fn signing_bytes(&self) -> Hash {
        tagged_parts(
            "BC/tx/v1",
            &[
                &self.version.to_le_bytes(),
                &self.nonce.to_le_bytes(),
                &self.sender,
                &self.fee.to_le_bytes(),
                &self.gas_limit.to_le_bytes(),
                &[self.payload as u8],
                &self.body,
            ],
        )
    }

    /// The transaction's identity, signature included.
    ///
    /// Distinct from [`Tx::signing_bytes`] on purpose: two transactions
    /// differing only in signature are the same *intent* but different
    /// *objects*, and the Merkle tree in a block commits to objects.
    pub fn hash(&self) -> Hash {
        tagged_parts("BC/txid/v1", &[&self.signing_bytes(), &self.signature])
    }

    pub fn is_dispute(&self) -> bool {
        self.payload.is_dispute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tx(payload: Payload) -> Tx {
        Tx {
            version: 1,
            nonce: 3,
            sender: [1u8; 32],
            fee: 10,
            gas_limit: 21_000,
            payload,
            body: vec![9, 9, 9],
            signature: [4u8; 64],
        }
    }

    #[test]
    fn payload_tags_round_trip_and_are_stable() {
        for p in Payload::ALL {
            assert_eq!(Payload::from_u8(p as u8), Some(p));
        }
        // Pinned: these are wire values, and renumbering one silently
        // changes what already-signed transactions mean.
        assert_eq!(Payload::Transfer as u8, 0);
        assert_eq!(Payload::DisputeMove as u8, 4);
        assert_eq!(Payload::SlashServer as u8, 9);
        assert_eq!(Payload::from_u8(10), None);
    }

    /// The reserve is only a defence if the set it protects is small.
    #[test]
    fn only_the_dispute_family_may_spend_the_reserve() {
        let reserved: Vec<_> = Payload::ALL
            .into_iter()
            .filter(|p| p.is_dispute())
            .collect();
        assert_eq!(
            reserved,
            vec![
                Payload::DisputeOpen,
                Payload::DisputeMove,
                Payload::DisputeClaimTerminal,
                Payload::DisputeRefute,
                Payload::DisputeFinalize,
            ]
        );
        assert!(!Payload::Transfer.is_dispute());
        // The cooperative close is not under a deadline, so delaying it
        // robs nobody and it does not get privileged blockspace.
        assert!(!Payload::CloseGame.is_dispute());
    }

    #[test]
    fn every_signed_field_changes_the_signing_bytes() {
        let base = tx(Payload::Transfer);
        let mut seen = vec![base.signing_bytes()];
        let mut v = base.clone();
        v.version ^= 1;
        seen.push(v.signing_bytes());
        let mut v = base.clone();
        v.nonce ^= 1;
        seen.push(v.signing_bytes());
        let mut v = base.clone();
        v.sender[0] ^= 1;
        seen.push(v.signing_bytes());
        let mut v = base.clone();
        v.fee ^= 1;
        seen.push(v.signing_bytes());
        let mut v = base.clone();
        v.gas_limit ^= 1;
        seen.push(v.signing_bytes());
        let mut v = base.clone();
        v.payload = Payload::OpenGame;
        seen.push(v.signing_bytes());
        let mut v = base.clone();
        v.body.push(0);
        seen.push(v.signing_bytes());
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), 8, "a field is not covered by the signature");
    }

    /// The signature is outside the signed bytes and inside the id. Both
    /// directions matter: the first because it must be, the second because
    /// a block commits to the transactions it actually contains.
    #[test]
    fn the_signature_is_outside_the_signed_bytes_and_inside_the_id() {
        let a = tx(Payload::Transfer);
        let mut b = a.clone();
        b.signature[0] ^= 1;
        assert_eq!(a.signing_bytes(), b.signing_bytes());
        assert_ne!(a.hash(), b.hash());
    }
}
