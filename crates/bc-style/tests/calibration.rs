//! Regression guards for the personality estimator.
//!
//! A smaller, faster version of `bin/calibrate`. These assert the properties
//! that make the instrument trustworthy — above all that it reports ~0 when
//! there is no personality to find. If that one breaks, every number the crate
//! produces becomes worthless, silently.

use bc_style::model::{cross_entropy, fit_personality, train, Lcg, Sample, TrainCfg, Weights};
use bc_style::synth::{
    corpus, plausible_base, samples_for, true_divergence, SynthGame, SynthPlayer,
};

const LAMBDAS: [f32; 5] = [1e-3, 3e-3, 1e-2, 3e-2, 1e-1];

struct Fixture {
    pop: Weights,
    deltas: Vec<Weights>,
    players: Vec<SynthPlayer>,
    train_games: Vec<SynthGame>,
    val_games: Vec<SynthGame>,
    test_games: Vec<SynthGame>,
}

fn fixture() -> Fixture {
    let mut rng = Lcg::new(0x000C_A11B_2026);
    let base = plausible_base();

    let mut players = vec![SynthPlayer::null("null", &base)];
    for (name, sigma) in [("mild", 0.3f32), ("strong", 0.55), ("extreme", 0.85)] {
        players.push(SynthPlayer::random(name, &base, sigma, 0.25, &mut rng));
    }

    let mut games = corpus(&players, 90, 70, &mut rng);
    // Shuffle before splitting: `corpus` emits games grouped by player, so an
    // index split would hand early players the training data and late players
    // the test data, confounding personality strength with sample size.
    for i in (1..games.len()).rev() {
        games.swap(i, (rng.next() % (i as u64 + 1)) as usize);
    }
    let a = games.len() * 6 / 10;
    let b = games.len() * 8 / 10;
    let test_games = games.split_off(b);
    let val_games = games.split_off(a);
    let train_games = games;

    let mut pop_samples = Vec::new();
    for i in 0..players.len() {
        pop_samples.extend(samples_for(&train_games, i));
    }
    let pop = train(
        &pop_samples,
        &Weights::zeros(),
        &TrainCfg {
            epochs: 6,
            lr: 0.08,
            l2: 0.0,
        },
    );

    let deltas = (0..players.len())
        .map(|i| {
            fit_personality(
                &samples_for(&train_games, i),
                &samples_for(&val_games, i),
                &pop,
                &LAMBDAS,
                8,
                0.05,
            )
            .delta
        })
        .collect();

    Fixture {
        pop,
        deltas,
        players,
        train_games,
        val_games,
        test_games,
    }
}

fn d_self(f: &Fixture, i: usize) -> f64 {
    let test = samples_for(&f.test_games, i);
    cross_entropy(&test, &f.pop.0) - cross_entropy(&test, &f.pop.plus(&f.deltas[i]).0)
}

#[test]
fn estimator_does_not_fabricate_personality() {
    // THE critical control. A "chimera" is a player-shaped pile of decisions,
    // each drawn from a random member of the population — so its true policy is
    // the population mixture and its true divergence is ~0 by construction.
    //
    // A positive reading here means the estimator invents personality out of
    // noise, and every other number it produces is meaningless.
    let f = fixture();
    let mut rng = Lcg::new(0x0C41_BEEF);
    let mut pick = |gs: &[SynthGame]| -> Vec<Sample> {
        let mut pool = Vec::new();
        for i in 0..f.players.len() {
            for s in samples_for(gs, i) {
                if rng.unit() < 1.0 / f.players.len() as f32 {
                    pool.push(s);
                }
            }
        }
        pool
    };
    let (tr, va, te) = (
        pick(&f.train_games),
        pick(&f.val_games),
        pick(&f.test_games),
    );
    let fit = fit_personality(&tr, &va, &f.pop, &LAMBDAS, 8, 0.05);
    let d = cross_entropy(&te, &f.pop.0) - cross_entropy(&te, &f.pop.plus(&fit.delta).0);
    assert!(
        d.abs() < 0.01,
        "chimera must measure ~0, got {d:+.4} nats/move"
    );
}

#[test]
fn a_real_personality_is_recovered() {
    let f = fixture();
    for i in 1..f.players.len() {
        let truth = true_divergence(&f.test_games, i, &f.players, &f.pop);
        let est = d_self(&f, i);
        assert!(
            est > 0.0,
            "{}: expected positive, got {est:+.4}",
            f.players[i].name
        );
        // Conservative on purpose: the features cannot express everything a
        // policy does, and the regulariser shrinks delta toward zero. Both push
        // the estimate down, so it is a lower bound.
        assert!(
            est < truth * 1.3,
            "{}: estimate {est:.4} should not exceed truth {truth:.4}",
            f.players[i].name
        );
        assert!(
            est > truth * 0.15,
            "{}: estimate {est:.4} too far below truth {truth:.4}",
            f.players[i].name
        );
    }
}

#[test]
fn someone_elses_personality_does_not_fit_you() {
    // If wearing another player's delta improved your score, the estimator
    // would be measuring something shared (rating, the population's own
    // quirks) rather than identity.
    let f = fixture();
    let mut worse = 0;
    let mut total = 0;
    for i in 0..f.players.len() {
        let test = samples_for(&f.test_games, i);
        let base = cross_entropy(&test, &f.pop.0);
        for j in 0..f.players.len() {
            if i == j {
                continue;
            }
            total += 1;
            worse += usize::from(base - cross_entropy(&test, &f.pop.plus(&f.deltas[j]).0) < 0.0);
        }
    }
    assert!(
        worse * 2 > total,
        "most cross-player fits should be worse than the population model ({worse}/{total})"
    );
}

#[test]
fn players_are_identifiable_from_held_out_games() {
    let f = fixture();
    let (mut correct, mut total) = (0usize, 0usize);
    for i in 0..f.players.len() {
        for g in &f.test_games {
            if g.white != i && g.black != i {
                continue;
            }
            let s = samples_for(std::slice::from_ref(g), i);
            if s.len() < 10 {
                continue;
            }
            let best = (0..f.players.len())
                .map(|j| (j, cross_entropy(&s, &f.pop.plus(&f.deltas[j]).0)))
                .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                .unwrap()
                .0;
            correct += usize::from(best == i);
            total += 1;
        }
    }
    let chance = 1.0 / f.players.len() as f64;
    let acc = correct as f64 / total as f64;
    assert!(
        acc > chance * 2.0,
        "identification {acc:.2} should beat chance {chance:.2} substantially"
    );
}

#[test]
fn softmax_is_a_distribution_and_survives_extreme_weights() {
    use bc_chess::Position;
    use bc_style::features::extract_all;
    use bc_style::model::policy;

    let pos = Position::startpos();
    let legal = pos.generate_legal();
    let feats = extract_all(&pos, legal.as_slice());

    for scale in [0.0f32, 1.0, 50.0, 1e4] {
        let w = vec![scale; bc_style::features::N_FEATURES];
        let p = policy(&feats, &w);
        let sum: f32 = p.iter().sum();
        assert!((sum - 1.0).abs() < 1e-4, "scale {scale}: sums to {sum}");
        assert!(
            p.iter().all(|x| x.is_finite() && *x >= 0.0),
            "scale {scale}: not finite"
        );
    }
}
