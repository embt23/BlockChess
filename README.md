# Chess Frontier Atlas

An interactive, multilayered search map for chess — built from a page of a chess
journal that asked where the *borders* of a game are.

`index.html` is a single self-contained page. No build step, no dependencies:
open it in a browser, or serve the directory.

## What it does

Every layer is computed from scratch in the page — there is a complete chess
engine in there (move generation verified against the standard perft suites:
startpos, kiwipete and the en-passant/promotion position, to depth 4).

| Layer | What it draws |
|---|---|
| Pieces | The position as it stands |
| Territory | White attacks minus black attacks, washed across the board |
| Contours | Level lines of that field, as on a survey map |
| **Frontier** | The zero contour — the border, where the two claims cancel |
| Tension | Pieces in contact; filled marks are hanging |
| Vision | Every line the long pieces see down |
| Abstraction | Each square reduced to one arrow of net push |
| Pressure | The raw signed counts |

Readings alongside: frontier length in squares, tension pairs, contested
squares, mirror symmetry of the pressure field, field entropy, and the whole
board compressed to four bits a square as one bit string.

**Story mode** animates the construction one abstraction at a time.
**Explore mode** hands over the layer stack, the game timeline, and search from
any position. The search map grows a tree of candidate futures — hover a node
and the board morphs to show that position's border.

Load any game or position by pasting a PGN or FEN.

## Journal terms → layers

first abstraction → Abstraction · vision → Vision · cover → Tension ·
information / compression / symbol → Compression · borders → Frontier ·
symmetries → Mirror symmetry reading
