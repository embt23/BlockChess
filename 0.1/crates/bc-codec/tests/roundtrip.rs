//! `decode(encode(g)) == g`, over as many games as can be generated.
//!
//! `papers/06-decisions.md` D10 calls this the perft of the storage layer.
//! Perft's virtue is that it is exact and mechanical over millions of cases
//! rather than over the handful a human thought to write down, and the way to
//! get that here is to generate the games rather than author them: a
//! deterministic pseudo-random walk through legal moves reaches promotions,
//! en passant, castling, stalemate and the fifty-move rule without anyone
//! having to remember them.

use bc_chess::{Move, Position};
use bc_codec::{decode, encode, Scheme};

/// xorshift64*. Deterministic, so a failure reproduces from its seed alone.
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

/// One random legal game from the starting position.
fn playout(rng: &mut Rng, max_plies: usize) -> Vec<Move> {
    let mut pos = Position::startpos();
    let mut moves = Vec::new();
    for _ in 0..max_plies {
        let legal = bc_codec::order::canonical_moves(&pos);
        if legal.is_empty() || pos.halfmove >= 100 {
            break;
        }
        let m = legal[(rng.next() % legal.len() as u64) as usize];
        moves.push(m);
        pos = pos.make_move(m);
    }
    moves
}

fn check(scheme: Scheme, start: &Position, moves: &[Move]) {
    let bytes = encode(scheme, start, moves).expect("encode");
    let back = decode(scheme, start, &bytes, moves.len()).expect("decode");
    assert_eq!(
        back,
        moves,
        "{} round trip changed the game from {}",
        scheme.name(),
        start.to_fen()
    );
}

#[test]
fn round_trip_random_games() {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    for _ in 0..300 {
        let moves = playout(&mut rng, 200);
        check(Scheme::Fixed, &Position::startpos(), &moves);
        check(Scheme::Index, &Position::startpos(), &moves);
    }
}

#[test]
fn round_trip_empty_game() {
    // A game with no moves must encode to nothing and decode back to nothing.
    // The degenerate case is worth pinning because the mixed-radix integer for
    // it is zero, which has an empty canonical byte string.
    let start = Position::startpos();
    for scheme in [Scheme::Fixed, Scheme::Index] {
        let bytes = encode(scheme, &start, &[]).unwrap();
        assert!(
            bytes.is_empty(),
            "{} should encode nothing to nothing",
            scheme.name()
        );
        assert!(decode(scheme, &start, &bytes, 0).unwrap().is_empty());
    }
}

#[test]
fn e7_is_smaller_than_e3_and_hits_the_theoretical_size() {
    let mut rng = Rng(12345);
    let mut e3_total = 0usize;
    let mut e7_total = 0usize;

    for _ in 0..50 {
        let moves = playout(&mut rng, 120);
        if moves.len() < 20 {
            continue;
        }
        let start = Position::startpos();
        let e3 = encode(Scheme::Fixed, &start, &moves).unwrap();
        let e7 = encode(Scheme::Index, &start, &moves).unwrap();
        e3_total += e3.len();
        e7_total += e7.len();

        // The mixed-radix payload must be exactly ceil(Σ log2 b / 8) bytes.
        // This is the claim in the bc-codec docs that E7 has no rounding waste
        // except once per game, and it is checkable rather than assertable.
        let bits = bc_codec::index_bits(&start, &moves);
        let expected = (bits / 8.0).ceil() as usize;
        assert!(
            e7.len() <= expected,
            "E7 spent {} bytes where {:.2} bits ({} bytes) suffice",
            e7.len(),
            bits,
            expected
        );
    }
    assert!(
        e7_total * 2 < e3_total,
        "E7 should be well over 2x smaller than E3: {e7_total} vs {e3_total}"
    );
}

#[test]
fn decode_rejects_non_canonical_trailing_data() {
    // Two byte strings must never decode to the same game. Appending a
    // non-zero byte to a valid payload has to be an error, not silently
    // ignored -- otherwise the encoding is not canonical and a game has many
    // spellings. papers/02-encodings.md requires exactly one.
    let mut rng = Rng(777);
    let moves = playout(&mut rng, 40);
    let start = Position::startpos();
    let mut bytes = encode(Scheme::Index, &start, &moves).unwrap();
    bytes.push(0xFF);
    assert!(
        decode(Scheme::Index, &start, &bytes, moves.len()).is_err(),
        "trailing data must be rejected"
    );
}

#[test]
fn encode_refuses_an_illegal_move() {
    // The encoder is the first place a fabricated game is caught: a move that
    // is not in the legal list has no index, so there is nothing to write.
    let start = Position::startpos();
    let nonsense = Move::normal(0, 63); // a1 to h8, which is not a chess move
    assert!(encode(Scheme::Index, &start, &[nonsense]).is_err());
}
