//! Episode 07 — the game channel.
//!
//! The attack this defeats: *"eighty transactions per game is absurd."*
//!
//! Two players exchange moves directly and sign every position. The chain
//! sees two transactions for a whole game: the moment money is locked, and
//! the moment it is paid out. It is a court, not a referee (`P1`).
//!
//! ## Why this is easier than a payment channel
//!
//! Lightning needs revocation secrets, penalty transactions and watchtowers
//! that hold those secrets, because state *n−1* and state *n* differ in the
//! split of funds and either party might prefer the older split.
//!
//! Here, payout is a function of `status` alone, and `status` is `Ongoing` for
//! every non-terminal state. **Non-terminal states pay nobody.** Posting an
//! old one cannot move money; it only pays gas and invites the opponent to
//! post a newer one. So `higher ply wins` (`P2`) replaces the entire
//! revocation apparatus, and a watchtower here holds no secrets and cannot
//! steal — which is why it can be a stranger's cron job.
//!
//! ## The layout
//!
//! | Module | What it is |
//! |---|---|
//! | [`state`] | the 109-byte object both players sign, and its hash chain |
//! | [`offer`] | terms, the offer, and the channel id they derive |
//! | [`msg`] | what goes on the wire; half-signed versus certified |
//! | [`clock`] | Fischer accounting, and enforcement by refusal to countersign |
//! | [`rules`] | which terminal claims a receiver should believe |
//! | [`game`] | one player's state machine |
//! | [`ledger`] | a stub escrow, standing in for episodes 05–06 |
//!
//! ## What is not here yet
//!
//! The uncooperative path. Everything in this crate assumes both players keep
//! answering; when one stops, the honest player currently has a certified
//! state and nowhere to take it. Giving it somewhere to go is episode 08, and
//! it is the difference between a convenient system and a trustless one.

pub mod clock;
pub mod game;
pub mod ledger;
pub mod msg;
pub mod offer;
pub mod rules;
pub mod state;

pub use game::{Channel, ChannelError};
pub use ledger::{Ledger, LedgerError, Payout};
pub use msg::{MoveMsg, Signed};
pub use offer::{GameOffer, GameTerms};
pub use state::{pos_hash, rep_hash, GameState, Status};
