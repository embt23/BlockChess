//! Stage 1 — a game becomes a vector.
//!
//! One vector per *side* per game, computed from the move list alone. There is
//! deliberately no engine here: an evaluation function would import somebody
//! else's idea of what good chess is, and this layer measures **behaviour, not
//! correctness**. See `spec/10-personality.md` stage 1.

use bc_chess::types::{file_of, rank_of, Color, Piece, Square, FLAG_CASTLE, FLAG_EP};
use bc_chess::{Move, Position};

/// Number of features. Also the dimension the basis is discovered in.
pub const D: usize = 18;

pub const FEATURE_NAMES: [&str; D] = [
    "capture_rate",
    "check_rate",
    "pawn_share",
    "knight_share",
    "bishop_share",
    "rook_share",
    "queen_share",
    "king_share",
    "enemy_half_rate",
    "center_rate",
    "edge_rate",
    "own_mobility",
    "opp_mobility",
    "take_rate",
    "queen_dev",
    "castle_ply",
    "mean_material",
    "length",
];

/// The sixteen central squares, c3–f6.
fn is_center(s: Square) -> bool {
    (2..=5).contains(&file_of(s)) && (2..=5).contains(&rank_of(s))
}

fn is_edge(s: Square) -> bool {
    let (f, r) = (file_of(s), rank_of(s));
    f == 0 || f == 7 || r == 0 || r == 7
}

fn in_enemy_half(s: Square, side: Color) -> bool {
    match side {
        Color::White => rank_of(s) >= 4,
        Color::Black => rank_of(s) <= 3,
    }
}

const PIECE_VALUE: [f64; 6] = [1.0, 3.0, 3.0, 5.0, 9.0, 0.0];
/// Total material at the start, used to normalise `mean_material`.
const START_MATERIAL: f64 = 78.0;

fn material(p: &Position) -> f64 {
    let mut total = 0.0;
    for piece in Piece::ALL {
        let bb = p.piece_bb[piece.idx()];
        total += (bb.count_ones() as f64) * PIECE_VALUE[piece.idx()];
    }
    total
}

/// Replay a game and extract one side's feature vector.
///
/// `moves` is the full alternating move list from `start`. Moves made by the
/// other side advance the position but contribute nothing.
///
/// Returns `None` if the side never moved — a zero-move sample is not a
/// personality, and averaging over an empty set would silently produce a point
/// at the origin, which is a *real* location in personality space and would be
/// a lie.
pub fn extract(start: &Position, moves: &[Move], side: Color) -> Option<[f64; D]> {
    let plies = moves.len().max(1) as f64;
    let mut pos = *start;

    let mut own = 0.0;
    let (mut captures, mut checks, mut enemy_half, mut center, mut edge) =
        (0.0, 0.0, 0.0, 0.0, 0.0);
    let mut piece_share = [0.0f64; 6];
    let (mut own_mob, mut opp_mob, mut mat) = (0.0, 0.0, 0.0);
    let (mut take_chances, mut takes_made) = (0.0, 0.0);
    let mut queen_dev = None;
    let mut castle_ply = None;

    for (ply, &m) in moves.iter().enumerate() {
        let mover = pos.side;
        if mover != side {
            pos = pos.make_move(m);
            continue;
        }
        own += 1.0;

        let legal = pos.generate_legal();
        own_mob += legal.len() as f64;
        if legal.as_slice().iter().any(|&c| is_capture(&pos, c)) {
            take_chances += 1.0;
        }
        mat += material(&pos) / START_MATERIAL;

        if is_capture(&pos, m) {
            captures += 1.0;
            takes_made += 1.0;
        }
        if let Some((_, p)) = pos.piece_at(m.from()) {
            piece_share[p.idx()] += 1.0;
            if p == Piece::Queen && queen_dev.is_none() {
                queen_dev = Some(ply as f64 / plies);
            }
        }
        if m.flag() == FLAG_CASTLE && castle_ply.is_none() {
            castle_ply = Some(ply as f64 / plies);
        }
        if in_enemy_half(m.to(), side) {
            enemy_half += 1.0;
        }
        if is_center(m.to()) {
            center += 1.0;
        }
        if is_edge(m.to()) {
            edge += 1.0;
        }

        pos = pos.make_move(m);
        opp_mob += pos.generate_legal().len() as f64;
        if pos.in_check(mover.flip()) {
            checks += 1.0;
        }
    }

    if own == 0.0 {
        return None;
    }

    // Mobility is divided by 40 — a typical middlegame move count — purely to
    // land it on the same rough scale as the rates. Standardisation makes the
    // constant irrelevant to the basis; it only keeps raw vectors readable.
    let mut f = [0.0f64; D];
    f[0] = captures / own;
    f[1] = checks / own;
    for i in 0..6 {
        f[2 + i] = piece_share[i] / own;
    }
    f[8] = enemy_half / own;
    f[9] = center / own;
    f[10] = edge / own;
    f[11] = own_mob / own / 40.0;
    f[12] = opp_mob / own / 40.0;
    f[13] = if take_chances > 0.0 {
        takes_made / take_chances
    } else {
        0.0
    };
    f[14] = queen_dev.unwrap_or(1.0);
    f[15] = castle_ply.unwrap_or(1.0);
    f[16] = mat / own;
    f[17] = (plies / 200.0).min(1.0);
    Some(f)
}

fn is_capture(pos: &Position, m: Move) -> bool {
    m.flag() == FLAG_EP || pos.piece_at(m.to()).is_some()
}
