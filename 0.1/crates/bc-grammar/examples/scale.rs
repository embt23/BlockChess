//! Does the inducer finish at corpus scale?
//!
//! `src/repair.rs` claims it does. This measures it, because a performance
//! claim nobody has run is not a result.
//!
//! The corpus here is **synthetic and shaped like chess** — many sequences
//! sharing prefixes drawn from a skewed popularity distribution, then
//! diverging into noise. It says nothing whatever about chess, and its
//! compression ratio is meaningless because its tails are random. It measures
//! exactly one thing: wall-clock time against corpus size.
//!
//! ```sh
//! cargo run --release -p bc-grammar --example scale -- 200000
//! ```

fn main() {
    // Synthetic corpus shaped like chess: many games sharing opening prefixes
    // drawn from a Zipf-ish popularity distribution, then diverging.
    // This measures whether the inducer FINISHES at scale. It says nothing
    // about chess.
    let mut x: u64 = 88172645463325252;
    let mut rng = || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x
    };
    let books: Vec<Vec<u32>> = (0..400)
        .map(|i| {
            (0..(6 + i % 20))
                .map(|j| ((i * 37 + j * 11) % 1800) as u32)
                .collect()
        })
        .collect();
    let n_games: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(100_000);
    let seqs: Vec<Vec<u32>> = (0..n_games)
        .map(|_| {
            let r = rng() % 1000;
            let b = if r < 500 { 0 } else { (r % 400) as usize };
            let mut s = books[b].clone();
            for _ in 0..(40 + rng() % 60) {
                s.push((rng() % 1800) as u32);
            }
            s
        })
        .collect();
    let total: usize = seqs.iter().map(|s| s.len()).sum();
    let t = std::time::Instant::now();
    let g = bc_grammar::induce(
        &seqs,
        bc_grammar::Limits {
            min_occurrences: 20,
            max_rules: 20000,
        },
    );
    let el = t.elapsed().as_secs_f64();
    println!(
        "{:>10} games  {:>10} symbols  {:>8.2}s  {:>7} rules  {} -> {} ({:.2}x)",
        n_games,
        total,
        el,
        g.rules.len(),
        total,
        g.total_size(),
        total as f64 / g.total_size() as f64
    );
}
