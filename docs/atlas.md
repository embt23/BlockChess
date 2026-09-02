# The Atlas

*The knowledge map for BlockChess — and the syllabus for the series.*

---

## The organising principle

> **The attack list is the curriculum.**

Every cryptographic primitive in this project exists because of one specific way
to steal money or cheat at chess. Nothing is introduced abstractly. This is the
structure of the series and it is also the structure that makes the material
stick: you remember *why* a Merkle tree exists if you first watched someone
forge a game history without one.

Each episode below is: **an attack**, then **the primitive that kills it**, then
**working code**, then **the mathematics underneath**.

---

## The spine

| # | The attack | The primitive | The mathematics | You build |
|---|---|---|---|---|
| 01 | "I moved first, actually" | hash functions, hash chains | preimage/collision resistance, Merkle–Damgård, birthday bound | SHA-256 from the spec |
| 02 | "That wasn't my move" | Ed25519 signatures | elliptic curve groups, Edwards form, discrete log, Schnorr linearity | sign & verify a move |
| 03 | "That move was legal, trust me" | rules as code | bitboards, perfect hashing (magic bitboards), `GF(2)⁶⁴` and Zobrist XOR | a move generator that passes `perft(6)` |
| 04 | "Here's a different game history" | Merkle & sparse Merkle trees | tree commitments, proofs of absence, log-size witnesses | the state tree |
| 05 | "My version of history is the real one" | proof-of-work | difficulty as a control loop, Poisson block arrival, the longest-chain rule | a toy PoW chain |
| 06 | "I'll reorg away your dispute" | BFT finality | FLP impossibility, partial synchrony, the ⅓ bound, quorum intersection | Tendermint-style consensus |
| 07 | "80 transactions per game is absurd" | state channels | off-chain state, hash-chained states, signature-as-evidence | the P2P move loop |
| 08 | "I'll just stop replying" | challenge windows | deadlines in blocks not seconds, clock dilation as a change of time base | the adjudicator |
| 09 | "That's checkmate (it isn't)" | optimistic claims & fraud proofs | ∀ vs ∃ asymmetry, refutation games, the "no new liveness assumption" test | mate claims + refutation |
| 10 | "I'll censor your dispute" | forced inclusion | reserved blockspace, censorship-resistance bounds | dispute gas reserve |
| 11 | "This bet is fair, honest" | handicap odds | Elo as a **Boltzmann distribution**, Bradley–Terry, log-odds, logistic regression as energy fitting | an odds calculator |
| 12 | "You can get rich playing even games" | Kelly criterion | `G = 1 − H₂(p)` **is channel capacity**; entropy; risk of ruin; concavity of log growth | a bankroll simulator |
| 13 | "I'm not using an engine" | sequential hypothesis testing | likelihood ratios, KL divergence, Wald's SPRT, detection time `≈ ln(1/α)/D_KL` | a cheat detector |
| 14 | "Your rating is 1200, honest" | Bayesian rating | Gaussian posteriors, Glicko-2's `RD`, TrueSkill factor graphs, Kalman analogy | a rating service |
| 15 | "I chose the tournament bracket" | VRFs | unique + verifiable + pseudorandom; grinding resistance vs plain commitments | verifiable pairings |
| 16 | "I can see every game you played" | stealth addresses | ECDH; `r(vG) = v(rG)` is the entire construction | unlinkable game addresses |
| 17 | "I can see how much you bet" | Pedersen commitments | homomorphic hiding, perfect-hiding vs perfect-binding **impossibility**, Bulletproofs range proofs | confidential stakes |
| 18 | "Disputes reveal my whole game" | SNARK settlement | arithmetisation, recursion, zkVMs proving your own `apply()` | one-proof settlement |

Eighteen episodes. Each is genuinely a different region of the map, and the
sequence has no gaps — you never need a concept you have not already built.

---

## Bridges from electrical engineering

You already have most of the intuition for the hard parts. These are the
translations.

| You know | It is also | Why it transfers |
|---|---|---|
| **Thermodynamic entropy** `S = k ln W` | **Shannon entropy** `H = −Σ p log p` | The same quantity in different units. Boltzmann's `k` and Shannon's `log 2` are unit conversions. This is the bridge into §12. |
| **Boltzmann factor** `p ∝ e^(−E/kT)` | **Elo ratings** | Literally the same formula. Rating is negative energy; `kT = 400/ln10 ≈ 173.7` Elo. See `spec/06-economics.md` §1. |
| **Clock domain crossing / metastability** | **Distributed consensus / FLP** | You cannot sample an async signal with zero metastability probability; you cannot achieve deterministic async consensus with a faulty node. Both escape by adding timing assumptions and driving failure probability below what matters. Synchroniser flops ↔ Tendermint timeouts. |
| **PID control, stability, loop gain** | **Difficulty adjustment, EIP-1559 fee market** | Bitcoin's retarget is a proportional controller with gain 1 and 2016-block sampling on a high-delay plant. Its oscillation under hashrate steps is a textbook stability problem. |
| **CRC, LFSRs, polynomial arithmetic over GF(2)** | **Finite fields in cryptography** | You have already done arithmetic in `GF(2)[x]/p(x)`. Elliptic curves are the same *kind* of object over `F_p` — a set with an operation that behaves like arithmetic. |
| **Nyquist rate, sampling** | **Block time** | Block time is the sampling rate of the world. Everything faster than it is aliased into one block, which is where MEV and transaction-ordering games come from. |
| **Channel capacity, coding theory** | **Kelly betting** | Not an analogy — the same equation. `1 − H₂(p)` is both the BSC capacity and your maximum bankroll growth rate in bits per game. |
| **Side channels, timing attacks** | **Cheat detection** | You catch a cheater by their *timing*, not their moves. Honest thought correlates with position complexity; an engine query does not. Same reasoning as extracting a key from power traces. |
| **Impedance matching / conservation laws** | **Homomorphic commitments** | `C₁ + C₂ = C(v₁+v₂)` lets you verify conservation of value without measuring the values, the way KCL constrains a node without knowing any individual current. |
| **State machines, Mealy/Moore diagrams** | **The whole blockchain** | A blockchain is a replicated deterministic state machine. You have been drawing these since first year. Consensus is only the problem of agreeing on the input sequence. |

The last one is the most useful and the least obvious: **a blockchain is a state
machine, and the interesting part is not the machine but the agreement on its
inputs.** Once you see it that way, most of the mystique evaporates.

---

## Prerequisite graph

```
                        ┌─────────────┐
                        │ Rust basics │
                        └──────┬──────┘
                 ┌─────────────┴─────────────┐
                 ▼                           ▼
        ┌────────────────┐          ┌─────────────────┐
        │ hash functions │          │  bitboards      │   ◀── independent!
        │  (ep 01)       │          │  movegen (ep03) │       parallel tracks
        └───────┬────────┘          └────────┬────────┘
                ▼                            │
        ┌────────────────┐                   │
        │  signatures    │                   │
        │  (ep 02)       │                   │
        └───────┬────────┘                   │
                ▼                            │
        ┌────────────────┐                   │
        │ Merkle / SMT   │                   │
        │  (ep 04)       │                   │
        └───────┬────────┘                   │
                ▼                            │
        ┌────────────────┐                   │
        │  consensus     │                   │
        │  (ep 05, 06)   │                   │
        └───────┬────────┘                   │
                ▼                            │
        ┌────────────────┐                   │
        │ state channel  │◀──────────────────┘
        │  (ep 07)       │      ← the two tracks MERGE here
        └───────┬────────┘
                ▼
        ┌────────────────┐
        │  adjudicator   │  ep 08, 09, 10   ← the hardest part of the project
        └───────┬────────┘
                ▼
        ┌────────────────┐
        │ servers, elo,  │  ep 11–15   ← mostly independent of each other
        │ VRF, detection │
        └───────┬────────┘
                ▼
        ┌────────────────┐
        │ privacy ladder │  ep 16–18   ← each independently deployable
        └────────────────┘
```

**The single most useful thing on this diagram:** the chess track and the crypto
track are completely independent until episode 7. You can build the move
generator while you are still confused about consensus, and vice versa. When one
gets frustrating, switch. That is not procrastination — it is the actual
dependency structure.

---

## Timeline

Assuming ~10–15 hours a week around a third-year EE course load. Ranges are
honest, not optimistic.

```
MONTH  1   2   3   4   5   6   7   8   9  10  11  12
       │   │   │   │   │   │   │   │   │   │   │   │
Rust   ███▓▓                                          learn by writing ep 01–03
       │
CHESS  ░░████████                                     movegen → perft(6) green
TRACK  │       ▲
       │       └── MILESTONE A: perft passes. The rules are correct.
       │
CRYPTO ░░░███████                                     hash, sigs, SMT
TRACK  │        ▲
       │        └── MILESTONE B: you can sign and verify a state
       │
CHAIN      ░░░░░████████                              PoW (1wk) → BFT
       │              ▲
       │              └── MILESTONE C: two nodes agree on a block
       │
CHANNEL            ░░░░░░████████                     ◀── TRACKS MERGE
       │                        ▲
       │                        └── MILESTONE D: a full game off-chain,
       │                            signed, settled cooperatively.
       │                            ** first playable **
       │
ADJUD.                     ░░░░░░████████             disputes, dilation, mate claims
       │                                ▲
       │                                └── MILESTONE E: you can win a game
       │                                    against an opponent who quits.
       │                                    ** first thing that is actually a
       │                                       trustless wagering system **
       │
SERVER                             ░░░░░░████████     registry, arena, VRF
       │                                        ▲
       │                                        └── MILESTONE F: bot arena
       │                                            running unattended
       │
ELO/                                        ░░░░█████ ratings, odds, detection
DETECT │
       │
PRIVACY                                          ░░░██  stealth addresses (L2)
```

### The branch points

Three places where the project can legitimately go different directions:

**Branch 1 — after Milestone A (month ~3).**
The chess engine is correct and standalone. You could stop and ship it as a
plain chess engine with a UI, gaining users and feedback before any blockchain
exists. *Recommended if* you want an audience early. *Skip if* you would rather
keep momentum on the protocol.

**Branch 2 — after Milestone D (month ~7).**
You have a working signed-state channel and cooperative settlement. Two paths:

- **Depth:** build the adjudicator (E) and make it genuinely trustless. Hard,
  and it is the intellectual core of the project. ▶ Recommended.
- **Breadth:** skip the adjudicator, ship a T3 custodial server, get real users
  playing months earlier, come back to trustlessness later. Faster to an
  audience, but you would be shipping the thing every existing platform already
  is, which undercuts the point.

**Branch 3 — after Milestone F (month ~10).**
The system works. Now choose what the project *is*:

- **Privacy** (episodes 16–18) — the mathematically richest path, and the best
  content. Ends at SNARK settlement.
- **Economics** (episodes 11–14) — ratings, markets, detection, tournaments.
  More product, less cryptography.
- **Decentralisation** — recruit validators, DHT discovery, make it genuinely
  permissionless in practice rather than in principle.

You do not have to pick one; you have to pick the *order*, and the order should
follow whichever you can currently explain most excitedly on camera.

### Realistic bottom line

- **Month 3:** a correct chess engine.
- **Month 7:** two people play a wagered game that settles on your own chain,
  as long as neither cheats.
- **Month 9–10:** the "as long as neither cheats" disappears. This is the real
  deliverable.
- **Month 12:** bots wagering against each other unattended, with ratings.

Milestone E is the project. Everything before it is prerequisites, everything
after it is expansion. If you are ever unsure what to work on, ask which task
most directly shortens the path to "I can win against an opponent who
disconnects".

---

## Reading list, by episode

| Episodes | Source |
|---|---|
| 01–02 | *Serious Cryptography*, Aumasson — chapters on hashing and signatures |
| 02 | Bernstein et al., "High-speed high-security signatures" (the Ed25519 paper) |
| 03 | Chess Programming Wiki — bitboards, magic bitboards, perft |
| 04 | Laurie & Kasper, "Revocation Transparency" (sparse Merkle trees) |
| 05–06 | Nakamoto 2008; Buchman, "Tendermint: Byzantine Fault Tolerance in the Age of Blockchains"; Fischer–Lynch–Paterson 1985 |
| 07–09 | Poon & Dryja, "The Bitcoin Lightning Network"; Coleman, Horne & Xuanji, "Counterfactual: Generalized State Channels" |
| 11 | Elo, *The Rating of Chessplayers*; Glickman, "Parameter estimation in large dynamic paired comparison experiments" |
| 12 | Kelly, "A New Interpretation of Information Rate" (1956); Cover & Thomas, *Elements of Information Theory*, ch. 6 |
| 13 | Regan & Haworth, "Intrinsic Chess Ratings"; Wald, *Sequential Analysis* |
| 15 | RFC 9381 (VRFs) |
| 16 | van Saberhagen, *CryptoNote v2.0* (stealth addresses) |
| 17 | Bünz et al., "Bulletproofs" |
| 18 | Thaler, *Proofs, Arguments, and Zero-Knowledge* (free online) |

Kelly's 1956 paper is nine pages, requires no background beyond logarithms, and
contains the single best idea in the entire project. Read it first, before
anything else on this list.
