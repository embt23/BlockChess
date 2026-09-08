# 00 — Overview

## The one-sentence version

Two people lock money in a contract, play chess over a private link where every
position is signed by both of them, and then hand the chain a single signed
final position; the chain knows the rules of chess well enough to punish anyone
who lies about that final position or refuses to produce one.

## The three reframings this design rests on

Everything below follows from three observations. If you remember nothing else,
remember these.

### 1. Chess is a game of perfect information, so all encryption here is against third parties, not against your opponent

There is nothing to hide from the person you are playing. You both see the whole
board. This deletes an entire discipline from the project — no mental poker, no
commitment schemes to hide a hand, no verifiable shuffling. Compare a poker
chain, where hiding cards *from your opponent* is the central problem.

What remains is hiding the game from **everyone else**: the chain, the network,
the server, the observer. That is a strictly easier problem and it is solved by
different tools (transport encryption, stealth addresses, value commitments).

### 2. A chess channel does not need Lightning's penalty machinery

Payment channels have a hard problem: an old state may be *more profitable* for
you (you had more money then), so the protocol needs revocation secrets, penalty
transactions, and watchtowers to stop you broadcasting the past.

Chess has no such problem. State is indexed by **ply**, which only increases, and
money moves **only on terminal states**. Posting an old position does not pay you;
it just rewinds the game, and your opponent holds signatures for every later
position and simply posts one. So the entire rule is:

> **Higher ply always wins.**

No revocation keys. No penalties. No asymmetric watchtowers. This is the single
biggest simplification in the design, and it exists because of a structural fact
about the game, not because of a clever trick.

### 3. Proving checkmate is expensive; refuting it is cheap

"This is checkmate" is a statement about *all* moves: for every legal move, the
king is still attacked. Verifying that on-chain means generating up to 218 moves
and testing each.

"This is not checkmate" is a statement about *one* move: here it is.

So we never verify mate. We let the claimant **assert** it, and give the
opponent a window to post a single escaping move. Honest claims cost almost
nothing; false claims are always refutable by the exact person harmed by them.

This ∀-is-expensive / ∃-is-cheap asymmetry is the general shape of every fraud
proof in the design.

---

## Layer stack

```
  L5   Clients            wallets, boards, engines, analysis
       ─────────────────────────────────────────────────────
  L4   Servers            matchmaking, tournaments, ratings, arenas
       (permissionless, bonded, non-custodial by default)
       ─────────────────────────────────────────────────────
  L3   Game channel       off-chain signed positions, P2P, encrypted
       ─────────────────────────────────────────────────────
  L2   Adjudicator        on-chain chess referee, disputes, timeouts
       ─────────────────────────────────────────────────────
  L1   Chain              accounts, state tree, blocks, consensus
       ─────────────────────────────────────────────────────
  L0   Primitives         hash, signature, encoding, transport
```

Traffic volume falls off a cliff going downward. A 40-move blitz game is ~80
signed states at L3 and **two transactions** at L1. This ratio is the whole
reason the system can be both private and cheap.

## Threat model

Who we defend against, and with what.

| # | Adversary | Wants | Defence |
|---|---|---|---|
| A1 | Opponent | Claim a win they didn't get | On-chain adjudicator; every position is countersigned |
| A2 | Opponent | Stall to avoid losing on time | Challenge window Δ; forfeit on expiry |
| A3 | Opponent | Rewind to a better position | Higher-ply-wins rule |
| A4 | Opponent | Grief you into slow on-chain play | Clock dilation; initiator pays gas; reputation at L4 |
| A5 | Opponent | Play a move that isn't legal | On-chain move verification during dispute |
| A6 | Server | Steal the stake | Servers hold signatures, not funds (T0/T1) |
| A7 | Server | Rig tournament pairings | VRF-derived pairings with published proofs |
| A8 | Server | Fake ratings | Ratings are signed attestations; anyone can recompute from public games |
| A9 | Validator | Censor your dispute tx and steal by timeout | Δ measured in **blocks, not seconds**; reserved dispute gas |
| A10 | Validator | Reorg away a settled dispute | BFT finality, not probabilistic finality |
| A11 | Observer | Learn who played whom for how much | Stealth addresses, value commitments (L8) |
| A12 | Cheater | Use an engine against humans | **Not solvable cryptographically.** Statistical detection + bonds + segregated pools |
| A13 | Sybil | Farm rewards with many identities | **Emit no rewards for playing.** Identity bonds at L4 |

A12 deserves emphasis: **no cryptography prevents a human from consulting
Stockfish**, because the human is outside the system. Anyone who tells you
otherwise is selling something. The design responds by *segregating* — arenas
where engines are expected and legal, and human pools defended statistically
and economically. See `06-economics.md`.

## Glossary

- **Ply** — one move by one player. A 40-move game is 80 plies. Our sequence number.
- **Position** — board + side to move + castling rights + en-passant file +
  halfmove clock. Everything needed to determine legality and repetition.
- **State** — a position plus channel id, ply, both clocks, and status.
- **Certified state** — a state signed by both players.
- **Channel** — one funded game between two keys.
- **Δ (delta)** — the challenge window, measured in blocks.
- **Adjudicator** — the on-chain chess referee.
- **Clock dilation** — the map from game-clock milliseconds to on-chain block budget.
- **Rake** — the fee taken from the pot at settlement.
