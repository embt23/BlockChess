# 03 — The corpus is the model

*The result in this paper is the one that changes the architecture. Computed
figures come from `measure/plyrate.py`.*

---

## 1. The idea that looks free

Chess games share prefixes. Millions of games begin 1.e4, hundreds of thousands
follow the same twenty plies of a main line. A database of games is naturally a
**trie**: each node a position reached by one exact move order, each edge a
move, each leaf a game.

So the obvious design: store the trie once, and store each game as *a pointer to
the node where it left the book, plus the moves after that*. The opening — the
most repeated part of the corpus — costs nothing, because it is already there.

And the trie is not overhead. It is the feature. It is the opening book. Asking
"who has played this line, and how did it go" is walking down it. **The data
structure that makes the database searchable and the data structure that
compresses it are the same object.** That is the shape of the whole project and
it is why 0.1 is a better idea than 0.0.

Then you cost it, and the free lunch is not there.

## 2. Why the trie saves nothing you were not already getting

Take a book of 4,096 known opening lines. A game leaves the book at one of them.

**As a pointer:** naming one of 4,096 nodes costs `log2(4096) = 12` bits,
whichever node it is.

**As an entropy code:** opening popularity is heavily skewed. Under a Zipf
distribution with s = 1 the entropy of the choice is 8.8 bits, so an arithmetic
coder spends 8.8 bits on average — 3.2 bits less than the pointer.

| lines | pointer | entropy | pointer wastes |
|---|---|---|---|
| 256 | 8.0 | 6.2 | 1.8 |
| 4,096 | 12.0 | 8.8 | 3.2 |
| 65,536 | 16.0 | 11.1 | 4.9 |

The pointer is worse. That is not a defect in the trie — it is the observation
that **a pointer is the entropy code you get when you decline to use the
popularity information you already hold.** A fixed-width index over `k` items is
exactly the optimal code for a uniform distribution over `k` items, and opening
popularity is about as far from uniform as a distribution gets.

Now the other direction. Suppose instead of a pointer you code the opening ply
by ply, spending `−log2 P(move | position)` bits at each step, with `P` the
empirical frequency in the corpus. By the chain rule the total is

```
−log2 P(first m plies) = Σ over plies of −log2 P(move | prefix)
```

which is *exactly* the entropy-coded cost of naming the node. Same number. The
trie walk and the per-ply predictive coder are the same computation.

> **Prefix sharing and predictive coding are one saving, not two. Whichever you
> implement, you have taken it, and implementing both takes it once.**

The corollary is what matters for the architecture. Scheme E10 in
`02-encodings.md` — "use the corpus as the prior" — is not an improvement on top
of E9. It *is* E9, with the corpus playing the model's role. The right question
is not "should we do both" but "which prior, and what does holding it cost".

## 3. What the self-referential encoding actually costs

Suppose you go ahead: game *N*+1 is coded against the statistics of games 1..*N*.
Three costs, and the third is fatal in its naive form.

**Decode is no longer local.** To read game 5,000,000 you must first know the
corpus as it stood at game 4,999,999, which means replaying the whole chain.
Random access is gone. A reader who wants one game must have everything.

**The corpus is consensus state and it changes every block.** Every node must
agree, bit for bit, on the frequency table after every game, forever. That is an
enormous, constantly mutating, consensus-critical object — the single worst kind
of thing to put in a chain.

**It is fragile in exactly the wrong direction.** If two nodes disagree by one
count anywhere in the table, they decode different games from the same bytes.
Not "one game is corrupt" — the arithmetic decoder desynchronises and every
subsequent game is garbage.

### The checkpoint repair

The standard fix works. Freeze the statistics periodically:

- Every *E* blocks, compute a frequency table from everything up to that point
  and commit its hash in the block header.
- Every game names the checkpoint it was coded against.
- Decoding needs only that checkpoint, not the whole history.

Now decode is local again, the consensus object changes *E*-blocks at a time
instead of continuously, and desynchronisation is contained inside one epoch.

It costs: every full node holds every checkpoint table forever, because old
games are coded against old tables. The tables are the model, and `04` is about
what it costs to keep a model forever. There is no version of this where the
model is not permanent — that is what an immutable log means.

## 4. What the corpus prior is actually worth

Given §2, the honest way to state the benefit is as a comparison of priors,
all measured against the same corpus:

| prior | bits/ply | what must be permanent |
|---|---|---|
| uniform over legal moves (E7) | ~4.9 | the rules |
| static heuristic ranking (E8) | ~4 `[lit]` | the rules + a small table |
| learned policy (E9) | ~1.7 `[lit]` | the rules + a model file |
| empirical corpus frequencies (E10) | ≤1.7, unmeasured | the rules + every checkpoint |

E10's number is unmeasured and there is a specific reason to expect it to
disappoint outside the opening. The corpus is a good prior exactly where games
repeat, and games stop repeating around move 10–15. After the novelty, the
frequency table has seen the current position zero times and has nothing to say;
it falls back to whatever smoothing it uses, and a Krichevsky–Trofimov or
Dirichlet fallback on an unseen context is close to uniform over legal moves —
which is E7, at 4.9 bits/ply.

So the expected shape is: **near-zero cost for the first ~20 plies, roughly E7
cost for the remaining ~60.** Estimate the whole game at

```
20 plies × (small) + 60 plies × 4.9 bits ≈ 37 bytes
```

against E7's 49 and E9's 17. **The corpus prior is probably worse than a
learned model and better than uniform, at a much higher permanence cost than
either.** That is a bad position on the trade-off surface, and it is why the
recommendation in `06-decisions.md` D4 is against it. **Experiment X4.**

The result stands on its own regardless: the trie is the right *index* and the
wrong *compressor*, and the reason is that it was never a separate saving.

## 5. Transpositions: paths against the graph

The trie stores paths, so `1.d4 Nf6 2.c4 e6 3.Nc3` and `1.c4 e6 2.d4 Nf6 3.Nc3`
sit in different subtrees despite being the same position. A study database
should absolutely collapse those. A storage layer must absolutely not.

| | trie of paths | DAG of positions |
|---|---|---|
| keyed by | move sequence | position hash |
| transpositions | duplicated | shared |
| move order | preserved | lost |
| "which games reached this" | one subtree | all in-edges |
| can reconstruct a game | yes | no |

The DAG is genuinely a DAG, which is worth stating because it is not obvious.
Positions can repeat within a game — that is what threefold repetition is — so
the graph of positions has cycles. But include the halfmove clock in the
position key and every cycle is broken: the clock increases on every move and
resets only on a capture or a pawn move, and both of those are irreversible.
So the position key ordered by (men captured, pawns advanced, halfmove clock) is
a topological order, and **the position graph keyed on the full position is
acyclic.**

Both structures are derived and both are rebuildable by replaying the chain,
so neither needs to be consensus state. Build both. The DAG answers "what
happens in this position", the trie answers "how did people get here", and the
questions this project exists to serve need both.

## 6. Summary

| Result | Consequence |
|---|---|
| Prefix sharing ≡ predictive coding | Do not budget for both |
| A pointer wastes 3–5 bits against a skewed distribution | Never index a book with a fixed-width id |
| Self-referential coding breaks random access | Checkpoint, or do not do it |
| Checkpoints make every table permanent | The model cost is unavoidable, only bounded |
| Corpus prior ≈ E7 after the novelty | Expected ~37 bytes/game, worse than E9 |
| Position graph with the halfmove clock is acyclic | Safe to build a DAG index |
| Trie and DAG are both derived | Neither is consensus state; build both |
