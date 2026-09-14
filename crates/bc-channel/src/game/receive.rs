//! Verifying what the opponent sent.
//!
//! Six checks, in the order `spec/04-channel.md` lists them: the chain links,
//! the move is legal, the position hash is what the move produces, the clock
//! debit is plausible, any terminal claim is justified, and the signature is
//! theirs. A failure at any of them means stop signing — which is the same
//! response as the opponent going silent, and needs no new machinery.

use super::{Channel, ChannelError};
use crate::clock::plausible;
use crate::msg::{MoveMsg, Signed};
use crate::rules::{clock_of, justified, loser_is, NO_MOVE};
use crate::state::{pos_hash, rep_hash, GameState};
use bc_chess::Color;

impl Channel {
    /// Verify and accept the opponent's move.
    ///
    /// `observed_elapsed_ms` is how long the receiver waited. It only ever
    /// makes the clock check stricter; pass 0 to skip it.
    pub fn receive(&mut self, msg: &MoveMsg, observed_elapsed_ms: u32) -> Result<(), ChannelError> {
        if self.is_over() {
            return Err(ChannelError::GameOver);
        }
        let them = self.me.flip();
        if self.position.side != them {
            return Err(ChannelError::NotYourTurn);
        }
        self.absorb_countersig(msg, them)?;

        // The new state is the next link in *this* chain.
        let s = msg.state;
        if s.channel_id != self.channel_id {
            return Err(ChannelError::WrongChannel);
        }
        if s.ply != self.head.state.ply + 1 {
            return Err(ChannelError::WrongPly);
        }
        if s.ply > self.offer.terms.max_plies {
            return Err(ChannelError::PlyLimit);
        }
        if s.prev_hash != self.head.state.hash() {
            return Err(ChannelError::BrokenChain);
        }

        // A conceded flag moves no piece; anything else must be a legal move
        // producing exactly the position hash claimed.
        let conceding = s.mv == NO_MOVE && s.status == loser_is(them);
        let after = if conceding {
            if s.pos_hash != self.head.state.pos_hash {
                return Err(ChannelError::WrongPosition);
            }
            self.position
        } else {
            if !self.position.is_move_legal(msg.mv) || msg.mv.0 != s.mv {
                return Err(ChannelError::IllegalMove);
            }
            let after = self.position.make_move(msg.mv);
            if s.pos_hash != pos_hash(&after) {
                return Err(ChannelError::WrongPosition);
            }
            after
        };

        self.check_clocks(&s, them, observed_elapsed_ms, conceding)?;
        if !justified(s.status, &after, self.repetitions_of(&after), them) {
            return Err(ChannelError::UnjustifiedStatus);
        }
        if !self.key_of(them).verify(&s.hash(), &msg.sig) {
            return Err(ChannelError::BadSignature);
        }

        let mut signed = Signed::new(s);
        signed.put(them, msg.sig);
        self.head = signed;
        self.position = after;
        if !conceding {
            self.rep_hashes.push(rep_hash(&after));
        }
        Ok(())
    }

    fn absorb_countersig(&mut self, msg: &MoveMsg, them: Color) -> Result<(), ChannelError> {
        if self.head.sig(them).is_some() || self.head.state.ply == 0 {
            return Ok(());
        }
        let cs = msg.countersig.ok_or(ChannelError::MissingCountersig)?;
        if !self.key_of(them).verify(&self.head.state.hash(), &cs) {
            return Err(ChannelError::BadSignature);
        }
        self.head.put(them, cs);
        self.certified = self.head;
        self.certified_pos = self.position;
        Ok(())
    }

    fn check_clocks(
        &self,
        s: &GameState,
        them: Color,
        observed_ms: u32,
        conceding: bool,
    ) -> Result<(), ChannelError> {
        if clock_of(s, self.me) != self.clock(self.me) {
            return Err(ChannelError::OpponentClockMoved);
        }
        if conceding {
            // A conceded flag asserts an empty clock and nothing else.
            return match clock_of(s, them) {
                0 => Ok(()),
                _ => Err(ChannelError::ImplausibleClock),
            };
        }
        if plausible(
            self.clock(them),
            clock_of(s, them),
            observed_ms,
            self.offer.terms.increment_ms,
            self.grace_ms,
        ) {
            Ok(())
        } else {
            Err(ChannelError::ImplausibleClock)
        }
    }
}
