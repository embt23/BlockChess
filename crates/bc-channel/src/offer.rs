//! Opening a channel: the terms, the offer, and the identifier they derive.
//!
//! The structural point of this module is in one line of [`Ledger::open_game`]
//! elsewhere: the transaction is *assembled* by whoever is submitting —
//! usually a matchmaking server — but the funds move under the **players'**
//! signatures. A server that vanishes with the transaction unsubmitted costs
//! its users a wasted offer; it cannot cost them a stake. That removes the
//! dominant failure mode of real money-gaming platforms structurally rather
//! than by promising to behave.

use bc_hash::{tagged, tagged_parts, Hash};
use bc_sig::{Signature, VerifyingKey};

pub const TERMS_LEN: usize = 53;
pub const OFFER_LEN: usize = 32 + 32 + 16 + 16 + TERMS_LEN + 32 + 32 + 2 + 8;

/// Largest rake a server may take, in basis points.
pub const MAX_RAKE_BPS: u16 = 500;

/// Smallest challenge window the protocol will accept, in blocks. Below this
/// a chain hiccup starts costing people games (`P4`).
pub const MIN_DELTA_BLOCKS: u32 = 64;

/// Longest game the channel will carry. 600 plies is well past any real game
/// and bounds the worst-case dispute replay.
pub const MAX_PLIES_LIMIT: u16 = 600;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameTerms {
    pub start_pos_hash: Hash,
    pub base_time_ms: u32,
    pub increment_ms: u32,
    pub max_plies: u16,
    /// Δ, the challenge window, in **blocks**. Never seconds (`P4`).
    pub delta_blocks: u32,
    /// Clock dilation constant — see `spec/05-adjudication.md`.
    pub budget_tau_ms: u32,
    pub rules_mask: u8,
    pub adjudicator_ver: u16,
}

impl GameTerms {
    pub fn encode(&self) -> [u8; TERMS_LEN] {
        let mut b = [0u8; TERMS_LEN];
        b[0..32].copy_from_slice(&self.start_pos_hash);
        b[32..36].copy_from_slice(&self.base_time_ms.to_le_bytes());
        b[36..40].copy_from_slice(&self.increment_ms.to_le_bytes());
        b[40..42].copy_from_slice(&self.max_plies.to_le_bytes());
        b[42..46].copy_from_slice(&self.delta_blocks.to_le_bytes());
        b[46..50].copy_from_slice(&self.budget_tau_ms.to_le_bytes());
        b[50] = self.rules_mask;
        b[51..53].copy_from_slice(&self.adjudicator_ver.to_le_bytes());
        b
    }

    pub fn valid(&self) -> bool {
        self.delta_blocks >= MIN_DELTA_BLOCKS
            && self.max_plies > 0
            && self.max_plies <= MAX_PLIES_LIMIT
            && self.base_time_ms > 0
    }
}

/// What both players sign before a channel exists.
///
/// `open_nonce` is not in `spec/04`'s field list but is in its `channel_id`
/// derivation, so it lives here: without it the same two players agreeing the
/// same terms for the same stakes twice would derive the same channel, and the
/// second game's states would be valid evidence in the first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameOffer {
    pub white_pk: VerifyingKey,
    pub black_pk: VerifyingKey,
    pub stake_white: u128,
    pub stake_black: u128,
    pub terms: GameTerms,
    /// Zero if there is no server.
    pub server_pk: VerifyingKey,
    pub open_nonce: [u8; 32],
    pub rake_bps: u16,
    /// The offer is void after this height, so a server cannot sit on a signed
    /// offer and submit it when the odds have moved.
    pub expiry_block: u64,
}

impl GameOffer {
    pub fn encode(&self) -> [u8; OFFER_LEN] {
        let mut b = [0u8; OFFER_LEN];
        let mut n = 0;
        let mut put = |src: &[u8]| {
            b[n..n + src.len()].copy_from_slice(src);
            n += src.len();
        };
        put(self.white_pk.as_bytes());
        put(self.black_pk.as_bytes());
        put(&self.stake_white.to_le_bytes());
        put(&self.stake_black.to_le_bytes());
        put(&self.terms.encode());
        put(self.server_pk.as_bytes());
        put(&self.open_nonce);
        put(&self.rake_bps.to_le_bytes());
        put(&self.expiry_block.to_le_bytes());
        debug_assert_eq!(n, OFFER_LEN);
        b
    }

    /// The bytes a player signs to accept. Tagged `BC/offer/v1`: an offer
    /// signature must never be replayable as a signature over anything else.
    pub fn signing_bytes(&self) -> Hash {
        tagged("BC/offer/v1", &self.encode())
    }

    /// `spec/04`'s derivation, with each part length-prefixed rather than
    /// concatenated — see `tagged_parts`. A raw concatenation of two public
    /// keys and two stakes has boundaries an attacker can slide.
    pub fn channel_id(&self) -> Hash {
        tagged_parts(
            "BC/chanid/v1",
            &[
                self.white_pk.as_bytes(),
                self.black_pk.as_bytes(),
                &self.terms.encode(),
                &self.stake_white.to_le_bytes(),
                &self.stake_black.to_le_bytes(),
                &self.open_nonce,
            ],
        )
    }

    pub fn pot(&self) -> u128 {
        self.stake_white + self.stake_black
    }

    pub fn has_server(&self) -> bool {
        self.server_pk.0 != [0u8; 32]
    }

    /// Everything checkable about an offer without consulting a ledger.
    ///
    /// The last clause matters more than it looks: a rake with no server to
    /// pay it to is a burn, and a burn is a mint with the sign flipped. `E1`
    /// says the supply only ever moves sideways.
    pub fn valid(&self) -> bool {
        self.terms.valid()
            && self.rake_bps <= MAX_RAKE_BPS
            && self.white_pk != self.black_pk
            && self.pot() > 0
            && (self.has_server() || self.rake_bps == 0)
    }

    pub fn verify_acceptance(&self, white_sig: &Signature, black_sig: &Signature) -> bool {
        let msg = self.signing_bytes();
        self.white_pk.verify(&msg, white_sig) && self.black_pk.verify(&msg, black_sig)
    }
}
