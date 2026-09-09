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

---

## X11 — Do bits track seconds? — *the cheapest way to find out if any of this is real*

**Question.** Our encoder prices each move in bits. The player spent some number
of seconds on it. Are the two correlated?

**Why it matters.** `papers/10-players.md` §1. If a statistical model built from
strangers and one person's hesitation get confused in the same places, that is a
real result about both. If they do not correlate at all, a large part of why
this project believes compression has anything to do with understanding is
wrong — and that is worth knowing in a week rather than a year.

**Data.** Lichess PGN carries `[%clk H:MM:SS]` in a comment after each move.
Per-move think time is the difference between consecutive clock readings for the
same player, plus the increment. `bc-pgn` currently discards comments, so step
one is to keep the clock tags.

**Status: built.** `blockchess think <file.pgn> --corpus <big.pgn>`.

```sh
./target/release/blockchess think corpus/lichess.pgn --corpus corpus/lichess.pgn
```

**Protocol.**
1. ~~Extend the PGN reader to retain `%clk`.~~ Done, with the increment term —
   `think = clock_before − clock_after + increment` — which is the trap that
   would otherwise bias every measurement in this experiment by the increment.
2. ~~Record `(bits, seconds)` per ply.~~ Done, under both notions of surprise:
   branchiness, and `−log2 P(move | position)` from the corpus. Only the second
   is the hypothesis.
3. ~~Report the correlation split by phase and time control.~~ Done. Rating
   band is not split on yet.

**Instrument validated before use** (`fixtures/README.md`): against a corpus
with random think times it reports **+0.009**, and against one where think
time is long exactly where players leave the main line it reports **+0.617**.
It sees the effect when the effect is there and reports nothing when it is
not, so a null on real data would be a null rather than a broken tool.

**Controls, because the confounds here are severe and each would manufacture a
correlation on its own:**
- **Time pressure.** Under thirty seconds left, everything is fast regardless
  of recognition. Exclude, or model separately.
- **Memorised theory.** A move can be instant because the player *learned* it,
  not because they understood it. Opening moves are cheap in bits *and* fast in
  seconds for a reason that is not the hypothesis. Test the middlegame
  separately — that is where the claim lives.
- **Fast means bad.** Blunders are quick. Some correlation will exist through
  move quality alone; an engine evaluation as a covariate would separate them.
- **Increment.** Bullet and classical are different regimes. Do not pool them.

**Prediction on file:** positive correlation in the middlegame, strongest at
intermediate ratings — beginners have no chunks to recognise, and the very
strongest have so many that little surprises them.

---

## X12 — A compressor per player — *style, with a number*

**Question.** Build the grammar from one player's games. Does the gap between
their model and the general model identify their moves?

**Protocol.** Per-player grammars for players with enough games (PGN Mentor
publishes complete single-player collections; Lichess has per-account
archives). Price held-out moves under both models. Report bits saved by the
personal model, and rank moves by the gap.

**The test that makes it a result rather than a description:** hold out games,
and see whether the personal model identifies *who played them* better than
chance. If style is real and this captures it, that should work.

---

## X13 — The modes of the player graph — *what the dimensions turn out to be*

**Question.** Encode player A's games with player B's model; the extra bits are
a distance. Over every pair, what is the shape?

**Protocol.** Build the pairwise cross-entropy matrix, symmetrise it, and take
the eigendecomposition of the graph Laplacian. Report the spectrum, and the
players at each extreme of the lowest few non-trivial eigenvectors.

**Why the low modes specifically.** `papers/10-players.md` §3: for a connected
body the Laplacian's eigenvectors *are* its modes of vibration, low frequency
first, and the lowest move the whole body together. Those are the axes along
which every player varies. High modes are individual quirks that connect to
nobody.

**The point is that nothing is named in advance.** Compute the modes, look at
who sits at either end, and find out afterwards what the dimension was. If mode
one separates attacking players from positional ones, that is a century-old
distinction falling out of arithmetic. If it separates something nobody has a
word for, better.

Needs X12 first.
