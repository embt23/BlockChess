//! `blockchess pack` / `unpack` — the round trip.
//!
//! The property that matters is `unpack(pack(x)) == x`, over a whole corpus.
//! `papers/06-decisions.md` D10 calls this the perft of the storage layer:
//! an exact, mechanical check that either passes on millions of real games or
//! does not. `pack` verifies every game as it writes it, so a file that was
//! produced at all is a file that decoded correctly at least once.

use std::io::Write;

use bc_chess::Position;
use bc_codec::Scheme;

/// `.bcg` — BlockChess games. A container, not a compression format: the
/// compression is `bc-codec`, and this only says where one game ends and the
/// next begins.
///
/// ```text
/// magic     4  "BCG\0"
/// version   1  currently 1
/// rules     2  RuleSetId — papers/04-permanence.md §5. 1 = standard chess.
/// scheme    1  0 = E3 fixed, 1 = E7 index
/// count     4  number of games
/// then, per game:
///   plies   2  ply count — the payload does not know its own length
///   flags   1  bit 0: a FEN start position follows
///   [fen]      length-prefixed, only if flags bit 0
///   len     4  payload length in bytes
///   payload
/// ```
///
/// Every field is little-endian. There is no per-game metadata yet — no
/// players, no result, no date — because `papers/06-decisions.md` D7 and D8
/// have not been decided and inventing an identity format before then would
/// be building underneath an unanswered question.
const MAGIC: [u8; 4] = *b"BCG\0";
const VERSION: u8 = 1;
const RULES_STANDARD: u16 = 1;

pub fn pack(args: &[String]) -> Result<(), String> {
    let input = args
        .first()
        .ok_or("usage: blockchess pack <in.pgn> <out.bcg>")?;
    let output = args
        .get(1)
        .ok_or("usage: blockchess pack <in.pgn> <out.bcg>")?;
    let scheme = if args.iter().any(|a| a == "--fixed") {
        Scheme::Fixed
    } else {
        Scheme::Index
    };

    let text = std::fs::read_to_string(input).map_err(|e| format!("{input}: {e}"))?;
    let (games, errors) = bc_pgn::parse_all(&text);
    if games.is_empty() {
        return Err(format!("{input}: no games could be read"));
    }

    let mut out = Vec::new();
    out.extend_from_slice(&MAGIC);
    out.push(VERSION);
    out.extend_from_slice(&RULES_STANDARD.to_le_bytes());
    out.push(match scheme {
        Scheme::Fixed => 0,
        Scheme::Index => 1,
    });
    out.extend_from_slice(&(games.len() as u32).to_le_bytes());

    let mut total_bits = 0.0;
    let mut plies = 0usize;

    for (i, game) in games.iter().enumerate() {
        let payload = bc_codec::encode(scheme, &game.start, &game.moves)
            .map_err(|e| format!("game {}: {e}", i + 1))?;

        // Verify before writing. A codec that only round-trips on the games
        // you remembered to test is a codec that loses data.
        let back = bc_codec::decode(scheme, &game.start, &payload, game.moves.len())
            .map_err(|e| format!("game {}: re-decode failed: {e}", i + 1))?;
        if back != game.moves {
            return Err(format!(
                "game {}: round trip changed the moves — refusing to write",
                i + 1
            ));
        }

        out.extend_from_slice(&(game.moves.len() as u16).to_le_bytes());
        let custom = game.start != Position::startpos();
        out.push(u8::from(custom));
        if custom {
            let fen = game.start.to_fen();
            out.extend_from_slice(&(fen.len() as u16).to_le_bytes());
            out.extend_from_slice(fen.as_bytes());
        }
        out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        out.extend_from_slice(&payload);

        total_bits += bc_codec::index_bits(&game.start, &game.moves);
        plies += game.moves.len();
    }

    std::fs::File::create(output)
        .and_then(|mut f| f.write_all(&out))
        .map_err(|e| format!("{output}: {e}"))?;

    let pgn_size = text.len();
    println!();
    println!("  {:<22}{:>12}", "scheme", scheme.name());
    println!("  {:<22}{:>12}", "games", games.len());
    if !errors.is_empty() {
        println!("  {:<22}{:>12}", "skipped", errors.len());
    }
    println!("  {:<22}{:>12}", "plies", plies);
    println!();
    println!("  {:<22}{:>12}  bytes", "PGN in", pgn_size);
    println!("  {:<22}{:>12}  bytes", ".bcg out", out.len());
    println!(
        "  {:<22}{:>11.1}x",
        "ratio",
        pgn_size as f64 / out.len() as f64
    );
    println!();
    println!(
        "  {:<22}{:>12.3}  bits/ply",
        "theoretical E7",
        total_bits / plies as f64
    );
    println!(
        "  {:<22}{:>12.3}  bits/ply, container included",
        "actual",
        out.len() as f64 * 8.0 / plies as f64
    );
    println!();
    println!("  every game was decoded back and compared before writing.");
    println!();
    Ok(())
}

pub fn unpack(args: &[String]) -> Result<(), String> {
    let input = args
        .first()
        .ok_or("usage: blockchess unpack <in.bcg> [out.pgn]")?;
    let bytes = std::fs::read(input).map_err(|e| format!("{input}: {e}"))?;

    let mut r = Reader { b: &bytes, i: 0 };
    if r.take(4)? != MAGIC {
        return Err(format!("{input} is not a .bcg file"));
    }
    let version = r.u8()?;
    if version != VERSION {
        return Err(format!(
            "{input} is version {version}, this build reads {VERSION}"
        ));
    }
    let rules = r.u16()?;
    if rules != RULES_STANDARD {
        // Exactly the case papers/04-permanence.md §5 asks for: say which rule
        // set, rather than decoding it wrongly under the rules we do have.
        return Err(format!(
            "{input} is under rule set {rules}, which this build does not implement"
        ));
    }
    let scheme = match r.u8()? {
        0 => Scheme::Fixed,
        1 => Scheme::Index,
        other => return Err(format!("unknown scheme {other}")),
    };
    let count = r.u32()? as usize;

    let mut pgn = String::new();
    for i in 0..count {
        let plies = r.u16()? as usize;
        let custom = r.u8()? != 0;
        let start = if custom {
            let n = r.u16()? as usize;
            let fen = std::str::from_utf8(r.take(n)?).map_err(|e| e.to_string())?;
            Position::from_fen(fen).map_err(|e| format!("game {}: {e}", i + 1))?
        } else {
            Position::startpos()
        };
        let len = r.u32()? as usize;
        let payload = r.take(len)?;

        let moves = bc_codec::decode(scheme, &start, payload, plies)
            .map_err(|e| format!("game {}: {e}", i + 1))?;

        if custom {
            pgn.push_str(&format!("[FEN \"{}\"]\n", start.to_fen()));
        }
        pgn.push_str("[Result \"*\"]\n\n");
        let mut pos = start;
        for (ply, &m) in moves.iter().enumerate() {
            if ply % 2 == 0 {
                pgn.push_str(&format!("{}. ", ply / 2 + 1));
            }
            pgn.push_str(&bc_pgn::to_san(&pos, m));
            pgn.push(if (ply + 1) % 10 == 0 { '\n' } else { ' ' });
            pos = pos.make_move(m);
        }
        pgn.push_str("*\n\n");
    }

    match args.get(1) {
        Some(path) => {
            std::fs::write(path, &pgn).map_err(|e| format!("{path}: {e}"))?;
            println!("  {count} games -> {path}");
        }
        None => print!("{pgn}"),
    }
    Ok(())
}

struct Reader<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        if self.i + n > self.b.len() {
            return Err("file ends in the middle of a record".into());
        }
        let s = &self.b[self.i..self.i + n];
        self.i += n;
        Ok(s)
    }
    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, String> {
        let s = self.take(2)?;
        Ok(u16::from_le_bytes([s[0], s[1]]))
    }
    fn u32(&mut self) -> Result<u32, String> {
        let s = self.take(4)?;
        Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }
}
