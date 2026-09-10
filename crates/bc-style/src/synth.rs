//! Synthetic players, so the pipeline has ground truth.
//!
//! The lab's central claim is that games cluster by who played them. That claim
//! is testable only if you already know the answer — so here are players whose
//! styles are *defined* rather than inferred, and whose games the rest of the
//! crate must be able to sort back out. If four constructed personalities do not
//! separate, the pipeline is broken, and no amount of real data would tell you
//! that as clearly.
//!
//! This is the same move `bc-chess` makes with `perft` and `bc-sig` with RFC
//! 8032: check against something whose answer is known in advance.

use crate::corpus::GameRecord;
use bc_chess::types::{file_of, rank_of, Color, Piece, FLAG_EP};
use bc_chess::{Move, Position};

/// A deterministic generator. Not cryptographic; it seeds a fixture.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// What a synthetic player likes. Each weight scores a property of a move; the
/// player picks the highest-scoring legal move, with noise.
#[derive(Clone, Debug)]
pub struct Style {
    pub name: &'static str,
    pub capture: f64,
    pub check: f64,
    pub center: f64,
    pub advance: f64,
    pub knight: f64,
    pub bishop: f64,
    pub queen: f64,
    pub pawn: f64,
    /// Positive rewards *restricting* the opponent — prophylaxis.
    pub restrict: f64,
    pub noise: f64,
}

/// Four deliberately different players.
pub const ARCHETYPES: [Style; 4] = [
    Style {
        name: "Tal",
        capture: 2.2,
        check: 2.6,
        center: 0.6,
        advance: 1.4,
        knight: 0.5,
        bishop: 0.3,
        queen: 0.9,
        pawn: -0.4,
        restrict: 0.0,
        noise: 0.5,
    },
    Style {
        name: "Petrosian",
        capture: -0.5,
        check: -0.6,
        center: 0.5,
        advance: -0.9,
        knight: 0.6,
        bishop: 0.0,
        queen: -0.8,
        pawn: 0.5,
        restrict: 2.4,
        noise: 0.5,
    },
    Style {
        name: "Capablanca",
        capture: 1.1,
        check: -0.2,
        center: 1.6,
        advance: 0.2,
        knight: 0.2,
        bishop: 0.6,
        queen: -0.3,
        pawn: 0.1,
        restrict: 0.7,
        noise: 0.5,
    },
    Style {
        name: "Morphy",
        capture: 0.9,
        check: 1.2,
        center: 1.2,
        advance: 1.8,
        knight: 0.4,
        bishop: 1.1,
        queen: 0.2,
        pawn: -0.6,
        restrict: -0.4,
        noise: 0.5,
    },
];

fn score(style: &Style, pos: &Position, m: Move, rng: &mut Rng) -> f64 {
    let mut s = style.noise * rng.unit();
    let side = pos.side;

    if m.flag() == FLAG_EP || pos.piece_at(m.to()).is_some() {
        s += style.capture;
    }
    if let Some((_, p)) = pos.piece_at(m.from()) {
        s += match p {
            Piece::Knight => style.knight,
            Piece::Bishop => style.bishop,
            Piece::Queen => style.queen,
            Piece::Pawn => style.pawn,
            _ => 0.0,
        };
    }
    let (f, r) = (file_of(m.to()), rank_of(m.to()));
    if (2..=5).contains(&f) && (2..=5).contains(&r) {
        s += style.center;
    }
    let forward = match side {
        Color::White => rank_of(m.to()) as i32 - rank_of(m.from()) as i32,
        Color::Black => rank_of(m.from()) as i32 - rank_of(m.to()) as i32,
    };
    s += style.advance * (forward as f64) * 0.25;

    // Properties that need the resulting position.
    if style.check != 0.0 || style.restrict != 0.0 {
        let after = pos.make_move(m);
        if style.check != 0.0 && after.in_check(side.flip()) {
            s += style.check;
        }
        if style.restrict != 0.0 {
            // Fewer replies for the opponent is better, scaled to roughly the
            // same magnitude as the other terms.
            s -= style.restrict * (after.generate_legal().len() as f64) / 30.0;
        }
    }
    s
}

/// Play one game between two styles.
pub fn play(white: &Style, black: &Style, rng: &mut Rng, max_plies: usize) -> GameRecord {
    let start = Position::startpos();
    let mut pos = start;
    let mut moves = Vec::new();

    for _ in 0..max_plies {
        let legal = pos.generate_legal();
        if legal.is_empty() || pos.halfmove >= 100 {
            break;
        }
        let style = if pos.side == Color::White {
            white
        } else {
            black
        };
        let mut best = legal.as_slice()[0];
        let mut best_score = f64::NEG_INFINITY;
        for &m in legal.as_slice() {
            let s = score(style, &pos, m, rng);
            if s > best_score {
                best_score = s;
                best = m;
            }
        }
        pos = pos.make_move(best);
        moves.push(best);
    }

    GameRecord {
        white: white.name.to_string(),
        black: black.name.to_string(),
        start,
        moves,
        white_elo: None,
        black_elo: None,
    }
}

/// A round-robin: every archetype plays every other, both colours, `rounds`
/// times each.
pub fn round_robin(rounds: usize, seed: u64, max_plies: usize) -> Vec<GameRecord> {
    let mut rng = Rng::new(seed);
    let mut out = Vec::new();
    for _ in 0..rounds {
        for (i, white) in ARCHETYPES.iter().enumerate() {
            for (j, black) in ARCHETYPES.iter().enumerate() {
                if i != j {
                    out.push(play(white, black, &mut rng, max_plies));
                }
            }
        }
    }
    out
}
