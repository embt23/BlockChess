# BlockChess

Chess on a blockchain. Two versions live here, and they are different projects
that happen to share a name.

| | | |
|---|---|---|
| [**0.1**](0.1/) | current | A public, permanent, searchable record of chess games. No money. The mathematics of compressing chess into a permanent log — and a Rust engine and codec you can run today. |
| [**0.0**](0.0/) | archived, complete as far as it went | A from-scratch blockchain for wagered peer-to-peer chess. Full protocol spec, plus SHA-256, Ed25519, a move generator passing `perft(6)`, and Merkle trees, all written from the standards. |

## 0.0 — proof of concept

Archived, not abandoned. Nine specification documents, four Rust crates, every
one checked against an oracle someone else published: FIPS 180-4 vectors,
RFC 8032, RFC 6962, and the published perft counts. `perft(6) = 119,060,324`
exact, first run.

It is left exactly as it was, including `0.0/docs/build-log.md`, which is the
most useful thing in it — three bugs and why each one hid. CI still builds and
tests it; the only change made when archiving was the path.

Its subject was money: escrow, disputes, adjudication, handicap odds, cheat
detection. 0.1 removes all of that on purpose.

## 0.1 — what the project is actually for

The moves should be public. Not the wagers, not the ratings, not the economy —
the games. A growing chess database anyone can study, that nobody owns, and that
does not disappear when a company does.

Which turns the project into one question with a real answer:

> **What is the cheapest permanent encoding of a chess game, and what does each
> saving cost you in something other than bits?**

Permanence is what makes this different from ordinary compression. A compressor
can be upgraded; a consensus decoder cannot, and the price of that is
computable. Four results so far, derived in [`0.1/papers/`](0.1/papers/) and
computed by [`0.1/measure/`](0.1/measure/):

1. **Store moves, not positions** — a position costs ~150 bits, a move ~5.
2. **Chess has almost no symmetry** — a general middlegame position has a
   symmetry group of order 1, so symmetry is worth ~0 bits as compression and
   ~4× as an index key.
3. **The opening trie and the entropy coder are the same saving** — you cannot
   bank it twice, and the trie's real job is search.
4. **Below ~50 bytes a game you are storing signatures, not chess** — batching
   the attestations beats every move-encoding decision combined.

### Run it

```sh
cd 0.1 && cargo build --release
./target/release/blockchess perft 5                      # the rules are right
./target/release/blockchess play                         # a board in the terminal
./target/release/blockchess measure corpus/classics.pgn  # the real numbers
./target/release/blockchess pack corpus/classics.pgn out.bcg
```

Start at [`0.1/START-HERE.md`](0.1/START-HERE.md) if you want the walkthrough,
or [`0.1/README.md`](0.1/README.md) for the summary.

## Licence

TBD — intended to be permissive and open source.
