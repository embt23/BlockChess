# Start here

You are building a public chess database that nobody owns. This page is the
one to open when you have lost the thread. It has three parts: **run it**,
**what each piece is for**, and **what to do next**.

---

## 1. Run it

You need Rust. If you have not got it: <https://rustup.rs>, one command, then
reopen your terminal.

```sh
cd 0.1
cargo build --release
```

First build takes a minute or two. After that, five things to try, in order.
Each one takes seconds.

### The rules are correct

```sh
./target/release/blockchess perft 5
```

```
  perft(5) = 4,865,609
  0.07s, 65.4 Mnps
  published    4,865,609   MATCH
```

`perft` counts every leaf of the move tree to a given depth. The count is
published — other people computed it independently, decades ago — so if yours
matches, your rules are right. This is the only place in the project where
correctness is checkable *exactly*, and everything else is built on top of it.
Try `perft 6` too; it takes a couple of seconds and counts 119 million nodes.

### Look at a position

```sh
./target/release/blockchess show "r1bqkbnr/pppp1ppp/2n5/4p3/2B1P3/5Q2/PPPP1PPP/RNB1K1NR w KQkq - 4 4"
```

Draws the board, then lists every legal move **in the order the file format
defines**, with its index. That list is the thing the whole storage design is
about: the encoder writes down "move number 23", not "queen takes f7".

### Play

```sh
./target/release/blockchess play
```

You play both sides. Type `e4`, or `e2e4`, whichever you prefer. `list` shows
the legal moves and their indices, `bits` shows what the game so far costs in
each encoding, `back` undoes, `quit` leaves.

This is the piece with no research value at all. It is here because being able
to *see* the thing makes the rest of it stop being abstract.

### Measure real games

```sh
./target/release/blockchess measure corpus/classics.pgn
```

This is the important one. It reads PGN — the format every chess database in
the world already exports — and computes the numbers the papers were missing.

Point it at a big file and it answers **experiment X1**: the real average cost
of a chess move, which every storage figure in `papers/` depends on.

### Compress

```sh
./target/release/blockchess pack corpus/classics.pgn games.bcg
./target/release/blockchess unpack games.bcg
```

`pack` decodes every game back and compares it before writing, so a file that
exists is a file that round-tripped. `unpack` prints the games again as PGN.

---

## 2. What each piece is for

```
0.1/
├── papers/          the thinking.  Read 00 then 04; the rest on demand.
├── measure/         Python. Arithmetic that needs no chess engine.
├── crates/          Rust. Everything that touches an actual game.
│   ├── bc-chess     the rules            ← lifted from 0.0, perft-verified
│   ├── bc-pgn       reading real games   ← how a corpus gets in
│   ├── bc-codec     games → bytes → games
│   └── bc-cli       the `blockchess` command
└── corpus/          four games, including two famous ones
```

### `bc-chess` — the rules

Bitboards and legal move generation, carried over from 0.0 unchanged. It is
here rather than rewritten because it is *the rules of chess*, not
architecture: it was checked against an oracle somebody else published, and
rewriting it would have bought a month of nothing.

### `bc-pgn` — reading real games

PGN is how every chess database on earth exports. Getting games *in* is what
turns the papers from arithmetic into measurement.

The hard part is that SAN is ambiguous on purpose. `Nf3` does not say which
knight, so resolving it means generating the legal moves and finding the one
that fits — which is exactly why `papers/02-encodings.md` rejects SAN as
storage: it costs 40 bits a move **and** still needs the rules engine to read.

### `bc-codec` — the actual subject of 0.1

Two encodings, so every claim is a comparison rather than an assertion:

| | what it stores | cost | needs the rules? |
|---|---|---|---|
| **E3** | the move itself, 16 bits | 16 bits/ply | no |
| **E7** | the move's *index* in the legal list | `log2 b` bits/ply | yes |

E7 is the interesting one. If there are 30 legal moves, naming one takes
`log2 30 = 4.9` bits instead of 16. You pay for it by needing the full rules
of chess to read the file at all.

The neat part is how the fractional bits are not wasted. Rounding `4.9` up to
5 throws away 10% of the payload, so instead **the whole game is one integer in
a mixed radix**:

```
N = ((… d₃)·b₂ + d₂)·b₁ + d₁
```

`dᵢ` is the move index at ply `i`, `bᵢ` is how many moves were legal there.
Decoding peels plies off the front — `d = N mod b; N /= b` — and `b` is always
known because the decoder has replayed everything before it. The output is
exactly `⌈log2 ∏ bᵢ⌉` bits: the information-theoretic floor, with no rounding
waste anywhere except once at the end of the game.

That is what an arithmetic coder would give you, without the carry logic that
makes arithmetic coders famously hard to get right. It works because every
model here is uniform, and for uniform models the two are the same code.

---

## 3. The one thing that is easy to get wrong

**The order of the legal move list is part of the file format.**

E7 stores "move number 23". If a future version of the move generator lists
the moves in a different order, move 23 becomes a *different move* — and the
game still decodes, still looks legal, and is now a different game. No error,
no crash, silent corruption of everything already stored.

So the order is written down as a rule about chess rather than inherited from
whatever the generator happens to emit:

> Ascending by origin square, then destination square, then promotion piece in
> the order N, B, R, Q.

and `crates/bc-codec/tests/enumeration_order.rs` fails loudly if it ever
changes. If you change it deliberately, that is a new rule set with a new id
(`papers/04-permanence.md` §5), not a new expectation in that test.

---

## 4. What to do next

**Get a real corpus.** Everything is waiting on this. Download a PGN archive —
a few thousand games is plenty to start — and run `measure` on it. That single
command replaces the placeholder in `papers/02-encodings.md` and every figure
downstream of it, and it turns the project's headline claims from other
people's numbers into yours.

Then, roughly in order:

1. **`measure --csv`** on that corpus, and plot the per-ply curve.
   `papers/03-corpus.md` predicts it rises and then flattens; find out.
2. **E8** — rank the legal moves by a static heuristic and code the rank
   instead of the index. `papers/02-encodings.md` says this reaches ~4 bits a
   ply. It is the one remaining encoding that could beat E7 without putting a
   neural network in consensus. Experiment X2.
3. **The index** — a trie of openings and a position→games map, so that
   "who has played this" becomes a query. This is where the project stops
   being a compressor and starts being the thing you actually want.

Note that none of those three needs a blockchain. Stages 1–3 give you a
complete, useful chess database with no chain in it at all, and it is worth
noticing that before starting on stage 4.

---

## 5. Where the mathematics is

`papers/`, in this order:

- **`00-problem.md`** — what the question is, and the three different
  questions people confuse it with.
- **`04-permanence.md`** — the main result. Start here if you only read one.
  Below ~50 bytes a game you are storing signatures, not chess.
- **`01-position-space.md`** — the counting and the group theory. Why chess
  has almost no symmetry, and why that is the right answer rather than a
  disappointing one.
- **`02-encodings.md`** — fifteen ways to write down a game.
- **`03-corpus.md`** — why the opening book and the compressor are the same
  object, and why that means you cannot bank the saving twice.
- **`05-index.md`** — making it searchable.
- **`06-decisions.md`** — every fork, with a recommendation. **D7 is the open
  one**, and it is the one that decides whether the thing is useful.
