//! The pre-registered kill threshold for D20, as code.
//!
//! `spec/09` D20 fixes these numbers and they were signed **before** the
//! measurement ran. They live here rather than only in prose because a
//! threshold you can edit while looking at the result is not a threshold.
//! Changing any constant in this file changes a test, which changes a diff,
//! which someone has to justify.
//!
//! ## The cuts are on *true* divergence
//!
//! The estimator recovers a mean 60% of the truth
//! (`docs/d20-calibration.md`), so the two scales differ by nearly a factor
//! of two and the distinction decides verdicts. Stated on the raw estimate,
//! the cuts would fail in the worst possible direction: a true `D` of 0.02 —
//! the low end of `spec/10` §7's own figure — reads back as 0.012 and would
//! have scored **amber**, and at the bottom of the recovery range as 0.0046,
//! which would have scored **Act II is dead**. The thesis being exactly right
//! would have registered as the thesis failing.
//!
//! So the raw estimate is corrected before it is compared, and re-measuring
//! the recovery factor later moves [`RECOVERY_MEAN`] and leaves the
//! thresholds untouched.

/// Mean fraction of true divergence the estimator recovers.
/// Measured over ten synthetic players spanning a 13× range of true `D`.
pub const RECOVERY_MEAN: f64 = 0.60;
/// Best and worst per-player recovery observed in calibration.
pub const RECOVERY_BEST: f64 = 0.97;
pub const RECOVERY_WORST: f64 = 0.23;

/// At or above this true divergence, `spec/10` stands as written.
pub const THESIS_HOLDS: f64 = 0.02;
/// Below this true divergence, Act II is not a product.
pub const ACT_II_DEAD: f64 = 0.005;

/// What the chimera — a player-shaped pile of other people's decisions —
/// measured. The floor below which the instrument cannot tell a player from
/// nobody.
pub const CHIMERA: f64 = 0.0006;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// `spec/10` §7 stands.
    Holds,
    /// Act II survives, but every timescale in `spec/10` and `spec/13` is
    /// wrong and must be rewritten.
    Amber,
    /// Under one bit of identity per game. The cheat detector and the style
    /// asset both need hundreds of games to say anything, and neither is a
    /// product.
    Dead,
}

impl std::fmt::Display for Verdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Verdict::Holds => "THESIS HOLDS",
            Verdict::Amber => "AMBER",
            Verdict::Dead => "ACT II IS DEAD",
        })
    }
}

/// The estimate, on both scales, with the interval the calibration supports.
#[derive(Debug, Clone, Copy)]
pub struct Estimate {
    /// What the estimator reported.
    pub raw: f64,
    /// Point estimate of true divergence.
    pub corrected: f64,
    /// `[raw/best, raw/worst]` — narrow end from the best-recovered synthetic
    /// player, wide end from the worst.
    pub interval: (f64, f64),
}

impl Estimate {
    pub fn from_raw(raw: f64) -> Estimate {
        Estimate {
            raw,
            corrected: raw / RECOVERY_MEAN,
            interval: (raw / RECOVERY_BEST, raw / RECOVERY_WORST),
        }
    }

    /// The verdict, read off the corrected point estimate.
    pub fn verdict(&self) -> Verdict {
        if self.corrected >= THESIS_HOLDS {
            Verdict::Holds
        } else if self.corrected >= ACT_II_DEAD {
            Verdict::Amber
        } else {
            Verdict::Dead
        }
    }

    /// Is the verdict robust across the whole recovery interval, or does it
    /// depend on which end of the calibration this player resembles?
    pub fn unambiguous(&self) -> bool {
        let lo = Estimate {
            raw: 0.0,
            corrected: self.interval.0,
            interval: self.interval,
        };
        let hi = Estimate {
            raw: 0.0,
            corrected: self.interval.1,
            interval: self.interval,
        };
        lo.verdict() == hi.verdict()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The reason the table is stated on true divergence. If these cuts were
    /// applied to the raw estimate, `spec/10`'s own low-end figure would
    /// score against the thesis it came from.
    #[test]
    fn the_thesis_being_right_does_not_read_as_the_thesis_failing() {
        let raw_if_true_is_0_02 = THESIS_HOLDS * RECOVERY_MEAN;
        assert!(raw_if_true_is_0_02 < THESIS_HOLDS, "0.012 < 0.02");

        // Uncorrected, that would have been amber.
        assert_eq!(
            Estimate {
                raw: raw_if_true_is_0_02,
                corrected: raw_if_true_is_0_02,
                interval: (0.0, 0.0)
            }
            .verdict(),
            Verdict::Amber
        );
        // Corrected, it is what it actually is.
        assert_eq!(
            Estimate::from_raw(raw_if_true_is_0_02).verdict(),
            Verdict::Holds
        );

        // And at the worst observed recovery it would have read as dead.
        let worst = THESIS_HOLDS * RECOVERY_WORST;
        assert!(worst < ACT_II_DEAD, "{worst} < {ACT_II_DEAD}");
    }

    #[test]
    fn the_cuts_are_where_the_spec_says() {
        assert_eq!(Estimate::from_raw(0.030).verdict(), Verdict::Holds);
        assert_eq!(Estimate::from_raw(0.012).verdict(), Verdict::Holds);
        assert_eq!(Estimate::from_raw(0.0119).verdict(), Verdict::Amber);
        assert_eq!(Estimate::from_raw(0.0030).verdict(), Verdict::Amber);
        assert_eq!(Estimate::from_raw(0.0029).verdict(), Verdict::Dead);
        assert_eq!(Estimate::from_raw(0.0).verdict(), Verdict::Dead);
    }

    /// The dead-floor on the raw scale must sit clear of what the instrument
    /// reads for a dataset with no player behind it, or the test is measuring
    /// its own noise.
    #[test]
    fn the_dead_floor_clears_the_chimera() {
        let raw_floor = ACT_II_DEAD * RECOVERY_MEAN;
        assert!(
            raw_floor > CHIMERA * 4.0,
            "raw floor {raw_floor} is not clear of chimera {CHIMERA}"
        );
    }

    #[test]
    fn an_interval_spanning_a_cut_is_flagged_as_ambiguous() {
        // Comfortably inside one band.
        assert!(Estimate::from_raw(0.060).unambiguous());
        // Straddling the holds/amber cut, because 0.012 raw is 0.0124-0.052
        // depending on which synthetic player this one resembles.
        assert!(!Estimate::from_raw(0.012).unambiguous());
    }
}
