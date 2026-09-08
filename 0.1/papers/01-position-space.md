# 01 — The space of positions, and the group that acts on it

*All computed figures in this paper come from `measure/counting.py` and
`measure/symmetry.py`; the raw output is `measure/RESULTS.md`.*

---

## 1. The border is the whole story

Start with the observation that motivated this paper: **the chessboard does not
reflect back on itself.** It is not a torus. A knight on b1 has fewer moves than
a knight on d4, not because of anything in the rules, but because the board
stops.

That is measurable, and it is the cleanest single number in this project. Take
the king's move — the 3×3 neighbourhood minus the centre — and count how many
squares survive the edge:

| squares | king's neighbours | why |
|---|---|---|
| 4 | 3 | corners: two thirds of the neighbourhood is off the board |
| 24 | 5 | edge, non-corner: one third is off the board |
| 36 | 8 | interior: nothing is lost |

Total adjacency, summed over all squares: **420**.

On a torus every square would have 8 neighbours and the total would be 512. The
border costs 92 adjacencies, 18% of the graph. Every counting result below
inherits that deficit, and every "and now apply symmetry" argument dies on it.

The immediate consequence. Ordered pairs of distinct, non-adjacent kings:

```
64 × 63 − 420 = 3612
```

On a torus it would be `64 × 63 − 512 = 3520`. The board's edges *add* 92 legal
king configurations, because pieces at the edge crowd each other less.

**3612** is the count that seeds every bound in §2.

## 2. A ladder of upper bounds

Each row adds exactly one constraint. What matters is not the counts but the
`saved` column: the price, in bits, of each fact about chess.

| | bound | count | bits | saved |
|---|---|---|---|---|
| B0 | every square is one of 13 states | 1.961e71 | 236.83 | — |
| B1 | + pawns only on ranks 2–7 | 1.354e70 | 232.97 | **+3.9** |
| B2 | + exactly two kings, non-adjacent | 6.000e66 | 221.83 | **+11.1** |
| B3 | + ≤16 men a side, promotion budget | 2.148e49 | 163.88 | **+58.0** |
| B4 | × side to move | 4.296e49 | 164.88 | −1.0 |
| B5 | × castling rights (≤16) | 6.873e50 | 168.88 | −4.0 |
| B6 | × en passant file (≤9) | 6.186e51 | 172.05 | −3.2 |

Read that column. Three observations, in increasing order of usefulness.

**The pawn-rank rule is nearly free.** Ruling out 16 of 64 squares for two of
the twelve piece types buys 3.9 bits out of 237. The constraint everyone states
first is the one that matters least.

**Material caps are the entire game.** B3 alone is worth 58 bits — more than a
third of the whole encoding, and more than every other constraint combined.
The reason is that B0–B2 permit boards with forty rooks on them. Chess's real
constraint is not *where* pieces may stand but *how many there may be*, and the
promotion budget (every piece beyond the starting complement cost a pawn) is
doing most of that work.

**The three state flags cost 8.2 bits.** Side to move, castling rights and the
en passant file are not "the board", but they are part of the position, and
they undo 8 of the 58 bits that material caps won. That is a real number to
carry into `02-encodings.md`: the flags are 5% of the encoding, and the naive
castling encoding (4 free bits) wastes some of it, because castling rights are
only meaningful when the king and rook are still on their home squares.

### What B3 enforces, and what it does not

Enforced exactly: two non-adjacent kings; pawns on ranks 2–7 only; at most 16
men a side; and per side

```
p + n + b + r + q ≤ 15
max(0,n−2) + max(0,b−2) + max(0,r−2) + max(0,q−1) ≤ 8 − p
```

the second being the statement that every extra piece arrived by promotion and
every promotion spent a pawn.

Not enforced: bishops needing opposite colour complexes to exist without
promotion; pawn structures reachable only through captures that the material
count contradicts; the side *not* to move being in check; and any notion of the
position being reachable from the initial position at all.

## 3. The gap, and why it is not a counting problem

| | bits | source |
|---|---|---|
| B6, derived here | 172.1 | `measure/counting.py` |
| Chinchalkar's upper bound on legal positions | ≈153.7 | `[lit]` 1.7986e46 |
| Tromp's estimate of legal positions | ≈148.4 | `[lit]` 4.82e44 |
| Shannon's figure | ≈142.8 | `[lit]` 1e43 |

Our bound is **23.6 bits above** the best estimate. That is a factor of 13
million, and it is worth being precise about where it went, because the answer
is structural rather than a matter of trying harder.

Every constraint in the ladder is *local*: it is a statement about one square,
one pair of squares, or one tally. Local constraints are exactly the ones a
product formula can express. The remaining 23.6 bits are **reachability** — the
property of being the endpoint of some legal game — and reachability is not a
predicate on a position, it is a predicate on the existence of a path.

> **A counting argument can bound the positions that satisfy the rules. It
> cannot bound the positions that satisfy the rules *and* have a history. That
> second set is defined by a dynamical system, and its size is measured by
> sampling, not by multiplying.**

This is why Tromp's figure is an *estimate* with a confidence interval and not
a formula: it is a Monte Carlo estimate over the set B3-like bounds define. It
is also why any encoder that tried to hit 148 bits would need to enumerate the
reachable set, which nobody can do.

**Practical floor for a self-contained position encoder: about 154 bits, or 20
bytes.** The last 5 bits are unavailable to a formula, and the 23.6 are
unavailable to anyone.

## 4. The number that matters: positions versus moves

A position costs ~150 bits. A move costs about 5 (`02-encodings.md`). A game is
about 80 plies. So:

| store a game as | bits | bytes |
|---|---|---|
| 80 positions at Tromp's floor | 11,872 | 1,484 |
| 80 positions at our derived bound | 13,764 | 1,721 |
| 80 legal-move indices (b ≈ 30) | 393 | 49 |

**Storing positions costs thirty times more than storing moves.** No refinement
of position encoding recovers that. Even a perfect 148-bit position encoder,
which cannot exist, loses to a lazy move encoder by a factor of 30.

The whole of §2 and §3 exists to establish that this ratio is real and not an
artefact of a bad position encoder. It is real. **Store moves.**

Positions are still needed — for indexing, for transposition detection, for
"who else has reached this?" — but as *derived* keys, computed by replaying
moves, not as stored data. That distinction is the subject of `05-index.md`.

## 5. What group actually acts on a chess position

Now the geometry. A position lives on a square, and the square has the dihedral
group `D4` of order 8: four rotations and four reflections. The question is how
much of `D4` survives contact with the rules.

Define a group element as a pair (permutation of the 64 squares, swap colours).
Swapping colours also flips the side to move, so the operation maps a position
to the same position seen from the other side.

**Pawns kill rank reversal.** A rank-reversing permutation makes white pawns
move down the board. It is only a symmetry if it is paired with a colour swap,
which turns the white pawn into a black pawn moving the right way.

**Castling kills file reversal.** The file mirror sends the king from e1 to d1.
A position with castling rights available has a king on e1 by definition, and
its mirror image has a king on d1 with castling rights, which is not a chess
position.

**The diagonals need both to be dead.** A diagonal reflection maps a file to a
rank, so it turns pawn moves sideways. There is no colour swap that repairs it.
Diagonals only survive on a pawnless board.

That gives four regimes, measured in `measure/symmetry.py`:

| regime | group | order |
|---|---|---|
| no pawns, no castling rights | `D4 × ⟨colour swap⟩` | **16** |
| pawns on the board, no castling rights | `⟨file mirror⟩ × ⟨colour swap⟩ ≅ Z2 × Z2` | **4** |
| castling rights outstanding | `⟨colour swap⟩ ≅ Z2` | **2** |
| the general case | trivial | **1** |

The user-facing summary: **chess has almost no symmetry.** The intuition that
the board "does not reflect back on itself" is right, and this table is the
quantitative form of it. A square has eight symmetries; a real middlegame
position, with pawns and with at least one side still able to castle, has one.

## 6. What symmetry is worth in bits

The group order is a ceiling, not a saving. A canonical form saves
`log2(mean orbit size)`, and orbits are smaller than the group whenever a
position is fixed by some group element. Measured:

| family | \|X\| | orbits | bits saved | ceiling |
|---|---|---|---|---|
| K vs K, pawnless | 7,224 | 462 | **3.967** | 4.00 |
| KQ vs K, pawnless | 447,888 | 56,112 | **2.997** | 4.00 |
| KQ vs K *and* K vs KQ, pawnless | 895,776 | 56,112 | **3.997** | 4.00 |
| KP vs K, one pawn | 336,048 | 168,024 | **1.000** | 2.00 |
| KP vs K *and* K vs KP, one pawn | 672,096 | 168,024 | **2.000** | 2.00 |
| KP vs K, castling rights outstanding | 336,048 | 336,048 | **0.000** | 1.00 |

Three things fall out, and rows 2 and 3 are the pair to look at.

**The colour swap only pays if your set is closed under it.** KQ vs K saves 3.0
bits; the union of KQ vs K and K vs KQ saves 4.0. The extra bit is not a
property of the position, it is a property of the *collection*. Storing white's
games alone gets nothing from the colour swap; indexing all games gets a bit.
This is the first appearance of a theme that runs through the rest of the
papers: **savings that exist in the index do not exist in the record.**

**The file mirror achieves its ceiling exactly.** 1.000 bits, not 0.98. The
mirror `f ↦ 7−f` has no fixed square — it would need file 3.5 — so no position
is ever its own mirror image, every orbit has exactly two elements, and the
saving is exact. That is the border again: on a board with an odd number of
files there would be a fixed centre file, positions could be self-mirrored, and
the saving would be strictly less than a bit.

**In the regime that describes almost every real position, symmetry is worth
zero.** Last row. Pawns on the board and castling rights outstanding is the
first twenty moves of essentially every game ever played, and there the group is
trivial after the colour swap is excluded, and the saving is nil.

### The conclusion, stated flatly

> **Symmetry reduction is worth between 0 and 1 bit per position in practice,
> and 0 bits per move, since we store moves. As compression it is not worth
> implementing. As an index key it is worth quite a lot, because it collapses
> mirrored openings into one line for study.**

Both halves of that matter. The first half stops you writing a canonicalisation
routine to save space. The second half is a genuine feature: a Sicilian and its
file-mirrored twin are the same idea, and a database that shows them as one
entry is a better database. Symmetry belongs in `05-index.md`, not here.

## 7. Group structure that is load-bearing

Symmetry turned out to be worth little. Two other group-theoretic facts are
worth a great deal, and both are about *incremental* computation rather than
about compression.

### 7.1 Board edits form a group; position hashing is a homomorphism from it

Fix a random 64-bit key `k(p,s)` for each piece type `p` and square `s`. Define

```
Z(position) = ⊕ { k(p,s) : piece p stands on square s }  ⊕  flags
```

The set of board edits under composition is a free abelian group; `(GF(2)^64, ⊕)`
is an abelian group in which every element is its own inverse; and `Z` is a
homomorphism between them. Applying a move is two XORs. Undoing it is applying
the same two XORs again.

The reason this matters here rather than as a curiosity: **the homomorphism
property is what makes transposition detection cheap.** Two different move
orders reaching the same position produce the same `Z`, automatically, because
the map forgets the order — that is exactly what "homomorphism from an *abelian*
group" means. Order-independence is not a trick, it is the group's commutativity
showing through.

It also tells you the limit. `Z` is a homomorphism into a group of order `2^64`,
so it has a kernel of colossal size, and the birthday bound puts collisions at
`2^32` positions. Fine inside one engine's transposition table; not fine as a
public database key that anyone can grind against.

### 7.2 The upgrade: a set commitment with a hard kernel

The same construction over a group where the discrete logarithm is hard —
multiplicative in a large prime field, or an elliptic curve — gives an
*incrementally updatable, order-independent, collision-resistant* commitment to
the set of men on the board. Same two-operation update, same forgetting of move
order, but now no adversary can find two positions with the same key.

This is a real design option for `05-index.md`: it makes "prove this position
occurred in this game" cheap, and it makes the position key adversary-safe.
It costs a group operation instead of an XOR, which is roughly a hundred times
slower and entirely affordable at database-write rates.

### 7.3 Games are paths in a groupoid, not elements of a group

The temptation is to say the move set generates a group acting on positions. It
does not, and the failure is informative. A move is a *partial* function: it is
defined only on the positions where it is legal. Partial invertible maps compose
into a groupoid, not a group. And a capture is not invertible from the position
alone — the captured piece is gone — so undoing needs the move *plus* the
captured man.

Which is to say: the object to reason about is the **directed graph of
positions**, with legal moves as edges, and a game is a path in it. Two
questions follow immediately and they are the fork in `03-corpus.md`:

- Store **paths**, and transpositions are duplicated but move order is kept.
- Store the **graph**, and transpositions are shared but move order is lost.

The graph is a DAG only if you quotient out repetitions; with the fifty-move
counter in the position it genuinely is one, because the halfmove clock is
monotone except on captures and pawn moves, which are themselves irreversible.
That is a pleasant fact and it is worth stating carefully in `05-index.md`.

## 8. Summary

| Result | Value | Where it goes |
|---|---|---|
| King adjacency total | 420 of a possible 512 | the border, quantified |
| Legal king pairs | 3,612 | seeds every bound |
| Derived upper bound on positions | 172.1 bits | §2 |
| Best literature estimate | ≈148.4 bits `[lit]` | §3 |
| Gap attributable to reachability | 23.6 bits | not a counting problem |
| Position vs move | ~30× | **store moves** |
| Symmetry group, general position | trivial | §5 |
| Symmetry worth, in practice | 0–1 bit per position | index only, not storage |
| Zobrist collision resistance | `2^32` birthday bound | engine only, not a public key |
