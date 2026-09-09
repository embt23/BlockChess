import pkg from '/opt/node22/lib/node_modules/playwright/index.js'; const { chromium } = pkg;
import fs from 'fs';
const html=fs.readFileSync('/home/user/BlockChess/index.html','utf8');
fs.writeFileSync('/tmp/preview.html','<!doctype html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><style>html{color-scheme:light}body{margin:0;font:14px system-ui;background:#fafaf8}img{max-width:100%}[hidden]{display:none!important}</style></head><body>'+html+'</body></html>');
const b=await chromium.launch();
const p=await b.newPage({viewport:{width:1400,height:1000},deviceScaleFactor:3,reducedMotion:'no-preference'});
const errs=[]; p.on('pageerror',e=>errs.push(e.message));
await p.goto('file:///tmp/preview.html'); await p.waitForTimeout(1500);
await p.click('#mExplore'); await p.waitForTimeout(200);

const ALL=['pieces','terr','contour','frontier','tension','vision','arrows','numbers'];
async function setLayers(on){
  for(const k of ALL){
    const is=await p.$eval('.lrow[data-k="'+k+'"]',e=>e.dataset.on==='1');
    const want=on.includes(k);
    if(is!==want) await p.click('.lrow[data-k="'+k+'"]');
  }
}
async function setGame(i){ await p.selectOption('#gameSel',String(i)); await p.waitForTimeout(1400); }
async function setPly(n){ await p.evaluate(n=>{document.querySelectorAll('.moves button')[n].click();},n); await p.waitForTimeout(1600); }
async function shot(name){
  await p.waitForTimeout(1400);
  await p.screenshot({path:'deck/'+name+'.png',clip:await (await p.$('#board')).boundingBox()});
  console.log('  '+name);
}
// --- opening position, mirroring the journal's "Start" board ---
await setGame(4);
await setLayers(['pieces']);                      await shot('01-start-pieces');
await setLayers(['pieces','numbers']);            await shot('02-start-numbers');
await setLayers(['arrows']);                      await shot('03-start-arrows');
await setLayers(['pieces','vision']);             await shot('04-start-vision');
// --- a real middlegame, for territory and the border ---
await setGame(0); await setPly(31);
await setLayers(['pieces','terr']);               await shot('05-terr');
await setLayers(['pieces','terr','contour']);     await shot('06-contour');
await setLayers(['pieces','frontier']);           await shot('07-frontier');
await setLayers(['pieces','terr','frontier','tension']); await shot('08-full');
await setLayers(['arrows','frontier']);           await shot('09-symbol');
// closed-centre study: a border that holds
await setGame(2); await setLayers(['pieces','terr','frontier']); await shot('10-closed');
// search map
await setGame(0); await setPly(31); await p.waitForTimeout(2500);
const tb=await (await p.$('#tree')).boundingBox();
if(tb) await p.screenshot({path:'deck/11-tree.png',clip:{x:tb.x,y:tb.y,width:Math.min(tb.width,1050),height:tb.height}});
console.log('  11-tree');
console.log('ERRORS:',errs.length?errs:'none');
await b.close();
