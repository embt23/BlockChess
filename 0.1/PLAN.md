# The plan

*Say **"launch N"** and I will do stage N. Stages are ordered so each one is
useful on its own and none of them needs the next one to justify it.*

---

## Where this is

| | | |
|---|---|---|
| **Papers** | ten documents, every number computed or attributed | done |
| **Engine** | `bc-chess`, passes `perft(6)` against the published count | done |
| **Reader** | `bc-pgn`, hardened against files we did not write | done |
| **Codec** | `bc-codec`, E3 and E7, at the information-theoretic floor | done |
| **Index** | `bc-index`, opening trie + canonical position map | done |
| **Grammar** | `bc-grammar`, Re-Pair, knows nothing about chess | done |
| **Ground truth** | 3,810 named openings, vendored, CC0 | done |
| **Corpus** | 121,332 lichess games | **got it** |
| **X1, X7** | run, and written up in `papers/11-results.md` | **done** |
| **The chain** | nothing | not started, deliberately |

40 tests. `blockchess` has ten commands. **Stages 0, 1 (partly) and 3 are
done** — see `papers/11-results.md`: one prediction confirmed, one wrong, one
refined.

---

## Stage 0 — the corpus  ·  *yours, two minutes, and it unblocks 1 through 5*

Everything downstream needs real games, and I cannot fetch them: this session's
egress policy blocks `database.lichess.org`, `pgnmentor.com` and
`theweekinchess.com` outright (HTTP 403, organisation policy — I did not route
around it). Your machine has no such restriction.

```sh
cd ~/Desktop/claude/BlockChess/0.1
mkdir -p corpus
curl -L -o corpus/lichess.pgn.zst \
  https://database.lichess.org/standard/lichess_db_standard_rated_2013-01.pgn.zst
zstd -d corpus/lichess.pgn.zst          # or: unzstd corpus/lichess.pgn.zst
```

That month is the smallest they publish — around 120,000 games, a few hundred
MB decompressed. Plenty. If `zstd` is missing: `sudo pacman -S zstd`.

A master-games collection would be a better corpus for the theory question and
a worse one for the branching-factor question, because strong players and club
players make different numbers of legal moves available. **Getting both and
comparing is a result in itself** — it would say whether "chess theory" as the
compressor finds it is the same object at 1500 and at 2600.

Check it landed:

```sh
./target/release/blockchess measure corpus/lichess.pgn --limit 5000
```

---

## Stage 1 — turn the papers' placeholders into measurements  ·  *say "launch 1"*

The papers currently rest on one guessed number and two borrowed ones. This
stage removes all three.

- **X1** — `E[log2 b]` over real games, by phase. Replaces the `b ≈ 30`
  placeholder in `02-encodings.md` and every byte figure downstream of it in
  `04-permanence.md`.
- **X6** — pack the corpus, then try gzip and xz on the output. `08-layers.md`
  §4 predicts it will not shrink. On four games it already grows; at scale the
  result means something, and it *bounds how much room the layered idea has*.
- **X5** — index scale: distinct positions against games ingested, the
  singleton fraction, and how much the symmetry canonicalisation collapses.
  `01-position-space.md` §6 predicts about 4× in the pawn regime; this measures
  it on a corpus instead of on families.

**Deliverable:** every `[lit]` and every placeholder in `papers/` either
replaced by our own figure or explicitly retained with a reason. A short
`papers/11-results.md` with the numbers and the corpus they came from.

---

## Stage 2 — the layers, as something you can watch  ·  *say "launch 2"*

Your image: the story of the ground-up layers of abstraction. A published page
that walks it, using the real corpus rather than illustrations:

```
a square        →  the board's geometry, and why it has almost no symmetry
a move          →  one of ~30, so ~4.9 bits, and here is the list
a game          →  80 of those, one integer, ~49 bytes
the corpus      →  games share prefixes; the trie is the opening book
the grammar     →  the compressor invents symbols; here is what it named
theory          →  and here is which of those names humans already had
```

Each panel driven by numbers this repo produced. The last two panels are the
argument of `08-layers.md` made watchable.

**Deliverable:** an Artifact with its own URL, that you can send to someone.

---

## Stage 3 — X7  ·  **DONE 2026-09-09** — `papers/11-results.md`

Compression half false, discovery half true. Re-Pair costs 10.17 bits/ply
against E7's 4.625, so it is a bad compressor — but past four plies the
sequences it names are named by humans 3–6× more often than the ones it did
not name, against a control of real game prefixes. The control mattered: below
four plies the apparent agreement was *worse* than chance and would have been
reported as a success without it.

### 3b — **DONE 2026-09-09**

Mid-game symbols are readable now. Each is located in the corpus, rendered from
the position it actually starts in, with real scoresheet numbering (`2... Nc6
3. Bb5 a6`) and a board. A `PATTERNS NOBODY NAMED` section prints the top
middlegame motifs.

It also fixed a stated result. Symbols that begin mid-game were being scored
against a table of **openings**, which is the wrong answer key and inflated
"inside a named line". The output now marks which symbols an opening table can
judge and which it cannot.

**Left open:** whether any middlegame pattern is a real idea or an artefact of
how club players trade pieces. The machine can only say they repeat.

<details><summary>original stage 3 brief</summary>

Run grammar induction on the real corpus and cross-check every discovered
symbol against the 3,810 named openings.

The headline number: **how many of the top discovered symbols are things
humans already named.** The machinery is built and tested; this is pressing go
and then reading the output carefully.

Two predictions are already on file so the result can contradict them:

- `08-layers.md` §6 — on bits alone this lands near a learned model, ~1.5–2
  bits/ply, and does **not** beat it.
- `09-lineage.md` §5 — agreement will be high on long forced lines and poor on
  flexible schemas, because Re-Pair's symbols are rigid and a master's units
  have slots (Gobet & Simon's template theory).

**Deliverable:** the result, written up honestly including the ways the number
could be flattering itself — which the command already reports separately.

</details>

---

## Stage 4 — E8  ·  **BUILT 2026-09-09, needs your corpus to judge**

Implemented in `bc-codec/src/rank.rs`, specification written in prose first,
eight tests written from that prose rather than from the code. `blockchess
measure` now reports it.

On the four games in the repo it beats E7 by 0.4 bits/ply and ranks the played
move first 25.9% of the time — but 112 plies is far too few, entropy fitted to
a small sample is biased downward, and **that bias looks exactly like a win**.
The command refuses to be quoted below 100,000 plies.

```sh
./target/release/blockchess measure corpus/lichess.pgn
```

That decides D3. If E8 wins on 8 million plies, the encoding decision should be
re-argued — E8 costs only a heuristic that fits on one page and still passes
the reconstruct-from-prose test that rules a neural model out.

<details><summary>original stage 4 brief</summary>

Rank the legal moves by a static heuristic and code the rank instead of the
index. `02-encodings.md` puts this at ~4 bits/ply against E7's ~4.9, and it is
the only remaining scheme that could win **without** putting a neural network
into consensus forever.

Includes writing the heuristic down first, in specification form, before
measuring it — so the comparison is honest and the thing is reimplementable
from prose, which is `04-permanence.md` §2's criterion for anything permanent.

**Deliverable:** E8 implemented, measured against E7 on the same corpus, and
D3/D4 either confirmed or reopened with our own numbers instead of borrowed
ones.

</details>

---

## Stage 5 — chunks  ·  **BUILT 2026-09-09, needs your corpus**

Built as the **spatial** version rather than a grammar over position
sequences, because that is what `09-lineage.md` §5 says chunking theory is
actually about. A position is a set of facts — "white king on g1" — and a
chunk is a set of facts occurring together far more often than independence
predicts.

```sh
./target/release/blockchess chunks corpus/lichess.pgn --from-ply 20
```

Validated before use, same discipline as X11: it recovers a planted castled
kingside from a background of noise, finds nothing above 4× lift in
independently placed pieces, and respects the support floor. On the
degenerate 200-game fixture it already returns `Rf1 Kg1` — Chase & Simon's own
example — from anonymous facts.

**Read lift carefully on small samples.** The independence baseline collapses
and the ratio explodes; a six-figure lift means the sample is degenerate, not
that the finding is strong. The command says so below 5,000 positions.

This is also what your drawn shapes compare against.

<details><summary>original stage 5 brief</summary>

`09-lineage.md` §5: chunking theory is about *positions*, not move sequences.
Chase & Simon's masters remember configurations. What we currently induce over
is a move stream, which is the easier thing and the wrong object.

Induce over canonical position keys instead. `bc-index` already computes them.
This is the version that could actually be compared against the psychology, and
it is where "does the compressor learn to see like a player" stops being a
figure of speech.

**Deliverable:** X9. Slower, harder, and the most interesting thing on this
page.

</details>

---

## Stage 6 — what is worth arguing about  ·  *say "launch 6"*

Schmidhuber's compression-progress idea (`09-lineage.md` §3), applied: induce a
grammar on games up to time *t*, then *t+1*, and diff. Lines whose encoding is
still changing are where theory is unsettled. Lines whose encoding has
stabilised are settled.

This gives `05-index.md` the thing it wanted and could not get — a way to
surface what deserves discourse **without an editor** — and it needs the
timestamps a real corpus carries.

**Deliverable:** X10, and a "live frontier" view of the opening tree.

---

## Stage 6b — players  ·  *say "launch players"*

The thread from 2026-09-09, written up in `papers/10-players.md`. Pure data —
moves and clock times, no interviews. Three experiments, cheapest first.

- **X11 — do bits track seconds?** Lichess records the clock at every move.
  Think time is where a player's own compression failed, so this asks whether
  our encoder's surprise and a human's hesitation land in the same places. One
  week, and the most informative thing on this page if it comes back negative.
- **X12 — a compressor per player.** Build the grammar from one person's games
  and it encodes them rather than chess. Style is the gap between their model
  and everyone's. Testable: can it identify who played a held-out game?
- **X13 — the modes.** Cross-entropy between every pair of players is a
  distance matrix; the low eigenvectors of its Laplacian are, literally, that
  body's lowest modes of vibration — the axes along which everybody varies.
  The temperaments fall out instead of being named in advance.

**Deliverable:** X11 first, because it is a join between two columns the corpus
already has, and because it is the one that could kill the framing cheaply.

---

## Stage 7 — the chain  ·  *say "launch 7"*

Only now, and this ordering is the point: stages 1–6 give a complete, useful,
studiable chess database with no blockchain in it at all.

- Blocks, and games batched into them — `06-decisions.md` **D5**, which
  `04-permanence.md` measured as the largest single saving available anywhere
  in the project (12× on the dominant term, against 13% for every move-encoding
  decision combined).
- The `RuleSetId` registry — **D9**. Already forced by the encoding, and it is
  what makes variants first-class rather than an afterthought.
- Identity carried on every game — **D7/D8**, your "assume trust" call, with
  the field present from the first block because it cannot be back-filled.

**Deliverable:** two nodes agreeing on a block of games.

---

## Stage 8 — something to look at  ·  *say "launch 8"*

`06-decisions.md` **D12**: compile the core crates to WebAssembly and put an
ordinary web page on top. A database whose argument is "anyone can read it"
should not require installing a binary, and compiling the *same* codec that
wrote the file is what stops the browser and the command line ever disagreeing
about what a game says.

The crates were kept dependency-free and I/O-free for exactly this. Nothing
about it gets harder by waiting.

---

## What I would do first, if you asked

**Stage 0, then 3.** Stage 1 is more rigorous and stage 3 is more likely to
tell you something nobody knows. The grammar result is the one that would
justify the whole framing — or kill it, which is nearly as valuable and much
faster than finding out in a year.

Stage 2 is the one to do if you want to show somebody what this is.

---

## Things I will not do without you saying so

- Push anything to `main`. Everything lives on
  `claude/youthful-cori-l69v75`.
- Open a pull request.
- Start stage 7. The chain is downstream of D7, and D7 is deferred rather than
  solved.
