//! E3 — write each move's 16-bit packed form, little-endian.
//!
//! The baseline. No chess knowledge required in either direction, which makes
//! it the fallback that keeps a game readable even if the rules registry is
//! lost, and the control against which E7's saving is measured.

use bc_chess::Move;

use crate::CodecError;

pub fn encode(moves: &[Move]) -> Vec<u8> {
    let mut out = Vec::with_capacity(moves.len() * 2);
    for m in moves {
        out.extend_from_slice(&m.0.to_le_bytes());
    }
    out
}

pub fn decode(bytes: &[u8], ply_count: usize) -> Result<Vec<Move>, CodecError> {
    if bytes.len() < ply_count * 2 {
        return Err(CodecError(format!(
            "need {} bytes for {} plies, got {}",
            ply_count * 2,
            ply_count,
            bytes.len()
        )));
    }
    // `as_chunks` rather than `chunks_exact`: the slice is exactly `ply_count`
    // pairs long, so the remainder is empty by construction, and the array form
    // hands `from_le_bytes` a `[u8; 2]` without re-indexing it.
    Ok(bytes[..ply_count * 2]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&c| Move(u16::from_le_bytes(c)))
        .collect())
}
