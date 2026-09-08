# BlockChess 0.1

A public, permanent, append-only record of chess games. Anyone can read it,
anyone can search it, nobody owns it. If a line has been played, you can find
it.

No money. No stakes, no wagers, no rake, no economy. Those were 0.0's subject
and their absence here is the point, not an omission.

## Status

**Papers, plus a working engine and codec you can run.** No chain yet, and
that is the plan rather than a gap: `papers/06-decisions.md` D10 sets the order
as measurement, then the codec, then the chain, and stages 1 and 2 are built.

```sh
cd 0.1 && cargo build --release
./target/release/blockchess perft 5                    # the rules are right
./target/release/blockchess play                       # a board, in the terminal
./target/release/blockchess measure corpus/classics.pgn  # the real numbers
./target/release/blockchess pack corpus/classics.pgn out.bcg
```

**New here, or lost? Read [`START-HERE.md`](START-HERE.md).** It walks through
each command, says what each crate is for, and names the one thing that is easy
to get catastrophically wrong.

| crate | what it is |
|---|---|
| `bc-chess` | the rules — lifted from 0.0, passes `perft(6) = 119,060,324` |
| `bc-pgn` | reading real games, so a corpus can get in |
| `bc-codec` | E3 and E7: games to bytes and back, losslessly |
| `bc-cli` | the `blockchess` command |

## The question

> **What is the cheapest permanent encoding of a chess game, and what does each
> saving cost you in something other than bits?**

Not "how small can a chess game get" — that is answered in the literature and
the answer is about one and a half bits per ply. The interesting word is
*permanent*. A compressor may be upgraded; a consensus decoder may not. So every
scheme is scored on four axes and ratio is the least important of them:

| Axis | Question |
|---|---|
| **Ratio** | bits per ply |
| **Decode cost** | what must run to turn bits back into moves |
| **Permanence** | what must never change again for old data to stay readable |
| **Reach** | what the scheme forbids you from ever storing |

## The four results

**1. Store moves, not positions — a factor of thirty.** A position needs ~150
bits; a move needs ~5. `papers/01-position-space.md` derives the position count
from scratch to establish that the ratio is real and not an artefact of a lazy
position encoder, and then uses it to justify never storing a position again.

**2. Chess has almost no symmetry, and what it has belongs in the index.** The
square has a symmetry group of order 8. Pawns kill rank reversal; castling
rights kill file reversal; diagonals need both dead. A general middlegame
position has a symmetry group of order **1**. Measured: symmetry is worth 0–1
bit as compression and about **4× fewer distinct keys** as an index, which is
where it goes.

**3. Prefix sharing and predictive coding are the same saving.** The obvious
design — store the opening trie once, store each game as a pointer into it plus
the tail — is not an improvement on top of entropy coding. It *is* entropy
coding, done worse: a fixed-width pointer into a book of 4,096 lines costs 12
bits where the entropy of the choice is 8.8. You cannot bank the saving twice.
The trie is still exactly right as the *index*, which is the shape of this whole
project: **the structure that makes the database searchable and the structure
that would compress it are one object, and it can only be paid for once.**

**4. Below ~50 bytes a game, you are not storing chess, you are storing
signatures.** Two Ed25519 signatures and two public keys are 192 bytes. A
well-encoded 40-move game is 49. Compressing the moves a further 3× — the whole
argument about whether a learned model may live in consensus — shrinks the
record by **13%**. Batching the attestations shrinks it by **12× on the dominant
term**. Batch first, then compress, then re-evaluate.

Result 4 is the one that reorders the work.

## The papers

| File | What it settles |
|---|---|
| [`00-problem.md`](papers/00-problem.md) | The question, the four axes, and the three problems that get confused with each other |
| [`01-position-space.md`](papers/01-position-space.md) | Counting the space, the group that acts on it, why symmetry is nearly worthless |
| [`02-encodings.md`](papers/02-encodings.md) | Fifteen encodings, scored on all four axes |
| [`03-corpus.md`](papers/03-corpus.md) | Why the opening book and the compressor are one object |
| [`04-permanence.md`](papers/04-permanence.md) | What immutability costs in bits, and where compression should stop |
| [`05-index.md`](papers/05-index.md) | Transpositions, mirrors, proofs of absence, and where annotations go |
| [`06-decisions.md`](papers/06-decisions.md) | Every fork, with recommendations |
| [`07-sources.md`](papers/07-sources.md) | Every literature value, attributed, including the two weak ones |

## The arithmetic

Every figure in the papers is computed, an arithmetic identity, or an explicitly
attributed literature value marked `[lit]`.

```sh
python measure/run_all.py           # regenerate measure/RESULTS.md
python measure/run_all.py --check   # what CI runs
```

Standard library only, and no chess engine — anything needing one is a Rust
command instead. See [`measure/README.md`](measure/README.md), and
[`measure/EXPERIMENTS.md`](measure/EXPERIMENTS.md) for the five numbers the
papers need and do not yet have — chiefly `E[log2 b]` over a real corpus, which
tightens every storage figure here.

## The unsolved problem

`papers/06-decisions.md` **D7**. Anyone can fabricate a legal chess game between
two keys they own, at millions of games per second. A public database whose
contents can be manufactured for free is worthless for study, because the
statistics *are* the product. Removing the money from 0.0 removed a defence and
has not yet replaced it.

The provisional answer is to accept everything on chain and let the filtering
happen in the index, where it is contestable and where several people may
disagree in public. That is nearer to the discourse this version is for than any
consensus rule would be, and it is still not a solution. It is the decision that
determines whether the thing is useful, and it should not be closed quietly.

## What is *not* here

No consensus mechanism, no networking, no wallet, no VM, no chess engine. All of
those are downstream of decisions this version has not made, and 0.0's lesson is
that building the layer below a question you have not answered is how you get
months of work you have to throw away.

Stages 1–3 of the build order produce a complete, useful, studiable chess
database with no blockchain in it at all. That is worth noticing before starting
stage 4.
