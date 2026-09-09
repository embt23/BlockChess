import * as E from './engine.mjs';
const T=[
 // fen, square, expected, why
 ["4k3/8/8/3p4/4P3/8/8/4K3 w - - 0 1","e4",100,"undefended pawn, black pawn takes it"],
 ["4k3/8/8/3p4/4P3/5P2/8/4K3 w - - 0 1","e4",0,"defended pawn: dxe4 fxe4 is even"],
 ["4k3/8/3p4/4p3/8/8/4R3/4R1K1 w - - 0 1","e5",0,"Rxe5 dxe5 Rxe5 loses 300 — x-ray rook must be seen"],
 ["4k3/8/8/3q4/4P3/8/8/4K3 w - - 0 1","d5",900,"queen hanging to a pawn"],
 ["4r3/8/8/8/8/8/4p3/4K3 w - - 0 1","e2",0,"Kxe2 illegal, rook defends"],
 ["8/8/8/8/8/8/4p3/4K3 w - - 0 1","e2",100,"Kxe2 legal, pawn is free"],
 ["4k3/8/2n5/8/3P4/8/8/4K3 w - - 0 1","d4",100,"knight wins the pawn clean"],
 ["4k3/8/2n5/8/3P4/5N2/8/4K3 w - - 0 1","d4",0,"…but not when it is defended"],
];
let bad=0;
for(const [fen,sq,exp,why] of T){
  const p=E.parseFEN(fen);
  const got=E.riskAt(p.b,E.IDX(sq));
  const ok=got===exp;
  if(!ok)bad++;
  console.log((ok?"OK  ":"FAIL")+"  "+sq+" risk="+got+" expected="+exp+"   "+why);
}
// board-level readings, on the real game position from the app's own PGN
const OPERA=`1.e4 e5 2.Nf3 d6 3.d4 Bg4 4.dxe5 Bxf3 5.Qxf3 dxe5 6.Bc4 Nf6 7.Qb3 Qe7 8.Nc3 c6
9.Bg5 b5 10.Nxb5 cxb5 11.Bxb5+ Nbd7 12.O-O-O Rd8 13.Rxd7 Rxd7 14.Rd1 Qe6 15.Bxd7+ Nxd7
16.Qb8+ Nxb8 17.Rd8#`;
const plies=E.parsePGN(OPERA);
const at=n=>{const p=E.clone(E.parseFEN(E.START));for(let i=0;i<n;i++)E.make(p,plies[i].m);return p;};
console.log("\nply  move      contacts  stake   riskW  riskB");
for(const n of [12,13,14,20,25,31,32]){
  const f=E.fields(at(n));
  console.log(String(n).padStart(3)+"  "+(plies[n-1]?plies[n-1].san:"-").padEnd(8)+
    String(f.contacts).padStart(6)+"  "+(f.stake/100).toFixed(1).padStart(6)+
    "  "+(f.riskW/100).toFixed(1).padStart(5)+"  "+(f.riskB/100).toFixed(1).padStart(5));
}
console.log(bad?("\n"+bad+" FAILURES"):"\nall SEE cases pass");
