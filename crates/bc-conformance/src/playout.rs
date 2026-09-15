//! Random legal games, reproducibly.
//!
//! A differential test is only as good as the positions it reaches, and
//! hand-chosen positions reach exactly the cases their author already thought
//! of — which is the failure mode `G1` exists to catch. Random playouts reach
//! the cases nobody thought of, and endgames in particular: a game played to
//! its end walks through the material distribution where insufficient-material
//! and fifty-move actually fire.
//!
//! Determinism matters more than statistical quality here. A differential
//! failure is only useful if it reproduces, so the generator is a fixed
//! xorshift seeded per game and there is no thread, clock or entropy source
//! anywhere in it.

use bc_chess::{Move, Position};

/// xorshift64*, chosen because it is four lines and the test does not need
/// more. Not for anything that matters cryptographically.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        // Zero is the fixed point of xorshift; nudge it rather than trusting
        // every caller to avoid it.
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform in `0..n`. Modulo bias is irrelevant at these magnitudes — `n`
    /// is a legal-move count, at most 218.
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

/// One random legal game, yielded position by position.
///
/// Stops at the first position with no legal move, or at `max_plies`. It does
/// **not** stop on insufficient material or the fifty-move clock, because
/// those are the predicates under test — halting on them would hide exactly
/// the disagreement the test is looking for, and playing on past them is
/// legal chess anyway.
pub struct Playout {
    pos: Position,
    rng: Rng,
    left: usize,
    done: bool,
}

impl Playout {
    pub fn new(seed: u64, max_plies: usize) -> Playout {
        Playout::from(Position::startpos(), seed, max_plies)
    }

    pub fn from(pos: Position, seed: u64, max_plies: usize) -> Playout {
        Playout {
            pos,
            rng: Rng::new(seed),
            left: max_plies,
            done: false,
        }
    }
}

impl Iterator for Playout {
    /// The position, and the move played from it — `None` at the last
    /// position, which is the one with no legal reply.
    type Item = (Position, Option<Move>);

    fn next(&mut self) -> Option<(Position, Option<Move>)> {
        if self.done {
            return None;
        }
        let here = self.pos;
        let legal = here.generate_legal();
        if legal.is_empty() || self.left == 0 {
            self.done = true;
            return Some((here, None));
        }
        let mv = legal.as_slice()[self.rng.below(legal.len())];
        self.pos = here.make_move(mv);
        self.left -= 1;
        Some((here, Some(mv)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_playout_is_reproducible() {
        let a: Vec<_> = Playout::new(7, 40).filter_map(|(_, m)| m).collect();
        let b: Vec<_> = Playout::new(7, 40).filter_map(|(_, m)| m).collect();
        assert_eq!(a, b);
        assert_ne!(
            a,
            Playout::new(8, 40)
                .filter_map(|(_, m)| m)
                .collect::<Vec<_>>()
        );
    }

    /// The last position yielded is the only one allowed to have no move, and
    /// every move yielded was legal in the position it was yielded with.
    #[test]
    fn a_playout_only_plays_legal_moves() {
        let steps: Vec<_> = Playout::new(3, 60).collect();
        for (i, (pos, mv)) in steps.iter().enumerate() {
            match mv {
                Some(m) => assert!(pos.is_move_legal(*m), "illegal move at ply {i}"),
                None => assert_eq!(i, steps.len() - 1, "stopped early at ply {i}"),
            }
        }
    }

    /// Random play from the start position reaches endgames within a few
    /// hundred plies. If it did not, the differential test would be checking
    /// the opening only and insufficient material would never fire.
    #[test]
    fn random_play_reaches_positions_with_little_material() {
        let fewest = (0..20)
            .map(|s| {
                Playout::new(s, 400)
                    .map(|(p, _)| p.occupied().count_ones())
                    .min()
                    .unwrap()
            })
            .min()
            .unwrap();
        assert!(fewest <= 6, "thinnest position had {fewest} pieces");
    }
}
