//! When a game is over, and which of the endings a chain can check cheaply.
//!
//! `spec/03-position.md` splits the terminal conditions by what the
//! adjudicator has to compute:
//!
//! | Condition | On-chain cost |
//! |---|---|
//! | fifty-move | O(1) — a field in the position |
//! | insufficient material | O(1) — popcounts |
//! | threefold repetition | O(1) — three countersigned states with equal `pos_hash` |
//! | stalemate, checkmate | **optimistic** — asserted, refuted by one legal move |
//!
//! The last row is invariant `P3`, and it is the reason this module exists
//! separately from the move generator. Deciding "is this mate" quantifies over
//! every legal move; refuting a mate claim needs one. The client calls
//! [`Position::outcome`] because it is cheap off-chain and it wants to know;
//! the adjudicator never calls it, and instead accepts a claim and waits to be
//! shown [`Position::refutes_terminal_claim`].
//!
//! Threefold repetition is absent on purpose: it is a property of a game, not
//! of a position, and here it is proved by three signatures rather than by
//! replaying history.

use crate::bitboard::*;
use crate::position::Position;
use crate::types::*;

/// The halfmove clock value at which a draw may be claimed: 50 moves by each
/// player, counted in plies.
pub const FIFTY_MOVE_PLIES: u8 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// `winner` delivered mate; the side to move has no legal move and is in
    /// check.
    Checkmate { winner: Color },
    /// The side to move has no legal move and is *not* in check.
    Stalemate,
    /// Neither side has mating material.
    InsufficientMaterial,
    /// The halfmove clock reached 100. Under FIDE this is *claimable* rather
    /// than automatic, and the protocol treats it the same way: it is a draw
    /// the moment either player asserts it, not before.
    FiftyMove,
}

impl Outcome {
    /// Is this a draw?
    pub fn is_draw(self) -> bool {
        !matches!(self, Outcome::Checkmate { .. })
    }
}

impl Position {
    /// Has the side to move no legal reply?
    ///
    /// Costs a full legal move generation. Off-chain that is nothing; on-chain
    /// it is exactly the ∀ that `P3` refuses to pay for.
    pub fn no_legal_moves(&self) -> bool {
        self.generate_legal().is_empty()
    }

    pub fn is_checkmate(&self) -> bool {
        self.in_check(self.side) && self.no_legal_moves()
    }

    pub fn is_stalemate(&self) -> bool {
        !self.in_check(self.side) && self.no_legal_moves()
    }

    /// Can the fifty-move draw be claimed?
    pub fn fifty_move_claimable(&self) -> bool {
        self.halfmove >= FIFTY_MOVE_PLIES
    }

    /// Can *neither* side deliver mate, however badly the other plays?
    ///
    /// FIDE's test is whether mate is *possible*, not whether it can be
    /// forced, so the bar is low and the rule is narrower than it first
    /// looks. With no pawns, rooks or queens on the board, exactly two
    /// families qualify:
    ///
    /// - **Bishops only, all on one square colour.** Any number, split
    ///   between the players however you like. A bishop confined to the light
    ///   squares can never attack a dark one, so no arrangement of them
    ///   covers a king's escape squares. This covers K vs K, K+B vs K, and
    ///   same-colour K+B vs K+B as special cases.
    /// - **A single knight, and no bishops.** One knight cannot mate.
    ///
    /// Everything else is excluded, including K+N+N vs K — it cannot be
    /// *forced*, but a cooperative defender can walk into it, so it is a
    /// fifty-move draw and not this one.
    ///
    /// This function used to require exactly one bishop per side, which made
    /// K+2B-on-one-colour vs K read as sufficient. It was wrong for eighteen
    /// months and no hand-written test noticed; the `shakmaty` differential
    /// found it at ply 387 of random game 303 (`docs/build-log.md` §15). It
    /// is why `G1` is a rule.
    pub fn insufficient_material(&self) -> bool {
        if self.piece_bb[Piece::Pawn.idx()]
            | self.piece_bb[Piece::Rook.idx()]
            | self.piece_bb[Piece::Queen.idx()]
            != 0
        {
            return false;
        }
        let knights = self.piece_bb[Piece::Knight.idx()];
        let bishops = self.piece_bb[Piece::Bishop.idx()];

        if knights == 0 {
            (bishops & LIGHT_SQUARES) == bishops || (bishops & DARK_SQUARES) == bishops
        } else {
            bishops == 0 && knights.count_ones() == 1
        }
    }

    /// The outcome of this position, if it has one.
    ///
    /// Precedence follows FIDE: mate ends the game even when the halfmove
    /// clock has run out or neither side had material to spare a move ago,
    /// because the mating move happened first.
    pub fn outcome(&self) -> Option<Outcome> {
        if self.no_legal_moves() {
            return Some(if self.in_check(self.side) {
                Outcome::Checkmate {
                    winner: self.side.flip(),
                }
            } else {
                Outcome::Stalemate
            });
        }
        if self.insufficient_material() {
            return Some(Outcome::InsufficientMaterial);
        }
        if self.fifty_move_claimable() {
            return Some(Outcome::FiftyMove);
        }
        None
    }

    /// The cheap half of `P3`: does `m` refute a claim that this position is
    /// checkmate or stalemate?
    ///
    /// One legal move is a complete refutation of both, and finding one costs
    /// a single `make_move` and a check test. This is the asymmetry the whole
    /// adjudicator design rests on — proving mate is ∀ over ~218 moves,
    /// refuting it is ∃ over one — so it is written as its own function to
    /// make the cost visible where it is called.
    pub fn refutes_terminal_claim(&self, m: Move) -> bool {
        self.is_move_legal(m)
    }
}
