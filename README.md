# BlockChess

Structural analysis of chess positions recorded as **fragments** — a handful of
men written down mid-game, often without the kings, often not in order of
placement.

That is a different object from a chess position, and the library is built for
it. A note denotes a *set* of positions, not one; a game is a partially observed
DAG, not a move list; and a man missing from a note was probably not written
down rather than captured.

```bash
python -m blockchess report games/01-self-play/study.json
python -m blockchess note "white [pawn: A5, B4/bishop: A3] black [knight: C6]"
python -m blockchess attach games/01-self-play/study.json position-2.json
```

```python
from blockchess import parse_note, coupling_table, compare

board = parse_note("white [pawn: A5, B4/bishop: A3] "
                   "black [pawns D5, E5/bishop D7/knight C6]")

coupling_table(board)[0]          # b4, static 4 — the position's control point
compare(board, [{"move": "a3b2"}, {"move": "b4b5"}])
```

The notebook format parses as written, so notes do not have to be rewritten to
be analysed.

## What it measures

| | |
|---|---|
| **Beams** | every ray of every long-range piece, what stops it, and whose fault it is |
| **Coupling** | how many systems a square carries — static (live now) and latent (within one move) |
| **Scope** | mobility *unlocked* from men already present vs. merely *imported* by a new one |
| **Commitment** | whether a move can be taken back; pawn moves and captures cannot |
| **Growth** | scope, optionality and commitment per ply, so branches compare on all three |

Nothing here evaluates who is better. Structure is the intended altitude.

## Layout

```
blockchess/    board · metrics · tree · notation · report · CLI
games/         held games as JSON
docs/          holding-model.md — the data model and its limits
case-studies/  written analyses, pinned to the engine by tests
tests/         29 tests holding case study 01 to the numbers
```

- **[docs/holding-model.md](docs/holding-model.md)** — how a game is held, why it
  is not held as a move list, and what the framework deliberately does not do.
- **[case-studies/01 — The b4 Inflection](case-studies/01-the-b4-inflection.md)**
  — Position 1: coupling, unlock vs. import, and ordering by irreversibility.

## Tests

```bash
python -m unittest discover -s tests
```

The case study states its numbers in prose; the tests pin them to the engine, so
a measure that changes makes the prose fail loudly instead of drifting.
