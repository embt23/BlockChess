//! D20 — calibrating the personality estimator against known ground truth.
//!
//!   cargo run --release -p bc-style --bin calibrate
//!
//! `spec/10-personality.md` claims `D(π_you ‖ π_pop)` is measurable and worth
//! ~0.02–0.10 nats/move. Before that estimate can be checked on real games, the
//! estimator itself has to be shown to work — because the naive measurement is
//! biased upward by construction: a model fitted on your games scores your
//! games better whether or not you have any style.
//!
//! So: build players whose divergence is known exactly, and see whether the
//! estimator recovers it — including the case where the answer is zero.

use bc_style::model::{cross_entropy, fit_personality, train, Lcg, Sample, TrainCfg, Weights};
use bc_style::synth::{
    corpus, plausible_base, samples_for, true_divergence, SynthGame, SynthPlayer,
};

fn main() {
    let mut rng = Lcg::new(0xD20_CA11B);
    let base = plausible_base();

    // Five personality strengths, including zero. The null player is the
    // control that decides whether any of this is trustworthy.
    let spec: [(&str, f32); 5] = [
        ("null", 0.00),
        ("faint", 0.15),
        ("mild", 0.30),
        ("strong", 0.50),
        ("extreme", 0.80),
    ];

    let mut players = Vec::new();
    for (label, sigma) in spec {
        for k in 0..2 {
            let name = format!("{label}-{k}");
            players.push(if sigma == 0.0 {
                SynthPlayer::null(&name, &base)
            } else {
                SynthPlayer::random(&name, &base, sigma, 0.25, &mut rng)
            });
        }
    }

    const GAMES_PER_PLAYER: usize = 120;
    const MAX_PLIES: usize = 80;
    println!("generating {} games...", players.len() * GAMES_PER_PLAYER);
    let mut games = corpus(&players, GAMES_PER_PLAYER, MAX_PLIES, &mut rng);

    // Shuffle before splitting. `corpus` emits games grouped by player, so a
    // split by index would give early players almost all the training data and
    // late players almost all the test data — confounding personality strength
    // with sample size. (Found by noticing the train/test columns were wildly
    // uneven across rows.)
    for i in (1..games.len()).rev() {
        games.swap(i, (rng.next() % (i as u64 + 1)) as usize);
    }

    // Three-way split. Train fits the models, validation picks how strongly to
    // regularise each personality, test is touched exactly once at the end.
    // Two splits rather than one because choosing a hyper-parameter on the test
    // set is just a slower way of fitting to it.
    let a = games.len() * 6 / 10;
    let b = games.len() * 8 / 10;
    let (train_games, rest) = games.split_at(a);
    let (val_games, test_games) = rest.split_at(b - a);
    println!(
        "{} games: {} train / {} validation / {} held out\n",
        games.len(),
        train_games.len(),
        val_games.len(),
        test_games.len()
    );

    // π_pop — everyone's training moves pooled.
    let mut pop_samples = Vec::new();
    for i in 0..players.len() {
        pop_samples.extend(samples_for(train_games, i));
    }
    println!(
        "fitting population model on {} decisions...",
        pop_samples.len()
    );
    let pop = train(
        &pop_samples,
        &Weights::zeros(),
        &TrainCfg {
            epochs: 6,
            lr: 0.08,
            l2: 0.0,
        },
    );

    // δ for each player, on their training moves only, with the regularisation
    // strength chosen on validation. "No personality" is always a candidate.
    println!("fitting {} personality offsets...\n", players.len());
    let lambdas = [3e-4f32, 1e-3, 3e-3, 1e-2, 3e-2, 1e-1];
    let fits: Vec<_> = (0..players.len())
        .map(|i| {
            fit_personality(
                &samples_for(train_games, i),
                &samples_for(val_games, i),
                &pop,
                &lambdas,
                10,
                0.05,
            )
        })
        .collect();
    let deltas: Vec<Weights> = fits.iter().map(|f| f.delta.clone()).collect();

    println!(
        "{:<10} {:>6} {:>6} {:>8} {:>9} {:>9} {:>9}",
        "player", "train", "test", "lambda", "D_true", "D_self", "D_cross"
    );
    println!("{}", "-".repeat(64));

    let mut rows = Vec::new();
    for i in 0..players.len() {
        let test = samples_for(test_games, i);
        if test.len() < 50 {
            continue;
        }

        let w_self = pop.plus(&deltas[i]);
        let ce_pop = cross_entropy(&test, &pop.0);
        let ce_self = cross_entropy(&test, &w_self.0);

        // Control: somebody else's personality, applied to your games. If this
        // is not ~0 the estimator is measuring something other than identity.
        let other = (i + 1) % players.len();
        let ce_cross = cross_entropy(&test, &pop.plus(&deltas[other]).0);

        let d_true = true_divergence(test_games, i, &players, &pop);
        let d_self = ce_pop - ce_self;
        let d_cross = ce_pop - ce_cross;

        println!(
            "{:<10} {:>6} {:>6} {:>8} {:>9.4} {:>9.4} {:>9.4}",
            players[i].name,
            samples_for(train_games, i).len(),
            test.len(),
            fits[i]
                .lambda
                .map_or("none".to_string(), |l| format!("{l:.0e}")),
            d_true,
            d_self,
            d_cross
        );
        rows.push((players[i].name.clone(), d_true, d_self, d_cross));
    }

    // The fabrication control. A "chimera" is a player-shaped pile of moves
    // with no coherent player behind it: each decision is drawn from a random
    // member of the population. Its true policy *is* the population mixture, so
    // its true divergence is ~0 by construction.
    //
    // If the estimator reports a positive number here, it is manufacturing
    // personality out of nothing and no figure it produces can be believed.
    {
        let mut crng = Lcg::new(0x0C41_BEEF_D20C_0DE5);
        let mut pick = |gs: &[SynthGame]| -> Vec<Sample> {
            let mut pool: Vec<Sample> = Vec::new();
            for i in 0..players.len() {
                for s in samples_for(gs, i) {
                    if crng.unit() < 1.0 / players.len() as f32 {
                        pool.push(s);
                    }
                }
            }
            pool
        };
        let (ctr, cva, cte) = (pick(train_games), pick(val_games), pick(test_games));
        let f = fit_personality(&ctr, &cva, &pop, &lambdas, 10, 0.05);
        let d = cross_entropy(&cte, &pop.0) - cross_entropy(&cte, &pop.plus(&f.delta).0);
        println!(
            "\nchimera control ({} train / {} test decisions, no real player behind them)",
            ctr.len(),
            cte.len()
        );
        println!(
            "  lambda = {:<6}  D = {d:+.4} nats/move   <- must be ~0",
            f.lambda.map_or("none".to_string(), |l| format!("{l:.0e}"))
        );
    }

    // Player identification: score each held-out game under every player's
    // model and take the best. Intuitive, and a much harder test than any
    // single number.
    println!();
    let mut correct = 0usize;
    let mut total = 0usize;
    for i in 0..players.len() {
        for g in test_games {
            if g.white != i && g.black != i {
                continue;
            }
            let s = samples_for(std::slice::from_ref(g), i);
            if s.len() < 10 {
                continue;
            }
            let best = (0..players.len())
                .map(|j| (j, cross_entropy(&s, &pop.plus(&deltas[j]).0)))
                .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                .unwrap()
                .0;
            correct += usize::from(best == i);
            total += 1;
        }
    }
    println!(
        "identification: {correct}/{total} = {:.1}%  (chance {:.1}%)",
        100.0 * correct as f64 / total as f64,
        100.0 / players.len() as f64
    );

    // How much data does a personality need before it can be seen at all?
    // This is the design parameter for any real run: how active must a player
    // be to be worth modelling.
    println!(
        "\nsample-size sweep (a 'mild' player, D_true = {:.4}):",
        rows.iter()
            .find(|r| r.0 == "mild-0")
            .map_or(f64::NAN, |r| r.1)
    );
    println!("{:>8} {:>10} {:>9}", "moves", "lambda", "D_self");
    let target = players.iter().position(|p| p.name == "mild-0").unwrap();
    let full_train = samples_for(train_games, target);
    let val = samples_for(val_games, target);
    let test = samples_for(test_games, target);
    for n in [250usize, 500, 1000, 2000, 3000, 4000, 5000] {
        if n > full_train.len() {
            break;
        }
        let f = fit_personality(&full_train[..n], &val, &pop, &lambdas, 10, 0.05);
        let d = cross_entropy(&test, &pop.0) - cross_entropy(&test, &pop.plus(&f.delta).0);
        println!(
            "{:>8} {:>10} {:>9.4}",
            n,
            f.lambda.map_or("none".to_string(), |l| format!("{l:.0e}")),
            d
        );
    }

    // Verdict.
    println!();
    let recov: Vec<f64> = rows.iter().map(|r| r.2 / r.1).collect();
    let mean_recov = recov.iter().sum::<f64>() / recov.len().max(1) as f64;
    let mean_cross = rows.iter().map(|r| r.3).sum::<f64>() / rows.len().max(1) as f64;
    println!(
        "recovered fraction of D_true : mean {:.0}%, range {:.0}-{:.0}%",
        100.0 * mean_recov,
        100.0 * recov.iter().cloned().fold(f64::INFINITY, f64::min),
        100.0 * recov.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
    );
    println!("cross-player D               : {mean_cross:+.4} nats/move  (must be < 0)");
    println!();
    println!("The estimate is a LOWER BOUND, and undershoots for two reasons that");
    println!("both push the same way: the feature set cannot express everything a");
    println!("real policy does, and the regulariser shrinks delta toward zero. A");
    println!("measurement on real games is therefore conservative — which is the");
    println!("direction you want a number you intend to build an economy on.");
}
