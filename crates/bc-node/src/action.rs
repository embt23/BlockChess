//! Transaction bodies for the dispute family, on the wire.
//!
//! `bc-block` keeps `Tx::body` opaque, deliberately: a miner orders
//! transactions and must not need the chess rules to do it. Somebody has to
//! decode them eventually, and that somebody is a node — this crate.
//!
//! Only the payloads the reorg demonstration needs are implemented, and
//! that is stated rather than hidden. `Transfer`, `RegisterServer` and
//! `SlashServer` decode to [`Action::Unsupported`] and are skipped by the
//! executor.

use bc_adjudicator::dispute::ClaimKind;
use bc_adjudicator::state::{GameState, STATE_LEN};
use bc_block::{Payload, Tx};
use bc_channel::msg::Signed;
use bc_channel::offer::{GameOffer, OFFER_LEN};
use bc_chess::{Color, Move};
use bc_hash::Hash;
use bc_sig::Signature;

/// A decoded transaction body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    OpenGame {
        offer: Box<GameOffer>,
        white_sig: Signature,
        black_sig: Signature,
    },
    DisputeOpen {
        initiator: Color,
        signed: Box<Signed>,
        packed_pos: Vec<u8>,
    },
    DisputeMove {
        channel_id: Hash,
        mover: Color,
        mv: Move,
        claim: Option<ClaimKind>,
    },
    DisputeFinalize {
        channel_id: Hash,
    },
    /// A payload this node does not execute. Not an error: a node that
    /// halted on an unknown payload would be a node that cannot be
    /// upgraded.
    Unsupported,
}

fn colour(b: u8) -> Option<Color> {
    match b {
        0 => Some(Color::White),
        1 => Some(Color::Black),
        _ => None,
    }
}

fn claim(b: u8) -> Option<Option<ClaimKind>> {
    Some(match b {
        0 => None,
        1 => Some(ClaimKind::Checkmate),
        2 => Some(ClaimKind::Stalemate),
        3 => Some(ClaimKind::FiftyMove),
        4 => Some(ClaimKind::InsufficientMaterial),
        5 => Some(ClaimKind::Threefold),
        6 => Some(ClaimKind::Resign),
        7 => Some(ClaimKind::DrawAgreed),
        _ => return None,
    })
}

fn claim_tag(c: Option<ClaimKind>) -> u8 {
    match c {
        None => 0,
        Some(ClaimKind::Checkmate) => 1,
        Some(ClaimKind::Stalemate) => 2,
        Some(ClaimKind::FiftyMove) => 3,
        Some(ClaimKind::InsufficientMaterial) => 4,
        Some(ClaimKind::Threefold) => 5,
        Some(ClaimKind::Resign) => 6,
        Some(ClaimKind::DrawAgreed) => 7,
    }
}

impl Action {
    pub fn payload(&self) -> Payload {
        match self {
            Action::OpenGame { .. } => Payload::OpenGame,
            Action::DisputeOpen { .. } => Payload::DisputeOpen,
            Action::DisputeMove { .. } => Payload::DisputeMove,
            Action::DisputeFinalize { .. } => Payload::DisputeFinalize,
            Action::Unsupported => Payload::Transfer,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        match self {
            Action::OpenGame {
                offer,
                white_sig,
                black_sig,
            } => {
                let mut b = offer.encode().to_vec();
                b.extend_from_slice(&white_sig.0);
                b.extend_from_slice(&black_sig.0);
                b
            }
            Action::DisputeOpen {
                initiator,
                signed,
                packed_pos,
            } => {
                let mut b = vec![*initiator as u8];
                b.extend_from_slice(&signed.state.encode());
                for sig in [signed.white_sig, signed.black_sig] {
                    b.push(u8::from(sig.is_some()));
                    b.extend_from_slice(&sig.unwrap_or(Signature([0u8; 64])).0);
                }
                b.push(packed_pos.len() as u8);
                b.extend_from_slice(packed_pos);
                b
            }
            Action::DisputeMove {
                channel_id,
                mover,
                mv,
                claim,
            } => {
                let mut b = channel_id.to_vec();
                b.push(*mover as u8);
                b.extend_from_slice(&mv.0.to_le_bytes());
                b.push(claim_tag(*claim));
                b
            }
            Action::DisputeFinalize { channel_id } => channel_id.to_vec(),
            Action::Unsupported => Vec::new(),
        }
    }

    pub fn decode(payload: Payload, b: &[u8]) -> Option<Action> {
        match payload {
            Payload::OpenGame => {
                if b.len() != OFFER_LEN + 128 {
                    return None;
                }
                let mut w = [0u8; 64];
                let mut k = [0u8; 64];
                w.copy_from_slice(&b[OFFER_LEN..OFFER_LEN + 64]);
                k.copy_from_slice(&b[OFFER_LEN + 64..]);
                Some(Action::OpenGame {
                    offer: Box::new(GameOffer::decode(&b[..OFFER_LEN])?),
                    white_sig: Signature(w),
                    black_sig: Signature(k),
                })
            }
            Payload::DisputeOpen => {
                let head = 1 + STATE_LEN + 2 * 65;
                if b.len() < head + 1 {
                    return None;
                }
                let state = GameState::decode(&b[1..1 + STATE_LEN])?;
                let mut sigs = [None, None];
                for (i, slot) in sigs.iter_mut().enumerate() {
                    let o = 1 + STATE_LEN + i * 65;
                    if b[o] == 1 {
                        let mut s = [0u8; 64];
                        s.copy_from_slice(&b[o + 1..o + 65]);
                        *slot = Some(Signature(s));
                    }
                }
                let len = b[head] as usize;
                if b.len() != head + 1 + len {
                    return None;
                }
                Some(Action::DisputeOpen {
                    initiator: colour(b[0])?,
                    signed: Box::new(Signed {
                        state,
                        white_sig: sigs[0],
                        black_sig: sigs[1],
                    }),
                    packed_pos: b[head + 1..].to_vec(),
                })
            }
            Payload::DisputeMove => {
                if b.len() != 36 {
                    return None;
                }
                let mut id = [0u8; 32];
                id.copy_from_slice(&b[..32]);
                Some(Action::DisputeMove {
                    channel_id: id,
                    mover: colour(b[32])?,
                    mv: Move(u16::from_le_bytes([b[33], b[34]])),
                    claim: claim(b[35])?,
                })
            }
            Payload::DisputeFinalize => {
                if b.len() != 32 {
                    return None;
                }
                let mut id = [0u8; 32];
                id.copy_from_slice(b);
                Some(Action::DisputeFinalize { channel_id: id })
            }
            _ => Some(Action::Unsupported),
        }
    }

    /// Wrap this action in a transaction. The signature field is left
    /// empty: authority here comes from the signatures **inside** the
    /// body, which is `P6` — whoever submits a transaction is irrelevant
    /// to it, and a server is a relay rather than a custodian.
    pub fn into_tx(self, nonce: u64, sender: [u8; 32]) -> Tx {
        Tx {
            version: 1,
            nonce,
            sender,
            fee: 1,
            gas_limit: 50_000,
            payload: self.payload(),
            body: self.encode(),
            signature: [0u8; 64],
        }
    }
}
