// Position packing: 4 bits a square, 64 squares, 256 bits — one EVM word.
//
// A square holds 13 states: empty, plus six piece types in two colours. That is
// the author's alphabet exactly — six marks and a rotation rule — and 13 fits in
// 4 bits with 3 codes to spare.
//
// The board packs to 32 bytes. Side-to-move, castling rights and the en-passant
// file are a further 9 bits, carried in a separate 2-byte word for clarity.
//
// NOT encoded: the halfmove and fullmove clocks. The territory variant ends on a
// move limit rather than the 50-move rule, so it does not need them; a standard
// chess contract would, and would need a third word.
import { parseFEN, toFEN, clone, make, parsePGN, START, F, R, SQ } from './engine.mjs';

const CODE = { "": 0, P:1, N:2, B:3, R:4, Q:5, K:6, p:7, n:8, b:9, r:10, q:11, k:12 };
const PIECE = Object.keys(CODE).reduce((m, k) => (m[CODE[k]] = k, m), {});

export function packBoard(pos) {
  const out = new Uint8Array(32);
  for (let i = 0; i < 64; i++) {
    const c = CODE[pos.b[i]];
    if (c === undefined) throw new Error("unencodable square: " + pos.b[i]);
    if (i % 2 === 0) out[i >> 1] = c << 4;
    else out[i >> 1] |= c;
  }
  return out;
}
export function packMeta(pos) {
  let m = pos.t === "w" ? 1 : 0;
  const cas = pos.cas || "-";
  m |= (cas.includes("K") ? 1 : 0) << 1;
  m |= (cas.includes("Q") ? 1 : 0) << 2;
  m |= (cas.includes("k") ? 1 : 0) << 3;
  m |= (cas.includes("q") ? 1 : 0) << 4;
  m |= (pos.ep >= 0 ? F(pos.ep) + 1 : 0) << 5;   // 0 = none, else file+1
  return m;                                       // 9 bits
}
export function unpack(bytes, meta) {
  const b = new Array(64).fill("");
  for (let i = 0; i < 64; i++) {
    const nib = (i % 2 === 0) ? (bytes[i >> 1] >> 4) : (bytes[i >> 1] & 15);
    b[i] = PIECE[nib];
    if (b[i] === undefined) throw new Error("reserved code " + nib + " at " + i);
  }
  const t = (meta & 1) ? "w" : "b";
  let cas = ((meta >> 1) & 1 ? "K" : "") + ((meta >> 2) & 1 ? "Q" : "")
          + ((meta >> 3) & 1 ? "k" : "") + ((meta >> 4) & 1 ? "q" : "");
  const epf = (meta >> 5) & 15;
  // the en-passant square's rank follows from the side to move
  const ep = epf ? SQ(epf - 1, t === "w" ? 2 : 5) : -1;
  return { b, t, cas: cas || "-", ep, hm: 0, fm: 1 };
}
export const toHex = u8 => "0x" + [...u8].map(x => x.toString(16).padStart(2, "0")).join("");
const shortFEN = f => f.split(/\s+/).slice(0, 4).join(" ");

/* ---------------- test ---------------- */
if (import.meta.url === `file://${process.argv[1]}`) {
  const GAMES = {
    opera: `1.e4 e5 2.Nf3 d6 3.d4 Bg4 4.dxe5 Bxf3 5.Qxf3 dxe5 6.Bc4 Nf6 7.Qb3 Qe7 8.Nc3 c6
9.Bg5 b5 10.Nxb5 cxb5 11.Bxb5+ Nbd7 12.O-O-O Rd8 13.Rxd7 Rxd7 14.Rd1 Qe6 15.Bxd7+ Nxd7
16.Qb8+ Nxb8 17.Rd8#`,
    immortal: `1.e4 e5 2.f4 exf4 3.Bc4 Qh4+ 4.Kf1 b5 5.Bxb5 Nf6 6.Nf3 Qh6 7.d3 Nh5 8.Nh4 Qg5
9.Nf5 c6 10.g4 Nf6 11.Rg1 cxb5 12.h4 Qg6 13.h5 Qg5 14.Qf3 Ng8 15.Bxf4 Qf6 16.Nc3 Bc5
17.Nd5 Qxb2 18.Bd6 Bxg1 19.e5 Qxa1+ 20.Ke2 Na6 21.Nxg7+ Kd8 22.Qf6+ Nxf6 23.Be7#` };

  let n = 0, bad = 0, ep = 0, cas = 0;
  for (const [name, pgn] of Object.entries(GAMES)) {
    const plies = parsePGN(pgn);
    const p = clone(parseFEN(START));
    const check = () => {
      const bytes = packBoard(p), meta = packMeta(p);
      if (bytes.length !== 32) { console.log("  FAIL not 32 bytes"); bad++; return; }
      const back = unpack(bytes, meta);
      const a = shortFEN(toFEN(p)), b = shortFEN(toFEN(back));
      if (a !== b) { console.log("  FAIL " + name + " ply " + n + "\n    was " + a + "\n    got " + b); bad++; }
      if (p.ep >= 0) ep++;
      if ((p.cas || "-") !== "-") cas++;
      n++;
    };
    check();
    for (const pl of plies) { make(p, pl.m); check(); }
  }
  const p0 = parseFEN(START);
  console.log("positions round-tripped : " + n + (bad ? "  " + bad + " FAILURES" : "  all identical"));
  console.log("  of which en passant   : " + ep);
  console.log("  of which castling     : " + cas);
  console.log("board payload           : " + packBoard(p0).length + " bytes = 256 bits = 1 EVM word");
  console.log("metadata                : 9 bits (side, castling, ep file)");
  console.log("opening position        : " + toHex(packBoard(p0)));
  process.exit(bad ? 1 : 0);
}
