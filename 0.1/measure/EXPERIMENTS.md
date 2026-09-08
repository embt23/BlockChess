# Open experiments

Numbers the papers need and do not have. Each one names what it decides, so
that none of them is done for its own sake.

The rule from 0.0 applies: **a reference you wrote five minutes ago is not an
oracle.** Any move generator built for these experiments must pass the published
perft counts before a single figure derived from it is quoted.

---

## X1 — The branching histogram — *decides `E[log2 b]`, and so D3 exactly*

**Question.** What is `E[log2 b]` over real games, where `b` is the number of
legal moves in the position to move?

**Why it matters.** `02-encodings.md` uses `b ≈ 30` as a placeholder, giving
4.907 bits/ply and ~49 bytes/game. The true figure is `E[log2 b]`, which is
strictly below `log2 E[b]` by Jensen, so 4.907 is an *upper* bound on the E7
cost and the real number is better. How much better is unknown, and every
storage table in `04-permanence.md` moves with it.

**Status: the tool is built; it needs a corpus.**

```sh
cd 0.1 && cargo build --release
./target/release/blockchess measure <file.pgn> --csv per-ply.csv
```

Steps 1 and 3 below are done — `crates/bc-chess` passes perft and
`blockchess measure` reports every statistic listed. What is missing is step 2,
which is a download.

**Protocol.**
1. ~~A move generator that passes perft.~~ Done: `blockchess perft 6` prints
   the published count and MATCH. No figure may be quoted if it does not.
2. **A corpus.** Any large public archive; state which, and its size, with the
   result. Record the selection — a database of master games has a different
   branching profile from a database of blitz. *This is the outstanding step.*
3. ~~For every ply of every game, record `b`.~~ Done: the command reports the
   histogram, `E[b]`, `E[log2 b]`, `E[⌈log2 b⌉]`, and the split by ply bucket.
   `--csv` writes the per-ply rows for plotting.
4. Confirm the 218-move maximum: `blockchess show` on the known witness
   position counts its legal moves. Free, and it removes a `[lit]` value.

**Sanity figure.** On the four games in `corpus/classics.pgn` the command
reports `E[log2 b] = 4.686`, `log2 E[b] = 4.974`, max `b = 52`. Four games is
not a corpus; this is here to show the pipeline works, not as a result.

**Output.** Replaces the `b = 30` row in `02-encodings.md` and every derived
byte figure in `04-permanence.md`.

---

## X2 — The rank distribution — *decides whether E8 is worth its table*

**Question.** Order the legal moves by a stated static heuristic. What is the
distribution of the played move's rank, and what is its entropy?

**Why it matters.** E8 is quoted at ~4 bits/ply from the literature. If our own
heuristic reaches that, E8 becomes a live alternative to E7 at ~9 bytes/game
cheaper; if it does not, E7 stands unchallenged and D3 is closed.

**Protocol.**
1. Fix a heuristic and *write it down first*, in the form it would appear in a
   specification: iteration order over piece types, captures ordered by
   victim value then attacker value, then checks, then castling, then quiet
   moves by piece-square table. Integer arithmetic only.
2. Over the X1 corpus, record the rank of the played move.
3. Report the rank histogram, its entropy, and the Huffman code built from it.
4. Report the entropy split by ply bucket. A heuristic that is excellent in the
   opening and useless in the endgame is a different proposition from one that
   is uniformly mediocre.

**Output.** Our own E8 figure, and the actual table if E8 survives.

---

## X3 — E8 against E9 — *decides whether D4 can be reopened*

**Question.** On the same corpus, what is the real gap between a
specification-sized heuristic and a learned policy prior?

**Why it matters.** `04-permanence.md` prices permanence as the difference
between two literature values, ~4 and ~1.7 bits/ply. That is the weakest link in
the papers and it is load-bearing for D4. If the true gap is much larger, D4
deserves to be argued again with real numbers instead of dismissed.

**Protocol.** Run a policy network over the X1 corpus, arithmetic-code the
played move against its output distribution, and report bits/ply against X2's
figure on identical games. Report the model's size on disk alongside, because
that is the other half of the trade.

**Output.** Either confirmation of D4 or a reason to reopen it. Note in advance:
even a gap of 3× only changes the record by 13% at per-game attestation, and by
49% once D5 batching lands. Decide which of those two the number is being
compared against before running it.

---

## X4 — The corpus prior — *decides D4's E10 branch*

**Question.** Coding each game against the empirical frequencies of the games
before it, what is the actual bits/ply, split by ply?

**Prediction on file, so that it can be wrong.** `03-corpus.md` §4 predicts
near-zero cost for roughly the first 20 plies and roughly E7 cost thereafter,
giving ~37 bytes/game — worse than E9's ~17 at a much higher permanence cost.

**Protocol.** Build the frequency trie incrementally over the corpus in
chronological order. For each ply, record `−log2 P(move | prefix)` under a
Krichevsky–Trofimov or Dirichlet fallback for unseen contexts. Report the
per-ply curve. The interesting number is the ply at which the curve reaches E7,
which is a measurement of when games stop repeating.

**Output.** Confirms or refutes the prediction. Also produces the first real
picture of where novelty begins in the corpus, which is a result the study
database wants for its own sake.

---

## X5 — Index scale — *decides the systems work in a later version*

**Question.** How many distinct position keys are there in a corpus of *N*
games, and how does that grow with *N*?

**Why it matters.** `05-index.md` estimates 4.8e11 postings at six billion games
and proposes ply sampling and popularity thresholding to control it. Both knobs
need the actual distribution: specifically, what fraction of distinct positions
are reached by exactly one game.

**Protocol.** Over the X1 corpus, count distinct clock-free position keys as a
function of games ingested. Report the curve, the singleton fraction, and the
same figures after symmetry canonicalisation — which `05-index.md` §4 predicts
will cut distinct keys by about 4× in the pawn regime.

**Output.** Sizes the index, and turns the symmetry claim in `01-position-space.md`
§6 from a family-by-family measurement into a corpus-wide one.

---

## X6 — Is our output already incompressible? — *bounds how much room L3 has*

**Question.** Take a packed `.bcg` file and try to compress it again with a
general-purpose tool. Anything that shrinks is structure our encoding left
behind.

**Why it matters.** `papers/08-layers.md` §4: an optimal compressor's output is
indistinguishable from random, because any remaining pattern is redundancy it
failed to spend. So re-compressibility is a direct, free measure of how much
room the layered idea has above what we already do. This is the cheapest
experiment here and it bounds the value of the most expensive one (X7).

**Protocol.** Pack a real corpus under E3 and E7. Run gzip -9, xz -9, and if
available a general-purpose neural compressor over each. Report bytes before
and after. Do the same to the source PGN as a control — it should shrink a lot.

**Already run, on four games** (`corpus/classics.pgn`, too small to conclude
from — gzip's header is a real fraction of these files):

| file | bytes | gzip -9 | |
|---|---|---|---|
| PGN | 1,237 | 718 | shrinks 1.72× |
| `.bcg` E3 | 304 | 321 | grows |
| `.bcg` E7 | 145 | 167 | grows |

The direction is right. Note that gzip only looks for repeated byte strings, so
passing this rules out one family of leftover structure, not all of them.

---

## X7 — Grammar induction — *the test of the project's central claim*

**Question.** Run Re-Pair or SEQUITUR over the corpus's move streams. What does
it discover, and what does it cost?

**Why it matters.** `papers/08-layers.md` §3 claims that searching for the
shortest encoding of the corpus is the same search as searching for chess
theory. This is the experiment that tests it.

**Protocol.** Encode games as symbol sequences, induce a grammar, report:
bits/ply against E7 and E9; the number of symbols discovered; the length
distribution of the symbols.

Then the part that is not a compression result, and is the actual point:
**how many of the top 100 discovered symbols correspond to named openings?**
Match against ECO codes. A high number is the most interesting single fact this
project could produce — a compressor rediscovering opening theory from nothing
but repetition.

**Prediction on file** (`08-layers.md` §6): on bits alone this lands near the
model-prior figure, ~1.5–2 bits/ply, and does **not** beat it. That is fine.
The output is a list of named ideas, which a policy network does not give you,
and the list is the deliverable.

---

## X8 — Do the layers keep paying? — *is L4 real*

**Question.** Run the grammar procedure again on its own output, and again.
Bits saved per layer.

**Why it matters.** The layered claim is that each level of abstraction hands
the next one new tools. If the saving collapses after two rounds, L3 is a
one-off trick and should be described as one. If it keeps finding structure,
there is a hierarchy in chess that nobody has named, and finding out what the
symbols at level three and four *mean* is a research programme rather than an
engineering task.

**No prediction on file.** Nobody knows, which is the reason to run it.
