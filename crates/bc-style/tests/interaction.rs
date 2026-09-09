//! The oracle for episode 18.
//!
//! A test that finds an effect is worthless until you have shown it can also
//! find nothing. The negative control below is the load-bearing test in this
//! file: same two styles in every game, with the opponent's *name* randomised,
//! so the label carries no information by construction. If the instrument
//! reports significance there, it reports significance everywhere and the
//! positive result means nothing.

use bc_style::interaction;
use bc_style::synth::{self, Rng};
use bc_style::{Corpus, Lab};

fn round_robin_lab(rounds: usize) -> Lab {
    let mut c = Corpus::new();
    for g in synth::round_robin(rounds, 0x5EED_1234, 120) {
        c.push(g);
    }
    Lab::fit(c, 4).expect("should fit")
}

/// Every game is Tal versus Petrosian. Only the *label* on the opponent varies,
/// drawn at random from three fictitious names, so it is pure noise.
fn placebo_lab(games: usize, seed: u64) -> Lab {
    let mut rng = Rng::new(seed);
    let mut c = Corpus::new();
    let fakes = ["A", "B", "C"];
    for _ in 0..games {
        let mut g = synth::play(&synth::ARCHETYPES[0], &synth::ARCHETYPES[1], &mut rng, 120);
        g.white = "Subject".to_string();
        g.black = fakes[(rng.next_u64() as usize) % fakes.len()].to_string();
        c.push(g);
    }
    Lab::fit(c, 4).expect("should fit")
}

/// The control. Nothing to find, so nothing may be found.
#[test]
fn a_meaningless_opponent_label_produces_no_effect() {
    let lab = placebo_lab(90, 0xDEADBEEF);
    let rows = interaction::analyse(&lab, 200, 0x1_9AC7_2E51);

    let subject = rows
        .iter()
        .find(|r| r.player == "Subject")
        .expect("the subject plays every game");
    assert!(subject.opponents >= 2, "needs several labels to compare");
    assert!(
        subject.p_value > 0.05,
        "the instrument found an effect in pure noise: p = {:.3}, ratio {:.3}",
        subject.p_value,
        subject.ratio
    );
}

/// And the positive case: with real opponents, the effect is there.
#[test]
fn who_you_play_changes_how_you_play() {
    let lab = round_robin_lab(10);
    let rows = interaction::analyse(&lab, 200, 0x1_9AC7_2E51);

    assert_eq!(rows.len(), 4, "every archetype should be measured");
    for r in &rows {
        assert_eq!(r.opponents, 3, "{} should face three opponents", r.player);
        assert!(
            r.p_value < 0.05,
            "{} shows no opponent effect: p = {:.3}",
            r.player,
            r.p_value
        );
    }
}

/// The archetypes have constant policies, so the effect cannot be a personality
/// change. It is also smaller than game-to-game scatter, and saying so is part
/// of reporting it honestly.
#[test]
fn the_effect_is_real_but_smaller_than_a_players_own_variance() {
    let rows = interaction::analyse(&round_robin_lab(10), 200, 0x1_9AC7_2E51);
    for r in &rows {
        assert!(r.between > 0.0 && r.within > 0.0);
        assert!(
            r.ratio < 1.0,
            "{} moves more between opponents than within one — \
             that would mean the opponent matters more than the game",
            r.player
        );
    }
}

/// No finite number of shuffles can demonstrate p = 0, so the estimator must
/// never claim it.
#[test]
fn p_values_are_bounded_away_from_zero() {
    let rows = interaction::analyse(&round_robin_lab(8), 100, 0xABCD);
    for r in &rows {
        assert!(
            r.p_value >= 1.0 / 101.0 - 1e-12,
            "{} claims p = {} — below the resolution of 100 shuffles",
            r.player,
            r.p_value
        );
        assert!(r.p_value <= 1.0);
    }
}

#[test]
fn a_player_with_one_opponent_is_skipped() {
    let lab = placebo_lab(20, 0x1234);
    let rows = interaction::analyse(&lab, 50, 0x9999);
    // The three fictitious opponents each face only "Subject", so only the
    // subject has anyone to compare across.
    for r in &rows {
        assert!(
            r.opponents >= 2,
            "{} was measured against one opponent",
            r.player
        );
    }
}

/// Episode 18's headline: some opponents drag everybody further than others.
#[test]
fn the_pull_ranking_is_ordered_and_covers_everyone() {
    let lab = round_robin_lab(10);
    let pulls = interaction::pulls(&lab);

    assert_eq!(pulls.len(), 4);
    for p in &pulls {
        assert_eq!(p.players, 3, "{} should pull on three others", p.opponent);
        assert!(p.magnitude > 0.0 && p.magnitude.is_finite());
        assert_eq!(p.vector.len(), lab.basis.k);
    }
    for w in pulls.windows(2) {
        assert!(
            w[0].magnitude >= w[1].magnitude,
            "pulls must come back sorted"
        );
    }
}

/// The prophylactic archetype is the only one rewarded for restricting its
/// opponent, so it should be the one that most makes others play unlike
/// themselves. This is the interpretable claim, and it is worth pinning.
#[test]
fn the_restricting_archetype_pulls_hardest() {
    let pulls = interaction::pulls(&round_robin_lab(10));
    assert_eq!(
        pulls[0].opponent, "Petrosian",
        "expected the restricting style to pull hardest, got {}",
        pulls[0].opponent
    );
}
