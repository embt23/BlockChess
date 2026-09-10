# The math of holding chess games

How BlockChess stores a game, why it is not stored the way chess software
normally stores one, and what can be computed once it is stored this way.

---

## 1. The problem: these notes are not a game

A chess application holds a game as a **start position plus a move list**. That
is a *path*: totally ordered, complete, every ply present. Everything downstream
— engines, databases, PGN — assumes it.

The BlockChess notebook holds something else:

> `white [pawn: A5, B4/bishop: A3] black [pawns D5, E5/bishop D7/knight C6]`
> *(not in order of placement)*

Three properties break the path model at once:

| Property | Consequence |
|---|---|
| **Partial** — the kings were never written down | This is not a position. It is a *set* of positions. |
| **Unordered** — "not in order of placement" | There is no ply index to hang it on. |
| **Sparse in time** — positions 1, 2, 3, with gaps | The edges between them are unknown. |

So the object being held is not a path. It is a **partially observed DAG**:
nodes are observations, edges are move sequences, and most of both was never
recorded.

Every design decision below follows from that one fact.

---

## 2. An observation is a position class

A note does not denote a position. It denotes the **set of full positions
consistent with it** — every legal placement of the men that were not written
down.

That set is countable, and `placement_space()` counts it. For Position 1, with
only the two kings unrecorded:

```
2,862 positions consistent    11.5 bits missing
```

(57 empty squares, two distinguishable kings, minus the placements where the
kings stand adjacent — which is why the count is 2,862 and not 57 × 56 = 3,192.)

This gives **the information content of a note a unit**. Writing down where the
kings are is worth 11.5 bits. Writing down a rook is worth more. The measure is
exact for small rosters, and falls back to a falling-factorial approximation for
large ones — the return value says which.

> This is the honest version of "how it interacts with information theory."
> Not entropy as a metaphor: a countable state space that a note shrinks.

---

## 3. Records are open by default

A man missing from a note was **probably not written down**, not captured. This
is the single most important rule in the model, because getting it backwards
makes the framework confidently wrong.

So every check is one-sided:

- **Open record** (default). Positive evidence only. A man appearing in a later
  note that was absent from an earlier one raises a *warning*, not a violation —
  it was probably always there.
- **Closed record** (`closed: true`). The recorder asserts that everything on the
  board is written down. Now material monotonicity and pawn monotonicity become
  binding, and their breaches are *violations*.

`link_check()` returns violations and warnings separately, and never promotes a
warning to a violation on an open record.

---

## 4. Attaching later data

Given a new note, the question is not "what ply is this?" but **"where can this
sit, and by what moves?"** `Study.attach()` answers it in two stages:

1. **Cheap necessary conditions** (`link_check`) — pawn monotonicity per file,
   material monotonicity between closed records. Rules out impossible parents
   fast, with a stated reason:

   ```
   possible: False
     violation: w pawn on file b: most advanced goes b4 -> b3; pawns do not retreat
   ```

2. **Exact bounded search** (`find_paths`) — where the check allows it, actually
   search for move sequences that produce every man the child records. A path
   succeeds when all recorded men stand where the note says; men the note does
   not mention are unconstrained, which is what makes the record open.

   ```
   from position-1: possible
     1 path(s), shortest 2 plies:
       b4b5 c6d4
   ```

The search is exponential and therefore depth-capped. It is not trying to
reconstruct the game — it is answering whether two notes can be *k* moves apart,
and if so, how. Where several paths exist, that ambiguity is the finding: the
notes do not determine the line.

---

## 5. The four measures

Each replaces something the case study was doing by hand.

### Beams — `beams()`, `self_blocked_beams()`
Every ray of every sliding piece, with what stops it and **whose fault it is**.
The `self_blocked` flag separates a bishop with somewhere to go from a bishop
staring at its own pawn.

> *Law 01 — a long-range piece's value is a property of its line, not its square.*
> Ba3 and Bb2 are the same bishop one square apart, worth 2 moves and 6.

### Coupling — `coupling_table()`
For each occupied square, how many **distinct systems** change state if its
occupant changes. Five computable predicates:

`blocks_friendly_line` · `blocks_enemy_line` · `defends` · `sole_control` · `lever`

Reported in two forms:

- **Static** — live in the position as recorded.
- **Latent** — switches on somewhere in the one-move neighbourhood.

The gap between them is the useful signal. In Position 1, `b4` scores static 4
(highest on the board) while `d5` and `e5` score static 1 / latent 3 — dormant
now, loaded. *The latent column is the measure independently finding the `Bb2`
idea that the case study found by hand.*

> *Law 03 — the control point is the least valuable man carrying the most load.*

### Scope — `evaluate_move()`
Splits mobility gained into two currencies that are **not interchangeable**:

- **Unlocked** — gained by men already in the fragment. Structural; compounds.
- **Imported** — carried in by a man arriving from off-record. Linear in pieces
  developed.

`Nc3` posts the biggest raw number in Position 1 (+8) and unlocks nothing.
`Bb2` posts +4, all of it unlocked.

### Commitment — `evaluate_move().reversible`
A piece move is reversible: it spends tempo, not options. A **pawn move or a
capture permanently deletes a region of the position's future state space**.
A pawn *introduced* onto e3 counts as irreversible too — it got there by a pawn
move, and no later play takes that back.

---

## 6. Balanced growth

`growth_profile()` walks a branch recording four columns per ply: **scope,
opponent scope, cumulative unlocked/imported, cumulative commitment.**

The columns move independently, which is the entire point. Scope can rise while
optionality collapses. Commitment only ever rises. `balance` reports scope
gained per irreversible decision, and is `None` when nothing was spent — which
means *free*, not *best*.

**Balanced growth is not maximal growth.** A branch that gains the most scope
while spending the most commitment is not obviously ahead of one that gains less
for free; those are different currencies and the profile refuses to collapse them
into a single score on your behalf. `compare()` orders candidates
reversible-first, then by scope unlocked — the ordering rule from case study 01,
not raw gain.

---

## 7. What this deliberately does not do

Stated plainly, because a framework that hides its gaps is worse than one that
has them:

- **No en passant, castling, or promotion** in move generation. Fragments rarely
  carry the rights, and inventing them would be worse than omitting them.
- **Pseudo-legal moves unless both kings are recorded.** Exact when no king is
  present (nothing can be in check); check-filtered when both are.
- **Mobility counts destinations, not quality.** A knight with eight bad squares
  outscores a queen with three good ones. It is a scope measure, not an
  evaluation.
- **The five coupling predicates are a choice, not a theorem.** They were picked
  because they are computable and they reproduce the hand analysis. A sixth would
  change every number in the table.
- **`find_paths` is exponential.** Depth-capped by design.
- **No engine evaluation.** Nothing here knows who is better, and it should not
  start pretending to — the whole premise is that structure is the interesting
  altitude.

---

## 8. Layout

```
blockchess/
  board.py      geometry, rays, attacks, mobility  — the fragment world
  metrics.py    beams, coupling, scope, commitment — the four measures
  tree.py       observations, links, search, growth — the holding model
  notation.py   reads the notebook format as written
  report.py     text reports
  __main__.py   CLI
games/
  01-self-play/study.json
docs/
  holding-model.md
tests/
  test_position1.py   29 tests pinning case study 01 to the engine
```

```
python -m blockchess report games/01-self-play/study.json
python -m blockchess note "white [pawn: A5, B4/bishop: A3] black [knight: C6]"
python -m blockchess attach games/01-self-play/study.json position-2.json
```

---

## 9. Adding positions 2 and 3

1. Write the note in the notebook format — no rewriting needed.
2. `parse_note(...)` → `Observation(key, board, note=..., closed=False)`.
3. `study.attach(obs)` → candidate parents, ruled-out parents with reasons, and
   the move sequences that connect them.
4. Where the search finds several paths, that ambiguity **is** the result: record
   the link as `min_plies` rather than asserting a line.
5. Add branches at the new node and re-run the report to compare growth from
   there.

The one thing worth recording that is not yet in the notes: **the kings.** They
are 11.5 bits, and they are what would turn Branch A's value from scope into
something concrete.
