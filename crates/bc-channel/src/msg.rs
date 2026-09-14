//! What travels between the two players, and what counts as evidence.
//!
//! A state signed by one player is a *claim*. A state signed by both is a
//! *certificate*, and only a certificate is usable against its other signer.
//! That distinction is the whole of the protocol's evidence model, and it is
//! the same rule `docs/duality.md` applies to the design itself.

use crate::state::GameState;
use bc_chess::{Color, Move};
use bc_hash::{tagged_parts, Hash};
use bc_sig::{Signature, VerifyingKey};

/// A state with whatever signatures exist on it so far.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Signed {
    pub state: GameState,
    pub white_sig: Option<Signature>,
    pub black_sig: Option<Signature>,
}

impl Signed {
    pub fn new(state: GameState) -> Signed {
        Signed {
            state,
            white_sig: None,
            black_sig: None,
        }
    }

    pub fn sig(&self, c: Color) -> Option<Signature> {
        match c {
            Color::White => self.white_sig,
            Color::Black => self.black_sig,
        }
    }

    pub fn put(&mut self, c: Color, s: Signature) {
        match c {
            Color::White => self.white_sig = Some(s),
            Color::Black => self.black_sig = Some(s),
        }
    }

    /// Signed by both. The unit of evidence.
    pub fn is_certified(&self) -> bool {
        self.white_sig.is_some() && self.black_sig.is_some()
    }

    /// Both signatures present *and* valid under these keys.
    pub fn verify_certified(&self, white_pk: &VerifyingKey, black_pk: &VerifyingKey) -> bool {
        let msg = self.state.hash();
        match (self.white_sig, self.black_sig) {
            (Some(w), Some(b)) => white_pk.verify(&msg, &w) && black_pk.verify(&msg, &b),
            _ => false,
        }
    }
}

/// The one message the happy path sends: a move, the state it produces, the
/// mover's signature on it, and the mover's countersignature on the state
/// before it.
///
/// Bundling the countersignature is what makes the protocol one round trip per
/// move *pair* rather than two. There is never a bare acknowledgement on the
/// wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveMsg {
    pub mv: Move,
    pub state: GameState,
    /// The mover's signature over `state`.
    pub sig: Signature,
    /// The mover's signature over the *previous* state, certifying it.
    /// Absent only for the first move of the game, which has no predecessor
    /// needing a countersignature.
    pub countersig: Option<Signature>,
}

/// Bytes for a resignation: `("BC/resign/v1", channel, ply)`.
///
/// Resignation is a record and not a state, which is why it costs the chain
/// one signature check. It does not advance the ply, so it cannot be confused
/// with a move and cannot break the monotonicity the adjudicator relies on.
pub fn resign_bytes(channel_id: &Hash, ply: u16) -> Hash {
    tagged_parts("BC/resign/v1", &[channel_id, &ply.to_le_bytes()])
}

/// Bytes for a draw agreement: `("BC/draw/v1", channel, ply)`. Two signatures
/// over this is a draw; one is an offer.
pub fn draw_bytes(channel_id: &Hash, ply: u16) -> Hash {
    tagged_parts("BC/draw/v1", &[channel_id, &ply.to_le_bytes()])
}
