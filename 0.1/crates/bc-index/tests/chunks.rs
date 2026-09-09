//! Does the chunk finder find a chunk that is there, and stay quiet when one
//! is not?
//!
//! Same discipline as the X11 fixtures: validate the instrument against known
//! answers before pointing it at anything real, so that a null on real data
//! is a null rather than a broken tool.

use bc_chess::{Color, Piece, Position};
use bc_index::chunks::{discover, fact, Counts};

/// Build a position from (colour, piece, square-name) triples.
fn make(men: &[(Color, Piece, &str)]) -> Position {
    let mut p = Position::empty();
    for &(c, piece, name) in men {
        let b = name.as_bytes();
        let sq = (b[1] - b'1') * 8 + (b[0] - b'a');
        p.color_bb[c.idx()] |= 1u64 << sq;
        p.piece_bb[piece.idx()] |= 1u64 << sq;
    }
    p
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn pick<'a, T>(&mut self, v: &'a [T]) -> &'a T {
        &v[(self.next() % v.len() as u64) as usize]
    }
}

const NOISE: [&str; 12] = [
    "a4", "b4", "c4", "d4", "e4", "f4", "a5", "b5", "c5", "d5", "e5", "f5",
];

#[test]
fn a_planted_configuration_is_recovered() {
    // Half the positions contain a castled kingside: king g1 with pawns on
    // f2, g2, h2 and a rook on f1. The other half do not. Everything else is
    // scattered noise so the group has to be found against a background.
    let mut rng = Rng(0xC0FFEE);
    let mut counts = Counts::default();
    let castled = [
        (Color::White, Piece::King, "g1"),
        (Color::White, Piece::Rook, "f1"),
        (Color::White, Piece::Pawn, "f2"),
        (Color::White, Piece::Pawn, "g2"),
        (Color::White, Piece::Pawn, "h2"),
    ];

    for i in 0..4000 {
        let mut men: Vec<(Color, Piece, &str)> = Vec::new();
        if i % 2 == 0 {
            men.extend_from_slice(&castled);
        } else {
            // Uncastled: king stays home, rook on the other side.
            men.push((Color::White, Piece::King, "e1"));
            men.push((Color::White, Piece::Rook, "a1"));
        }
        for _ in 0..3 {
            men.push((Color::Black, Piece::Knight, *rng.pick(&NOISE[..])));
        }
        men.push((Color::Black, Piece::King, "e8"));
        counts.observe(&make(&men), true);
    }

    let found = discover(&counts, 200, 6, 8);
    assert!(!found.is_empty(), "found nothing at all");

    // Some chunk must contain the whole planted group.
    let want: Vec<u16> = castled
        .iter()
        .map(|&(c, p, n)| {
            let b = n.as_bytes();
            fact(c, p, (b[1] - b'1') * 8 + (b[0] - b'a'))
        })
        .collect();
    let hit = found
        .iter()
        .any(|c| want.iter().all(|f| c.facts.contains(f)));
    assert!(
        hit,
        "the planted castled kingside was not recovered; got {:?}",
        found
            .iter()
            .map(|c| c
                .facts
                .iter()
                .map(|&f| bc_index::chunks::fact_name(f))
                .collect::<Vec<_>>())
            .collect::<Vec<_>>()
    );
}

#[test]
fn independent_pieces_produce_no_high_lift_group() {
    // The control. Every man is placed independently, so nothing co-occurs
    // beyond chance and any "chunk" found would be the finder hallucinating.
    let mut rng = Rng(999);
    let mut counts = Counts::default();
    for _ in 0..4000 {
        let mut men: Vec<(Color, Piece, &str)> = vec![(Color::Black, Piece::King, "e8")];
        for _ in 0..5 {
            men.push((Color::White, Piece::Knight, *rng.pick(&NOISE[..])));
        }
        counts.observe(&make(&men), true);
    }
    let found = discover(&counts, 200, 6, 8);
    for c in &found {
        assert!(
            c.lift() < 4.0,
            "found lift {:.1}x in independent placements: {:?}",
            c.lift(),
            c.facts
                .iter()
                .map(|&f| bc_index::chunks::fact_name(f))
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn a_group_below_the_support_floor_is_not_reported() {
    // Rare things must not surface however well they correlate: lift computed
    // on a handful of positions is noise with a big number attached.
    let mut counts = Counts::default();
    for i in 0..2000 {
        let mut men: Vec<(Color, Piece, &str)> = vec![(Color::Black, Piece::King, "e8")];
        if i < 10 {
            men.push((Color::White, Piece::Queen, "a1"));
            men.push((Color::White, Piece::Queen, "h8"));
        } else {
            men.push((Color::White, Piece::Knight, "d4"));
        }
        counts.observe(&make(&men), true);
    }
    let found = discover(&counts, 100, 4, 8);
    let rare = fact(Color::White, Piece::Queen, 0); // a1
    assert!(
        !found.iter().any(|c| c.facts.contains(&rare)),
        "a 10-position group cleared a 100-position floor"
    );
}
