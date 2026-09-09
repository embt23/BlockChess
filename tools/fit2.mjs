import pkg from '/opt/node22/lib/node_modules/playwright/index.js'; const { chromium } = pkg;
import fs from 'fs';
const frames=JSON.parse(fs.readFileSync('runs.json','utf8'));
const b=await chromium.launch(); const p=await b.newPage();
await p.setContent('<div id=m style="position:relative"></div>');
const res=await p.evaluate(frames=>{
  const m=document.getElementById('m');
  const sub=f=>/Cambria|Times|Georgia|Bookman|Century/i.test(f)?'"Liberation Serif",serif'
            :/Courier/i.test(f)?'"Liberation Mono",monospace':'"Liberation Sans",Arial,sans-serif';
  return frames.map(fr=>{
    const d=document.createElement('div');
    d.style.cssText='position:absolute;visibility:hidden;width:'+(fr.w*96)+'px;line-height:1.24';
    for(const para of fr.paras){
      const pEl=document.createElement('p'); pEl.style.margin='0';
      for(const r of para){
        const sp=document.createElement('span');
        const pad=/Liberation Serif/.test(sub(r.f))?1.09:1.0;
        sp.style.cssText='font-family:'+sub(r.f)+';font-size:'+(r.sz*pad*96/72)+'px;font-weight:'+(r.b?700:400);
        sp.textContent=r.t; pEl.appendChild(sp);
      }
      d.appendChild(pEl);
    }
    m.appendChild(d);
    const hIn=d.getBoundingClientRect().height/96; d.remove();
    return {slide:fr.slide,h:fr.h,need:+hIn.toFixed(2),over:+(hIn-fr.h).toFixed(2),
            txt:fr.paras.map(pp=>pp.map(r=>r.t).join('')).join(' | ')};
  });
},frames);
const bad=res.filter(r=>r.over>0.03).sort((a,b)=>b.over-a.over);
console.log('frames:',res.length,'· overflow:',bad.length);
for(const r of bad) console.log('  s'+r.slide+' h='+r.h+' need='+r.need+' over=+'+r.over+'  "'+r.txt.slice(0,60)+'"');
await b.close();
