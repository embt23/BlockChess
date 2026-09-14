# BlockChess — read this first

**This file holds only what cannot be re-derived from the code.** Facts
recoverable by reading a source file do not belong here. Decisions, invariants,
and the reasons behind them do. Compress accordingly when editing it.

---

## What this is

A blockchain, built from scratch, where two people wager on a chess game.
Peer-to-peer, non-custodial, open source. Above it, anyone may run a server —
friend lobbies, tournaments, bot arenas, teaching ladders, style markets.

The build doubles as a documentary series: an atlas of cryptography, data
structures, group theory and information theory, with chess wagering as the
through-line that makes every abstract primitive concrete.

## The thesis

> **Everything worth money here is a divergence between two models of the same
> thing.**

Four quantities, one operation `D(· ‖ ·)`:

| | Is | File |
|---|---|---|
| **Edge** — bankroll growth | `D(belief ‖ odds)` | `spec/06` §4 |
| **Identity** — how much you are you | `D(π_you ‖ π_pop)` | `spec/10` §4 |
| **Evidence** — that someone cheated | `D(observed ‖ claimed)` | `spec/06` §5 |
| **Value** — what a style is worth | `D(posterior ‖ prior)` | `spec/10` §5 |

A corollary that collapses two planned subsystems into one: **the cheat detector
and the style asset are the same object.** One says *"this does not look like
you"*; the other says *"this is what I look like."*

## The four structural insights

Everything in the design follows from these. If a proposed change contradicts
one, it is almost certainly wrong.

1. **The chain is a court, not a referee.** Moves are exchanged peer-to-peer and
   signed; the chain sees two transactions per game. It is visited only when
   someone cheats.
2. **A chess channel is strictly easier than a payment channel.** Ply only
   increases and money moves only on terminal states, so old states are
   harmless. `Higher ply wins` replaces Lightning's entire revocation apparatus.
   Consequence: watchtowers here hold no secrets and cannot steal, so they can
   be run by strangers.
3. **∀ is expensive, ∃ is cheap.** Proving checkmate quantifies over ~218 moves;
   refuting it takes one. So claims are optimistic and refutations are
   verified — and it is free, because refuting requires no liveness the protocol
   did not already require.
4. **The game carries no information; the players carry all of it.** The rules
   are common knowledge and common knowledge has zero surprisal. A game is a
   sample from two policies. *The policy is the asset; the game is evidence of
   it.*

---

## Invariants

Violating one of these is a bug, not a design choice. Cite them by tag.

**Protocol**
- `P1` Moves never go on-chain except under dispute.
- `P2` Higher ply always wins. No revocation secrets, no penalty transactions.
- `P3` Never verify checkmate. Assert it; allow refutation by a single move.
- `P4` Challenge windows are counted in **blocks, never seconds**. A halted chain
  must not expire anyone's window.
- `P5` The chess rules exist **exactly once** (`bc-chess`) and are compiled for
  both the client and the on-chain adjudicator. Two implementations means a
  consensus split with money on it.
- `P6` Servers relay signatures; by default they never custody funds.

**Economic**
- `E1` **Never mint tokens for playing.** The moment playing pays, two bots farm
  each other. No exceptions, no "activity rewards", no emissions.
- `E2` You may publish any function of **your own** decisions. Never a model that
  predicts your opponents'. (Reason is economic, not just privacy: it stops
  strong players being farmed for data.)
- `E3` Claims are **attribution, never exclusion**, and every claim costs a real
  countersigned game.

**Engineering**
- `G1` **Every layer is verified against an oracle someone else published.**
  FIPS vectors, RFC 8032, perft counts, CT vectors. A reference you wrote
  yourself is not an oracle — it is a second implementation with its own bugs.
  This rule exists because it caught a real bug that twelve hand-written tests
  missed.
- `G2` No file over ~300 lines. No function that cannot be held in the head.
- `G3` Spec before code. The spec files are the scripts for the series.

---

## Where to look

Task-indexed. Read the row, not the whole tree.

| If you are… | Read |
|---|---|
| orienting from scratch | this file, then [`docs/duality.md`](docs/duality.md), then `spec/00-overview.md` |
| touching the channel or the move protocol | `spec/04-channel.md` |
| touching disputes, timeouts, clocks | `spec/05-adjudication.md` |
| touching chess rules, board encoding, terminal conditions | `spec/03-position.md` + `crates/bc-chess` |
| touching hashing, signatures, encodings | `spec/01-primitives.md` |
| touching blocks, state, consensus, censorship | `spec/02-chain.md` |
| touching stakes, odds, rake, ratings, cheat detection | `spec/06-economics.md` |
| touching servers, matchmaking, trust, jurisdiction | `spec/07-servers.md` |
| touching privacy, stealth addresses, ZK | `spec/08-privacy.md` |
| **touching style, identity, or why any of this matters** | **`spec/10-personality.md`** |
| touching tokens, wagers, style markets, novelty claims | `spec/11-resources.md` |
| about to make a design decision | `spec/09-open-questions.md` — check it is not already decided |
| planning work or an episode | `docs/atlas.md` |
| touching the channel implementation, clocks, or settlement | `crates/bc-channel` — start at its `lib.rs` |
| wondering why something is written oddly | `docs/build-log.md` |
| measuring style, or reading a PGN | `docs/d20-calibration.md` + `crates/bc-style` |
| about to treat a design question as settled | `docs/duality.md` — check it is not HALF-SIGNED or DISPUTED |

## Where the surprises are

Read `docs/build-log.md` before debugging anything in `bc-hash` or `bc-sig`.
Two entries in particular:

- A hash that **passed every official FIPS vector** and still silently discarded
  buffered bytes across `update()` calls. Official vectors are single-call; the
  bug lived between calls.
- A field-arithmetic borrow bug that fired only on the doubled identity element,
  and whose wrong answers were **still valid points on the curve** — so the
  obvious sanity check passed. RFC 8032 caught it; nothing else did.

Both are why `G1` exists.

---

## Two voices

This project has two authors with two epistemologies — one that reaches for
meaning and whole structures, one that reaches for theorems and counterexamples.
[`docs/duality.md`](docs/duality.md) keeps both readings of every major idea,
with a status on each: **CERTIFIED**, **AMENDED**, **HALF-SIGNED**, or
**DISPUTED** — the same countersignature rule the channel protocol uses, applied
to the design itself.

`G4` **Neither voice deletes the other.** An intuition that cannot yet be
formalised stays, marked half-signed; it is not decoration and not an
untidiness. An entry marked HALF-SIGNED or DISPUTED is **not settled**, however
confident the analytic column sounds. Surface it rather than resolving it
silently.

## State

| Episode | Crate | Oracle | Status |
|---|---|---|---|
| 01 hashing | `bc-hash` | FIPS 180-4 | ✅ |
| 02 signatures | `bc-sig` | RFC 8032 | ✅ |
| 03 chess rules | `bc-chess` | perft counts | ✅ `perft(6) = 119,060,324` |
| 04 Merkle trees | `bc-merkle` | CT vectors | ✅ |
| D20 style estimator | `bc-style` | synthetic ground truth | ✅ calibrated; awaiting a real corpus |
| 05–06 consensus | — | — | not started |
| 07 state channel | `bc-channel` | Morphy 1858 | ✅ **Milestone D** — a whole wagered game, signed and settled |
| 08 adjudication | — | — | not started ← **the next thing that matters** |

109 tests, clippy and fmt clean. CI runs the suite, the slow exact perft runs,
and the full-game demo.

```sh
cargo test --workspace                          # fast suite
cargo test --workspace --release -- --ignored   # perft(6), Kiwipete perft(5)
cargo run --release --bin perft -- 6
cargo run --release --bin play                  # a whole wagered game
```

**`bc-sig` must not sign with real keys.** `Point::mul_scalar` is not constant
time; it exists to be read. The node links `ed25519-dalek`.

## Next

Episode 07 is done and it was built against a stub ledger (`bc-channel::ledger`)
rather than waiting for consensus, exactly as planned: consensus slides
underneath later without the channel noticing.

**Episode 08, the adjudicator.** Everything in `bc-channel` assumes both
players keep answering. When one stops, the honest player holds a certified
state and nowhere to take it. Giving it somewhere to go is the whole remaining
distance to Milestone E, and it needs: `DisputeOpen`, the Δ window counted in
blocks (`P4`), clock dilation (`spec/05`), and the optimistic mate claim with
its one-move refutation (`P3`). The refutation half already exists —
`Position::refutes_terminal_claim`.

**Milestone E is the project: *you can win against an opponent who
disconnects*.** Everything before it is prerequisites; everything after is
expansion. When unsure what to work on, ask which task most shortens the path to
that sentence.

Note the one task that is *not* on this path and is still worth doing first:
**D20** (`spec/09`), the divergence measurement. It needs no protocol, it can
be done today, and it either grounds Act II in a real number or kills it.

---

## Vocabulary

- **ply** — one move by one player. The sequence number. Monotonic.
- **certified state** — a game state signed by *both* players. The unit of
  evidence. One signature is a claim; two is a fact.
- **pos_hash / rep_hash** — two hashes over the same packed position, asking
  different questions. `pos_hash` commits to everything including the halfmove
  clock, because a dispute replays moves from it. `rep_hash` clears that clock,
  because two occurrences of a position always differ in it — that is what a
  repetition is. Confusing them is `docs/build-log.md` §05.
- **Δ** — the challenge window, in blocks.
- **clock dilation** — the map from game-clock milliseconds to an on-chain block
  budget, preserving the ratio so stalling cannot buy time back (`spec/05`).
- **π_you / π_pop** — your policy, and the population's. Divergence between them
  is identity (`spec/10`).
- **style** — a model of `π_you`. Publishable under `E2`.
- **novelty** — a `(position, move)` pair in no prior public game. Claimable by
  proof-of-play plus a Merkle **non-inclusion** proof (`spec/11` L3).
- **GAME / STAKE / STYLE / CLAIM** — the four resource layers (`spec/11`):
  consumable, fungible, informational, positional.
