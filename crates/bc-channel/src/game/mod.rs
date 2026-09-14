//! One player's view of a live channel.
//!
//! Two entry points carry the whole happy path: [`Channel::play`] for the
//! mover and [`Channel::receive`] for the opponent. Everything else is
//! bookkeeping around them.
//!
//! The asymmetry worth noticing is in what each player holds. After a move,
//! the player *not* to move holds a certified state; the player to move holds
//! a certified state one ply behind, plus their own half-signed one. That is
//! the right way round — the person who needs evidence is the person waiting.

use crate::clock::{debit, DEFAULT_GRACE_MS};
use crate::msg::{MoveMsg, Signed};
use crate::offer::GameOffer;
use crate::rules::{claimable_status, clock_of, loser_is, NO_MOVE};
use crate::state::{pos_hash, rep_hash, GameState, Status};
use bc_chess::{Color, Move, Position};
use bc_hash::Hash;
use bc_sig::{Signature, SigningKey, VerifyingKey};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelError {
    NotYourTurn,
    GameOver,
    IllegalMove,
    /// The move took longer than the mover had. Answer with
    /// [`Channel::concede_time`].
    Flagged,
    PlyLimit,
    WrongChannel,
    WrongPly,
    BrokenChain,
    WrongPosition,
    ImplausibleClock,
    OpponentClockMoved,
    UnjustifiedStatus,
    BadSignature,
    MissingCountersig,
}

impl std::fmt::Display for ChannelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ChannelError {}

mod receive;

pub struct Channel {
    pub offer: GameOffer,
    pub channel_id: Hash,
    pub me: Color,
    /// Clock under-claim tolerated before refusing to countersign.
    pub grace_ms: u32,
    sk: SigningKey,
    position: Position,
    head: Signed,
    certified: Signed,
    /// One repetition key per ply reached, oldest first. Threefold is a
    /// count over this, and the evidence for it is the opponent's own past
    /// signatures (`spec/03`).
    rep_hashes: Vec<Hash>,
}

impl Channel {
    /// Open a player's view of an agreed channel.
    ///
    /// The ply-0 state carries no signatures: it is fixed by the `OpenGame`
    /// transaction, which already holds both players' signatures over an offer
    /// committing to the start position and the base clock.
    pub fn open(offer: GameOffer, sk: SigningKey, me: Color, start: Position) -> Channel {
        let channel_id = offer.channel_id();
        let ph = pos_hash(&start);
        let genesis = Signed::new(GameState::genesis(channel_id, ph, offer.terms.base_time_ms));
        let rep = rep_hash(&start);
        Channel {
            offer,
            channel_id,
            me,
            grace_ms: DEFAULT_GRACE_MS,
            sk,
            position: start,
            head: genesis,
            certified: genesis,
            rep_hashes: vec![rep],
        }
    }

    pub fn position(&self) -> &Position {
        &self.position
    }
    /// The highest state held, certified or not.
    pub fn head(&self) -> &Signed {
        &self.head
    }
    /// The highest state signed by both. What you take to the chain.
    pub fn certified(&self) -> &Signed {
        &self.certified
    }
    pub fn ply(&self) -> u16 {
        self.head.state.ply
    }
    pub fn is_over(&self) -> bool {
        self.head.state.status.is_terminal()
    }
    pub fn clock(&self, c: Color) -> u32 {
        clock_of(&self.head.state, c)
    }

    /// How many times the current position has occurred. Three is a draw, and
    /// the proof is three countersigned states whose positions share a
    /// [`rep_hash`] — the opponent's own past signatures stand in for a
    /// history replay (`spec/03`).
    pub fn repetitions(&self) -> usize {
        let last = self.rep_hashes.last().expect("genesis is always present");
        self.rep_hashes.iter().filter(|h| *h == last).count()
    }

    /// Make a move: advance the position, debit my own clock, sign the new
    /// state, and countersign the opponent's previous one.
    pub fn play(&mut self, mv: Move, elapsed_ms: u32) -> Result<MoveMsg, ChannelError> {
        self.check_can_move()?;
        if !self.position.is_move_legal(mv) {
            return Err(ChannelError::IllegalMove);
        }
        let after = self.position.make_move(mv);
        let remaining = debit(
            self.clock(self.me),
            elapsed_ms,
            self.offer.terms.increment_ms,
        )
        .ok_or(ChannelError::Flagged)?;

        let status = claimable_status(&after, self.repetitions_of(&after));
        let next = self.build(mv.0, pos_hash(&after), remaining, status);
        Ok(self.commit(next, mv, after))
    }

    /// Concede on time, which is what an honest client does when its own flag
    /// falls. A dishonest one simply stops sending, and the opponent goes to
    /// chain — which is why this costs the protocol nothing to support.
    ///
    /// The state advances the ply with `mv = 0` and an unchanged position: a
    /// transition that is not a move. It is accepted only because it declares
    /// the sender the loser.
    pub fn concede_time(&mut self) -> Result<MoveMsg, ChannelError> {
        self.check_can_move()?;
        let pos = self.position;
        let next = self.build(NO_MOVE, self.head.state.pos_hash, 0, loser_is(self.me));
        Ok(self.commit(next, Move(NO_MOVE), pos))
    }

    /// Countersign the head so it can be settled. Called on a state already
    /// accepted by [`Channel::receive`], which is where the checking is.
    pub fn countersign(&mut self) -> Result<Signed, ChannelError> {
        if self.head.sig(self.me).is_none() {
            let s = self.sk.sign(&self.head.state.hash());
            self.head.put(self.me, s);
        }
        if !self.head.is_certified() {
            return Err(ChannelError::MissingCountersig);
        }
        self.certified = self.head;
        Ok(self.head)
    }

    /// Resign: a record, not a state. It does not advance the ply, so it
    /// cannot be mistaken for a move, and it costs the chain one signature.
    pub fn resign(&self) -> Signature {
        self.sk
            .sign(&crate::msg::resign_bytes(&self.channel_id, self.ply()))
    }

    /// Sign a draw at the current ply. Two of these is a draw; one is an offer.
    pub fn sign_draw(&self) -> Signature {
        self.sk
            .sign(&crate::msg::draw_bytes(&self.channel_id, self.ply()))
    }

    // -- internals ---------------------------------------------------------

    fn check_can_move(&self) -> Result<(), ChannelError> {
        if self.is_over() {
            return Err(ChannelError::GameOver);
        }
        if self.position.side != self.me {
            return Err(ChannelError::NotYourTurn);
        }
        if self.head.state.ply >= self.offer.terms.max_plies {
            return Err(ChannelError::PlyLimit);
        }
        Ok(())
    }

    fn build(&self, mv: u16, ph: Hash, my_clock: u32, status: Status) -> GameState {
        let mut next = GameState {
            channel_id: self.channel_id,
            ply: self.head.state.ply + 1,
            prev_hash: self.head.state.hash(),
            mv,
            pos_hash: ph,
            clock_w_ms: self.head.state.clock_w_ms,
            clock_b_ms: self.head.state.clock_b_ms,
            status,
        };
        match self.me {
            Color::White => next.clock_w_ms = my_clock,
            Color::Black => next.clock_b_ms = my_clock,
        }
        next
    }

    /// Sign the new state, countersign the previous one, and advance.
    fn commit(&mut self, next: GameState, mv: Move, after: Position) -> MoveMsg {
        // The countersignature on the state the opponent sent rides along
        // with this move rather than costing its own round trip.
        let countersig = if self.head.sig(self.me).is_none() && self.head.state.ply > 0 {
            let s = self.sk.sign(&self.head.state.hash());
            self.head.put(self.me, s);
            Some(s)
        } else {
            None
        };
        if self.head.is_certified() {
            self.certified = self.head;
        }

        let sig = self.sk.sign(&next.hash());
        let mut signed = Signed::new(next);
        signed.put(self.me, sig);
        self.head = signed;
        if mv.0 != NO_MOVE {
            self.rep_hashes.push(rep_hash(&after));
        }
        self.position = after;

        MoveMsg {
            mv,
            state: next,
            sig,
            countersig,
        }
    }

    fn key_of(&self, c: Color) -> VerifyingKey {
        match c {
            Color::White => self.offer.white_pk,
            Color::Black => self.offer.black_pk,
        }
    }

    /// How many times `after`'s position will have occurred, counting now.
    fn repetitions_of(&self, after: &Position) -> usize {
        let h = rep_hash(after);
        self.rep_hashes.iter().filter(|x| **x == h).count() + 1
    }
}
