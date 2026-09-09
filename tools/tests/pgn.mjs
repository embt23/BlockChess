import * as E from './engine.mjs';
const games={
opera:`1.e4 e5 2.Nf3 d6 3.d4 Bg4 4.dxe5 Bxf3 5.Qxf3 dxe5 6.Bc4 Nf6 7.Qb3 Qe7 8.Nc3 c6
9.Bg5 b5 10.Nxb5 cxb5 11.Bxb5+ Nbd7 12.O-O-O Rd8 13.Rxd7 Rxd7 14.Rd1 Qe6 15.Bxd7+ Nxd7
16.Qb8+ Nxb8 17.Rd8#`,
immortal:`1.e4 e5 2.f4 exf4 3.Bc4 Qh4+ 4.Kf1 b5 5.Bxb5 Nf6 6.Nf3 Qh6 7.d3 Nh5 8.Nh4 Qg5
9.Nf5 c6 10.g4 Nf6 11.Rg1 cxb5 12.h4 Qg6 13.h5 Qg5 14.Qf3 Ng8 15.Bxf4 Qf6 16.Nc3 Bc5
17.Nd5 Qxb2 18.Bd6 Bxg1 19.e5 Qxa1+ 20.Ke2 Na6 21.Nxg7+ Kd8 22.Qf6+ Nxf6 23.Be7#`};
for(const [k,v] of Object.entries(games)){
  try{const ms=E.parsePGN(v);console.log(k,'OK',ms.length,'plies · last:',ms[ms.length-1].san);}
  catch(e){console.log(k,'FAIL',e.message);}
}
const fens=["r1bq1rk1/pp2bppp/2np1n2/2p1p3/2P1P3/2NP1N2/PP2BPPP/R1BQ1RK1 w - - 0 9",
"r1b2rk1/pp1n1ppp/2pbpn2/q7/2BPP3/2N2N2/PP1BQPPP/R3K2R w KQ - 0 11"];
for(const f of fens){
  const p=E.parseFEN(f);
  console.log('fen ok · legal moves',E.legalMoves(p).length,'· check?',E.inCheck(p,'w'),E.inCheck(p,'b'));
}
