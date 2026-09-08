//! The published perft suite. These numbers are an external oracle — every
//! chess engine ever written agrees on them.

use bc_chess::{perft, Position};

/// One row: name, FEN, and the published (depth, node count) pairs.
type PerftCase = (&'static str, &'static str, &'static [(u32, u64)]);

const SUITE: &[PerftCase] = &[
    (
        "startpos",
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        &[(1, 20), (2, 400), (3, 8_902), (4, 197_281), (5, 4_865_609)],
    ),
    (
        // "Kiwipete" — dense with castling, pins and promotions. The single
        // best position for shaking out generator bugs.
        "kiwipete",
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        &[(1, 48), (2, 2_039), (3, 97_862), (4, 4_085_603)],
    ),
    (
        // Sparse, but full of en-passant and promotion edge cases, including
        // the en-passant-discovers-check case that breaks naive generators.
        "position3",
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        &[
            (1, 14),
            (2, 191),
            (3, 2_812),
            (4, 43_238),
            (5, 674_624),
            (6, 11_030_083),
        ],
    ),
    (
        "position4",
        "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
        &[(1, 6), (2, 264), (3, 9_467), (4, 422_333), (5, 15_833_292)],
    ),
    (
        "position5",
        "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
        &[(1, 44), (2, 1_486), (3, 62_379), (4, 2_103_487)],
    ),
    (
        "position6",
        "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
        &[(1, 46), (2, 2_079), (3, 89_890), (4, 3_894_594)],
    ),
];

#[test]
fn perft_suite() {
    for (name, fen, cases) in SUITE {
        let pos = Position::from_fen(fen).expect(name);
        for &(depth, expected) in *cases {
            let got = perft(&pos, depth);
            assert_eq!(
                got, expected,
                "{name} perft({depth}): got {got}, want {expected}"
            );
        }
    }
}

/// perft(6) from the start: 119,060,324 nodes. Slow, so it is opt-in.
/// Run with `cargo test --release -- --ignored`.
#[test]
#[ignore = "slow; run with --release --ignored"]
fn perft_startpos_depth_6() {
    let pos = Position::startpos();
    assert_eq!(perft(&pos, 6), 119_060_324);
}

#[test]
#[ignore = "slow; run with --release --ignored"]
fn perft_kiwipete_depth_5() {
    let pos = Position::from_fen(SUITE[1].1).unwrap();
    assert_eq!(perft(&pos, 5), 193_690_690);
}

#[test]
fn fen_roundtrips() {
    for (name, fen, _) in SUITE {
        let p = Position::from_fen(fen).expect(name);
        assert_eq!(&p.to_fen(), fen, "{name}");
    }
}
