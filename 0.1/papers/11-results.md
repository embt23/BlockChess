# 11 — Results

*First run against a real corpus, 2026-09-09. Every figure below was produced
by `blockchess` on Evan's machine and is reproducible with the commands given.
Predictions made before the data arrived are marked **CONFIRMED**, **WRONG**,
or **REFINED**, and the wrong ones are left in.*

---

## The corpus

| | |
|---|---|
| source | `lichess_db_standard_rated_2013-01.pgn` (Lichess open database, CC0) |
| games | 121,332 |
| plies | 8,155,187 |
| mean game length | **65.2 plies** |
| results (W/B/D) | 2577 / 2271 / 152 in the 5,000-game sample |

Club-level online blitz, not master play. That matters and is revisited in §5.

---

## 1. X1 — the number every storage figure was guessing at

```sh
blockchess measure corpus/lichess.pgn --limit 5000
```

| | value |
|---|---|
| `E[b]` | 29.2102 |
| `log2 E[b]` | 4.8684 |
| **`E[log2 b]`** | **4.6246** |
| Jensen gap | 0.2438 |
| max `b` observed | 88 |

`02-encodings.md` used **`b ≈ 30`** as a placeholder, giving 4.907 bits/ply.
The measured figure is **4.625** — the placeholder was 6% pessimistic, and the
direction was predicted: Jensen guarantees `E[log2 b] < log2 E[b]`, and the gap
is 0.24 bits.

### Cost per ply, measured

| scheme | bits/ply | bytes/game |
|---|---|---|
| E1 PGN/SAN text | 40.000 | 325.9 |
| E3 16-bit move | 16.000 | 130.4 |
| E6 index, whole bits | 5.105 | 41.6 |
| **E7 index, mixed radix** | **4.625** | **37.7** |

**E7 beats E3 by 3.46×.** And the rounding E6 throws away is **9.42%** — which
retroactively justifies the mixed-radix construction in `bc-codec`. Had that
number come in at 1%, the arithmetic would not have been worth writing.

### The phase curve — **prediction REFINED**

`03-corpus.md` predicted the per-ply cost would "rise then flatten". It rises
then **falls, steadily and a long way**:

| plies | `E[log2 b]` |
|---|---|
| 0–9 | 4.772 |
| 10–19 | **4.994** ← peak |
| 20–29 | 4.984 |
| 40–49 | 4.776 |
| 60–69 | 4.276 |
| 80–89 | 3.787 |
| 120–129 | 3.309 |
| 160–169 | 3.114 |

Obvious in hindsight and not predicted: pieces come off, so endgames offer
fewer legal moves. The middlegame peak at ply 10–19 is where the board is
fullest and most open at once.

**Consequence:** the cost of a game is not `plies × 4.625`. Long games are
cheaper per ply, so the byte figures above are averages over a distribution,
not a rate. Nothing downstream breaks, but the paper's wording was wrong.

---

## 2. The envelope — **CONFIRMED, and worse than stated**

`04-permanence.md` §3 claimed that below ~50 bytes of chess you are storing
signatures, not chess. With the real mean game length of 65.2 plies:

| | bytes | chess |
|---|---|---|
| two signatures per game | 237.7 | **15.9%** |
| batched by 100 | 54.7 | **68.9%** |

The paper estimated 19.7% chess at per-game attestation. Measured: **15.9%**.
Games are shorter than the assumed 80 plies, so the fixed 200-byte envelope
dominates even harder than argued.

**The batching decision (D5) is the largest single win available and this
confirms it at 4.3×.**

---

## 3. X7 — does a compressor rediscover chess theory?

```sh
blockchess grammar corpus/lichess.pgn --show 40
```

1,117 symbols invented from 8.1M plies in 1.82 seconds. Nothing about chess was
given to the inducer; it saw anonymous integers. Opening names were looked up
afterwards, never as input.

### 3a. As compression — **prediction WRONG**

`08-layers.md` §6 predicted Re-Pair would land "near the model-prior figure,
~1.5–2 bits/ply, and not beat it."

| | |
|---|---|
| alphabet | 1,962 → 3,079 symbols |
| apparent reduction (symbol count) | 1.139× |
| **net reduction in bits** | **1.075×** |
| **bits/ply after grammar** | **10.17** |

Against E7's 4.625 on the same corpus. **Re-Pair is more than twice as bad as
the codec we already had.** The prediction was right in direction and badly
wrong in magnitude — it is not near a learned model, it is not near anything.

Two things this exposed, both now fixed in the tool:

- The first run reported **1.14×**, which was a *symbol count*. Induction grows
  the alphabet, so surviving symbols cost more bits each. In bits it is 1.075×.
  Reporting symbols instead of bits flattered the result by 6 points.
- 10.17 bits/ply is *worse than storing raw move ids*, because the grammar's
  alphabet is wider and the savings do not pay for the width.

**This kills grammar induction as a storage scheme, permanently.** It was never
proposed as one — `08-layers.md` §6 said so in advance — but now it is measured
rather than predicted, and D3 is not reopened.

### 3b. As discovery — **the result**

Here the answer is yes, and the control is what makes it worth saying.

| plies | symbols | named | control | **lift** |
|---|---|---|---|---|
| 2 | 696 | 41% | 99% | **0.41×** |
| 3 | 263 | 59% | 86% | **0.68×** |
| 4 | 92 | 68% | 46% | **1.50×** |
| 5 | 29 | 79% | 26% | **3.05×** |
| 6 | 24 | 62% | 20% | **3.12×** |
| 7 | 10 | 70% | 12% | **5.83×** |
| 8–10 | 1 each | 100% | 2–4% | *(n = 1, ignore)* |

The control is the same number of **real game prefixes** of the same length,
taken straight from the corpus and scored identically. It answers the only
question that matters: is the compressor picking sequences that are named *more
often than the openings people actually play*?

**There is a crossover at four plies.**

**Below it, the agreement was an illusion — worse than chance.** At two plies,
99% of real openings are inside some named line, because ECO covers essentially
every two-move start. The compressor's two-ply symbols score 41%, *worse*, because
many of them are mid-game fragments that are not opening prefixes at all. The
raw "49.2% occur inside a named line" headline is therefore meaningless on its
own, and without the control it would have been reported as a success.

**Above it, the signal is strong and grows with length.** At five plies, only
26% of the openings people actually play are named — but 79% of the
compressor's five-ply picks are. **Three times better than the thing it is
drawn from.** At seven plies, 5.8×.

That is the shape a real effect makes. Coincidence gets *weaker* with length,
because longer sequences have fewer chances to match. This gets stronger.

### 3c. **Prediction CONFIRMED**

`09-lineage.md` §5 predicted, from Gobet & Simon's template theory, that
agreement would be "high on long forced lines and poor on flexible schemas,
because Re-Pair's symbols are rigid and a master's units have slots."

The lift column is exactly that, and quantified: rigid long forced lines are
where it wins, and the win grows monotonically with how forced the line is.
What it finds at 5–7 plies is the Scotch Game, the Sicilian Open, the Giuoco
Piano, the Ruy Lopez Steinitz — all sequences where each move is close to
forced given the last.

### What it named, unprompted

Italian Game · Ruy Lopez · Ruy Lopez Steinitz Defense · Scotch Game · Sicilian
Defense Open · Sicilian Old · French Defense · Philidor Defense · Queen's
Gambit · Queen's Gambit Accepted · Queen's Gambit Declined · Scandinavian
Defense · Bishop's Opening · Giuoco Piano.

### The vocabulary is stable

20,000 games and 121,332 games give almost identical figures — 10.3% vs 10.7%
exact, 48.7% vs 49.2% contained. **Six times the data changes the answer by
half a point.** Whatever it is finding, it converges fast, which is itself
evidence that it is finding structure rather than noise.

---

## 4. What is still wrong with this

Recorded because the result is only worth what its caveats allow.

**Mid-game symbols are unreadable.** Symbols shown as `…+2` are patterns found
away from the opening, so they are not legal from the starting position and the
display cannot render them. They pollute the 2-ply row and their ECO matches are
probably coincidence. They are also, potentially, the most interesting thing in
the output — repeated *middlegame* motifs nobody has a name for — and right now
they cannot be inspected at all. **Fix this next.**

**The threshold is doing a lot of work.** `min occurrences` scaled to 243, so
only patterns in ≥0.2% of games got named. That the deepest symbols found (8,
9, 10 plies) are all named suggests deeper theory is sitting just under the cut.
Lowering it is one flag.

**One corpus, one month, one population.** Club blitz from January 2013. Master
games would have a different branching factor and, plausibly, deeper agreement,
because strong players stay in theory longer. Running both and comparing is a
result in itself and is not done.

**`contains_run` is O(entries × length) and matches anywhere.** For short runs
that is close to meaningless, which is what the control demonstrated.

---

## 5. Where this leaves the argument

`08-layers.md` claimed that searching for the shortest encoding of the corpus
is the same search as searching for chess theory. After one real run:

> **The compression half is false and the discovery half is true.**

Re-Pair compresses chess badly — worse than the codec already in the repo. But
the sequences it chooses to name, once they are longer than four plies, are
named by humans three to six times more often than the sequences it did not
choose. It is not a good compressor and it is a good *finder*, and those turn
out to be separable in a way the paper did not anticipate.

That is a narrower claim than the one the paper opened with, and it survives
its own control, which the wide version did not.

---

## Reproduce

```sh
blockchess measure corpus/lichess.pgn --limit 5000
blockchess grammar corpus/lichess.pgn --show 40
```

Corpus: `lichess_db_standard_rated_2013-01.pgn.zst` from
<https://database.lichess.org/>, 16.93 MB compressed, 92,811,021 bytes
decompressed.
