# Chess Frontier Atlas

An interactive, multilayered search map for chess — built from a page of a chess
journal that asked where the *borders* of a game are.

**Start here:** [`ROADMAP.md`](ROADMAP.md) for where this is going ·
[`NOTES.md`](NOTES.md) for what the journal page says and what it withholds.

```
index.html               the atlas — open it in a browser, no build step
workbench.html           tag a photographed page into positions
NOTES.md                 the journal page: legible vs unread, kept honest
ROADMAP.md               phases, blockers, and the pattern-web spec
docs/presentation/       "Reading the Page" — 15 slides
docs/renders/            board images for every layer
tools/verify.sh          one command: perft, game data, SEE
tools/                   render harness, deck generator, text-fit checker
```

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
| Tension | Pieces in contact — ring sized by the stake, filled by real loss |
| Vision | Every line the long pieces see down |
| Abstraction | Each square reduced to one arrow of net push |
| Pressure | The raw signed counts |

Readings alongside: frontier length in squares, tension, material at risk,
contested squares, mirror symmetry of the pressure field, field entropy, and the
whole board compressed to four bits a square as one bit string.

### Tension is weighted by material

A pawn touching a pawn is not a rook touching a queen, so contacts are not
counted — they are weighed, in pawns:

- **Tension** — the material standing in contact, summed.
- **Material at risk** — full static exchange evaluation on each contested
  square: both sides capture cheapest-piece-first and either may stop when
  continuing loses, and each attacker stepping off the board uncovers the slider
  behind it, so batteries and x-rays count. Reported per side.

The board follows the same measure. Each ring grows with the piece standing in
it and fills in only when the exchange actually loses material — so Morphy's
16.Qb8+ shows a heavy filled ring around a queen with the frontier bending
around her, while a defended pawn contact stays a thin empty circle.

SEE is verified against hand-worked positions (undefended and defended pawns,
a hanging queen, an x-ray battery, and both king-capture legality cases).

**Story mode** animates the construction one abstraction at a time.
**Explore mode** hands over the layer stack, the game timeline, and search from
any position. The search map grows a tree of candidate futures — hover a node
and the board morphs to show that position's border.

Load any game or position by pasting a PGN or FEN.

## Journal terms → layers

first abstraction → Abstraction · vision → Vision · cover → Tension ·
information / compression / symbol → Compression · borders → Frontier ·
symmetries → Mirror symmetry reading
