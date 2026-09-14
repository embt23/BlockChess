//! Clock accounting, and why there is no trusted time source.
//!
//! The mover debits their own clock and asserts the result inside the state
//! they sign. The opponent decides whether to accept it. That is the whole
//! mechanism: **the clock is enforced by refusal to countersign.**
//!
//! It works because refusing costs the honest player nothing they did not
//! already have. If your opponent asserts a clock you do not believe, you stop
//! signing and go to chain — which is the same thing you do if they simply
//! stop replying. The protocol needs no new liveness assumption to enforce
//! time, which is the test `spec/05` applies to every mechanism in it.
//!
//! The asymmetry in [`plausible`] is deliberate. A mover who claims to have
//! used *more* time than you observed is giving you time; there is no reason
//! to object, and objecting would turn network jitter into a dispute. Only an
//! under-claim — time the mover spent but does not want debited — is refused.

/// How much under-claim to tolerate before refusing to countersign.
///
/// Absorbs network latency and honest clock skew. Too small and flaky wifi
/// produces disputes; too large and a player can steal `grace` milliseconds
/// per move, which over 40 moves is 12 seconds.
pub const DEFAULT_GRACE_MS: u32 = 300;

/// What the mover's clock reads after spending `elapsed_ms`.
///
/// `None` means the flag fell: the move took longer than the mover had.
/// Increment is added after the move, per Fischer.
pub fn debit(remaining_ms: u32, elapsed_ms: u32, increment_ms: u32) -> Option<u32> {
    if elapsed_ms > remaining_ms {
        return None;
    }
    Some((remaining_ms - elapsed_ms).saturating_add(increment_ms))
}

/// Would a receiver accept the mover's asserted clock?
///
/// `observed_elapsed_ms` is what the receiver measured locally, which is an
/// over-estimate of the mover's true think time by roughly one network
/// latency — hence the grace.
pub fn plausible(
    prev_remaining_ms: u32,
    asserted_ms: u32,
    observed_elapsed_ms: u32,
    increment_ms: u32,
    grace_ms: u32,
) -> bool {
    let ceiling = prev_remaining_ms as i64 + increment_ms as i64;
    let claimed_elapsed = ceiling - asserted_ms as i64;
    // Claiming a clock above the ceiling is claiming time that was never
    // there: it is minting, one ply at a time.
    if claimed_elapsed < 0 {
        return false;
    }
    claimed_elapsed + grace_ms as i64 >= observed_elapsed_ms as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fischer_increment_is_added_after_the_move() {
        assert_eq!(debit(60_000, 5_000, 2_000), Some(57_000));
    }

    #[test]
    fn the_flag_falls_when_the_move_outlasts_the_clock() {
        assert_eq!(debit(1_000, 1_001, 5_000), None);
        // Exactly on the buzzer is not a flag, and the increment still lands.
        assert_eq!(debit(1_000, 1_000, 5_000), Some(5_000));
    }

    #[test]
    fn honest_jitter_is_accepted_and_theft_is_not() {
        // Claimed 5s, receiver saw 5.2s: inside the grace.
        assert!(plausible(60_000, 57_000, 5_200, 2_000, DEFAULT_GRACE_MS));
        // Claimed 5s, receiver saw 9s: four seconds is not jitter.
        assert!(!plausible(60_000, 57_000, 9_000, 2_000, DEFAULT_GRACE_MS));
        // Claiming to have used *more* than observed is the opponent's loss
        // and nobody else's business.
        assert!(plausible(60_000, 50_000, 1_000, 2_000, DEFAULT_GRACE_MS));
    }

    #[test]
    fn a_clock_cannot_grow_past_its_ceiling() {
        // 60s remaining plus a 2s increment is 62s; 62.001s is minted time.
        assert!(plausible(60_000, 62_000, 0, 2_000, DEFAULT_GRACE_MS));
        assert!(!plausible(60_000, 62_001, 0, 2_000, DEFAULT_GRACE_MS));
    }
}
