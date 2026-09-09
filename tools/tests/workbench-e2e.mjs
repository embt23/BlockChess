import pkg from '/opt/node22/lib/node_modules/playwright/index.js'; const { chromium } = pkg;
import fs from 'fs';
const html=fs.readFileSync('/home/user/BlockChess/workbench.html','utf8');
fs.writeFileSync('/tmp/wb.html','<!doctype html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><style>html{color-scheme:light}body{margin:0;font:14px system-ui;background:#fafaf8}img{max-width:100%}[hidden]{display:none!important}</style></head><body>'+html+'</body></html>');
const b=await chromium.launch();
const p=await b.newPage({viewport:{width:1360,height:900},reducedMotion:'no-preference'});
const errs=[]; p.on('pageerror',e=>errs.push('PAGEERROR: '+e.message));
p.on('console',m=>{if(m.type()==='error'&&!m.text().includes('ERR_CONNECTION'))errs.push(m.text());});
await p.goto('file:///tmp/wb.html'); await p.waitForTimeout(600);

// hand it the alphabet render as a stand-in for the journal photo
await p.setInputFiles('#file','/tmp/claude-0/-home-user-BlockChess/1e087d2d-a6f1-50dc-b90d-a89667dace2e/scratchpad/journal-5.png');
await p.waitForTimeout(900);

const g=await p.evaluate(()=>{
  const s=document.getElementById('stage').getBoundingClientRect();
  const c=document.getElementById('cv').getBoundingClientRect();
  return {sw:s.width,sh:s.height,cx:c.x,cy:c.y};
});
const IW=1944, IH=1944;                       // the render is square
const scale=Math.min(g.sw/IW,g.sh/IH)*0.94;
const tx=(g.sw-IW*scale)/2, ty=(g.sh-IH*scale)/2;
const scr=(ix,iy)=>({x:g.cx+ix*scale+tx, y:g.cy+iy*scale+ty});

// draw a region around the whole board
await p.click('#modeRegion');
const a=scr(0,0), z=scr(IW,IH);
await p.mouse.move(a.x+1,a.y+1); await p.mouse.down();
await p.mouse.move((a.x+z.x)/2,(a.y+z.y)/2,{steps:8});
await p.mouse.move(z.x-1,z.y-1,{steps:8}); await p.mouse.up();
await p.waitForTimeout(400);

const KEY={p:'1',n:'2',b:'3',r:'4',q:'5',k:'6'};
const back=['r','n','b','q','k','b','n','r'];
const cell=(r,c)=>scr((c+0.5)/8*IW,(r+0.5)/8*IH);
async function tag(r,c,piece){
  await p.keyboard.press(KEY[piece]);
  const q=cell(r,c);
  await p.mouse.move(q.x,q.y); await p.mouse.down(); await p.mouse.up();
}
await p.keyboard.press('c');                              // brush → black
for(let c=0;c<8;c++) await tag(0,c,back[c]);
for(let c=0;c<8;c++) await tag(1,c,'p');
await p.keyboard.press('c');                              // brush → white
for(let c=0;c<8;c++) await tag(6,c,'p');
for(let c=0;c<8;c++) await tag(7,c,back[c]);
await p.waitForTimeout(500);

const out=await p.evaluate(()=>({
  verdict:document.getElementById('verdict').textContent.trim(),
  fen:document.getElementById('fen').textContent.trim(),
  filled:document.getElementById('filled').textContent.trim()}));
console.log('filled :',out.filled);
console.log('fen    :',out.fen);
console.log('verdict:',out.verdict.split('\n')[0]);
const WANT='rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1';
console.log(out.fen===WANT ? '\n*** GROUND TRUTH PASSED — tagging the board yields the standard opening FEN ***'
                           : '\n*** MISMATCH ***\nwant: '+WANT);
console.log('ERRORS:',errs.length?errs:'none');
await p.screenshot({path:'wb-tagged.png'});
await b.close();
