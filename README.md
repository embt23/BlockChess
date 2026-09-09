# BlockChess

**Your play is your name, and you don't get to hide it.**

You are a function from positions to moves. BlockChess compresses that function,
makes the compression your public name, and chains it over time so the record of
who you were becomes the record of who you became.

Your identity is not 32 random bytes you were handed. It is **derived** — a
position in a personality space discovered by factorising a corpus of real
games, which anyone holding that corpus can recompute for themselves. A single
hash is where you stand today. The chain of them is the narrative of how you got
there, and a link is earned by **changing**, not by playing: a thousand games
without changing how you play earns exactly one link.

It needs a chain because a lens is only fair if everyone can verify the corpus
it was fitted to. A mutable database would let an operator quietly add or drop
games, shift the axes, and silently re-judge every player who ever earned a
medal.

Read [`METAPLAN.md`](METAPLAN.md) first — it says what this is, what order it
happens in, and what was cut.

## Status

Episodes 01–12 implemented and green. `perft(6) = 119,060,324` exact. On four
constructed personalities the lab attributes a **single held-out game** to the
right player 72.5% of the time against a 25% chance baseline, and seven games
are enough to identify one 95% of the time.

| Crate | Episode | What it is | Oracle |
|---|---|---|---|
| [`bc-hash`](crates/bc-hash) | 01 | SHA-256 & SHA-512 from FIPS 180-4, domain separation, hash chains | FIPS test vectors |
| [`bc-sig`](crates/bc-sig) | 02 | Ed25519 from scratch — field arithmetic mod 2^255−19, twisted Edwards group law, point compression | RFC 8032 vectors |
| [`bc-chess`](crates/bc-chess) | 03 | Bitboards, legal move generation, FEN, perft | published perft counts |
| [`bc-merkle`](crates/bc-merkle) | 04 | RFC 6962 list tree (corpus commitment) and a sparse Merkle tree with proofs of absence | Certificate Transparency vectors |
| [`bc-style`](crates/bc-style) | 05–09 | The compression: features, a discovered basis, the medal, the chain of medals | analytic eigen-spectra; constructed personalities |
| [`bc-style::identify`](crates/bc-style/src/identify.rs) | 10, 12 | Held-out attribution, the convergence curve, and the medal's forgery margin | a train/test split the lens never sees |

Each crate is checked against an oracle *someone else* published. That is the
standard for this project: no layer is built on top of rules that have only been
verified by tests we wrote ourselves.

```sh
# Every command takes the same source: a .pgn file, or a number of
# synthetic round-robin rounds. Default is the constructed players.
cargo run --release -p bc-style --bin style -- demo              # a report
cargo run --release -p bc-style --bin style -- identify          # held-out attribution
cargo run --release -p bc-style --bin style -- interact          # the interaction term
cargo run --release -p bc-style --bin style -- viz > space.html  # a page
cargo run --release -p bc-style --bin style -- viz my.pgn > mine.html

cargo test --workspace                          # fast suite
cargo test --workspace --release -- --ignored   # perft(6), Kiwipete perft(5)

cargo run --release --bin perft -- 6            # 119,060,324 nodes
cargo run --release --bin perft -- divide 3     # per-move breakdown
```

**`bc-sig` must not sign with real keys.** `Point::mul_scalar` is not constant
time; it exists to be read. The node will link `ed25519-dalek`.

The crate refuses to be quoted on a corpus too small to support it — a basis
fitted to fewer games than features is a well-formed object containing nothing,
and it says so in the terminal, the JSON and the page.

Bugs found along the way, why they hid, and one caught before it bit, are in
[`docs/build-log.md`](docs/build-log.md).

## Specification

**Start here: [`METAPLAN.md`](METAPLAN.md)** — what this project is, the order
it happens in, and what is still undecided. It outranks every file below.

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
| [`spec/10-personality.md`](spec/10-personality.md) | **The compression — the current centre of the project** |
| [`docs/atlas.md`](docs/atlas.md) | The knowledge map — every primitive, and the attack that motivates it |
| [`docs/build-log.md`](docs/build-log.md) | Bugs found while building, and what each one teaches |

## Licence

TBD — intended to be permissive and open source.
