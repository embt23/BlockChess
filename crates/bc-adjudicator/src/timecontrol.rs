//! D22 — Δ and τ come from a class table, not from the wire.
//!
//! ## The attack this closes
//!
//! Free-form `delta_blocks` is Lightning's `to_self_delay` problem imported
//! wholesale. An opponent proposes Δ = 64 at open, a client that does not
//! check accepts it, and in a bullet game the victim now has about two minutes
//! to get a move onto a chain or forfeit the pot.
//!
//! Consensus-enforced *bounds* do not fix it, and this is the part worth
//! understanding: a single `[min, max]` band wide enough to serve both bullet
//! and correspondence is, by construction, wide enough to contain a hostile
//! value for either. The band cannot be tight and general at the same time.
//! `MIN_DELTA_BLOCKS = 64` was exactly that band, and 64 *is* the hostile
//! value at bullet.
//!
//! So no number crosses the wire. `GameTerms` names a **class**, consensus
//! holds one `(Δ, τ)` pair per class, and there is nothing left to lie about.
//!
//! ## Why the class is also checked rather than merely carried
//!
//! Naming a class does not by itself help, because the class is the thing an
//! attacker would lie about instead — declare a three-minute game
//! `Correspondence`, then vanish while losing, and the winner waits an hour
//! per move instead of two minutes. So the class must **agree with the time
//! control it accompanies**, and [`TimeControl::for_clock`] is that rule.
//!
//! Carrying it as well as deriving it costs one byte and buys the same thing
//! `D25` buys for `adjudicator_ver`: the negotiated intent stays readable
//! where humans read it, and is pinned where money depends on it.

/// The five classes. Wire values are fixed forever; never renumber.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum TimeControl {
    Bullet = 0,
    Blitz = 1,
    Rapid = 2,
    Classical = 3,
    Correspondence = 4,
}

impl TimeControl {
    pub const ALL: [TimeControl; 5] = [
        TimeControl::Bullet,
        TimeControl::Blitz,
        TimeControl::Rapid,
        TimeControl::Classical,
        TimeControl::Correspondence,
    ];

    pub fn from_u8(b: u8) -> Option<TimeControl> {
        Self::ALL.get(b as usize).copied()
    }

    /// The estimated duration a class is judged on: base plus the increment a
    /// forty-move game would earn. This is the standard convention and it is
    /// the reason a 0+10 game is not called "bullet".
    pub fn estimated_ms(base_time_ms: u32, increment_ms: u32) -> u64 {
        base_time_ms as u64 + 40 * increment_ms as u64
    }

    /// Which class a given time control **is**. Consensus derives this; the
    /// declared class must match it.
    pub fn for_clock(base_time_ms: u32, increment_ms: u32) -> TimeControl {
        match Self::estimated_ms(base_time_ms, increment_ms) {
            0..=179_999 => TimeControl::Bullet,               // under 3 min
            180_000..=599_999 => TimeControl::Blitz,          // 3 to 10 min
            600_000..=3_599_999 => TimeControl::Rapid,        // 10 to 60 min
            3_600_000..=86_399_999 => TimeControl::Classical, // 1 h to 1 day
            _ => TimeControl::Correspondence,
        }
    }

    /// Δ — the per-response window, in blocks (`P4`: never seconds).
    pub fn delta_blocks(self) -> u32 {
        PARAMETERS[self as usize].delta_blocks
    }

    /// τ — milliseconds of game clock per block of on-chain budget.
    pub fn tau_ms(self) -> u32 {
        PARAMETERS[self as usize].tau_ms
    }

    /// A representative fresh clock for this class, used to choose τ and to
    /// test that the choice leaves dynamic range below [`MAX_BUDGET`].
    pub fn nominal_clock_ms(self) -> u32 {
        PARAMETERS[self as usize].nominal_clock_ms
    }
}

struct Params {
    delta_blocks: u32,
    tau_ms: u32,
    nominal_clock_ms: u32,
}

/// **The table. `G0`: these five pairs are Evan's to set — they are the rule,
/// not the plumbing.** What follows is a proposal with its reasoning, and the
/// tests below check the properties any table must satisfy, whatever the
/// numbers are.
///
/// Δ is chosen so a disputed move is answerable without the pot being held
/// hostage: roughly two minutes at bullet rising to an hour at
/// correspondence, at two-second blocks.
///
/// τ is chosen so that a **fresh** clock for the class maps to somewhat below
/// [`MAX_BUDGET`]. That matters more than it looks: dilation preserves the
/// *ratio* between two players' clocks, and a budget pinned at the cap has no
/// ratio left to preserve. Leaving headroom is what keeps a player who is
/// losing on time still losing on time once the game moves on-chain.
static PARAMETERS: [Params; 5] = [
    // Bullet — 2+1 nominal.
    Params {
        delta_blocks: 64,
        tau_ms: 32,
        nominal_clock_ms: 120_000,
    },
    // Blitz — 5+0.
    Params {
        delta_blocks: 128,
        tau_ms: 80,
        nominal_clock_ms: 300_000,
    },
    // Rapid — 30+0. `spec/05`'s stated default Δ.
    Params {
        delta_blocks: 256,
        tau_ms: 450,
        nominal_clock_ms: 1_800_000,
    },
    // Classical — 2 h.
    Params {
        delta_blocks: 512,
        tau_ms: 1_800,
        nominal_clock_ms: 7_200_000,
    },
    // Correspondence — 3 days.
    Params {
        delta_blocks: 2_048,
        tau_ms: 65_000,
        nominal_clock_ms: 259_200_000,
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dilation::{budget_blocks, FLOOR_BLOCKS, MAX_BUDGET, MIN_MOVE_BLOCKS};

    /// The attack in one test: no free-form number crosses the wire, so the
    /// only thing an attacker can propose is a class — and a class that does
    /// not match the clock is rejected elsewhere (`GameTerms::valid`).
    #[test]
    fn a_bullet_game_cannot_be_given_a_correspondence_window() {
        assert_eq!(TimeControl::for_clock(120_000, 1_000), TimeControl::Bullet);
        assert_ne!(
            TimeControl::Bullet.delta_blocks(),
            TimeControl::Correspondence.delta_blocks()
        );
    }

    #[test]
    fn classification_matches_the_usual_boundaries() {
        use TimeControl::*;
        // 1+0 and 2+1 are bullet; 3+0 is not.
        assert_eq!(TimeControl::for_clock(60_000, 0), Bullet);
        assert_eq!(TimeControl::for_clock(120_000, 1_000), Bullet);
        assert_eq!(TimeControl::for_clock(180_000, 0), Blitz);
        // 0+10 earns enough increment to be blitz, which is the point of
        // counting base + 40·increment rather than base alone.
        assert_eq!(TimeControl::for_clock(0, 10_000), Blitz);
        assert_eq!(TimeControl::for_clock(600_000, 0), Rapid);
        assert_eq!(TimeControl::for_clock(3_600_000, 0), Classical);
        assert_eq!(TimeControl::for_clock(86_400_000, 0), Correspondence);
    }

    /// Δ must rise with the class. A longer game getting a *shorter* window
    /// would be the original attack wearing a class label.
    #[test]
    fn delta_is_monotone_in_the_class() {
        let mut last = 0;
        for tc in TimeControl::ALL {
            assert!(tc.delta_blocks() > last, "{tc:?} does not increase Δ");
            last = tc.delta_blocks();
        }
    }

    /// Every class must leave room for at least a few moves inside one
    /// window, or a player who is answering promptly still forfeits.
    #[test]
    fn every_window_holds_several_moves() {
        for tc in TimeControl::ALL {
            let moves = tc.delta_blocks() as u64 / MIN_MOVE_BLOCKS;
            assert!(moves >= 8, "{tc:?}: only {moves} moves fit in Δ");
        }
    }

    /// τ must leave headroom below the cap, or dilation stops preserving the
    /// ratio it exists to preserve — every fresh clock would pin to
    /// `MAX_BUDGET` and two players in very different trouble would arrive
    /// on-chain looking identical.
    #[test]
    fn a_fresh_clock_leaves_dynamic_range_below_the_cap() {
        for tc in TimeControl::ALL {
            let fresh = budget_blocks(tc.nominal_clock_ms(), tc.tau_ms());
            assert!(
                fresh < MAX_BUDGET,
                "{tc:?}: a fresh clock pins at the cap ({fresh})"
            );
            assert!(
                fresh > MAX_BUDGET / 2,
                "{tc:?}: only {fresh} blocks, wasting range"
            );
        }
    }

    /// The property that makes dilation worth having: a player with a tenth
    /// of the clock gets materially less budget, in every class.
    #[test]
    fn the_ratio_survives_in_every_class() {
        for tc in TimeControl::ALL {
            let rich = budget_blocks(tc.nominal_clock_ms(), tc.tau_ms());
            let poor = budget_blocks(tc.nominal_clock_ms() / 10, tc.tau_ms());
            assert!(poor < rich / 2, "{tc:?}: {poor} vs {rich}");
            // …and still enough to physically move.
            assert!(poor >= FLOOR_BLOCKS, "{tc:?}: {poor} below the floor");
        }
    }

    #[test]
    fn wire_values_round_trip_and_are_fixed() {
        for (i, tc) in TimeControl::ALL.iter().enumerate() {
            assert_eq!(*tc as u8, i as u8);
            assert_eq!(TimeControl::from_u8(i as u8), Some(*tc));
        }
        assert_eq!(TimeControl::from_u8(5), None);
    }
}
