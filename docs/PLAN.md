# Read the page, then build the territory game

## Context

The project so far turned one page of a chess journal into a working instrument:
`index.html` computes a control field for any position, draws the frontier (the
zero contour where each side's attacks cancel), weighs tension by static
exchange evaluation, and renders pieces in the author's own six-mark alphabet.
All of it is verified by `./tools/verify.sh`.

Two things are still true. **We have never once run it on the author's own
game** — every board shown has been a stand-in. And the journal's central idea,
that a position is *ground* rather than a set of objects, has never been made
into a game you can win.

This plan does both, and connects them: the alphabet that encodes the page is
also the encoding that makes the game cheap to put on-chain.

Two decisions from the author shape it:

- **One page exists.** So computer vision is the wrong tool — it would mean
  training a classifier on a dataset of one, taking days to build something less
  accurate than an afternoon of hand-tagging. Build a tagging workbench instead.
- **The on-chain target is a territory variant**, not standard chess.

### On "first"

Fully on-chain chess already exists in several implementations. This plan does
not claim, and must not market, a first. What may be unoccupied is a chess
*variant scored by territorial control* settled on-chain — but that is an
unverified belief about a fast-moving space, so no document produced here should
assert it as fact.

### One fact worth building on

The alphabet is six marks with one rotation rule, so a square holds 13 states
(6 pieces × 2 colours, plus empty). 13 fits in 4 bits; 64 × 4 = **256 bits =
exactly one EVM storage word**. A full position is one write. The journal's
obsession — information, compression, symbol — turns out to be precisely the
constraint that dominates on-chain design. Verify this claim in code before
relying on it.

---

## Track A · The page workbench — BUILT

`workbench.html` is done, published, and verified end to end: tagging a board of
the author's glyphs produces `rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR`, the
standard opening position, so the alphabet reads correctly. It takes JPGs and
PDFs (a photographed page is usually one embedded JPEG, pulled out with no
library). `tools/tests/workbench-e2e.mjs` and `workbench-pdf-e2e.mjs` cover both
paths.

**What remains on this track is not code — it is the page itself.** Every board
shown so far is still a stand-in. The author needs to locate the real photo of
the journal page (the one sent at the start of the conversation), and per the
intake rule above, confirm the exact file before anything is copied.

Also done since this plan was written: the glyph alphabet is decoded and
recorded in `NOTES.md` (six marks, one rotation rule, king as a Latin cross so
the rule has no exception), and position packing is proven — 80 positions
round-trip byte-identical through 32 bytes, one EVM word (`tools/tests/pack.mjs`).

Original spec, kept for reference:

1. **Load.** Drag the photo in; read it with `FileReader` and draw to canvas. No
   upload, no server, no capability needed — the file never leaves the machine.
   Pan and zoom.
2. **Regions.** Drag rectangles over the page and tag each one: `board`,
   `sequence`, `key`, `prose`, `diagram`. This alone is a useful map of a page
   that currently exists only as an image.
3. **Grid fit.** For a `board` region, overlay an n×m grid with draggable corner
   handles so it lines up with the hand-ruled lines. Rows and columns are
   adjustable — the page has grids that are not 8×8.
4. **Tag cells.** Click a cell, pick from a palette of the six glyphs × two
   rotations, plus empty. **Reuse `drawGlyph()` from `index.html`** so the
   palette is drawn by the same code that renders the board — the tagger sees
   exactly the marks the Atlas will draw.
5. **Validate live.** As cells are filled, run the tagged board through the
   engine: piece counts, one king a side, no pawns on rank 1 or 8, kings not
   adjacent. Show a running verdict. *The chess rules do the error correction* —
   the same idea as a constraint decoder, with a human as the classifier.
6. **Export.** A JSON file of regions and positions, plus FEN per board, plus a
   "send to Atlas" button that loads it straight into `index.html`.

Persistence: `localStorage` for in-progress work, JSON export as the real save.
If the tagging should survive across devices or be shareable, load the
`artifact-capabilities` skill first and use a storage capability instead.

**The built-in ground truth:** the page has a board explicitly labelled *Start*.
Tagging it must decode to the standard opening FEN. That single test validates
the workbench, the grid fitter, the palette, and my reading of the alphabet all
at once. Do it first.

Open question this resolves, from `NOTES.md`: whether the large glyph field is a
transcription or drills. Tag a corner of it and see whether it validates as
positions.

---

## Track B · The territory variant, off-chain first

**The risk here is not technical, it is whether the game is any good.** That is
answerable in a day, before any Solidity exists.

Add a **Territory game** mode to `index.html`, reusing `fields()` (which already
returns `tw` / `tb` / `tn`) as the live score.

Starting rules, to be played and then revised:

- Standard chess movement and captures.
- Score = squares where your attack count exceeds theirs — `fields().tw` and
  `.tb`, already computed.
- Checkmate remains an instant win, or the game stops being chess.
- Otherwise the game ends on a move limit, and most ground wins.

Material still matters, because material *buys* ground — which is the interesting
tension, and exactly the journal's reading of a sacrifice as material pushed over
the line.

**Design problems to test for, not hand-wave:**

- *Turtling.* Shuffling pieces to inflate attack counts without engaging. A
  likely fix is requiring ground to be held for a full turn before it scores.
- *Long-range dominance.* Raw attack counts over-reward queens and rooks. May
  need per-piece weighting, or counting only squares in the opponent's half.
- *Draws by symmetry.* Two mirrored players may deadlock at 32–32.

Play a dozen games in the Atlas before touching a chain. If it is not fun here,
it will not be fun on-chain and the rest of the plan should be dropped.

---

## Track C · On-chain

Only once Track B produces a game worth playing.

**C1 · Encoding** (small, self-contained, provable — can be done any time)

`tools/tests/pack.mjs`: encode a position to 256 bits and back.

- 4 bits a square, 13 used codes, 3 spare.
- Metadata — side to move, four castling rights, en-passant file — is 9 bits.
  Put it in a second word for clarity; note the option of hiding it in the spare
  per-square codes, but do not do that first.
- **Test:** round-trip every position from both embedded games (33 and 45 plies)
  through pack → unpack → `toFEN`, and assert byte-identical output and a
  32-byte payload. Wire into `tools/verify.sh`.

**C2 · Contract**

- Solidity. State is the packed position. Full move validation on-chain: it is
  simpler to get right than commit-reveal, and an L2 makes it affordable.
- Territory scoring is the expensive call — an attack map over 64 squares.
  Compute it at game end or on challenge, not every move.
- Deploy to an L2 testnet. **Measure gas, do not estimate it.**
- Foundry tests mirroring `tools/tests/` — the same positions, the same expected
  legality, so the Solidity and JavaScript engines are checked against each other.

**C3 · Client.** `index.html` already is the front end; add wallet connection and
read state from the contract.

**Deliberately excluded:** no token, no wagering, no sale of anything. This is a
game contract. Introducing money changes the project into one with real
regulatory obligations and should be a separate, explicit decision.

---

## Sequencing

Tracks A and B are independent and can run in either order.

1. **Workbench** → tag the *Start* board → confirm it yields the standard FEN.
2. Tag the rest of the page → the author's real game enters the Atlas at last.
3. **Territory mode** in the Atlas → play it → revise the rules.
4. **C1 encoding + tests** — cheap, and proves the one-word claim.
5. Contract, only if step 3 produced a good game.

## Verification

| What | How |
|---|---|
| Workbench | The *Start* board decodes to the standard opening FEN. Every tagged board passes the engine's legality checks. |
| Alphabet reading | Same test — if *Start* does not decode cleanly, my reading of the glyphs is wrong, not the tagger. |
| Territory rules | Twelve played games. Look specifically for turtling and for 32–32 deadlocks. |
| Encoding | Round-trip all 78 plies from the embedded games; assert identity and 32 bytes. Added to `./tools/verify.sh`. |
| Contract | Foundry tests reusing the same positions as `tools/tests/`; gas measured on a testnet, not estimated. |

## Files

- New: `workbench.html`, `tools/tests/pack.mjs`, later `contracts/`
- Modified: `index.html` (territory mode; export `drawGlyph` for reuse),
  `tools/verify.sh`, `ROADMAP.md`, `NOTES.md`
- Reused: the engine block in `index.html` (`parseFEN`, `toFEN`, `legalMoves`,
  `fields`, `drawGlyph`), `tools/extract-engine.py`
