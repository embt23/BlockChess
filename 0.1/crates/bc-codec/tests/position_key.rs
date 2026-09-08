//! The packed position and the symmetry group.
//!
//! `papers/01-position-space.md` §5 derived which symmetries survive the rules
//! of chess, and `measure/symmetry.py` measured what they are worth. This file
//! checks the Rust agrees with both. That agreement is the point: the Python
//! and the Rust were written from the same paper but not from each other, so
//! when they produce the same group orders and the same collapse factors,
//! that is two independent implementations of an argument, not one
//! implementation tested against itself.

use bc_chess::Position;
use bc_codec::position::{canonical_key, ep_is_real, pack, surviving_group, unpack};

fn pos(fen: &str) -> Position {
    Position::from_fen(fen).unwrap_or_else(|e| panic!("bad fen {fen}: {e}"))
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
}

#[test]
fn pack_round_trips_through_a_real_game() {
    let mut rng = Rng(0xA5A5_1234_9999_0001);
    let mut p = Position::startpos();
    for _ in 0..400 {
        let back = unpack(&pack(&p)).expect("unpack");
        // Everything except the clocks, which are deliberately not encoded.
        assert_eq!(back.color_bb, p.color_bb, "men moved in {}", p.to_fen());
        assert_eq!(back.piece_bb, p.piece_bb);
        assert_eq!(back.side, p.side);
        assert_eq!(back.castling, p.castling);
        assert_eq!(back.ep.is_some(), ep_is_real(&p), "ep canonicality broke");

        let legal = p.generate_legal();
        if legal.is_empty() {
            p = Position::startpos();
            continue;
        }
        let all = legal.as_slice();
        p = p.make_move(all[(rng.next() % all.len() as u64) as usize]);
    }
}

#[test]
fn a_fictional_en_passant_square_does_not_change_the_key() {
    // This is the canonicality trap from papers/03-corpus.md. Most FEN writers
    // set the ep target whenever a pawn made a double step, whether or not any
    // capture is available. If that leaked into the key, the same position
    // would index under two keys and the transposition index would silently
    // split a line in half.
    //
    // 1.e4 with no black pawn able to take: the ep square is noise.
    let with = pos("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1");
    let without = pos("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1");
    assert!(!ep_is_real(&with), "no black pawn can capture on e3");
    assert_eq!(
        pack(&with),
        pack(&without),
        "phantom ep changed the packing"
    );

    // And when the capture is real, it must be kept: these are different
    // positions and must not collide.
    let real = pos("rnbqkbnr/pppp1ppp/8/8/3pP3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1");
    let real_no_ep = pos("rnbqkbnr/pppp1ppp/8/8/3pP3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1");
    assert!(ep_is_real(&real), "d4 pawn can take on e3");
    assert_ne!(pack(&real), pack(&real_no_ep));
}

#[test]
fn the_halfmove_clock_is_not_in_the_key() {
    // papers/05-index.md §3: someone studying a structure does not care that
    // one game arrived with the clock at 4 and another at 11.
    let a = pos("4k3/8/8/8/8/8/8/4K2R w K - 4 20");
    let b = pos("4k3/8/8/8/8/8/8/4K2R w K - 11 60");
    assert_eq!(pack(&a), pack(&b));
}

#[test]
fn group_orders_match_the_paper() {
    // papers/01-position-space.md §5, and measure/symmetry.py prints the same
    // four numbers.
    assert_eq!(
        surviving_group(&pos("4k3/8/8/8/8/8/8/4K2R w K - 0 1")).len(),
        2,
        "castling rights outstanding: only the colour swap survives"
    );
    assert_eq!(
        surviving_group(&pos("4k3/8/8/4P3/8/8/8/4K3 w - - 0 1")).len(),
        4,
        "pawns, no castling: file mirror x colour swap"
    );
    assert_eq!(
        surviving_group(&pos("4k3/8/8/8/8/8/8/4K2R w - - 0 1")).len(),
        16,
        "pawnless, no castling: D4 x colour swap"
    );
}

#[test]
fn mirror_images_share_one_key() {
    // The a-file and h-file versions of the same idea are the same idea.
    //
    // The trap, worth writing down because it caught the first version of this
    // test: the file mirror has **no fixed square**. `f -> 7-f` would need
    // file 3.5 to hold still, so mirroring a position moves *everything*,
    // kings included. e-file goes to d-file. So the mirror of "rook a1, kings
    // on e1 and e8" is "rook h1, kings on d1 and d8" — not "rook h1, kings
    // still on e". That fixed-point-free-ness is also exactly why
    // papers/01-position-space.md §6 measures the saving as exactly 1.000
    // bits: no position is ever its own mirror image, so every orbit has
    // precisely two elements.
    let left = pos("4k3/8/8/8/8/8/8/R3K3 w - - 0 1");
    let right = pos("3k4/8/8/8/8/8/8/3K3R w - - 0 1");
    assert_eq!(
        canonical_key(&left).0,
        canonical_key(&right).0,
        "a position and its file mirror must index together"
    );
    // But the raw packings differ, which is what makes the canonicalisation
    // do real work rather than being a no-op.
    assert_ne!(pack(&left), pack(&right));
}

#[test]
fn castling_rights_block_the_file_mirror() {
    // With rights outstanding the mirror is not a chess position, so these two
    // must NOT collapse. Getting this wrong would merge kingside and queenside
    // castling lines, which are different openings.
    let left = pos("r3k3/8/8/8/8/8/8/R3K3 w Qq - 0 1");
    let right = pos("4k2r/8/8/8/8/8/8/4K2R w Kk - 0 1");
    assert_ne!(
        canonical_key(&left).0,
        canonical_key(&right).0,
        "castling lines must stay distinct"
    );
}

#[test]
fn colour_swap_collapses_a_position_and_its_reflection() {
    // White rook on a1 with white to move, versus black rook on a8 with black
    // to move: the same position seen from the other side.
    let w = pos("4k3/8/8/8/8/8/8/R3K3 w - - 0 1");
    let b = pos("r3k3/8/8/8/8/8/8/4K3 b - - 0 1");
    assert_eq!(canonical_key(&w).0, canonical_key(&b).0);
}

#[test]
fn canonicalisation_is_idempotent_and_stable() {
    // Applying the canonical map to an already-canonical position must not
    // move it, or the index key depends on which representative you started
    // from — which defeats the entire purpose.
    let mut rng = Rng(777_777);
    let mut p = Position::startpos();
    for _ in 0..300 {
        let (key, _) = canonical_key(&p);
        let recovered = unpack(&key).expect("canonical key must be unpackable");
        let (again, _) = canonical_key(&recovered);
        assert_eq!(
            key,
            again,
            "canonicalisation is not idempotent at {}",
            p.to_fen()
        );

        let legal = p.generate_legal();
        if legal.is_empty() {
            break;
        }
        let all = legal.as_slice();
        p = p.make_move(all[(rng.next() % all.len() as u64) as usize]);
    }
}

#[test]
fn the_measured_collapse_matches_the_python() {
    // measure/symmetry.py reports 1.000 bits saved for the KP vs K family --
    // exactly a factor of two, because the file mirror has no fixed square and
    // so no position is ever its own mirror image. Reproduce that here by
    // counting distinct canonical keys.
    //
    // Kings plus one white pawn, over every legal placement, both sides to
    // move. If the Rust and the Python disagree, one of them has misread the
    // paper.
    let mut raw = std::collections::HashSet::new();
    let mut canon = std::collections::HashSet::new();

    for wk in 0..64u8 {
        for bk in 0..64u8 {
            if wk == bk || kings_adjacent(wk, bk) {
                continue;
            }
            for pawn in 8..56u8 {
                if pawn == wk || pawn == bk {
                    continue;
                }
                for side in ["w", "b"] {
                    let fen = build_fen(wk, bk, pawn, side);
                    let Ok(p) = Position::from_fen(&fen) else {
                        continue;
                    };
                    raw.insert(pack(&p));
                    canon.insert(canonical_key(&p).0);
                }
            }
        }
    }

    let ratio = raw.len() as f64 / canon.len() as f64;
    assert!(
        (ratio - 2.0).abs() < 1e-9,
        "expected exactly 2x collapse (1.000 bits, as measure/symmetry.py reports), got {ratio} \
         from {} raw and {} canonical",
        raw.len(),
        canon.len()
    );
}

fn kings_adjacent(a: u8, b: u8) -> bool {
    let (ar, af) = ((a / 8) as i32, (a % 8) as i32);
    let (br, bf) = ((b / 8) as i32, (b % 8) as i32);
    (ar - br).abs() <= 1 && (af - bf).abs() <= 1
}

fn build_fen(wk: u8, bk: u8, pawn: u8, side: &str) -> String {
    let mut board = [[' '; 8]; 8];
    board[(wk / 8) as usize][(wk % 8) as usize] = 'K';
    board[(bk / 8) as usize][(bk % 8) as usize] = 'k';
    board[(pawn / 8) as usize][(pawn % 8) as usize] = 'P';
    let mut rows = Vec::new();
    for rank in board.iter().rev() {
        let mut row = String::new();
        let mut gap = 0;
        for &square in rank.iter() {
            if square == ' ' {
                gap += 1;
            } else {
                if gap > 0 {
                    row.push_str(&gap.to_string());
                    gap = 0;
                }
                row.push(square);
            }
        }
        if gap > 0 {
            row.push_str(&gap.to_string());
        }
        rows.push(row);
    }
    format!("{} {} - - 0 1", rows.join("/"), side)
}
