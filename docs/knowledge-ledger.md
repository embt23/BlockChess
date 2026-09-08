# The Knowledge Ledger

*What is actually known, what is missing, and what to do about it.*

Companion to [`docs/atlas.md`](atlas.md). The atlas says what the project needs.
This file says what the builder currently has.

---

## Method, and why it matters

Every entry below was established by **being asked to reason**, not by
self-assessment against a checklist. That distinction is the point of this file.

The spec in `spec/` is a design document. A design document written *for* you
does not tell you what is in your head. The gap this ledger measures is the one
that matters for a project whose stated format is "each episode ships a tagged,
runnable commit" (`09-open-questions.md` D14): **which parts could be rebuilt,
defended, or taught on camera, and which parts would be recitation.**

### The finding that shaped this assessment

A first pass suggested large gaps in the electrical-engineering bridges the
atlas assumes. Probing showed most of those were **not** conceptual gaps. They
were missing names for owned concepts, or two owned concepts never introduced to
each other.

> Not knowing that `GF(2)[x]/p(x)` names the CRC you have already built is a
> five-minute problem. It looks identical, on a quiz, to not understanding CRCs.

Assessments are therefore graded on four tiers, and **the tier determines the
remedy**. Three of the four tiers do not call for study at all.

| Tier | Meaning | Remedy |
|---|---|---|
| **OWN** | Reconstructed under questioning | None. Build on it. |
| **NAME** | Concept held, term unknown | One sentence |
| **LINK** | Both halves held, never connected | One explanation |
| **GAP** | Genuinely absent | Real study — the only tier that earns a video |

Assessed 2026-09-03.

---

## OWN — verified by reconstruction

| Topic | Evidence | Where it lands |
|---|---|---|
| **Loop delay → oscillation** | Described late correction, overshoot, growing amplitude — marginal stability without the vocabulary | `02-chain.md`, difficulty adjustment as a control loop; atlas ep 05 |
| **Nakamoto / longest chain** | Most support → highest probability of extension → propagate early | atlas ep 05 |
| **Ply rule** | "Longest game including the opponent's countersignatures wins, to stop replay of an older state" | `04-channel.md` |
| **Reorg defeats a deadline** | *"They lost because of delay, not because they answered late"* — derived from a one-line definition of the challenge window | `02-chain.md`: money under a deadline requires deterministic finality. The entire BFT argument. |
| **Chain halt vs. wall-clock windows** | *"Even if everyone answered in time, they couldn't get it on-chain, and all games settle with players unable to do anything"* — derived unaided | `02-chain.md` §Censorship, defence 1. Why Δ is in blocks. |
| **The stalling hole** | Identified that stalling converts a lost-on-time position into free time | `05-adjudication.md`, the problem clock dilation exists to solve |
| **Escrow / refutation delay** | *"The claimer shouldn't be able to use the money until the other player has had time to refute"* | The challenge window, derived from scratch |
| **False claims must cost** | Proposed a penalty for false claims unprompted | `FALSE_CLAIM_PENALTY` — an independently derived design decision |
| **Shannon entropy** | Confirmed solid | atlas ep 12 |
| **Boltzmann entropy** | Recalled from earlier coursework | atlas ep 11 |
| **Shift registers, CRC, HDL** | A year of HDL; CRC read and understood on sight | atlas ep 03 (bitboards), ep 01 |
| **Nyquist / aliasing** | Self-reported solid | Block time as the sampling rate of the system |

**Three of these — reorg-vs-deadline, chain-halt, and the stalling hole — were
reconstructed from first principles with no prior exposure.** They are the
non-obvious arguments in the spec. That is the strongest signal in this
document and it should calibrate how the remaining gaps are read: this is a
naming and exposure problem, not a reasoning problem.

## NAME — the concept is held; only the term was missing

Cost: one sentence each. Recorded so they are never mistaken for study items.

| Term | What it actually is |
|---|---|
| **MTBF** | Mean time between failures. That is the whole content of the term. |
| **`GF(2)[x]/p(x)`** | Polynomial division with one-bit coefficients where addition is XOR — i.e. the CRC already understood. Algebra notation for a built object. |
| **Challenge window** | "Respond within Δ blocks or forfeit." A deadline in block height. |
| **Diffie–Hellman notation** | Concept held; notation was not. `G` is a fixed curve point; `vG` is `G` added to itself `v` times. `r(vG) = v(rG)` says scaling twice ignores order — the same commutativity that makes DH work. Stealth addresses (`08-privacy.md` L2) are DH used for addressing, so **atlas ep 16 is a notation exercise, not a new concept.** |

## LINK — both halves held, never connected

| Connection | Status |
|---|---|
| `S = k ln W` ↔ `H = −Σ p log p` | Both known, from different years, never joined. One explanation. |
| Elo ↔ Boltzmann distribution | Follows immediately from the above. `06-economics.md` §1. |

Consequence: the atlas's central pedagogical bet — that thermodynamics is the
on-ramp to information theory and thence to Elo and Kelly — **is sound.** Both
endpoints exist. Only the bridge is missing.

## GAP — genuine, needs real study

| # | Gap | Notes |
|---|---|---|
| G1 | **Rust** | From zero. The critical-path item. |
| G2 | **Chess engine: bitboards, magic bitboards, perft** | Paper only. Nothing has run. |
| G3 | **Kelly: why maximise log of bankroll** | Confirmed new. Non-obvious; widely misunderstood. atlas ep 12. |
| G4 | **The "no new liveness requirement" test** | Live misconception — see below. atlas ep 09. |
| G5 | **Metastability / clock domain crossing** | Open probe, unresolved at time of writing. The bridge to FLP. |

---

## Misconceptions caught and corrected

Recorded because a corrected misconception is more durable than a fact that was
never wrong — and because these will resurface on camera if they are not written
down.

### M1 — Why the ply rule needs no revocation machinery

*Believed:* because chess is a game of perfect information.

*Actual:* because **money only moves on terminal states.** In a payment channel
an old state is genuinely more profitable — you held a larger balance then — so
Lightning needs revocation secrets and penalties. Here every non-terminal state
pays nobody, so posting the past achieves nothing except handing your opponent a
free chance to post a higher ply.

The perfect-information fact is true and load-bearing, but it does different
work: it is why this project needs no mental poker or verifiable shuffling
(`00-overview.md`, reframing 1). A true fact attached to the wrong conclusion.

### M2 — Whether a vanished player needs an extra penalty

*Believed:* a player who never returns should be penalised, otherwise a false
mate claim steals from them.

*Actual:* they were **already** going to lose. A player who cannot post a
refutation within Δ equally cannot post a legal move within Δ, and the timeout
rule takes the game regardless. The optimistic mate claim requires them to be
online only at a moment they were already required to be online, so it is free
and needs no additional penalty.

The general test, and the thing to actually learn (G4):

> **Does this mechanism require the honest party to be online at a time they
> were not already required to be online?** If no, the optimism costs nothing.

The reach for a penalty came from not yet separating *already required* from
*newly required*. That distinction is the core of atlas ep 09.

### M3 — There are no smart contracts in this project

*Believed:* the agreed game terms are a smart contract.

*Actual:* the structure described is right — terms fixed at open, penalties
specified upfront — and it is `GameTerms`, signed by both players. But
`09-open-questions.md` D3 rejects a general-purpose VM. No Solidity, no deployed
contracts, no gas metering of arbitrary code. The adjudicator is native Rust
compiled into the chain.

This matters for study planning: **"blockchain and smart contracts" was named as
the highest-priority topic, and this project deliberately contains none of the
second half.** See Q1 in Open Questions.

### M4 — Vocabulary: "server" does not mean the chain

"Server" was used three times to mean the chain or the network. In this design
a **server** is a specific L4 actor — matchmaker, rating authority, arena — that
is permissionless, unprivileged by consensus, and in the default configuration
never touches funds (`07-servers.md`). The chain is the chain. The overload will
cause real confusion at the server layer; worth correcting the habit early.

---

## HDL → Rust: what transfers

Asked directly, answered honestly, because it changes the sequencing.

**Transfers:**
- Bit manipulation — masks, shifts, wraparound. The knight-attack code in
  `03-position.md` is shift-and-mask and should read almost like HDL.
- Fixed-width integer thinking (`u64` ↔ `logic[63:0]`).
- Determinism, no hidden state, no surprise allocation — the exact discipline
  `apply()` demands.
- Finite state machines. The channel lifecycle and the dispute state machine are
  literal FSMs.

**Does not transfer:**
- HDL is spatial and concurrent; you describe structure existing all at once.
  Rust is sequential and imperative.
- **Ownership and borrowing have no HDL analogue.** This is the month-one wall
  and nothing in the background shortens it.

**Net:** the HDL year makes the chess track (G2) meaningfully easier than
average and does nothing for the Rust curve (G1). This supports the atlas claim
that the two tracks are independent until ep 07 — and suggests **starting on the
chess track**, where existing skill applies, while Rust is still painful.

---

## What this implies for the atlas

1. **The EE bridges hold.** The atlas bets heavily on thermodynamics →
   information theory → Elo/Kelly. Both endpoints are confirmed present. The
   bridge is a LINK, not a GAP. That bet is safe.
2. **The reasoning is ahead of the vocabulary.** Three non-obvious spec
   arguments were reconstructed cold. Study should target named gaps, not
   general foundations, and should not be paced as if starting from zero.
3. **G1 and G2 are the entire critical path to Milestone A.** Everything else
   in this ledger is a sentence, an explanation, or a single concept.
4. **The atlas timeline is not threatened.** It allots months 1–2 to Rust and
   month 3 to perft. Zero Rust is already what that assumed.

---

## Open questions

**Q1 — Is "smart contracts" still a study goal?** It was named the top-priority
topic, and D3 removes it from this project entirely. Two defensible answers:
drop it and go deeper on this chain's own layers, or learn it separately as
ecosystem context. It should be a decision, not an accident.

**Q2 — G5, metastability.** Unresolved. Determines whether the CDC → FLP bridge
in the atlas is available or needs building from scratch.

---

## Watch list

**Pending.** Deliberately not yet written.

Videos are assigned only against **GAP** entries — G1–G5, minus whatever G5
resolves to. NAME and LINK entries are answered in a sentence, and sending
someone to watch an hour of video on a concept they already hold is the specific
waste this ledger exists to prevent.

Note that the atlas reading list is entirely papers and books. There is no video
column anywhere in this repository yet; it is being built from scratch against
the confirmed gaps above.
