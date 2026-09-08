# 00 — The problem, stated precisely

## What 0.1 is for

A public, permanent, append-only record of chess games, which anyone can read,
search, and study. If a line has been played, you can find it. If you want to
know what happens after 1.e4 c5 2.Nf3 d6 3.d4 cxd4 4.Nxd4 Nf6 5.Nc3 a6, you ask
the chain, not a company.

There is no money in this version. There are no stakes, no rake, no wagers, no
economy. Those were 0.0's subject and they are deliberately absent here.

## The question this version exists to answer

> **What is the cheapest permanent encoding of a chess game, and what does each
> saving cost you in something other than bits?**

Not "how small can a chess game get" — that question is answered in the
literature and the answer is about one and a half bits per ply. The question
here has the word *permanent* in it, and permanence is what makes the problem
different from ordinary compression:

- An ordinary compressor may be upgraded. A consensus decoder may not.
- An ordinary compressor may fail on adversarial input. A consensus decoder is
  fed by an adversary by definition.
- An ordinary compressor is judged on ratio. A consensus decoder is judged on
  ratio, on decode cost, on how much of itself must live in consensus forever,
  and on how much it constrains what the chain can ever store.

So every scheme below is scored on four axes, not one.

| Axis | Question |
|---|---|
| **Ratio** | bits per ply |
| **Decode cost** | what must run to turn bits back into moves |
| **Permanence** | what must never change again for old data to stay readable |
| **Reach** | what the scheme forbids you from ever storing |

**Ratio is the least interesting of the four.** That claim is defended in
`04-permanence.md` and it is the main result of this version.

## The three problems that get confused with each other

They have different answers and different optimal solutions, and most writing
on this subject silently switches between them.

### P1 — Encode one move

Given a position, name the move that was played. This is bounded below by the
entropy of the move actually chosen given the position, and bounded above by
`log2(number of legal moves)` if you refuse to model the player.

### P2 — Encode one position

Given nothing, name a position. This is bounded below by `log2(number of legal
positions)`, which `01-position-space.md` measures.

### P3 — Encode one game given every game already stored

Given a corpus, name a game. This is the one the database actually faces, and
it is bounded below by the conditional entropy of a game given the corpus.
`03-corpus.md` shows that P3 and a good solution to P1 are the *same problem*,
which is not obvious and which kills a design that otherwise looks clever.

### P4 — encode the *structure*

Having stacked the corpus, find the patterns in the pile, name them, and encode
against the names. Then do it again on what that leaves. This is a different
kind of question from P1–P3: there the units are handed to you by the rules,
here they are discovered from the data. `08-layers.md`, and it is where the
project's purpose lives rather than its storage budget.

**P1 beats P2 by a factor of about thirty.** A move costs about five bits; a
position costs about a hundred and fifty. Any design that stores positions
where it could store moves has already lost more than every other decision on
this page can win back. `01-position-space.md` derives that number, and the
first job of the position count is to *justify not using it*.

## What a game record has to contain

```
GameRecord {
  rules       which rule set was in force        ← see 04, this is load-bearing
  players     two identities
  result      1-0, 0-1, 1/2-1/2, or unfinished
  moves       the move stream                    ← the only compressible field
  attestation evidence that this game happened
}
```

Only `moves` is chess. Everything else is envelope, and the envelope does not
compress. `04-permanence.md` measures the ratio between them and it is the
reason the compression work has a natural stopping point.

## Ground rules for this version

Taken from what 0.0 got right, and they are the only thing carried over:

1. **No number without an oracle.** Every figure quoted in these papers is
   either computed by a script in `measure/`, an arithmetic identity, or an
   explicitly attributed literature value. Values in the third category are
   marked `[lit]` at the point of use and listed in `07-sources.md`.
2. **Unmeasured is said out loud.** Where a number needs a corpus we do not yet
   have, the paper gives the formula and names the experiment, rather than
   quoting a plausible figure. Open experiments are in `measure/EXPERIMENTS.md`.
3. **A design decision is a decision record.** `06-decisions.md`, with `▶`
   marking the recommendation and the reasoning that produced it.

## Reading order

| File | What it settles |
|---|---|
| `01-position-space.md` | How large the space is, what group acts on it, why symmetry is nearly worthless |
| `02-encodings.md` | Fifteen ways to encode a move stream, scored on all four axes |
| `03-corpus.md` | Why the opening book and the compressor are one object |
| `04-permanence.md` | What immutability costs in bits, and where compression should stop |
| `05-index.md` | Making it searchable: transpositions, mirrors, and proofs about the corpus |
| `06-decisions.md` | The forks, with recommendations |
| `07-sources.md` | Every literature value, attributed |
| `08-layers.md` | The layers above a game, and why the compression *is* the discovery |
