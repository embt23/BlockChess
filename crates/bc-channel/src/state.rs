//! The game state — the object both players sign, once per ply.
//!
//! 109 bytes, fixed, hashing to 32. `spec/04-channel.md` specifies the fields;
//! this module specifies their bytes, because the encoding is consensus
//! critical: two implementations that lay the fields out differently produce
//! different hashes, and a signature over one is not a signature over the
//! other.

use bc_hash::{tagged, Hash, ZERO_HASH};

/// Wire size of an encoded [`GameState`]: 32+2+32+2+32+4+4+1.
pub const STATE_LEN: usize = 109;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Status {
    Ongoing = 0,
    WhiteWins = 1,
    BlackWins = 2,
    Draw = 3,
}

impl Status {
    pub fn from_u8(b: u8) -> Option<Status> {
        Some(match b {
            0 => Status::Ongoing,
            1 => Status::WhiteWins,
            2 => Status::BlackWins,
            3 => Status::Draw,
            _ => return None,
        })
    }
    pub fn is_terminal(self) -> bool {
        self != Status::Ongoing
    }
}

/// One link in the game's hash chain.
///
/// `prev_hash` is what makes signing state *n* a statement about every state
/// before it. There is no separate history object and no need to retain old
/// signatures: the current signature commits to the whole game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameState {
    pub channel_id: Hash,
    /// Strictly increasing. 0 is the opening position, before any move.
    pub ply: u16,
    /// `state_hash` of ply−1, or zero at ply 0.
    pub prev_hash: Hash,
    /// The move that produced this state, as `bc_chess::Move`. Zero at ply 0.
    pub mv: u16,
    /// `H_domain("BC/pos/v1", packed_position)` of the position *after* the move.
    pub pos_hash: Hash,
    /// Clocks **after** this move, increment already added.
    pub clock_w_ms: u32,
    pub clock_b_ms: u32,
    pub status: Status,
}

impl GameState {
    /// The opening state. Signed by nobody: it is fixed by the `OpenGame`
    /// transaction, which already carries both players' signatures over the
    /// offer, and the offer commits to the start position and the clocks.
    pub fn genesis(channel_id: Hash, pos_hash: Hash, base_time_ms: u32) -> GameState {
        GameState {
            channel_id,
            ply: 0,
            prev_hash: ZERO_HASH,
            mv: 0,
            pos_hash,
            clock_w_ms: base_time_ms,
            clock_b_ms: base_time_ms,
            status: Status::Ongoing,
        }
    }

    pub fn encode(&self) -> [u8; STATE_LEN] {
        let mut b = [0u8; STATE_LEN];
        b[0..32].copy_from_slice(&self.channel_id);
        b[32..34].copy_from_slice(&self.ply.to_le_bytes());
        b[34..66].copy_from_slice(&self.prev_hash);
        b[66..68].copy_from_slice(&self.mv.to_le_bytes());
        b[68..100].copy_from_slice(&self.pos_hash);
        b[100..104].copy_from_slice(&self.clock_w_ms.to_le_bytes());
        b[104..108].copy_from_slice(&self.clock_b_ms.to_le_bytes());
        b[108] = self.status as u8;
        b
    }

    pub fn decode(b: &[u8]) -> Option<GameState> {
        if b.len() != STATE_LEN {
            return None;
        }
        Some(GameState {
            channel_id: b[0..32].try_into().ok()?,
            ply: u16::from_le_bytes(b[32..34].try_into().ok()?),
            prev_hash: b[34..66].try_into().ok()?,
            mv: u16::from_le_bytes(b[66..68].try_into().ok()?),
            pos_hash: b[68..100].try_into().ok()?,
            clock_w_ms: u32::from_le_bytes(b[100..104].try_into().ok()?),
            clock_b_ms: u32::from_le_bytes(b[104..108].try_into().ok()?),
            status: Status::from_u8(b[108])?,
        })
    }

    /// What both players sign.
    pub fn hash(&self) -> Hash {
        tagged("BC/state/v1", &self.encode())
    }
}

/// The position hash carried in a state. Commits to **everything**, halfmove
/// clock included, because the adjudicator replays moves from the packed form
/// and the fifty-move rule reads that field.
pub fn pos_hash(p: &bc_chess::Position) -> Hash {
    tagged("BC/pos/v1", p.pack().as_slice())
}

/// The repetition key: the same bytes with the halfmove clock cleared.
///
/// Two occurrences of a position always differ in the halfmove clock — plies
/// happened in between, that is what a repetition is — so comparing
/// [`pos_hash`] would never find one. This is the comparison FIDE actually
/// specifies: pieces, side to move, castling rights, en-passant availability.
///
/// It costs the chain one extra hash per claimed occurrence, because a
/// threefold claim must now carry the three packed positions rather than only
/// the three states. Still O(1), and still paid for by the opponent's own
/// signatures rather than by a history replay.
pub fn rep_hash(p: &bc_chess::Position) -> Hash {
    tagged("BC/rep/v1", p.pack().without_halfmove().as_slice())
}
