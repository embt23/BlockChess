//! The terms a channel commits to, and the class its deadlines come from.
//!
//! This lives in the adjudicator rather than beside `GameOffer` because it is
//! **consensus state**: it is inside `channel_id`, inside every signed
//! `GameState`, and the dispute machine reads it. The offer that wraps it —
//! stakes, keys, signatures, rake — is client and server business and stays
//! in `bc-channel`.

use crate::timecontrol::TimeControl;
use bc_hash::Hash;

/// 32 start_pos_hash + 4 base + 4 increment + 2 max_plies + 1 class
/// + 1 rules_mask + 2 adjudicator_ver.
///
/// Six bytes shorter than it was, because Δ and τ no longer travel (`D22`).
pub const TERMS_LEN: usize = 46;

/// Highest adjudicator version this build knows how to be.
///
/// `D25`: the integer is what channels negotiate and what a spec can name
/// before the build exists; consensus separately holds the hash of the
/// ruleset each integer denotes, so "every version is kept forever" is a
/// checkable claim rather than a promise. See [`crate::ruleset`].
pub const ADJUDICATOR_VER: u16 = 1;

/// Longest game the channel will carry. 600 plies is well past any real game
/// and bounds the worst-case dispute replay.
pub const MAX_PLIES_LIMIT: u16 = 600;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameTerms {
    pub start_pos_hash: Hash,
    pub base_time_ms: u32,
    pub increment_ms: u32,
    pub max_plies: u16,
    /// The class Δ and τ are looked up from (`D22`). No free-form window ever
    /// crosses the wire, so there is no hostile value to propose.
    pub time_control: TimeControl,
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
        b[42] = self.time_control as u8;
        b[43] = self.rules_mask;
        b[44..46].copy_from_slice(&self.adjudicator_ver.to_le_bytes());
        b
    }

    /// The inverse of [`GameTerms::encode`].
    ///
    /// Did not exist until transactions started travelling in blocks,
    /// which is the honest reason: until episode 05 the only thing that
    /// ever encoded terms was a hash, and a hash needs no inverse.
    ///
    /// Rejects an unknown time-control class rather than defaulting to
    /// one. A default here would mean two nodes with different builds
    /// silently agreeing on different deadlines for the same channel.
    pub fn decode(b: &[u8]) -> Option<GameTerms> {
        if b.len() != TERMS_LEN {
            return None;
        }
        let mut start_pos_hash = [0u8; 32];
        start_pos_hash.copy_from_slice(&b[0..32]);
        Some(GameTerms {
            start_pos_hash,
            base_time_ms: u32::from_le_bytes(b[32..36].try_into().ok()?),
            increment_ms: u32::from_le_bytes(b[36..40].try_into().ok()?),
            max_plies: u16::from_le_bytes(b[40..42].try_into().ok()?),
            time_control: TimeControl::from_u8(b[42])?,
            rules_mask: b[43],
            adjudicator_ver: u16::from_le_bytes(b[44..46].try_into().ok()?),
        })
    }

    /// Δ, the challenge window, in **blocks**. Never seconds (`P4`).
    pub fn delta_blocks(&self) -> u32 {
        self.time_control.delta_blocks()
    }

    /// τ, the clock dilation constant — see `spec/05-adjudication.md`.
    pub fn budget_tau_ms(&self) -> u32 {
        self.time_control.tau_ms()
    }

    /// Terms for a clock, with the class derived rather than chosen.
    pub fn for_clock(start_pos_hash: Hash, base_time_ms: u32, increment_ms: u32) -> GameTerms {
        GameTerms {
            start_pos_hash,
            base_time_ms,
            increment_ms,
            max_plies: MAX_PLIES_LIMIT,
            time_control: TimeControl::for_clock(base_time_ms, increment_ms),
            rules_mask: 0xFF,
            adjudicator_ver: ADJUDICATOR_VER,
        }
    }

    pub fn valid(&self) -> bool {
        // The class must be the one this clock actually is. Carrying it and
        // not checking it would just move the lie from Δ to the label.
        self.time_control == TimeControl::for_clock(self.base_time_ms, self.increment_ms)
            && self.max_plies > 0
            && self.max_plies <= MAX_PLIES_LIMIT
            && self.base_time_ms > 0
            && self.adjudicator_ver <= ADJUDICATOR_VER
    }
}
