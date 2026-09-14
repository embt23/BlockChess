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

Episodes 01–04, 07 and 08 implemented and green. `perft(6) = 119,060,324` exact.

**You can win a wagered game against an opponent who disconnects.** Milestone
E — the sentence the whole project was aimed at. No cooperation from the
loser, no trusted third party, and the chain is never asked who was right,
only whether anybody moved.

```sh
cargo run --release --bin play      # Morphy's Opera Game, signed every ply
cargo run --release --bin dispute   # …and one won against someone who left
```

The chain sees two transactions for a 33-ply game. It sees a few more when
someone vanishes, and then the game simply continues on-chain under the same
rules, slowly, until one side stops moving and forfeits.

| Crate | Episode | What it is | Oracle |
|---|---|---|---|
| [`bc-hash`](crates/bc-hash) | 01 | SHA-256 & SHA-512 from FIPS 180-4, domain separation, hash chains | FIPS test vectors |
| [`bc-sig`](crates/bc-sig) | 02 | Ed25519 from scratch — field arithmetic mod 2^255−19, twisted Edwards group law, point compression | RFC 8032 vectors |
| [`bc-chess`](crates/bc-chess) | 03 | Bitboards, legal move generation, FEN, perft, the canonical 26-byte packed position, terminal conditions | published perft counts, FEN, published mates |
| [`bc-merkle`](crates/bc-merkle) | 04 | RFC 6962 list tree (`tx_root`) and a sparse Merkle tree (`state_root`) with proofs of absence | Certificate Transparency vectors |
| [`bc-channel`](crates/bc-channel) | 07 | The game channel — signed states, the hash chain, Fischer clocks, cooperative settlement, and a stub escrow | a published game: Morphy 1858, mate on move 17 |
| [`bc-channel::dispute`](crates/bc-channel/src/dispute) | 08 | The adjudicator — clock dilation, higher-ply-wins, optimistic mate claims and one-move refutation | the worked dilation table in `spec/05` |

Each crate is checked against an oracle *someone else* published. That is the
standard for this project: no layer is built on top of rules that have only been
verified by tests we wrote ourselves.

```sh
cargo test --workspace                          # fast suite
cargo test --workspace --release -- --ignored   # perft(6), Kiwipete perft(5)

cargo run --release --bin perft -- 6            # 119,060,324 nodes
cargo run --release --bin perft -- divide 3     # per-move breakdown
cargo run --release --bin play                  # a whole wagered game
```

**`bc-sig` must not sign with real keys.** `Point::mul_scalar` is not constant
time; it exists to be read. The node will link `ed25519-dalek`.

Bugs found along the way, and why they hid, are in
[`docs/build-log.md`](docs/build-log.md).

> **Working on this repository?** Read [`CLAUDE.md`](CLAUDE.md) first. It holds
> the thesis, the invariants, and a task-indexed map of everything below.

## Specification

| File | Contents |
|---|---|
| [`spec/00-overview.md`](spec/00-overview.md) | Layer stack, threat model, glossary |
| [`spec/01-primitives.md`](spec/01-primitives.md) | Hashing, signatures, encoding, domain separation |
| [`spec/02-chain.md`](spec/02-chain.md) | Accounts, sparse Merkle state, blocks, consensus, censorship |
| [`spec/03-position.md`](spec/03-position.md) | Board encoding, move generation, Zobrist, terminal conditions |
| [`spec/04-channel.md`](spec/04-channel.md) | The game channel — the heart of the protocol, and `crates/bc-channel` |
| [`spec/05-adjudication.md`](spec/05-adjudication.md) | Disputes, clock dilation, fraud proofs — and `crates/bc-channel/src/dispute` |
| [`spec/06-economics.md`](spec/06-economics.md) | Handicap odds, rake, Kelly, cheat detection |
| [`spec/07-servers.md`](spec/07-servers.md) | The server layer and its trust ladder |
| [`spec/08-privacy.md`](spec/08-privacy.md) | The privacy ladder |
| [`spec/09-open-questions.md`](spec/09-open-questions.md) | Every decision that forks the project, and which of them are settled |
| [`spec/10-personality.md`](spec/10-personality.md) | **The thesis.** Why the player, not the game, is the asset |
| [`spec/11-resources.md`](spec/11-resources.md) | The four transactable layers: GAME, STAKE, STYLE, CLAIM |
| [`docs/atlas.md`](docs/atlas.md) | The knowledge map — every primitive, and the attack that motivates it |
| [`docs/build-log.md`](docs/build-log.md) | Bugs found while building, and what each one teaches |

## Governance

There is one thing worth stating plainly rather than leaving to be discovered.

`GameTerms.adjudicator_ver` pins each channel to a version of the chess rules,
and **every version is kept forever**, so an upgrade is purely additive: a
channel opened under version 3 is adjudicated by version 3 no matter what is
registered later. Nobody can reach into an open channel, change who won a
finished game, or strand money by shipping an upgrade.

Registering a *new* version is currently the privilege of **one key, held by
Evan ([@embt23](https://github.com/embt23))**, with effect delayed by a timelock
counted in blocks so that anyone who objects can close their channels before a
change lands.

This is a benevolent dictatorship and it is described as one. The alternative
available today — a two-thirds validator vote over a small, personally recruited
validator set — is a mechanism that would describe the same few people while
sounding like a constitution.

The condition for handing the key to validator control is *(to be filled in
before the first public testnet: a minimum count of independently operated
validators, and a chain carrying something other than play tokens)*.

Reasoning and alternatives: [`spec/09-open-questions.md`](spec/09-open-questions.md) D11, D25.

## Licence

Two licences, because the code and the prose do different jobs.

- **Code** — everything in `crates/` — is [Apache License 2.0](LICENSE).
  Permissive, and it carries the explicit patent grant MIT lacks.
- **Prose** — everything in `spec/` and `docs/`, and the Markdown at the root —
  is [CC BY 4.0](LICENSE-DOCS). Attribution, not share-alike: the spec files are
  the scripts for the video series, and share-alike would hang an unresolved
  copyleft question over works derived from them.

See [`NOTICE`](NOTICE), and [`spec/09-open-questions.md`](spec/09-open-questions.md) D13 for why AGPL was considered and rejected.
