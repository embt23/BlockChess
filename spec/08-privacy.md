# 08 — The privacy ladder

## First, what privacy means here

Chess is a **perfect-information** game. Your opponent already sees everything
you see. So "encrypted games" cannot mean encrypted *from your opponent* — there
is nothing to hide from them. It means encrypted from:

- the chain,
- the network,
- the matchmaking server,
- anyone watching later.

This is a much easier problem than it first sounds, and it deletes a whole
research area from the project. A poker chain needs mental poker, verifiable
shuffles, and commitment schemes just to deal a hand. We need none of it.

Each rung below is independently deployable. Build them in order; each is a
self-contained piece of mathematics.

---

## L0 — Transport encryption *(v1, required)*

Noise `XX` over X25519 + ChaCha20-Poly1305 (`01-primitives.md`). Moves are
encrypted and authenticated in transit with forward secrecy.

**Hides:** move content and timing from the network.
**Does not hide:** that two IP addresses are exchanging small packets at
chess-like intervals. Traffic analysis defeats this trivially.

## L1 — Off-chain by construction *(v1, free)*

The chain sees `OpenGame` and `CloseGame`. It never sees a move. A 40-move game
leaves a two-transaction footprint.

**Hides:** every move of every game that settles cooperatively — which is
almost all of them.
**Does not hide:** who played whom, for how much, and who won. And a *disputed*
game reveals the position in plaintext.

This rung is the largest single privacy win in the project and it costs nothing,
because we wanted state channels anyway for cost and latency. That is worth
noticing: **the privacy came free with the scalability.** It usually does.

---

## L2 — Stealth addresses *(v2)*

Right now, `OpenGame` names `white_pk` and `black_pk` — persistent identities.
Every game you play is publicly linked to every other game you play.

Fix: each player publishes a **view key** `V = vG` and a **spend key** `S = sG`.
To fund a game against you, my client:

1. generates an ephemeral scalar `r`, publishes `R = rG`,
2. computes the shared secret `c = H(rV)` — note `rV = r(vG) = v(rG) = vR`,
3. derives the one-time address `P = S + cG`.

You scan for `R` values, compute `c = H(vR)` with your view key, check whether
`P = S + cG` matches, and if so you can spend it because you know
`s + c` and `P = (s+c)G`.

**This is Diffie–Hellman used as an addressing scheme.** The whole construction
is two applications of `r(vG) = v(rG)` — the commutativity of scalar
multiplication on an elliptic curve group. Nothing else. It is the cleanest
possible demonstration that group structure *is* the useful thing about elliptic
curves.

**Hides:** the link between your identity and your games. An observer sees a
game between two fresh addresses.
**Does not hide:** amounts, or the fact that the two addresses were funded from
somewhere.

## L3 — Confidential stakes *(v3)*

Replace the plaintext `stake` field with a **Pedersen commitment**:

```
C  =  v·G  +  b·H
```

where `v` is the amount, `b` is a random blinding factor, and `H` is a second
generator whose discrete log with respect to `G` is unknown (obtained by hashing
`G` to a curve point — "nothing up my sleeve").

Pedersen commitments are **additively homomorphic**:

```
C₁ + C₂  =  (v₁+v₂)·G  +  (b₁+b₂)·H
```

so the chain can verify `inputs = outputs` — that no money was created — by
checking that a sum of commitment points equals the identity, **without learning
any amount**. The verification is a statement about the group, and the group
does not know what the numbers mean.

You additionally need a **range proof** that `0 ≤ v < 2⁶⁴`, otherwise negative
amounts mint money by wrapping the field. Bulletproofs give this in
`O(log n)` size — about 700 bytes for a 64-bit range — with no trusted setup.

Pedersen commitments are **perfectly hiding** and **computationally binding**:
for any `v` there exists a `b` making `C` a valid commitment, so `C` reveals
literally zero information about `v` even to an unbounded adversary — but you
cannot open it two ways unless you can solve the discrete log of `H` base `G`.

> **You cannot have both perfect hiding and perfect binding.** One of the two
> must rest on a computational assumption. This is a theorem, not an engineering
> limitation, and it is one of the most useful facts in all of cryptography.

## L4 — Private disputes *(research)*

Today a dispute publishes the position. To avoid that, replace
`DisputeMove{move}` with a zero-knowledge proof of:

> "I know a move `m` such that `apply(P, m) = P′`, where `H(P)` and `H(P′)` are
> the committed position hashes, and `m` is legal."

The circuit is a chess move validator — expensive, but bounded and fixed. The
same machinery gives private mate refutation: "I know a legal move that escapes
check" without saying which.

## L5 — Succinct settlement *(the endgame)*

The final form: settle a game with **one recursive SNARK** proving

> "starting from committed position `P₀`, there exists a legal sequence of moves
> reaching terminal state `T`, and both players signed the endpoints."

The chain verifies one constant-size proof and pays out. No adjudicator, no
dispute state machine, no on-chain chess engine, no revealed moves.

The practical route is a zkVM (RISC Zero, SP1) running your existing Rust move
generator, rather than hand-writing a circuit. **The same `apply()` function from
`03-position.md` becomes the proven program** — which is a very strong argument
for the "write the chess rules exactly once" rule stated there.

---

## What is still not hidden, at any rung

Be honest about the residue:

- **Timing correlation.** Two addresses funded within seconds and settling
  together are probably the same game, even with stealth addresses.
- **The server knows.** A T0 matchmaking relay sees who asked to play whom. Fix
  requires anonymous credentials or onion routing — a separate project.
- **Network metadata.** Noise hides content, not the existence of a connection.
  Tor or a mixnet is the answer, and it is orthogonal to everything here.
- **Your opponent.** They know everything about the game and who you are. That
  is not a bug; it is chess.

---

## Recommended build order

| Rung | Effort | Value | When |
|---|---|---|---|
| L0 transport | days | high | v1, mandatory |
| L1 off-chain | free | very high | v1, structural |
| L2 stealth addresses | weeks | high | v2 — best effort/reward ratio |
| L3 confidential stakes | months | medium | v3 |
| L4 private disputes | research | low (disputes are rare) | later |
| L5 SNARK settlement | research | high, and beautiful | the finale |

L2 is the one to prioritise. It is a few hundred lines, it uses only the curve
arithmetic you already have, and it converts "every game you ever played is
public and linked" into "an observer sees unlinkable one-time addresses". Best
privacy-per-line-of-code in the whole ladder.
