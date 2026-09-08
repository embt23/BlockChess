//! `perft` — the only acceptable correctness test for a move generator.
//!
//! It counts leaf nodes of the move tree to a fixed depth. The numbers for
//! standard positions are published and independently verified by every engine
//! ever written, so this is not "a test we wrote"; it is an exact external
//! oracle. Either you match it or you have a bug, with no room for opinion.
//!
//! Nothing above this layer — the channel, the adjudicator, the money — means
//! anything if the rules are wrong. This is the one place in the whole project
//! where correctness is cheap to verify exactly, so we verify it exactly.

use crate::position::Position;
use crate::types::{Move, MoveList};

/// Count leaf nodes at `depth`.
pub fn perft(pos: &Position, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }
    let mut moves = MoveList::new();
    pos.generate_pseudo(&mut moves);

    // At depth 1 we only need the count, so skip building a child position for
    // anything beyond the legality test.
    if depth == 1 {
        return moves
            .as_slice()
            .iter()
            .filter(|&&m| pos.is_legal(m))
            .count() as u64;
    }

    let mut nodes = 0;
    for &m in &moves {
        let child = pos.make_move(m);
        if !child.in_check(pos.side) {
            nodes += perft(&child, depth - 1);
        }
    }
    nodes
}

/// Per-move breakdown at the root.
///
/// This is the debugging tool that makes a wrong `perft` tractable: compare
/// your divide against a known-good engine's, find the one move whose subtree
/// count differs, descend into it, and repeat. A few rounds of that localises
/// any bug to a single position, however deep.
pub fn divide(pos: &Position, depth: u32) -> Vec<(Move, u64)> {
    let mut out = Vec::new();
    let mut moves = MoveList::new();
    pos.generate_pseudo(&mut moves);
    for &m in &moves {
        let child = pos.make_move(m);
        if !child.in_check(pos.side) {
            out.push((
                m,
                if depth <= 1 {
                    1
                } else {
                    perft(&child, depth - 1)
                },
            ));
        }
    }
    out.sort_by_key(|(m, _)| m.to_uci());
    out
}
