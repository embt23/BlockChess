//! A stub ledger: the smallest thing that can hold an escrow.
//!
//! Episode 07 does not need consensus. It needs *something* that can take two
//! stakes, hold them, and pay them out against two signatures — and that is
//! twenty lines of `BTreeMap`. Building the channel against this rather than
//! against a chain gets to a real wagered game in weeks instead of months, and
//! episodes 05–06 slide underneath later without the channel noticing.
//!
//! What this is **not**: it has no blocks, no consensus, no censorship
//! resistance, and no adjudicator. In particular it only settles games both
//! players agree about. The uncooperative path — the one that makes this a
//! trustless system rather than a convenient one — is episode 08.
//!
//! Two properties here are not stubs and must survive the real chain:
//!
//! - **Conservation.** Every settlement pays out exactly the pot. No path
//!   mints (`E1`), and [`Ledger::total_supply`] is asserted constant in the
//!   tests.
//! - **Non-custody.** [`Ledger::open_game`] moves money only against the
//!   players' own signatures. Whoever *submits* the transaction is irrelevant
//!   to it; a server is a relay, not a custodian (`P6`).
//!
//! The adjudicator hangs off this type in [`adjudicate`] — episode 08, and the
//! reason a stub was enough to build the channel against.

pub mod adjudicate;

use crate::dispute::Dispute;
use crate::msg::{draw_bytes, resign_bytes, Signed};
use crate::offer::GameOffer;
use crate::state::Status;
use bc_hash::Hash;
use bc_sig::{Signature, VerifyingKey};
use std::collections::BTreeMap;

pub type Account = [u8; 32];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerError {
    BadOffer,
    NotInDispute,
    AlreadyInDispute,
    BadPosition,
    Dispute(crate::dispute::DisputeError),
    Expired,
    InsufficientFunds,
    DuplicateChannel,
    UnknownChannel,
    AlreadySettled,
    NotTerminal,
    BadSignature,
    WrongChannel,
}

impl std::fmt::Display for LedgerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for LedgerError {}

/// Who got what, for the caller to check and for the log to show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Payout {
    pub white: u128,
    pub black: u128,
    pub rake: u128,
}

struct Escrow {
    offer: GameOffer,
    settled: bool,
}

impl From<crate::dispute::DisputeError> for LedgerError {
    fn from(e: crate::dispute::DisputeError) -> LedgerError {
        LedgerError::Dispute(e)
    }
}

pub struct Ledger {
    balances: BTreeMap<Account, u128>,
    channels: BTreeMap<Hash, Escrow>,
    /// Channels currently being played out on-chain.
    disputes: BTreeMap<Hash, Dispute>,
    pub height: u64,
}

impl Default for Ledger {
    fn default() -> Self {
        Self::new()
    }
}

impl Ledger {
    pub fn new() -> Ledger {
        Ledger {
            balances: BTreeMap::new(),
            channels: BTreeMap::new(),
            disputes: BTreeMap::new(),
            height: 0,
        }
    }

    /// Put money in an account. The only path that creates value, and it
    /// exists so tests have somewhere to start.
    pub fn credit(&mut self, who: &VerifyingKey, amount: u128) {
        *self.balances.entry(who.0).or_insert(0) += amount;
    }

    pub fn balance(&self, who: &VerifyingKey) -> u128 {
        *self.balances.get(&who.0).unwrap_or(&0)
    }

    /// Everything the ledger holds, escrows included. Constant across every
    /// operation but [`Ledger::credit`].
    pub fn total_supply(&self) -> u128 {
        let held: u128 = self
            .channels
            .values()
            .filter(|e| !e.settled)
            .map(|e| e.offer.pot())
            .sum();
        self.balances.values().sum::<u128>() + held
    }

    /// Lock both stakes. Submitted by anyone; authorised by the players.
    pub fn open_game(
        &mut self,
        offer: &GameOffer,
        white_sig: &Signature,
        black_sig: &Signature,
    ) -> Result<Hash, LedgerError> {
        if !offer.valid() {
            return Err(LedgerError::BadOffer);
        }
        if offer.expiry_block < self.height {
            return Err(LedgerError::Expired);
        }
        if !offer.verify_acceptance(white_sig, black_sig) {
            return Err(LedgerError::BadSignature);
        }
        let id = offer.channel_id();
        if self.channels.contains_key(&id) {
            return Err(LedgerError::DuplicateChannel);
        }
        if self.balance(&offer.white_pk) < offer.stake_white
            || self.balance(&offer.black_pk) < offer.stake_black
        {
            return Err(LedgerError::InsufficientFunds);
        }
        *self.balances.get_mut(&offer.white_pk.0).unwrap() -= offer.stake_white;
        *self.balances.get_mut(&offer.black_pk.0).unwrap() -= offer.stake_black;
        self.channels.insert(
            id,
            Escrow {
                offer: *offer,
                settled: false,
            },
        );
        Ok(id)
    }

    /// Settle on a certified terminal state. The chain verifies two
    /// signatures and reads one byte; **it verifies no chess at all**.
    ///
    /// Two people who agree about the result are allowed to be wrong about
    /// it. That is their money, and it is why the expensive on-chain engine
    /// almost never runs.
    pub fn close_game(&mut self, signed: &Signed) -> Result<Payout, LedgerError> {
        let esc = self
            .channels
            .get(&signed.state.channel_id)
            .ok_or(LedgerError::UnknownChannel)?;
        if esc.settled {
            return Err(LedgerError::AlreadySettled);
        }
        if !signed.state.status.is_terminal() {
            return Err(LedgerError::NotTerminal);
        }
        if !signed.verify_certified(&esc.offer.white_pk, &esc.offer.black_pk) {
            return Err(LedgerError::BadSignature);
        }
        self.settle(signed.state.channel_id, signed.state.status)
    }

    /// Settle on a resignation: one signature over `("BC/resign/v1", id, ply)`.
    pub fn close_by_resignation(
        &mut self,
        channel_id: &Hash,
        ply: u16,
        loser: &VerifyingKey,
        sig: &Signature,
    ) -> Result<Payout, LedgerError> {
        let esc = self
            .channels
            .get(channel_id)
            .ok_or(LedgerError::UnknownChannel)?;
        if esc.settled {
            return Err(LedgerError::AlreadySettled);
        }
        let status = if *loser == esc.offer.white_pk {
            Status::BlackWins
        } else if *loser == esc.offer.black_pk {
            Status::WhiteWins
        } else {
            return Err(LedgerError::WrongChannel);
        };
        if !loser.verify(&resign_bytes(channel_id, ply), sig) {
            return Err(LedgerError::BadSignature);
        }
        self.settle(*channel_id, status)
    }

    /// Settle on a draw agreement: two signatures over the same bytes.
    pub fn close_by_draw(
        &mut self,
        channel_id: &Hash,
        ply: u16,
        white_sig: &Signature,
        black_sig: &Signature,
    ) -> Result<Payout, LedgerError> {
        let esc = self
            .channels
            .get(channel_id)
            .ok_or(LedgerError::UnknownChannel)?;
        if esc.settled {
            return Err(LedgerError::AlreadySettled);
        }
        let msg = draw_bytes(channel_id, ply);
        if !esc.offer.white_pk.verify(&msg, white_sig)
            || !esc.offer.black_pk.verify(&msg, black_sig)
        {
            return Err(LedgerError::BadSignature);
        }
        self.settle(*channel_id, Status::Draw)
    }

    /// Pay out a pot. Exact: rake first, then the remainder in whole units,
    /// with the division arranged so the two shares sum to it and nothing is
    /// lost to rounding.
    fn settle(&mut self, id: Hash, status: Status) -> Result<Payout, LedgerError> {
        let esc = self
            .channels
            .get_mut(&id)
            .ok_or(LedgerError::UnknownChannel)?;
        let offer = esc.offer;
        esc.settled = true;

        let pot = offer.pot();
        let rake = pot * offer.rake_bps as u128 / 10_000;
        let rest = pot - rake;
        let (white, black) = match status {
            Status::WhiteWins => (rest, 0),
            Status::BlackWins => (0, rest),
            Status::Draw => {
                let w = rest * offer.stake_white / pot;
                (w, rest - w)
            }
            Status::Ongoing => return Err(LedgerError::NotTerminal),
        };

        *self.balances.entry(offer.white_pk.0).or_insert(0) += white;
        *self.balances.entry(offer.black_pk.0).or_insert(0) += black;
        if rake > 0 {
            *self.balances.entry(offer.server_pk.0).or_insert(0) += rake;
        }
        debug_assert_eq!(white + black + rake, pot, "settlement must conserve");
        Ok(Payout { white, black, rake })
    }
}
