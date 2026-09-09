# Reading the journal page

The source for this whole project is one photographed page of a chess journal.
This file is the ledger of what was read off it, and what was not. It is kept
separate from the code so the guesses stay visible as guesses.

**The page is not a game record.** There is no move list, no score, nothing
replayable. It is a working document in which the same board is drawn over and
over, each time with more thrown away.

## Legible

Written in the author's hand, and taken as given:

| On the page | Where | Became |
|---|---|---|
| `first abstraction` | top left, beside a crossed 3×3 grid | The Abstraction layer — pieces → marks |
| A descending column of grids filling with arrows | left edge | The arrow field: each square as net push |
| `cover` | mid left | Folded into Tension — what you hold |
| `Vision`, under an upward arrow | centre | The Vision layer — how far pieces see |
| `Information / Compression / Symbol` | centre, on a dashed rule | The Compression panel |
| `Start` + an 8×8 board in invented glyphs, pawns as single arrows | top right | Confirms a systematic notation |
| A substitution key — numeral ⇒ arrow ⇒ glyph | right margin | Evidence the alphabet was compiled, not doodled |
| `3bits`, bracketed over a row of marks | middle band | See below |
| `4bit`, annotating a second column | middle band | See below |
| `non pawns` | middle band | Piece class treated separately from pawns |
| `Boards`, a dotted enclosure of glyphs | lower left | Positions collected as specimens |
| `Symmetries`, over a dense woven lattice | lower left | The mirror-symmetry reading |
| A written binary string | lower left margin | The notation actually executed |

### The bit count is a real result

Three bits hold eight directions. A square with **no** net push is a ninth
symbol, and three bits cannot hold nine — so the count has to go to four. Both
numbers are on the page, written at different moments. Building the encoder ran
into the same collision independently: the first version emitted `000` for both
"no push" and "due east". The page got there first.

## Unread

Open questions. None of these are guessed at in the code:

- **What each glyph stands for.** The margin key is partial; the alphabet is not
  fully decodable from the sheet alone.
- **`8 geometry`** — boxed *and* circled, so it mattered. It is the only term on
  the page with no worked example beside it.
- **The large glyph field** (right two-thirds). Either a transcription of a real
  game or drills in the new alphabet. The distinction matters enormously: if it
  is a transcription, the game is on the page after all.
- **The solid black squares** recurring on several small boards.
- **The order the page was written in.**

## What is deliberately not inferred

The tool computes an *objective* frontier: the contour where white's attack
count equals black's. The page describes something else — where the author, mid
game, *felt* the border was. Those are different objects and the difference is
the interesting part. The felt border cannot be derived and has never been
guessed; it has to be drawn by hand.
