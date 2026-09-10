# Case Study 01 — The b4 Inflection

**Position 1, White to move.**
White: Pa5, Pb4, Ba3 · Black: Pd5, Pe5, Bd7, Nc6
Fragment of seven men; no kings, queens or rooks placed. Every claim here is
*structural* — about scope, tension and commitment, not about winning material.

> Working note: b4 is the point of inflection / centre of balance / rotation.
> **The arithmetic agrees — but b4 is not a move waiting to be played. It is a
> one-bit switch selecting which of two incompatible games White is playing.**

---

## 00 · The board

The bishop on a3 owns the a3–f8 diagonal. Every square of that diagonal is dark
— including **b4**, where White's own pawn stands. Ba3 has exactly two legal
moves: b2 and c1.

Two facts about colour do most of the work below:

- **Ba3 is dark-squared. Bd7 is light-squared.** They can never meet, block or
  trade. The position decomposes into two non-interacting sub-boards.
- **Black's only dark-square defender is Nc6**, and knights change colour every
  move. Black has no dark-squared bishop in the fragment.

White's three men *look* like a chain — b4 defends a5, Ba3 sits behind. It reads
as cohesion; it isn't. Black's three men genuinely are a chain: e5/d5 phalanx,
Nc6 behind them, Bd7 defending the knight. Each man supports the next and none
obstructs another.

**White's structure is self-obstructing. Black's is self-supporting.** The
material is symmetrical; the topology is not.

---

## 01 · Coupling — why b4 and not some other square

For each occupied square, count the *distinct systems* that change state if that
square's occupant changes. Call it the square's **coupling**.

| Square | Systems it participates in | Coupling |
|---|---|---|
| **b4** | blocks a3–f8 · defends a5 · controls c5 · occupies the b-file · is the lever vs c6/d6 | **5** |
| c6 | guards the dark complex · attacks a5 and b4 · defends e5 · blocks Bd7's b5–a4 diagonal | 4 |
| e5 | phalanx with d5 · supports d4 · terminates the b2–h8 diagonal · fixes a dark-square target | 4 |
| a5 | controls b6 · occupies the a-file · advance to a6 | 3 |
| d5 | phalanx with e5 · controls c4/e4 · sits on its own bishop's colour | 3 |
| a3 | owns one diagonal · blocks the a-file for a rook | 2 |
| d7 | defends c6 · one open diagonal to h3 | 2 |

Coupling is not value. A queen has high coupling everywhere. The interesting case
is a *pawn* with high coupling, because a pawn's state is expensive to change.

b4 tops the table while being the cheapest man on the board. That is what
"centre of rotation" should mean: **a low-value element carrying disproportionate
structural load.**

---

## 02 · The arithmetic of a blocked beam

Legal destinations per man, treating the fragment as the whole board:

| White | Moves | Black | Moves |
|---|---:|---|---:|
| Ba3 | 2 | Nc6 | 7 |
| Pb4 | 1 | Bd7 | 6 |
| Pa5 | 1 | Pd5 | 1 |
| — | — | Pe5 | 1 |
| **Total** | **4** | **Total** | **15** |

Three white men generate four moves — and a *long-range piece* generates two of
them. But mobility measured now understates a loaded spring. What each candidate
buys:

| Move | White mobility | Δ | Unlocked | Imported | Reversible? |
|---|---:|---:|---:|---:|---|
| — (Position 1) | 4 | — | — | — | — |
| Bb2 | 8 | +4 | 4 | 0 | yes |
| b5 | 10 | +6 | 5 | 1 | **no** |
| Nb1–c3 | 12 | +8 | 0 | 8 | yes |

*Unlocked* = squares gained by men already on the board. *Imported* = squares
brought by a man arriving from elsewhere. The two are not interchangeable.

**Nc3 produces the biggest raw number and unlocks nothing.** It adds mobility
linearly by importing a piece. b5 and Bb2 unlock mobility, which is structural
and compounds.

> Adding is linear. Unlocking is structural. Count them separately or you will
> keep mistaking development for progress.

This is the precise content of "flow": a setup flows well when a single move
raises the scope of pieces that were already there.

---

## 03 · The find — the bishop has a second exit, and it's free

Every reading of this position assumes the bishop's future runs through b4: push
the pawn, open the diagonal. That assumption is what makes b4 *feel* like the
pivot. But look at the other diagonal.

**Bb2** puts the bishop on b2–c3–d4–**e5**. The line is clear, and it terminates
on a black pawn. The bishop attacks e5 immediately, with b4 never moving.

And e5 is defended exactly once — by Nc6. Bd7 is light-squared and can *never*
defend a dark square. So White's two ideas turn out to be one idea:

- **Bb2** aims at the only dark-square target Black has.
- **b5** attacks the only man defending it.

No knight retreat holds e5: from c6, none of a7, b8, d8, e7 or d4 defends it.
Black's single resource is **…Nd4** — which saves e5 not by defending it but by
standing on the diagonal, supported by the e5 pawn itself.

Which makes the most important move in the position one that appears on no
candidate list: **e3**, denying d4 without blocking the long dark diagonal
(e3 is not on b2–h8).

---

## 04 · The switch — three moves, two games

The candidates are not three options. They belong to two mutually exclusive plans.

### Branch A — push the gate (the a3–f8 diagonal)
- **b5** opens the bishop and evicts the knight
- **Nc3** supports b5, hits d5
- Bishop stays on a3 — so **no rook on a3**
- Queenside space; a5–a6, b5–b6

*Price:* irreversible · a5 loses its defender · b4 and c5 become permanent holes
· …Nd4 arrives with comfort (supported by e5, hitting b5)

### Branch B — keep the gate (the b2–h8 diagonal)
- **Bb2** re-aims at e5, free of charge
- **e3** denies d4, the one square that shields e5
- **Ra3** takes the square the bishop vacated
- Knight must avoid **c3** — it blocks the new line

*Price:* slower · nothing structural conceded · b4 keeps defending a5 and
covering c5

**b5 and Ra3 cannot both happen.** The rook needs a3, and a3 only empties if
White has abandoned the a3–f8 diagonal. Nc3 belongs to Branch A and actively
damages Branch B, because c3 sits on b2–h8 and blocks the bishop exactly the way
b4 does.

The obstruction pattern repeats three times: **b4 blocks a3 · c3 blocks b2 ·
Nc3 blocks b2.** The recurring conflict here is not White against Black — it is
White's development against White's own only long-range piece.

---

## 05 · Where this meets information theory

The useful bridge is not entropy-as-metaphor. It is the economics of irreversible
decisions, and chess makes the distinction unusually clean:

- **Piece moves are reversible.** A knight on c3 can return to b1. The
  reachable-position set is unchanged; you spent a tempo, not an option.
- **Pawn moves are irreversible.** Pawns never retreat. Every pawn move
  permanently deletes a region of the position's future state-space.

| Candidate | Class | Commitment | Branch it selects | Information acquired |
|---|---|---|---|---|
| Nc3 | piece move | none — reversible | A only | none |
| Ra3 | piece move | none — reversible | B only | none |
| b5 | pawn move | **permanent** | A only | none |
| e3 | pawn move | minor — permanent | **A and B** | forces Black to reveal |

Read the last two columns together and the ordering rule falls out. When one
decision selects between two disjoint plan-spaces, make that decision **last** —
after accumulating the most evidence about which space is better. Everything
played before it should be branch-neutral, or cheap enough to abandon.

By that standard all three candidates share one flaw: **each commits to a branch
and none acquires information.** They are answers offered before the question has
been asked. The move the position wants is useful in both branches and forces
Black to declare first — **e3** being the clearest, since d4 is the pivot of
Black's defence in Branch A and Branch B alike.

> Order your moves by irreversibility, ascending. Spend tempo to buy information;
> spend structure only once the information is in.

---

## 06 · Three laws for the rest of the notes

**Law 01 — the blocked beam.** *A long-range piece's value is a property of its
line, not its square.* Ba3 and Bb2 are the same bishop one square apart, worth 2
moves and 6 moves. Never evaluate a bishop, rook or queen by where it stands:
trace the line, find what terminates it, ask what removing that terminator costs.

**Law 02 — the rival tenant.** *When two of your own men want the same line, a
pawn is usually the tiebreaker.* Before choosing a plan, list the squares your
own men are competing for and decide which piece gets routed around.

**Law 03 — the cheapest high-coupling man.** *The control point is the least
valuable piece carrying the most structural load.* Score coupling first, then
reversibility. A high-coupling **piece** is a resource; a high-coupling **pawn**
is a decision you only get to make once.

---

## 07 · Open questions before Position 2

1. **"Rook 3A" is not legal in Position 1.** a3 holds White's bishop, so no rook
   can go there. Three readings, leading to different studies: (a) a two-move
   plan presupposing the bishop leaves — Branch B, the reading taken above;
   (b) **Rb1**, rook behind the b-pawn backing the lever — Branch A; (c) the
   bishop is on a different square in the notes than a3. (a) and (b) are opposite
   plans, so this needs resolving.
2. **Where are the kings?** The a3–f8 diagonal points at f8. Whether that matters
   depends entirely on whether a black king ever goes there. With castling known,
   Branch A's value becomes concrete rather than structural.
3. **Was this reached, or composed?** "Not in order of placement" suggests a shape
   being built rather than a game replayed. A composed position tests a hypothesis
   about structure; a played one tests your reading of a real decision. Both are
   useful — the notes should say which.
4. **The missing candidate.** e3 never appeared on the list, and it is arguably
   the move the position most wants. The question worth asking of a case study
   about *how you see*: was the gap that you read b4's coupling correctly but
   assumed a pivot has to move?

---

*Notation is algebraic. Squares are called dark or light rather than black or
white, to keep the colour complex separate from the side to move. Mobility counts
treat the fragment as the whole board — with the rest of the men on, absolute
numbers shift but the unlock/import ratios hold.*
