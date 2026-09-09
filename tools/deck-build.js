const pptxgen = require('pptxgenjs');
const p = new pptxgen();
p.layout = 'LAYOUT_WIDE';                 // 13.33 x 7.5
p.author = 'Chess Frontier Atlas';
p.title  = 'Reading the Page';

const INK='1A1E18', WHITE='FFFFFF', BRASS='A1741F', BRASS_L='C8912F',
      VERD='00806F', OXIDE='AB3B28', MUTED='6E7268', FAINT='D8D5CB', PAPER='F4F3EF';
const H='Cambria', B='Calibri', M='Courier New';
const IMG=n=>({path:__dirname+'/'+n});

const eyebrow=(s,t,x,y,col)=>s.addText(t,{x:x||0.65,y:y||0.44,w:11,h:0.26,isTextBox:true,margin:0,
  fontFace:B,fontSize:10.5,bold:true,charSpacing:2.4,color:col||BRASS});
const title=(s,t,x,y,sz,col)=>s.addText(t,{x:x||0.65,y:y||0.72,w:11.4,h:0.8,isTextBox:true,margin:0,
  fontFace:H,fontSize:sz||33,bold:true,color:col||INK});
const body=(s,t,x,y,w,sz,col,h)=>s.addText(t,{x:x,y:y,w:w,h:h||3.6,isTextBox:true,margin:0,valign:'top',
  fontFace:B,fontSize:sz||14.5,color:col||'2B2F29',lineSpacing:22});
const cap=(s,t,x,y,w,col)=>s.addText(t,{x:x,y:y,w:w,h:0.5,isTextBox:true,margin:0,
  fontFace:B,fontSize:10.5,italic:true,color:col||MUTED});
const frame=(s,x,y,w,h)=>s.addShape(p.ShapeType.rect,{x:x,y:y,w:w,h:h,
  fill:{color:WHITE,transparency:100},line:{color:FAINT,width:0.75}});
const step=(s,n)=>{
  s.addShape(p.ShapeType.ellipse,{x:0.65,y:0.62,w:0.62,h:0.62,fill:{color:BRASS}});
  s.addText(n,{x:0.65,y:0.62,w:0.62,h:0.62,isTextBox:true,margin:0,align:'center',valign:'middle',
    fontFace:B,fontSize:15,bold:true,color:WHITE});
};
const board=(s,f,x,y,d)=>{ s.addImage({...IMG(f),x:x,y:y,w:d,h:d}); frame(s,x,y,d,d); };

/* ---------------- 1 · title ---------------- */
let s=p.addSlide(); s.background={color:INK};
s.addImage({...IMG('07-frontier.png'),x:7.55,y:0,w:5.78,h:5.78,transparency:12});
s.addShape(p.ShapeType.rect,{x:7.55,y:0,w:5.78,h:7.5,fill:{color:INK,transparency:55}});
s.addText('A CHESS JOURNAL, READ BACK',{x:0.75,y:1.5,w:7,h:0.3,isTextBox:true,margin:0,
  fontFace:B,fontSize:11,bold:true,charSpacing:3,color:BRASS_L});
s.addText('Reading the Page',{x:0.72,y:1.95,w:7.2,h:1.5,isTextBox:true,margin:0,
  fontFace:H,fontSize:60,bold:true,color:WHITE});
s.addText('The notation it invented, the border it was reaching for, and the places where the page goes quiet.',
  {x:0.75,y:3.5,w:6.4,h:1.1,isTextBox:true,margin:0,fontFace:H,fontSize:17,italic:true,color:'C9CCC2',lineSpacing:26});
s.addText([{text:'what the page shows',options:{color:BRASS_L}},{text:'   ·   ',options:{color:MUTED}},
  {text:'what it does not',options:{color:OXIDE}},{text:'   ·   ',options:{color:MUTED}},
  {text:'what was built from it',options:{color:'8FB5AC'}}],
  {x:0.75,y:5.6,w:6.6,h:0.3,isTextBox:true,margin:0,fontFace:M,fontSize:11});
s.addNotes('The page is a working document, not a game record. This deck reads it as an artifact.');

/* ---------------- 2 · the claim ---------------- */
s=p.addSlide(); s.background={color:WHITE};
eyebrow(s,'THE CLAIM');
title(s,'This page is not a record of a game.');
s.addText('It is a record of someone building a way to see one.',
  {x:0.65,y:1.62,w:11.4,h:0.6,isTextBox:true,margin:0,fontFace:H,fontSize:23,italic:true,color:BRASS});
body(s,'Nothing on the sheet is a move list. There is no score, no algebraic notation, no sequence anyone could replay. What covers it instead is a ladder of abstractions — the same board drawn over and over, each time with more thrown away.\n\nRead in that order, the page has a direction. It starts with pieces, ends with bits, and the thing it keeps circling is where one side’s board stops and the other’s starts.',
  0.65,2.45,6.1);
board(s,'01-start-pieces.png',7.5,2.15,3.9);
cap(s,'The opening position, drawn plainly — where the page also begins, top right, under the word “Start”.',7.5,6.18,4.6);
s.addNotes('Set the honest frame immediately: this is a notebook of method, not a game.');

/* ---------------- 3 · known / unknown ---------------- */
s=p.addSlide(); s.background={color:WHITE};
eyebrow(s,'BEFORE WE START');
title(s,'What the page tells us, and what it withholds');
s.addShape(p.ShapeType.rect,{x:0.65,y:1.95,w:5.85,h:4.55,fill:{color:PAPER}});
s.addText('LEGIBLE',{x:1.0,y:2.25,w:3,h:0.3,isTextBox:true,margin:0,fontFace:B,fontSize:11,bold:true,charSpacing:2.4,color:BRASS});
s.addText([
 {text:'The labels, in the author’s hand — first abstraction, cover, Vision, Information / Compression / Symbol, Boards, Symmetries.',options:{bullet:true,breakLine:true}},
 {text:'A substitution key in the right margin: 5 ⇒ …, 8 ⇒ …, mapping numerals onto movement glyphs.',options:{bullet:true,breakLine:true}},
 {text:'A bracket over a row of marks reading 3bits, and a second column annotated 4bit.',options:{bullet:true,breakLine:true}},
 {text:'A written binary string, and a note reading non pawns.',options:{bullet:true}}],
 {x:1.0,y:2.68,w:5.15,h:3.5,isTextBox:true,margin:0,fontFace:B,fontSize:12.5,color:'2B2F29',paraSpaceAfter:9});
s.addShape(p.ShapeType.rect,{x:6.85,y:1.95,w:5.85,h:4.55,fill:{color:'FBF1EF'}});
s.addText('UNREAD',{x:7.2,y:2.25,w:3,h:0.3,isTextBox:true,margin:0,fontFace:B,fontSize:11,bold:true,charSpacing:2.4,color:OXIDE});
s.addText([
 {text:'What each glyph stands for. The key is partial — the alphabet is not fully decodable from the sheet.',options:{bullet:true,breakLine:true}},
 {text:'What 8 geometry means. It is boxed and circled, so it mattered, and it is the one term with no worked example.',options:{bullet:true,breakLine:true}},
 {text:'Whether the large glyph field is a transcription of a real game or drills in the new alphabet.',options:{bullet:true,breakLine:true}},
 {text:'What the filled-in black squares mark. They recur on several small boards.',options:{bullet:true,breakLine:true}},
 {text:'The order the page was written in, and which game — if any — was in front of the author.',options:{bullet:true}}],
 {x:7.2,y:2.68,w:5.15,h:3.5,isTextBox:true,margin:0,fontFace:B,fontSize:12.5,color:'2B2F29',paraSpaceAfter:9});
s.addNotes('State the gaps up front so the rest of the deck is trusted.');

/* ---------------- 4 · step 01 ---------------- */
s=p.addSlide(); s.background={color:WHITE};
step(s,'01'); eyebrow(s,'TOP LEFT OF THE PAGE',1.45);
title(s,'First abstraction',1.45);
board(s,'01-start-pieces.png',0.65,1.95,4.35);
body(s,'The phrase is written on the sheet, next to a small grid with its squares crossed. It is the founding move of everything else: stop drawing pieces, start drawing marks.\n\nA knight is not a horse. It is a set of squares it bears on. Once that swap is made, the board stops being a picture of an army and becomes a field of claims — and a field can be measured.',
  5.45,2.05,7.2);
cap(s,'Thirty-two objects. Nothing yet about who holds what.',5.45,5.75,7);
s.addNotes('The labelled hinge of the page.');

/* ---------------- 5 · step 02 ---------------- */
s=p.addSlide(); s.background={color:WHITE};
step(s,'02'); eyebrow(s,'THE DESCENDING COLUMN',1.45);
title(s,'Everything becomes direction',1.45);
body(s,'Below that first grid the page runs a sequence — one small board, then another, then another, each filling with arrows. Up, down, up. The pieces have gone; what is left on each square is which way it is being pushed, and by whom.\n\nThis is the single most repeated image on the sheet. Whatever else the author was chasing, they kept coming back to a board of arrows.',
  0.65,2.05,6.3,null,null,3.3);
board(s,'03-start-arrows.png',7.15,1.95,4.35);
cap(s,'The same opening position with the pieces removed entirely — each square carrying only the resultant of the attacks landing on it.',0.65,5.5,6.3);
s.addNotes('Arrows are the motif that recurs most on the page.');

/* ---------------- 6 · step 03 ---------------- */
s=p.addSlide(); s.background={color:WHITE};
step(s,'03'); eyebrow(s,'CENTRE OF THE PAGE',1.45);
title(s,'Cover, and vision',1.45);
board(s,'04-start-vision.png',0.65,1.95,4.35);
body(s,'Two words appear here. Cover sits beside a grid ruled with long diagonals — lines drawn straight through the board rather than square by square. Vision is written under an arrow pointing up, next to the same kind of drawing.\n\nThe distinction looks deliberate: cover is what you hold, vision is how far you can see. A border is not built where pieces stand. It is built where they look.',
  5.45,2.05,7.2);
cap(s,'Every line the long pieces see down, drawn to where it stops.',5.45,5.75,7);
s.addNotes('Cover vs vision — held ground vs reach.');

/* ---------------- 7 · step 04 ---------------- */
s=p.addSlide(); s.background={color:WHITE};
step(s,'04'); eyebrow(s,'TOP RIGHT, AND THE MARGIN',1.45);
title(s,'An alphabet, and a key to it',1.45);
body(s,'The largest board on the page is labelled Start. Its back ranks are filled not with pieces but with invented glyphs, and beneath each pawn is a single arrow — the piece replaced by its move.\n\nIn the right margin sits a key: a numeral, an arrow, and a glyph it becomes. This is a working substitution table. The author was not sketching. They were compiling.',
  0.65,2.05,6.0);
board(s,'02-start-numbers.png',6.9,1.9,4.5);
cap(s,'Each square as a signed count — the machine equivalent of the glyph the page assigns it.',6.9,6.55,5.3);
s.addNotes('The margin key proves a systematic notation, not doodling.');

/* ---------------- 8 · step 05 ---------------- */
s=p.addSlide(); s.background={color:WHITE};
step(s,'05'); eyebrow(s,'MIDDLE BAND',1.45);
title(s,'Three bits, then four',1.45);
body(s,'A bracket over a row of marks reads 3bits. Further down, a second column is annotated 4bit. Both numbers are on the page, written at different moments.\n\nEight directions fit in three bits. But a square with no push at all is a ninth symbol — and three bits cannot hold nine. The count has to go to four. Building the encoder ran into that collision from the other direction, about an hour after reading it here.',
  0.65,2.05,6.0,null,null,3.15);
board(s,'09-symbol.png',6.9,1.9,4.5);
s.addShape(p.ShapeType.rect,{x:0.65,y:5.35,w:6.0,h:1.15,fill:{color:PAPER}});
s.addText([{text:'256 b',options:{fontSize:26,bold:true,color:BRASS}},
           {text:'  raw   →   ',options:{fontSize:12,color:MUTED}},
           {text:'195 b',options:{fontSize:26,bold:true,color:VERD}},
           {text:'  entropy-coded',options:{fontSize:12,color:MUTED}}],
  {x:0.95,y:5.55,w:5.6,h:0.75,isTextBox:true,margin:0,fontFace:M,valign:'middle'});
cap(s,'The whole board as one string, and how much of it is actually saying something.',6.9,6.55,5.3);
s.addNotes('The 3-vs-4 bit note on the page is a real result, not a stray number.');

/* ---------------- 9 · step 06 ---------------- */
s=p.addSlide(); s.background={color:WHITE};
step(s,'06'); eyebrow(s,'LOWER THIRD',1.45);
title(s,'Boards, symmetries, and a string of bits',1.45);
board(s,'10-closed.png',0.65,1.95,4.35);
s.addText('The bottom of the sheet turns taxonomic. A dotted enclosure gathers a crowd of glyphs and is simply labelled Boards. Beside it, a dense woven lattice is labelled Symmetries. A binary string is written out along the margin. Several small boards carry one square filled solid black.',
  {x:5.45,y:2.0,w:7.2,h:1.5,isTextBox:true,margin:0,fontFace:B,fontSize:14,color:'2B2F29',lineSpacing:21});
const cards=[['Boards','A set being collected — positions treated as specimens.'],
             ['Symmetries','Repeated structure, drawn as weave rather than written.'],
             ['The string','Notation actually executed: a position, encoded.'],
             ['Black squares','Unknown. A marked square, a captured piece, a blocked one.']];
cards.forEach((c,i)=>{
  const x=5.45+(i%2)*3.65, y=3.75+Math.floor(i/2)*1.42;
  s.addShape(p.ShapeType.rect,{x:x,y:y,w:3.45,h:1.24,fill:{color:PAPER}});
  s.addText(c[0],{x:x+0.22,y:y+0.14,w:3.0,h:0.28,isTextBox:true,margin:0,fontFace:B,fontSize:12.5,bold:true,
    color:i===3?OXIDE:BRASS});
  s.addText(c[1],{x:x+0.22,y:y+0.45,w:3.05,h:0.68,isTextBox:true,margin:0,fontFace:B,fontSize:11,color:'4A4E47',lineSpacing:14});
});
s.addNotes('Three legible ideas and one honest unknown.');

/* ---------------- 10 · the turn ---------------- */
s=p.addSlide(); s.background={color:INK};
s.addImage({...IMG('05-terr.png'),x:8.1,y:1.15,w:4.6,h:4.6,transparency:8});
frame(s,8.1,1.15,4.6,4.6);
s.addText('THE TURN',{x:0.75,y:1.35,w:6,h:0.3,isTextBox:true,margin:0,fontFace:B,fontSize:11,bold:true,charSpacing:3,color:BRASS_L});
s.addText('All of it was reaching for one line.',{x:0.72,y:1.85,w:6.9,h:1.9,isTextBox:true,margin:0,
  fontFace:H,fontSize:40,bold:true,color:WHITE,lineSpacing:46});
s.addText('Count the attacks on every square, subtract, and the board stops being sixty-four objects and becomes terrain. Brass is ground one side holds; verdigris the other. Where the colour drains out, nobody holds it.\n\nThat is the page’s destination, and it is computable.',
  {x:0.75,y:4.05,w:6.6,h:2.2,isTextBox:true,margin:0,fontFace:B,fontSize:14.5,color:'C9CCC2',lineSpacing:22});
s.addNotes('Pivot from reading the notebook to executing it.');

/* ---------------- 11 · the frontier ---------------- */
s=p.addSlide(); s.background={color:WHITE};
eyebrow(s,'THE VIEW');
title(s,'The frontier');
body(s,'One contour matters more than the rest: the line where white’s attacks and black’s exactly cancel. Neither side owns it. It is the border the page kept circling without ever drawing.\n\nIt has a length, and the length is a real number that moves as the game does. A short border is a quiet position. A long, ragged one is a knife fight.',
  0.65,2.05,6.0,null,null,3.1);
board(s,'07-frontier.png',6.9,1.9,4.5);
s.addShape(p.ShapeType.rect,{x:0.65,y:5.3,w:6.0,h:1.1,fill:{color:PAPER}});
s.addText([{text:'9.5',options:{fontSize:27,bold:true,color:BRASS}},
           {text:'  squares of border in this position',options:{fontSize:12,color:MUTED}}],
  {x:0.95,y:5.5,w:5.6,h:0.7,isTextBox:true,margin:0,fontFace:M,valign:'middle'});
cap(s,'The zero contour, drawn dashed.',6.9,6.55,5.3);
s.addNotes('The payoff of the whole abstraction ladder.');

/* ---------------- 12 · tension ---------------- */
s=p.addSlide(); s.background={color:WHITE};
eyebrow(s,'THE VIEW');
title(s,'A sacrifice has a shape');
board(s,'08-full.png',0.65,1.9,4.5);
body(s,'Weigh each contact by what the exchange actually costs, and one move on the board dominates everything else. The queen sits deep in enemy ground with the heaviest mark on the sheet — ten pawns of material deliberately hanging.\n\nAnd the border bends up and around her. She is on the wrong side of her own frontier. That is what a sacrifice is, drawn: material put over the line, with the line closing behind it.',
  5.6,2.0,7.05,null,null,3.15);
[['15.2','tension, in pawns',BRASS],['10.0','White material at risk',OXIDE]].forEach((c,i)=>{
  const x=5.6+i*3.6;
  s.addShape(p.ShapeType.rect,{x:x,y:5.28,w:3.4,h:1.12,fill:{color:PAPER}});
  s.addText(c[0],{x:x+0.24,y:5.4,w:3,h:0.5,isTextBox:true,margin:0,fontFace:M,fontSize:24,bold:true,color:c[2]});
  s.addText(c[1],{x:x+0.24,y:5.92,w:3,h:0.3,isTextBox:true,margin:0,fontFace:B,fontSize:10.5,color:MUTED});
});
cap(s,'A stand-in game — Morphy, Paris 1858 — standing where yours will go.',0.65,6.52,5.6,OXIDE);
s.addNotes('Flag clearly that this is not the user’s game.');

/* ---------------- 13 · search ---------------- */
s=p.addSlide(); s.background={color:WHITE};
eyebrow(s,'THE VIEW');
title(s,'Every future is a different border');
s.addImage({...IMG('11-tree.png'),x:0.65,y:2.1,w:7.5,h:2.62});
body(s,'Push the position forward and the frontier moves with it. Each branch here is a candidate continuation, and each one redraws the line somewhere else.\n\nSearch, in these terms, is just the question of which border you would rather be defending.',
  8.5,2.1,4.2,13.5);
cap(s,'Twenty candidate futures, scored and laid out left to right.',0.65,4.95,7.5);
s.addNotes('Ties the search engine back to the page’s territorial framing.');

/* ---------------- 14 · what is missing ---------------- */
s=p.addSlide(); s.background={color:WHITE};
eyebrow(s,'THE GAP',null,null,OXIDE);
title(s,'The one thing the page does not contain');
s.addText('Your game.',{x:0.65,y:1.6,w:11.4,h:0.7,isTextBox:true,margin:0,fontFace:H,fontSize:26,italic:true,color:OXIDE});
body(s,'Every board in this deck is a stand-in. The notes describe a method in detail and never once record the moves it was being applied to — so the atlas is currently running on famous games instead of yours.\n\nThat is a one-step fix, and it is the step only you can take.',
  0.65,2.5,6.0);
const need=[['Moves','A PGN, a lichess or chess.com link, or a scrappy move list — 1.e4 e5 2.Nf3 …'],
            ['Or a photo','A scoresheet works. So does anything with the sequence on it.'],
            ['Your borders','The squares where you felt the edge was, and when. That layer cannot be derived — it has to come from you.']];
need.forEach((c,i)=>{
  const y=2.35+i*1.45;
  s.addShape(p.ShapeType.ellipse,{x:6.95,y:y+0.08,w:0.4,h:0.4,fill:{color:i===2?OXIDE:BRASS}});
  s.addText(String(i+1),{x:6.95,y:y+0.08,w:0.4,h:0.4,isTextBox:true,margin:0,align:'center',valign:'middle',
    fontFace:B,fontSize:11,bold:true,color:WHITE});
  s.addText(c[0],{x:7.55,y:y+0.06,w:5.1,h:0.3,isTextBox:true,margin:0,fontFace:B,fontSize:13.5,bold:true,color:INK});
  s.addText(c[1],{x:7.55,y:y+0.42,w:5.1,h:0.85,isTextBox:true,margin:0,fontFace:B,fontSize:11.5,color:'4A4E47',lineSpacing:15});
});
s.addNotes('Concrete ask, three options.');

/* ---------------- 15 · end ---------------- */
s=p.addSlide(); s.background={color:INK};
s.addImage({...IMG('06-contour.png'),x:7.7,y:0.95,w:5.0,h:5.0,transparency:10});
frame(s,7.7,0.95,5.0,5.0);
s.addText('WHERE IT LANDS',{x:0.75,y:1.4,w:6,h:0.3,isTextBox:true,margin:0,fontFace:B,fontSize:11,bold:true,charSpacing:3,color:BRASS_L});
s.addText('You were drawing a map.',{x:0.72,y:1.9,w:6.5,h:1.5,isTextBox:true,margin:0,
  fontFace:H,fontSize:40,bold:true,color:WHITE,lineSpacing:46});
s.addText('The page works through pieces, arrows, cover, vision, an alphabet, and a string of bits — and every one of those steps is a way of asking the same question. Where does my board end and theirs begin?\n\nThe machine can now draw that line for any position. The one it cannot draw is the line you felt while you were playing. Put the two on top of each other and the difference is the interesting part.',
  {x:0.75,y:3.6,w:6.5,h:2.6,isTextBox:true,margin:0,fontFace:B,fontSize:14,color:'C9CCC2',lineSpacing:21});
s.addNotes('Close on the gap between the computed border and the felt one.');

p.writeFile({fileName:__dirname+'/Reading-the-Page.pptx'}).then(f=>console.log('wrote',f));
