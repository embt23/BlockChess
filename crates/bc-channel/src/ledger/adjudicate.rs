//! Driving a dispute from the chain's side: who may do what, and with what
//! evidence.
//!
//! Everything here is the boring half — signature checks, escrow lookups, and
//! turning a verdict into a payout. The interesting half is in
//! [`crate::dispute`]. The split is deliberate: this file knows about money
//! and keys, that one knows about chess and deadlines, and neither needs the
//! other's vocabulary.

use super::{Escrow, Ledger, LedgerError, Payout};
use crate::dispute::{ClaimKind, Dispute, Refutation};
use crate::msg::{draw_bytes, resign_bytes, Signed};
use crate::offer::GameOffer;
use crate::state::GameState;
use crate::state::{rep_hash, Status};
use bc_chess::{unpack, Color, Move, Position};
use bc_hash::Hash;
use bc_sig::{Signature, VerifyingKey};

/// Three countersigned occurrences of one position.
///
/// The evidence for threefold repetition is not a history — it is three
/// signatures your opponent already gave you. That is what turns an O(n) scan
/// of the game into three constant-time checks, and it is the clearest case
/// of the general principle: **a signature from your adversary is the
/// cheapest evidence that exists.** Structure the protocol so that what you
/// will need to prove later is something they had to sign earlier.
pub struct RepetitionProof<'a> {
    /// Each occurrence: the certified state, and the packed position it names.
    pub occurrences: [(Signed, &'a [u8]); 3],
}

/// What a claim needs behind it. The board-checkable kinds need nothing.
///
/// The repetition proof is passed by reference: three certified states is
/// the best part of a kilobyte, while every other variant is one or two
/// signatures, and an enum is as large as its largest arm.
pub enum Evidence<'a> {
    None,
    Resign(Signature),
    DrawAgreed { white: Signature, black: Signature },
    Threefold(&'a RepetitionProof<'a>),
}

impl Ledger {
    fn live_escrow(&self, id: &Hash) -> Result<&Escrow, LedgerError> {
        let e = self.channels.get(id).ok_or(LedgerError::UnknownChannel)?;
        if e.settled {
            return Err(LedgerError::AlreadySettled);
        }
        Ok(e)
    }

    pub fn dispute(&self, id: &Hash) -> Option<&Dispute> {
        self.disputes.get(id)
    }

    pub fn in_dispute(&self, id: &Hash) -> bool {
        self.disputes.contains_key(id)
    }

    /// Open a dispute, or supersede an existing one with a higher state.
    ///
    /// The state must be signed by the initiator's **opponent**. A state you
    /// signed yourself proves nothing, and this one line is what makes the
    /// whole mechanism work: because countersignatures ride along with moves,
    /// the player who is waiting always holds one.
    pub fn dispute_open(
        &mut self,
        initiator: Color,
        signed: &Signed,
        packed_pos: &[u8],
        height: u64,
    ) -> Result<(), LedgerError> {
        let esc = self.live_escrow(&signed.state.channel_id)?;
        let offer = esc.offer;

        authorise(signed, initiator, &offer)?;
        let pos = position_matching(packed_pos, &signed.state.pos_hash)?;
        let id = signed.state.channel_id;
        match self.disputes.get_mut(&id) {
            Some(d) => d.supersede(&signed.state, pos, &offer.terms, initiator, height)?,
            None => {
                let d = Dispute::open(&signed.state, pos, &offer.terms, initiator, height)?;
                self.disputes.insert(id, d);
            }
        }
        Ok(())
    }

    /// Play a move on-chain. Settles immediately if the game ends on the board.
    pub fn dispute_move(
        &mut self,
        id: &Hash,
        mover: Color,
        mv: Move,
        height: u64,
    ) -> Result<Option<Payout>, LedgerError> {
        self.live_escrow(id)?;
        let d = self.disputes.get_mut(id).ok_or(LedgerError::NotInDispute)?;
        match d.apply_move(mover, mv, height)? {
            crate::dispute::MoveOutcome::Continues => Ok(None),
            crate::dispute::MoveOutcome::Ended(status) => self.conclude(id, status).map(Some),
        }
    }

    /// Claim the game is over. `Ok(None)` means a refutation window opened.
    pub fn dispute_claim(
        &mut self,
        id: &Hash,
        claimant: Color,
        kind: ClaimKind,
        evidence: Evidence<'_>,
        height: u64,
    ) -> Result<Option<Payout>, LedgerError> {
        let offer = self.live_escrow(id)?.offer;
        let ply = self.disputes.get(id).ok_or(LedgerError::NotInDispute)?.ply;
        verify_evidence(
            kind,
            &evidence,
            id,
            ply,
            claimant,
            &offer.white_pk,
            &offer.black_pk,
        )?;

        let d = self.disputes.get_mut(id).ok_or(LedgerError::NotInDispute)?;
        match d.claim_terminal(kind, claimant, height)? {
            Some(status) => self.conclude(id, status).map(Some),
            None => Ok(None),
        }
    }

    /// Answer an optimistic claim with one legal move.
    pub fn dispute_refute(
        &mut self,
        id: &Hash,
        by: Color,
        mv: Move,
        height: u64,
    ) -> Result<Refutation, LedgerError> {
        self.live_escrow(id)?;
        let d = self.disputes.get_mut(id).ok_or(LedgerError::NotInDispute)?;
        Ok(d.refute(by, mv, height)?)
    }

    /// Settle a dispute whose deadline or refutation window has passed.
    /// Callable by anyone: it only reports what the heights already imply.
    pub fn dispute_finalize(&mut self, id: &Hash, height: u64) -> Result<Payout, LedgerError> {
        self.live_escrow(id)?;
        let status = self
            .disputes
            .get(id)
            .ok_or(LedgerError::NotInDispute)?
            .verdict(height)?;
        self.conclude(id, status)
    }

    fn conclude(&mut self, id: &Hash, status: Status) -> Result<Payout, LedgerError> {
        self.disputes.remove(id);
        self.settle(*id, status)
    }
}

/// May `initiator` open a dispute on this state?
///
/// Normally: only if their **opponent** signed it. A state you signed
/// yourself proves nothing.
///
/// Ply 0 is the exception, and missing it would have locked people's money up
/// forever. Nobody has signed a *state* at ply 0 — the opening position is
/// fixed by the `OpenGame` transaction, which already carries both players'
/// signatures over an offer committing to `start_pos_hash` and the base
/// clock. So the opening state needs no signature of its own; it needs only
/// to *be* the opening state, which the escrow can check for itself. Without
/// this, a player who vanishes before making their first move leaves both
/// stakes escrowed with nobody able to reclaim them, ever.
fn authorise(signed: &Signed, initiator: Color, offer: &GameOffer) -> Result<(), LedgerError> {
    if signed.state.ply == 0 {
        let genesis = GameState::genesis(
            offer.channel_id(),
            offer.terms.start_pos_hash,
            offer.terms.base_time_ms,
        );
        return (signed.state == genesis)
            .then_some(())
            .ok_or(LedgerError::BadPosition);
    }
    let opponent = initiator.flip();
    let their_key = match opponent {
        Color::White => offer.white_pk,
        Color::Black => offer.black_pk,
    };
    let sig = signed.sig(opponent).ok_or(LedgerError::BadSignature)?;
    their_key
        .verify(&signed.state.hash(), &sig)
        .then_some(())
        .ok_or(LedgerError::BadSignature)
}

/// Unpack a claimed position and check it is the one the state names.
fn position_matching(packed: &[u8], pos_hash: &Hash) -> Result<Position, LedgerError> {
    let pos = unpack(packed).map_err(|_| LedgerError::BadPosition)?;
    if crate::state::pos_hash(&pos) != *pos_hash {
        return Err(LedgerError::BadPosition);
    }
    Ok(pos)
}

fn verify_evidence(
    kind: ClaimKind,
    evidence: &Evidence<'_>,
    id: &Hash,
    ply: u16,
    claimant: Color,
    white_pk: &VerifyingKey,
    black_pk: &VerifyingKey,
) -> Result<(), LedgerError> {
    match (kind, evidence) {
        // Board-checkable kinds carry no evidence; the dispute checks them
        // against the position it already holds.
        (ClaimKind::FiftyMove, _)
        | (ClaimKind::InsufficientMaterial, _)
        | (ClaimKind::Checkmate, _)
        | (ClaimKind::Stalemate, _) => Ok(()),

        (ClaimKind::Resign, Evidence::Resign(sig)) => {
            // The claimant is the one giving up, so it is their signature.
            let key = match claimant {
                Color::White => white_pk,
                Color::Black => black_pk,
            };
            key.verify(&resign_bytes(id, ply), sig)
                .then_some(())
                .ok_or(LedgerError::BadSignature)
        }

        (ClaimKind::DrawAgreed, Evidence::DrawAgreed { white, black }) => {
            let msg = draw_bytes(id, ply);
            (white_pk.verify(&msg, white) && black_pk.verify(&msg, black))
                .then_some(())
                .ok_or(LedgerError::BadSignature)
        }

        (ClaimKind::Threefold, Evidence::Threefold(proof)) => {
            verify_repetition(proof, id, white_pk, black_pk)
        }

        _ => Err(LedgerError::BadSignature),
    }
}

/// Three certified states, at distinct plies, whose positions share a
/// `rep_hash`. No move is replayed and no history is walked.
fn verify_repetition(
    proof: &RepetitionProof<'_>,
    id: &Hash,
    white_pk: &VerifyingKey,
    black_pk: &VerifyingKey,
) -> Result<(), LedgerError> {
    let mut keys = [Hash::default(); 3];
    let mut plies = [0u16; 3];

    for (i, (signed, packed)) in proof.occurrences.iter().enumerate() {
        if signed.state.channel_id != *id {
            return Err(LedgerError::WrongChannel);
        }
        // Certified, not merely signed: one signature is a claim, and a claim
        // about your own past positions is worth nothing.
        if !signed.verify_certified(white_pk, black_pk) {
            return Err(LedgerError::BadSignature);
        }
        let pos = position_matching(packed, &signed.state.pos_hash)?;
        keys[i] = rep_hash(&pos);
        plies[i] = signed.state.ply;
    }

    if keys[0] != keys[1] || keys[1] != keys[2] {
        return Err(LedgerError::BadPosition);
    }
    // Distinct plies, or one state counts three times.
    if plies[0] == plies[1] || plies[1] == plies[2] || plies[0] == plies[2] {
        return Err(LedgerError::BadPosition);
    }
    Ok(())
}
