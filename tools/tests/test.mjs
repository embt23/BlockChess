import * as E from './engine.mjs';
const p=E.parseFEN(E.START);
const want=[1,20,400,8902,197281];
for(let d=1;d<=4;d++){const n=E.perft(E.parseFEN(E.START),d);console.log('perft',d,n,n===want[d]?'OK':'FAIL expected '+want[d]);}
// kiwipete: castling, ep, promotions
const KP="r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";
const kw=[48,2039,97862];
for(let d=1;d<=3;d++){const n=E.perft(E.parseFEN(KP),d);console.log('kiwipete',d,n,n===kw[d-1]?'OK':'FAIL expected '+kw[d-1]);}
// ep/promo position 3
const P3="8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1";
const w3=[14,191,2812,43238];
for(let d=1;d<=4;d++){const n=E.perft(E.parseFEN(P3),d);console.log('pos3',d,n,n===w3[d-1]?'OK':'FAIL expected '+w3[d-1]);}
