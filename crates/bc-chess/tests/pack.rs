//! The packed position encoding, checked for canonicality.
//!
//! There is no published test vector for *our* wire format, so the oracle here
//! is FEN — a format someone else specified — plus the position tree that
//! perft already walks. Two properties matter and both are tested against
//! positions the move generator produces rather than positions we invented:
//!
//! 1. **Round trip.** `unpack(pack(p))` is `p`, for every position reachable
//!    in the first few plies of five standard test positions.
//! 2. **Canonicality.** Positions that are equal *as positions* pack to
//!    identical bytes, and every non-canonical encoding is rejected.
//!
//! Property 2 is the one with money on it: the packed bytes are hashed, and
//! the hash is what a threefold-repetition claim compares.

use bc_chess::pack::unpack;
use bc_chess::{Move, Position};

/// The five positions the chess-programming community uses to localise
/// movegen bugs. Kiwipete and positions 3–5 between them exercise castling,
/// en passant, promotion, and pins.
const SUITE: [&str; 5] = [
    Position::START_FEN,
    "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
    "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
    "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
    "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
];

/// Everything about a position that the packed form is required to preserve.
/// Not `fullmove`: two positions differing only in move number are the same
/// position for repetition, so the encoding deliberately drops it.
fn canonical(p: &Position) -> (u64, [u64; 6], [u64; 2], u8, u8, Option<u8>, u8) {
    (
        p.occupied(),
        p.piece_bb,
        p.color_bb,
        p.side as u8,
        p.castling,
        p.ep_effective(),
        p.halfmove,
    )
}

fn walk(p: &Position, depth: u32, visit: &mut impl FnMut(&Position)) {
    visit(p);
    if depth == 0 {
        return;
    }
    for &m in &p.generate_legal() {
        walk(&p.make_move(m), depth - 1, visit);
    }
}

#[test]
fn round_trips_over_the_perft_tree() {
    let mut n = 0u64;
    for fen in SUITE {
        let root = Position::from_fen(fen).unwrap();
        walk(&root, 3, &mut |p| {
            n += 1;
            let packed = p.pack();
            assert!(
                packed.len() <= bc_chess::MAX_PACKED_LEN,
                "packed {} bytes in {}",
                packed.len(),
                p.to_fen()
            );
            let back = unpack(packed.as_slice())
                .unwrap_or_else(|e| panic!("{e} for {} ({packed:?})", p.to_fen()));
            assert_eq!(
                canonical(&back),
                canonical(p),
                "round trip changed {}",
                p.to_fen()
            );
            // Packing is a function: the same position always gives the same
            // bytes, which is what makes the hash comparable.
            assert_eq!(back.pack(), packed, "not idempotent: {}", p.to_fen());
        });
    }
    // Cheap guard against the walk silently doing nothing.
    assert!(n > 100_000, "only visited {n} positions");
}

#[test]
fn startpos_is_exactly_26_bytes() {
    // 32 pieces is the maximum, so the opening position is the longest
    // encoding that can exist: 8 occupancy + 16 nibbles + 2 flags.
    assert_eq!(Position::startpos().pack().len(), 26);
}

#[test]
fn a_bare_kings_endgame_is_eleven_bytes() {
    // Two pieces is two nibbles, which still costs a whole byte: 8 + 1 + 2.
    let p = Position::from_fen("8/8/4k3/8/8/4K3/8/8 w - - 0 1").unwrap();
    assert_eq!(p.pack().len(), 11);
}

// ---------------------------------------------------------------------------
// Canonicality
// ---------------------------------------------------------------------------

#[test]
fn unreachable_ep_is_not_recorded() {
    // After 1.e4 the ep target is e3, but no black pawn is anywhere near it.
    // The position is the same position as one with no ep square at all, and
    // must therefore have the same bytes.
    let after_e4 = Position::startpos().make_move(Move::normal(12, 28));
    assert_eq!(after_e4.ep, Some(20), "make_move should set the raw target");
    assert_eq!(after_e4.ep_effective(), None, "but nothing can capture it");

    let stated = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1";
    let omitted = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1";
    assert_eq!(
        Position::from_fen(stated).unwrap().pack(),
        Position::from_fen(omitted).unwrap().pack()
    );
    assert_eq!(after_e4.pack(), Position::from_fen(omitted).unwrap().pack());
}

#[test]
fn available_ep_is_recorded() {
    let fen = "rnbqkbnr/ppp1p1pp/8/3pPp2/8/8/PPPP1PPP/RNBQKBNR w KQkq f6 0 3";
    let p = Position::from_fen(fen).unwrap();
    assert_eq!(p.ep_effective(), Some(45), "exf6 e.p. is available");
    let back = unpack(p.pack().as_slice()).unwrap();
    assert_eq!(back.ep, Some(45));
    // And it is a different position from the one without the ep right.
    let no_ep = "rnbqkbnr/ppp1p1pp/8/3pPp2/8/8/PPPP1PPP/RNBQKBNR w KQkq - 0 3";
    assert_ne!(p.pack(), Position::from_fen(no_ep).unwrap().pack());
}

#[test]
fn ep_by_a_pinned_pawn_is_not_recorded() {
    // Black's e4 pawn is pinned down the e-file by the rook on e1, so exd3
    // e.p. is illegal and the ep right does not exist.
    let p = Position::from_fen("4k3/8/8/8/3Pp3/8/8/K3R3 b - d3 0 1").unwrap();
    assert_eq!(p.ep_effective(), None);
    let plain = Position::from_fen("4k3/8/8/8/3Pp3/8/8/K3R3 b - - 0 1").unwrap();
    assert_eq!(p.pack(), plain.pack());
}

#[test]
fn ep_that_would_expose_the_king_sideways_is_not_recorded() {
    // The exotic one: exd3 e.p. removes *two* pawns from the fourth rank at
    // once — the capturer and the captured — opening the rank from h4 to the
    // black king on a4. Neither pawn is pinned on its own.
    let p = Position::from_fen("8/8/8/8/k2Pp2Q/8/8/3K4 b - d3 0 1").unwrap();
    assert_eq!(p.ep_effective(), None);
    assert_eq!(
        p.pack(),
        Position::from_fen("8/8/8/8/k2Pp2Q/8/8/3K4 b - - 0 1")
            .unwrap()
            .pack()
    );
}

#[test]
fn rejects_non_canonical_encodings() {
    let bytes = Position::from_fen("8/8/4k3/8/8/4K3/8/8 w - - 0 1")
        .unwrap()
        .pack()
        .as_slice()
        .to_vec();
    assert!(unpack(&bytes).is_ok());

    // An odd piece count leaves one nibble spare. If it were ignored rather
    // than rejected, this position would have sixteen encodings and sixteen
    // hashes.
    let mut odd = Position::from_fen("8/8/4k3/8/8/4K3/4P3/8 w - - 0 1")
        .unwrap()
        .pack()
        .as_slice()
        .to_vec();
    assert_eq!(odd.len(), 8 + 2 + 2, "three pieces occupy two nibble bytes");
    odd[9] |= 0xF0;
    assert_eq!(unpack(&odd).unwrap_err().0, "spare nibble not zero");

    let mut bad_code = bytes.clone();
    bad_code[8] = 0x0D; // piece code 13 does not exist
    assert_eq!(unpack(&bad_code).unwrap_err().0, "piece code 12-15");

    let mut truncated = bytes.clone();
    truncated.pop();
    assert!(unpack(&truncated).is_err());

    let mut too_long = bytes.clone();
    too_long.push(0);
    assert_eq!(
        unpack(&too_long).unwrap_err().0,
        "length disagrees with occupancy"
    );

    // The halfmove clock cannot exceed 100: the game is drawable at 100 and
    // the field is the fifty-move rule's entire on-chain cost.
    let mut hm = bytes.clone();
    let n = hm.len();
    let flags = u16::from_le_bytes(hm[n - 2..].try_into().unwrap());
    hm[n - 2..].copy_from_slice(&((flags & 0x1FF) | (101 << 9)).to_le_bytes());
    assert_eq!(unpack(&hm).unwrap_err().0, "halfmove over 100");

    // ep_file 9-15 is unused space in the flags word.
    let mut ep = bytes.clone();
    let flags = u16::from_le_bytes(ep[n - 2..].try_into().unwrap());
    ep[n - 2..].copy_from_slice(&((flags & !(0xF << 5)) | (9 << 5)).to_le_bytes());
    assert_eq!(unpack(&ep).unwrap_err().0, "ep file 9-15");
}

#[test]
fn rejects_positions_no_game_can_reach() {
    let pack_of = |fen: &str| Position::from_fen(fen).unwrap().pack().as_slice().to_vec();

    // One king each, or this is not chess and the adjudicator should not
    // pretend to have an opinion about it.
    assert_eq!(
        unpack(&pack_of("8/8/4k3/8/8/8/8/8 w - - 0 1"))
            .unwrap_err()
            .0,
        "not exactly one king each"
    );

    // A castling right whose rook is not on its corner.
    assert_eq!(
        unpack(&pack_of("4k3/8/8/8/8/8/8/4K3 w KQ - 0 1"))
            .unwrap_err()
            .0,
        "castling right without its king and rook"
    );

    // White to move, Black in check: no legal move ends in that state, so no
    // game reaches it.
    assert_eq!(
        unpack(&pack_of("4k3/8/8/8/8/8/8/4R1K1 w - - 0 1"))
            .unwrap_err()
            .0,
        "side not to move is in check"
    );
    // The same board with Black to move is perfectly legal.
    assert!(unpack(&pack_of("4k3/8/8/8/8/8/8/4R1K1 b - - 0 1")).is_ok());

    // Pawns on rank 1 or rank 8 should have promoted or never existed.
    assert_eq!(
        unpack(&pack_of("4k3/8/8/8/8/8/8/4K1P1 w - - 0 1"))
            .unwrap_err()
            .0,
        "pawn on the back rank"
    );
    assert_eq!(
        unpack(&pack_of("4k1P1/8/8/8/8/8/8/4K3 w - - 0 1"))
            .unwrap_err()
            .0,
        "pawn on the back rank"
    );
}
