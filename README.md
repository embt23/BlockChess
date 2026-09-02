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

Design phase. The specification is being written before the code, in public.

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

## Licence

TBD — intended to be permissive and open source.
