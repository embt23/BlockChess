# 11 — Layers of transactable resource

Four kinds of thing move through this system, and they are four genuinely
different *kinds*. Conflating them is the commonest way these designs go wrong.

```
  L3  CLAIM   priority over a novelty      positional — exclusive in time, not in use
  L2  STYLE   a model of how you play      informational — copyable, which breaks markets
  L1  STAKE   what is at risk in a game    fungible — transferred
  L0  GAME    the right to play one game   consumable — burned
```

Each section lists the real options and marks a recommendation `▶`. None of
this is settled; it is the menu.

---

## L0 — GAME: the right to play

*"One token is one game you can play."*

The useful question is not what it is but **what makes it scarce.** A token
nobody has to work for is not a resource.

| | Option | For | Against |
|---|---|---|---|
| **A** ▶ | **Prepaid blockspace.** A game costs exactly two on-chain transactions (open, close). One GAME is prepaid gas for exactly one game, burned on open. | Prices a real cost honestly. Scarce because blockspace is. Reads as a utility/access instrument rather than a wager, which simplifies §7 of `07-servers.md`. Servers can buy them in bulk and hand them out — this is how free-to-play works. | A second asset to reason about |
| B | Free, rate-limited per identity | Simplest | Needs sybil resistance, which is the hard problem |
| C | Minted by servers against their bond | Servers control their own economy | Server-issued money; failure modes of T3 custody |
| D | No GAME token — pay gas in the stake asset | Fewest moving parts | No way to sponsor free play without giving people money |

▶ **A.** The decisive argument is sponsorship: a teaching server or a free
arena must be able to give someone a hundred games without giving them anything
of value. A separate consumable is the only clean way to do that.

**Invariant.** GAME is never minted by playing. Paying people to play means two
bots farming each other, every time, without exception (`06-economics.md` §7).

---

## L1 — STAKE: what is at risk

| | Option | For | Against |
|---|---|---|---|
| A | **Wager GAME itself** | One asset. And the loop is genuinely elegant: **your winnings are more games.** A strong player accumulates play-time; the economy is self-contained with no external value and no regulatory surface | Deflationary at the bottom — weak players get locked out and need a faucet. Conflates access with value |
| **B** ▶ | **A separate play-token, no cash value** | Every equation in `06-economics.md` is testable with worthless tokens; bots do not care what they are worth. Regulatorily inert | "Not real money" lowers the stakes, literally |
| C | External stablecoin | Real incentives, real adversaries | Requires the legal work in `07-servers.md` §Jurisdiction first |
| D | Multi-asset — channels are asset-agnostic, servers choose | Maximum flexibility | Every server becomes its own economy |

▶ **B for v1, with the channel built asset-agnostic (D) underneath**, so C is a
configuration change rather than a rewrite. Option A stays available and is
worth prototyping on one server precisely *because* "winnings are more games" is
a nice closed loop.

---

## L2 — STYLE: a model of how you play

The new layer. `10-personality.md` is the argument for why it is the valuable
one; this is what it has to be able to do.

### What is actually published

| | Option | For | Against |
|---|---|---|---|
| A | Raw policy weights | Simple, maximally useful | Copyable, resellable, and leaks specific games (membership inference) |
| **B** ▶ | **DP-noised model + provenance proof** | Bounded leakage about any single opponent; provable that it is really yours | Training cost; some accuracy lost to noise |
| C | Private weights, sell *queries* via FHE/MPC | Solves copyability — the buyer never holds the asset | Slow, and a research project |
| **D** ▶ | **A style *fingerprint*: summary statistics only** — opening repertoire distribution, aggression index, time-per-complexity curve, material-vs-initiative bias | Cheap, barely leaky, and probably 80% of the useful signal for matchmaking and prep | Not a playable model |

▶ **D first, then B.** The fingerprint is buildable now, is immediately useful
to matchmaking, and is the thing to measure `D(π_you ‖ π_pop)` on before
committing to anything larger (`10-personality.md` §10, risk 4).

### The three-part privacy stack

> *"How do you share how you play without showing any information encoding the
> match data of your opponent?"*

This is the sharpest question in the design and it deserves a precise answer.
The instinct to reach for homomorphic encryption is understandable but it is the
wrong tool for *this* problem — though it is the right tool for a different one,
below.

The reason it is wrong: **the question is about inference control, not access
control.** FHE answers *"who is allowed to compute on this?"* The question asked
is *"what does the output reveal about an input record?"* — and that is
differential privacy's entire subject.

| Need | Question it answers | Tool |
|---|---|---|
| **Provenance** | Is this model really built from games you actually played? | Signatures + ZK. Every certified state is already countersigned by the opponent (`04-channel.md`), so the training set can be proven to consist of games the claimant genuinely participated in. **The channel already emits exactly the evidence this needs.** |
| **Privacy** | Does publishing this leak any individual opponent's games? | **Differential privacy.** DP-SGD with per-opponent granularity: the published model is ε-indistinguishable whether or not any one opponent's games were in the training set |
| **Confidentiality** | Can someone use the model without extracting it? | **FHE / MPC.** This is where homomorphic encryption genuinely earns its place — see below |

### Why the leak is real but narrower than it looks

Chess is perfect information. Your opponent already saw every move of your game
together, and you saw theirs. A single game leaks nothing new to either party.

The leak is **purely through aggregation**: across many games, a model of you
encodes the distribution of positions your opponents steered toward. And a model
fitted on limited data is sharp where it has seen data and vague elsewhere — so
the *sharpness pattern itself* reveals which positions you encountered. That is
membership inference, and it is exactly what DP bounds.

### The rule, and the better reason for it

> **You may publish any function of your own decisions. You may not publish a
> model that predicts your opponents' decisions.**

The stated justification was privacy. There is a stronger one, and it is
economic:

> If opponents' styles were sellable, strong players become **prey** — farmable
> for data by anyone willing to lose a few games to them. The rule exists to
> stop the best players being mined.

That is a real incentive failure the rule prevents, and it holds even for people
who do not care about privacy.

### Where FHE belongs

Information is copyable; that is the defect of every information market. Sell a
model and the buyer can resell it. Three responses:

1. **Never sell the model — sell queries.** The buyer submits encrypted
   positions and receives encrypted move distributions. They get the use and
   never the asset. This is precisely what FHE is for, and it is the correct
   home for it in this project.
2. Subscription access with revocation.
3. Accept that it is a first-mover market and price accordingly.

▶ **1**, eventually. It is slow and hard, and it is the only one that actually
works.

---

## L3 — CLAIM: who played it first

> *"The first person to play in a certain area of chess should be able to own
> that area."*

### Pushing back on "own"

Ownership means exclusion, and exclusion here is both unenforceable and harmful.

- **Unenforceable.** You cannot stop anyone playing 1.e4. Any "ownership" would
  be a server-level social convention with no mechanism behind it.
- **Harmful.** Exclusive claims invite land-grabbing: bots enumerating openings
  to fence them off, with no games played and no value created.
- **Anti-thetical.** Chess theory advances by people playing each other's ideas.
  A system that makes that costly makes the game worse.

### The version that works: priority, not property

Not *"I own the Najdorf"* but *"I demonstrated this first, and the record proves
it."* Attribution rather than exclusion.

This is enforceable, because **timestamped commitment is the one thing a
blockchain is unambiguously for**. It is also positive-sum: citing your novelty
does not diminish it, it strengthens the claim.

The precedent is already how chess works. Opening novelties are informally
attributed — the Marshall Attack, the Botvinnik System — and mathematicians do
not own theorems but do get credit. Formalise that and you have a real system.

### The mechanism — and it uses a primitive already built

Define a **novelty** precisely: a `(position, move)` pair that appears in no
prior public game. That is exactly how chess already uses the term *TN,
theoretical novelty*.

The registry is a sparse Merkle tree keyed by `H(position ‖ move)`. To claim:

```
1.  a signed game record showing you reached `position` and played `move`
        — countersigned by your opponent, so it cost a real game

2.  a NON-INCLUSION proof against the registry root at an earlier block
        — proving nobody had played it before
```

**Episode 04 built exactly this.** A sparse Merkle tree's defining property is
that proving absence costs the same as proving presence (`spec/02-chain.md`,
`bc-merkle::smt`). "Prove nobody played this before" is a non-inclusion proof.
The primitive is already in the repository and already tested.

### Two properties that kill the spam

1. **Proof of play.** A claim requires a countersigned game. You cannot fence
   off a million lines without playing a million games against real opponents
   who agreed to it. The work is chess — which is proof-of-work in the only
   sense that ever mattered.
2. **Claims are free to make and worthless unless cited.** A novelty earns only
   when other games play into it. Enumerate junk lines all day; nobody will
   follow you into them and you will earn nothing. This is a citation economy,
   not a land registry, and citation economies are self-limiting.

| | Option | Note |
|---|---|---|
| A | No claim layer at all | Simplest; loses the idea |
| **B** ▶ | Attribution registry, proof-of-play, no exclusion | Enforceable, positive-sum, uses existing primitives |
| C | Exclusive rights *within a single server* | A server's own convention; harmless because opt-in |
| D | Royalty-bearing citations — novelties pay when played into | The full version of B; needs B working first |

---

## New server archetypes these layers create

Extending `07-servers.md`:

| Archetype | Tier | What it is |
|---|---|---|
| **Style exchange** | T2 | Hosts the fingerprint/model market; publishes provenance and DP parameters |
| **Novelty registry** | T1 | Runs the claim system; bonded, because equivocating on priority is slashable |
| **Prep server** | T0 | You buy access to a style and play a bot fitted to it. *"Play against a model of the person you face on Saturday."* Plausibly the most marketable thing in the project |
| **Glass house** | T4 | Already specified — and now doubly motivated: public games are both the anti-cheat corpus and the training set for `π_pop` |

The last row matters. `π_pop` — the population policy every divergence in
`10-personality.md` is measured against — has to be fitted on something. Public
game history is that something. **Transparency stops being a virtue and becomes
an input.**

---

## Open decisions

Recorded as D16–D20 in `09-open-questions.md`. The one to settle first is
cheapest and most informative: **measure `D(π_you ‖ π_pop)` on a real corpus**
before building anything on top of the estimate in `10-personality.md` §7.
