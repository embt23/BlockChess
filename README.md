# BlockChess

A from-scratch blockchain for wagered peer-to-peer chess.

Games are played **off-chain**, directly between two players, over an encrypted
link. Every position is signed by both players. The chain only ever sees two
things: the moment money is locked, and the moment money is paid out. The full
rules of chess exist on-chain as a **referee of last resort** — invoked only
when someone lies or disappears.

Above that base layer, anyone can run a **server**: a matchmaker, a tournament
organiser, a rating authority, a bot arena, a teaching ladder. Servers never
hold your keys and, in the default configuration, never hold your money.

## Status

Episodes 01–03 implemented and green. `perft(6) = 119,060,324` exact.

| Crate | Episode | What it is | Oracle |
|---|---|---|---|
| [`bc-hash`](crates/bc-hash) | 01 | SHA-256 & SHA-512 from FIPS 180-4, domain separation, hash chains | FIPS test vectors |
| [`bc-sig`](crates/bc-sig) | 02 | Ed25519 from scratch — field arithmetic mod 2^255−19, twisted Edwards group law, point compression | RFC 8032 vectors |
| [`bc-chess`](crates/bc-chess) | 03 | Bitboards, legal move generation, FEN, perft | published perft counts |

Each crate is checked against an oracle *someone else* published. That is the
standard for this project: no layer is built on top of rules that have only been
verified by tests we wrote ourselves.

```sh
cargo test --workspace                          # fast suite
cargo test --workspace --release -- --ignored   # perft(6), Kiwipete perft(5)

cargo run --release --bin perft -- 6            # 119,060,324 nodes
cargo run --release --bin perft -- divide 3     # per-move breakdown
```

**`bc-sig` must not sign with real keys.** `Point::mul_scalar` is not constant
time; it exists to be read. The node will link `ed25519-dalek`.

Bugs found along the way, and why they hid, are in
[`docs/build-log.md`](docs/build-log.md).

## Specification

| File | Contents |
|---|---|
| [`spec/00-overview.md`](spec/00-overview.md) | Layer stack, threat model, glossary |
| [`spec/01-primitives.md`](spec/01-primitives.md) | Hashing, signatures, encoding, domain separation |
| [`spec/02-chain.md`](spec/02-chain.md) | Accounts, sparse Merkle state, blocks, consensus, censorship |
| [`spec/03-position.md`](spec/03-position.md) | Board encoding, move generation, Zobrist, terminal conditions |
| [`spec/04-channel.md`](spec/04-channel.md) | The game channel — the heart of the protocol |
| [`spec/05-adjudication.md`](spec/05-adjudication.md) | Disputes, clock dilation, fraud proofs |
| [`spec/06-economics.md`](spec/06-economics.md) | Handicap odds, rake, Kelly, cheat detection |
| [`spec/07-servers.md`](spec/07-servers.md) | The server layer and its trust ladder |
| [`spec/08-privacy.md`](spec/08-privacy.md) | The privacy ladder |
| [`spec/09-open-questions.md`](spec/09-open-questions.md) | Decisions not yet made |
| [`docs/atlas.md`](docs/atlas.md) | The knowledge map — every primitive, and the attack that motivates it |
| [`docs/build-log.md`](docs/build-log.md) | Bugs found while building, and what each one teaches |

## Licence

TBD — intended to be permissive and open source.
