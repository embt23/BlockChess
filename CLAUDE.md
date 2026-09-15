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
  countersigned game. *Signed by both voices 2026-09-14 (`spec/09` D19). Until
  then this invariant asserted one side of an openly DISPUTED question — see the
  note on that entry in `docs/duality.md`, which is worth reading before adding
  any invariant.*

**Engineering**
- `G0` **The person filming types the code the episode is about.** Evan writes
  the episode's subject — the arithmetic, the rule, the check. Claude writes
  plumbing, tests, serialisation and oracle harnesses. `spec/09` D26.
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
| **touching anything a validator runs** | **`crates/bc-adjudicator`** — `no_std`, and its dependency list is a test |
| touching blocks, headers, transactions, fork choice | `crates/bc-block` — both engines implement its `Consensus` trait |
| touching consensus | `crates/bc-pow` (episode 05), `crates/bc-bft` (episode 06), `crates/bc-net` (the simulated network) |
| wiring consensus to the escrow, or asking what a reorg costs | `crates/bc-node` — start at `bin/reorg.rs` |
| about to type an episode's subject | [`docs/g0-holes.md`](docs/g0-holes.md) — the list, and what each test is for |
| adding an oracle, or wondering why one is not one | `crates/bc-conformance` |
| wondering why something is written oddly | `docs/build-log.md` |
| measuring style | `docs/d20-calibration.md` + `crates/bc-style` |
| reading a PGN or writing notation | `crates/bc-chess/src/san.rs` and `uci.rs` |
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
| **05 proof of work** | **`bc-pow`** + `bc-block` | its own reorg | ✅ ⚠️ **`G0`: the control loop is Evan's** |
| **06 BFT** | **`bc-bft`** + `bc-net` | partition & equivocation tests | ✅ ⚠️ **`G0`: the locking rules are Evan's** |
| 07 state channel | `bc-channel` | Morphy 1858 | ✅ **Milestone D** — a whole wagered game, signed and settled |
| 08 adjudication | `bc-adjudicator` | D23 split oracle | ✅ **Milestone E, earned** — against a real chain (`bc-node`) |
| D20 style estimator | `bc-style` | synthetic ground truth | ✅ **measured: AMBER** — `docs/d20-result.md` |
| D23 split oracle | `bc-conformance` | shakmaty + stateright | ✅ found two real bugs — `build-log` §15, §16 |
| 10 forced inclusion | — | — | not started — **the nearest real hole** |

286 tests, clippy and fmt clean. Two `#[ignore]`d suites are red on purpose
and are listed in [`docs/g0-holes.md`](docs/g0-holes.md).

```sh
cargo test --workspace                          # fast suite
cargo test --workspace --release -- --ignored   # perft(6), Kiwipete perft(5)
cargo run --release --bin perft -- 6
cargo run --release --bin play                  # a whole wagered game
cargo run --release --bin dispute               # …won against someone who left
cargo run --release -p bc-style --bin calibrate # the estimator's own bias
cargo run --release -p bc-style --bin measure -- <pgn-dir>   # D20, on humans
cargo run --release -p bc-node --bin reorg      # the same dispute on two chains
cargo test -p bc-conformance --release -- --ignored  # ~1M positions vs shakmaty
cargo test -p bc-pow -- --ignored               # G0: episode 05's subject
cargo test -p bc-bft -- --ignored               # G0: episode 06's subject
```

**`bc-sig` must not sign with real keys.** `Point::mul_scalar` is not constant
time; it exists to be read. The node links `ed25519-dalek`.

## Next

**Milestone E is earned, and the sentence had to change to survive it.**

Episode 08 won a wagered game against an opponent who stopped answering,
against a `BTreeMap` height counter. D21 said that was simulated rather
than earned and that a real chain would fix it. Half right. A real chain
does earn it — and a real *proof-of-work* chain then breaks it, because a
counter only goes up and a chain reorganises. A reorg landing after a
deadline converts a defence made correctly into a forfeit.

So the claim is now:

> You can win against an opponent who disconnects, **on a chain with
> deterministic finality.** On one without, you can defend correctly and
> lose anyway.

`cargo run --release -p bc-node --bin reorg` is that sentence, run twice
with only the engine swapped. It is also why `spec/02` chose BFT, and the
first time that choice has been demonstrated rather than argued.

`spec/09` D21–D25 are all closed. `docs/episode-08-gap.md` is kept because
the reasoning transfers.

### The two open `G0` holes

Episodes 05 and 06 are built except for the one function each is *about*
(`G0`, D26). Both have complete `#[ignore]`d test suites and a CI job that
runs them so they stay visible. [`docs/g0-holes.md`](docs/g0-holes.md).

- **`bc_pow::retarget::next_target`** — difficulty as a control loop. The
  closed-loop hashrate-step test is the episode.
- **`bc_bft::locking`** — what a validator must remember across a failed
  round. A single round needs none of it, which is exactly why the seam is
  there.

### Where the next real work is

1. **Episode 10, forced inclusion.** The nearest real hole and now the
   *only* structural one below the channel. Deadlines assume your
   transaction gets in; `Payload::is_dispute` marks which transactions the
   reserve is for and **nothing enforces the reserve**. A validator who
   censors `DisputeMove` for Δ blocks wins the game for your opponent, on
   either engine. Neither episode 05 nor 06 helps: BFT stops a committed
   block being un-included, and does nothing about one that never arrives.
2. **The P2P layer.** `spec/04`'s message set over Noise is entirely
   unbuilt — the two players are in-process objects and validators talk
   over `bc-net`, which is a simulator. This is the largest unwritten
   layer in the project.
3. **Everything downstream of D20.** `spec/10` §7's timescales are wrong by
   ~4× and the rewrite is unclaimed. The three *assumed* engine rows in
   `spec/06` §5 could be measured with `bc-style` and an engine as a player.

The invariant to keep asking: which task most shortens the path to
something a stranger would trust with money.

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
- **budget** — a player's total remaining on-chain blocks in a dispute, set
  once by dilating their game clock. Not per move; each move also costs at
  least `MIN_MOVE_BLOCKS`.
- **evidence** — the highest state your *opponent* signed. What `DisputeOpen`
  takes, and why countersignatures ride along with moves. `Channel::evidence`.
- **clock dilation** — the map from game-clock milliseconds to an on-chain block
  budget, preserving the ratio so stalling cannot buy time back (`spec/05`).
- **π_you / π_pop** — your policy, and the population's. Divergence between them
  is identity (`spec/10`).
- **style** — a model of `π_you`. Publishable under `E2`.
- **novelty** — a `(position, move)` pair in no prior public game. Claimable by
  proof-of-play plus a Merkle **non-inclusion** proof (`spec/11` L3).
- **GAME / STAKE / STYLE / CLAIM** — the four resource layers (`spec/11`):
  consumable, fungible, informational, positional.
