# Reconciliation

*Four projects live in this repository. Three of them do not know about each
other. This document maps them, finds what they share, names where they
genuinely conflict, and lists the decisions reconciliation requires.*

**It decides nothing.** Every fork below is the owner's to settle. The purpose
here is to make sure the choice is made with all four on the table, which has
not been possible until now.

---

## The four

| | Branch | The question it exists to answer |
|---|---|---|
| **W** | PR #1 · `chess-wagering-blockchain` | Can two people wager on chess without trusting anyone? |
| **P** | `youthful-cori` · `0.1/` | What is the cheapest *permanent* encoding of a chess game, and what does each saving cost in something other than bits? |
| **A** | `youthful-lamport` | Where are the *borders* of a game? What if a position is **ground** rather than a set of objects? |
| **I** | `laughing-pascal` | Can a player's identity be **derived** from how they play, rather than assigned? |

Maturity is not equal. **P** is far ahead — six crates, twelve papers, 121,332
games ingested, ten commands, results written up. **A** has a complete
instrument (a chess engine, eight visual layers and a glyph alphabet, all inside
one self-contained HTML page) blocked on one manual step. **I** has a complete
pipeline validated only on constructed players. **W** is superseded: `0.1`
opens by saying "there is no money in this version" and means it.

---

## What they actually share

They look like four subjects. They are **one operation at four scales.**

| Scale | Project | What is compressed | What falls out |
|---|---|---|---|
| a **position** | **A** | one board | a field, a frontier, ~4 bits per square |
| a **game** | **P** codec | one move sequence | ~1.5 bits per ply |
| a **corpus** | **P** grammar | many games | repeated structure — *which is chess theory* |
| a **player** | **I** | one person's games | a point, a medal, ~30 bits |

Read the column of outcomes: compress a position and you get a **shape**;
compress a game and you get a **code**; compress a corpus and you get a
**theory**; compress a player and you get an **identity**.

That is the reconciliation, and it is not a retrofit — `METAPLAN` N11 already
claimed three of these scales were one operation. **A** supplies the fourth,
underneath, and it is the one nearest to how a person actually sees a board.

**W** is not a fifth scale. It is a settlement layer that could sit beneath any
of them, which is why it survives as deferred rather than cut.

### One sentence, if the four were one project

> Chess compresses at four scales — position, game, corpus, player — and each
> scale, compressed, yields a different kind of object: a shape, a code, a
> theory, an identity.

---

## The duplication, concretely

Shared substrate, built three times:

| Component | **P** | **A** | **I** |
|---|---|---|---|
| Move generation | `bc-chess` (perft 6) | in-page engine (perft 4, three positions) | `bc-chess` (same code, forked copy) |
| PGN / SAN reading | `bc-pgn`, hardened, parses clocks | in-page | `bc-style::pgn` — **written in ignorance of `bc-pgn`, and worse** |
| Position encoding | `bc-codec`, at the information floor | 4-bit-per-square glyph packing, 256-bit words | 26-byte packed (spec only) |

Three engines, three readers, three encodings. Only one of each is needed, and
in every row the mature implementation already exists in **P**.

The clock parsing in `bc-pgn` deserves a specific mention: it is the only place
in any of the four that reads *per-move think time*, which `papers/10` argues is
the single most valuable field in the public data. **I** throws that away today.

---

## Where they genuinely conflict

Not duplication — actual incompatibility.

### C1 · What the chain is for

Four different answers, and this is the sharpest fork in the repository.

| | The chain's job |
|---|---|
| **W** | settle wagers between two players |
| **P** | be the permanent public record of games — "you ask the chain, not a company" |
| **I** | commit the corpus, so a personality lens is verifiable rather than an operator's opinion |
| **A** | settle an on-chain **territory variant**, not standard chess |

**P** and **I** are close and may be the same chain: both want an append-only,
verifiable corpus of games. **I**'s requirement is strictly weaker — it needs
the corpus committed, not necessarily complete. **A** wants something different
in kind, because a territory variant is a different *game*, not a different
view of chess.

### C2 · Layout and language

- **P** — versioned folders (`0.0/`, `0.1/`), no root `Cargo.toml`, Rust
- **I** — root Cargo workspace, Rust
- **A** — no Cargo at all; one self-contained HTML page, JavaScript

These cannot all be true of one repository root. **P**'s versioned-folder
convention is deliberate and documented (`365d7b1`: "why versions are folders
rather than branches"), so it is the one with a stated reason behind it.

### C3 · Money

**W** is built on it, **P** explicitly removes it, **I** defers it (`METAPLAN`
N17), **A** reintroduces it via an on-chain variant. Any reconciliation has to
say which.

### C4 · A methodological collision worth knowing about

**P**'s experiment X13 builds a distance matrix between players and takes its
Laplacian modes, treating each player as a fixed point.

**I** has now measured that players are **not** fixed points: their profile
shifts significantly with the opponent (p < 0.005 for all four archetypes, on
policies that provably never change). A pairwise distance computed from games A
played *against* B therefore has the interaction term folded into it.

That does not invalidate X13 — but its premise is now something to check rather
than assume, and the check is cheap.

---

## What is blocked on the owner, not on a session

Both mature projects are stalled on the same kind of step, and neither can be
unblocked from inside a session:

- **P**, Stage 0 — download a Lichess monthly dump. Egress to
  `database.lichess.org` is blocked here by policy.
- **A**, Phase 1 — tag the photographed journal page. One page exists, so
  computer vision is the wrong tool and hand-tagging is correct.
- **I**, O3 — the same Lichess corpus, to replace synthetic archetypes with
  humans. Every number it reports is currently about four programs.

**One download unblocks two of the three.**

---

## The decisions reconciliation needs

Stated as questions, with what turns on each. None is answered here.

| | Question | What turns on it |
|---|---|---|
| **R1** | Is this one project or several that share a repository? | Everything below. If several, most conflicts dissolve and the answer is separate repos with a shared engine crate. |
| **R2** | What is the chain *for* — settlement, record, or corpus commitment? | C1. **P** and **I** can likely share one; **A** probably cannot. |
| **R3** | Which layout wins? | C2. **P**'s versioned folders have a written rationale; nothing else does. |
| **R4** | Which chess engine survives, and do the others depend on it? | Three exist. **A**'s is JavaScript, so sharing means either a port or a boundary. |
| **R5** | Does **I** adopt `bc-pgn` and delete its own parser? | Yes on the merits — this is the one decision here I would call obvious. It also gains clock data for free. |
| **R6** | Is **A**'s territory variant part of this project or its own product? | It is the only piece whose subject is *a different game*. |
| **R7** | Which of the four gets attention next? | They are not equally close to done, and none of them is blocked on ideas. |

---

## The one observation worth ending on

None of these four is short of ideas. **P** has twelve papers and a staged plan;
**A** has a specified pattern-web and gap map; **I** has a curriculum of
eighteen episodes. Between them there is more designed work than one person
executes in a year.

What all three are short of is **one corpus of real games** and **a decision
about which of them is the project**. The first is a download. The second is the
conversation this document exists to make possible.
