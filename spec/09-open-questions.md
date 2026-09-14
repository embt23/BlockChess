# 09 — Decisions

Every decision that meaningfully forks the project. `▶` marks my recommendation
and the reasoning. Decisions marked **SETTLED** are already made.

**Settled 2026-09-14** in one pass, and recorded here rather than left in a
conversation: D11, D13, D14, D16, D17, D19, D20's threshold, the five new
decisions D21–D25 that episode 08 forced and this file did not previously
contain, and D26 on authorship. Everything with a `▶` and no **SETTLED** marker is still a
recommendation, not a decision.

---

## D1 — Own chain, or contracts on an existing chain? **SETTLED: own chain**

Consequences accepted: no external liquidity, no inherited security, validators
must be recruited, months rather than weeks to first real game. Consequences
gained: every layer is yours to explain, and the build *is* the curriculum.

---

## D2 — Implementation language

| Option | For | Against |
|---|---|---|
| **Rust** ▶ | the entire chain/crypto ecosystem is Rust; compiler catches the class of bug that historically stalls you; `u64` bit manipulation with no runtime magic; Cargo removes build-system pain; the zkVM path (L5) requires it | steep first month |
| Go | genuinely easy; Tendermint and geth are Go | GC and interfaces put a layer between you and the machine — the thing you said you dislike about Python |
| C++ | maximum control | you have already lost months to a missing semicolon in C++; the build systems are worse than the language |
| Python | fastest prototyping | you have said it feels like cheating, and you are right that it hides exactly the layer this project is about |

▶ **Rust.** The specific argument: your stated failure mode is *silent errors
that cost weeks*. Rust's entire design philosophy is converting silent runtime
errors into loud compile-time ones. `cargo check` is a tutor that never gets
bored. And bitboards want `u64` with explicit wrapping semantics, which Rust
gives you and Python does not.

Mitigation for the learning curve: **structure the repo as many small crates.**
No file over ~300 lines. You said you get overwhelmed around 100 lines of
unfamiliar code — so make sure no file requires holding more than that in your
head at once. This is good engineering independently of your comfort.

---

## D3 — Does the chain have a general-purpose VM?

| Option | For | Against |
|---|---|---|
| **Application-specific, no VM** ▶ | the adjudicator is native Rust; no gas metering for arbitrary code; no VM to write; vastly smaller attack surface | nobody can extend the chain without a hard fork |
| EVM / WASM VM | anyone can deploy contracts; extensible | you are now writing a VM, a gas schedule, and a compiler target — that is a separate multi-year project |

▶ **Application-specific.** You are building a chess settlement chain, not a
world computer. Extensibility lives at the *server* layer, which is
permissionless already. This decision alone removes about a year of work.

---

## D4 — Consensus: build PoW first, or go straight to BFT?

▶ **Build PoW first, deliberately, as a throwaway.** ~300 lines, one episode,
and it makes the BFT episode land because you can point at a concrete thing that
breaks. The argument in `02-chain.md` ("money under a deadline requires
deterministic finality") is abstract until you have watched your own chain reorg
away a dispute resolution.

Budget: one week for PoW, three to four weeks for Tendermint-style BFT.

---

## D5 — Does the token have monetary value?

| Option | For | Against |
|---|---|---|
| **Play-token testnet first** ▶ | protocol is identical; no regulated activity; you can publish everything and invite anyone; no legal spend before you know the design is right | "not real money" reduces the stakes and some of the drama |
| Real value from launch | real incentives, real adversaries, real test | real-money skill wagering is regulated per-jurisdiction; getting this wrong is not a technical problem you can patch |

▶ **Play-token first, and say so loudly.** Every piece of mathematics in
`06-economics.md` is testable with valueless tokens; bots do not care whether
the tokens are worth anything. Revisit only after the protocol is stable, and
with actual legal advice at that point rather than before.

---

## D6 — Which server archetype ships first?

▶ **The bot arena.** Reasons, in order of importance:

1. **No anti-cheat problem.** Engines are the intended participants, so A12 —
   the one adversary cryptography cannot touch — simply does not apply.
2. **Edges are large.** Per `06-economics.md` §4, near-even wagering has almost
   zero channel capacity. Two engines can differ by 300 Elo, where the maths
   actually produces visible results in tens of games rather than thousands.
3. **It runs unattended.** You get thousands of real games through the protocol
   overnight, which is exactly the load testing a channel protocol needs.
4. **Its users are the people who will contribute.**

The friends-lobby and teaching servers are easy follow-ons. The public
free-for-all with human money is *last*, because it is the one with the
unsolved problem in it.

---

## D7 — Equal stakes only, or handicap odds in v1?

▶ **Equal stakes in v1.** Handicap odds require ratings, ratings require a
trusted server, and that imports a trust dependency into the base layer's most
sensitive path. Ship equal-stake channels first (`stake_white == stake_black`
enforced by clients, not consensus), then enable asymmetric stakes once the
rating layer exists and has been attacked a bit. The protocol already supports
asymmetry; this is a client-side policy decision you can reverse.

---

## D8 — Persistent identities or stealth addresses from day one?

▶ **Persistent in v1, stealth in v2.** Stealth addresses (`08-privacy.md` L2)
are the highest-value privacy rung, but they complicate every debugging session
— you cannot easily look at the chain and see what happened. Build the system
legible, then make it private. Do not build it private and then try to debug it.

---

## D9 — Peer discovery

| Option | For | Against |
|---|---|---|
| **Server registry on-chain** ▶ | already needed for bonds; simple; servers are the natural rendezvous | servers are a discovery chokepoint |
| Kademlia DHT | fully decentralised | a whole subsystem, and NAT traversal is genuinely miserable |
| Both | complete | more work |

▶ **Registry first.** Players find servers on-chain; servers introduce players
to each other; the actual game connection is direct P2P (libp2p, with the server
as a fallback relay for NAT'd peers). Add a DHT only if servers become a real
censorship problem in practice.

---

## D10 — Where does rake go?

| Option | Effect |
|---|---|
| **To the named server** ▶ | funds the people doing the work; creates a real business; competitive pressure drives rake down |
| Burned | deflationary; but nobody is paid to run servers |
| To validators | validators already earn issuance; double-dipping |

▶ **To the server, burned if none.** Consensus caps it at 500 bps. Expect
competition to push real rates to 50–100 bps, because `06-economics.md` shows
players can compute exactly what a rake costs them in Elo — which is a rare case
where the market has perfect information about the fee.

---

## D11 — Upgrades and rule versioning

`GameTerms.adjudicator_ver` pins a channel to a specific version of the chess
rules at open time. A consensus upgrade that changes `apply()` must keep old
versions available for channels already open, or it retroactively changes who
won games in flight.

▶ **Keep every adjudicator version forever.** They are small and the alternative
is stealing money from someone whose game was open across the fork. Decide the
governance mechanism (validator vote? a foundation key? social consensus?) later
— but write the versioning in from day one, because retrofitting it is painful.

### Who may ship one — **SETTLED: a named key, timelocked, with a published handover condition**

The question was left open here as political rather than technical. It is
answered, and the answer is written down in public because that is the point of
asking it.

**What a version is (D25).** `adjudicator_ver` is a small integer on the wire;
consensus holds a registry mapping each integer to the hash of the ruleset it
denotes. The integer stays readable in specs and in negotiation; the hash makes
"keep every version forever" a checkable claim rather than a policy. The
registry is therefore the *only* object an upgrade authority controls.

**Who may write to it.** One published key, with the effect delayed by a
timelock counted in blocks (`P4`), so anyone who dislikes a pending change can
close their channels before it lands. That key is Evan's. The README says so in
those words, alongside the condition that moves it to validator control — an
independent validator count and a chain carrying value rather than play tokens.

**Why a key rather than a vote, for now.** A 2/3 validator vote on a recruited
validator set is a mechanism that describes three friends while sounding like a
constitution. The honest description is preferred, and it is revisable.

**Why this is lower-stakes than it looks.** Because every version is kept
forever, an upgrade is *purely additive*: a channel pinned to version 3 is
adjudicated by version 3 no matter what is registered afterwards. Whoever holds
the key cannot reach into an open channel, cannot change who won a finished
game, and cannot strand money. The realistic abuse is not theft but **refusal**
— declining to register someone else's fix. That is a much smaller thing to
guard against, and the timelock plus the public handover condition are
proportionate to it.

There is a second reason the stakes are low and it is specific to this project:
the state transition function models FIDE's rules, which have been externally
stable for over a century. Nearly every upgrade this chain will ever ship is a
bugfix, not a policy change.

---

## D12 — Parameter values

These are guesses that need empirical tuning once games are running:

| Parameter | Proposed | How to tune |
|---|---|---|
| `BLOCK_TIME_MS` | 2000 | validator geography; lower is better until propagation fails |
| `DELTA_BLOCKS` | 256 | must exceed worst realistic censorship; measure on testnet |
| `TAU_MS` | 50 | dilation constant — the key number in `05-adjudication.md`; play disputed games and see if the clock still *feels* like it has meaning |
| `MIN_MOVE_BLOCKS` | 8 | must exceed p99 tx inclusion latency |
| `MAX_BUDGET` | 5400 | how long is a dispute allowed to take? |
| `MAX_PLIES` | 600 | bounds worst-case validator cost |
| `DISPUTE_GAS_RESERVE` | 25% | raise if disputes are ever crowded out |

▶ Instrument all of these from the first testnet and publish the distributions.
"How I chose Δ by measuring my own chain" is a better episode than any amount of
reasoning about it in advance.

---

## D13 — Licence

**SETTLED: Apache 2.0 for code, CC BY 4.0 for prose.** `LICENSE`,
`LICENSE-DOCS` and `NOTICE` are in the repository.

This was not a formality. The repository was already public with **no licence
file at all**, which is not permissive by default but the opposite: all rights
reserved, no legal right for anyone to fork it, run it, or contribute to it.

**Apache 2.0** for everything in `crates/` — permissive, and it carries the
explicit patent grant MIT lacks, which matters more than usual here because
clock dilation and the refutation game are novel enough for someone else to
attempt to patent.

**CC BY 4.0** — attribution, *not* share-alike — for `spec/`, `docs/`, and the
root Markdown. This amends the original recommendation of CC BY-SA. The reason
is D14's corollary: **the spec files are the scripts for the series**, so the
most important works derived from this prose are videos, and share-alike would
hang an unresolved copyleft question over them. Attribution keeps the credit
requirement and drops the exposure.

**AGPL was considered and rejected.** It is the tool for forcing servers to
publish their modifications, and it cuts directly against D6: the first server
archetype is the bot arena, and the anti-cheat corpus in `06-economics.md` is
only worth anything if many servers exist.

---

## D14 — Series format

Not a technical decision, but it shapes the code.

▶ **Each episode ships a tagged, runnable commit.** The repository history
becomes the syllabus: `git checkout ep03` gives a viewer a working chess move
generator and nothing else. This forces the layering to be real — you cannot
tag a commit that only works because of code from three episodes later — and it
is the single biggest thing that separates a series people *follow along with*
from one they merely watch.

Corollary: **write the episode's spec section before the code.** These files are
the scripts.

### How this is actually done — **SETTLED: curated episode branches, CI-verified**

The history cannot deliver the promise retroactively. `d8fb3e6` is one commit
titled *"episodes 01-03: hashing, signatures, and the rules of chess"*, so a
state containing hashing and nothing else never existed to be tagged.

So the checkpoints are **not** historical, and are labelled as such. Each
episode gets a branch built from the current tree containing only the crates
that episode needs: `ep01` is `bc-hash` alone, `ep02` adds `bc-sig`, and so on.

The crates are already independent, so each of these branches genuinely builds
and tests green on its own — and **a CI job proves it does**. That proof is the
real content of D14's claim that you cannot check in a commit which only works
because of code from three episodes later. Tagging the existing history would
have asserted the same thing without testing it.

Cost accepted: each episode branch needs refreshing when a crate beneath it
changes, and CI is what catches the omission.

---

## D15 — Sparse Merkle tree: compress paths now, or later?

**Measured problem.** The straightforward sparse Merkle tree stores ~246 nodes
per key at depth 256 — roughly `depth − log₂(n)`. Random keys diverge within
about `log₂(n)` levels of the root, and below that each key owns a private chain
of single-child nodes down to its leaf.

```
    n=10       2,539 nodes   253.9 / key
    n=100     25,053 nodes   250.5 / key
    n=1,000  247,154 nodes   247.2 / key
```

At ~64 bytes a node: ~16 MB for a thousand accounts, ~15 GB for a million.

**The fix.** Path compression — store only nodes where the tree actually
branches, and compute the single-child chains on demand. A chain node's hash is
fully determined by the leaf beneath it plus the known empty hashes, so it can
always be recomputed.

**Why this is a safe thing to defer.** The compressed tree produces **the same
root hashes and the same proofs**. It is a pure internal storage optimisation,
invisible from outside the crate, so adding it later invalidates nothing already
committed and requires no migration of any signed or on-chain data.

That is unusual and worth noticing: most scaling problems in a blockchain are
consensus-visible and therefore must be got right before launch. This one is
not, because the *definition* of the tree and its *representation* were kept
separate.

| Option | For | Against |
|---|---|---|
| **Defer to after consensus works** ▶ | testnet scale is fine at 16 MB; the naive tree is already correct and tested, and makes an ideal oracle for the compressed one later | a million-account chain would need ~15 GB |
| Do it now | done once, properly | ~1.5× the crate's size, and the chain it serves does not exist yet |

▶ **Defer** — but keep the naive implementation and its dense-tree test as the
oracle for the compressed version when it lands. Building the obvious thing
first and then optimising against it as a reference is the same pattern used for
proof-of-work before BFT (D4).

---

# Act II decisions — the personality layer

Arising from `10-personality.md` and `11-resources.md`. None are urgent; all
should be settled before any code in this area is written.

## D16 — What makes a GAME token scarce? **SETTLED: prepaid blockspace**

▶ **Prepaid blockspace.** A game costs exactly two on-chain transactions, so one
GAME is prepaid gas for exactly one game, burned on open. The decisive argument
is sponsorship: a teaching server must be able to give someone a hundred games
without giving them anything of value, and a separate consumable is the only
clean way. Alternatives and their costs are in `11-resources.md` L0.

Subject to `E1` regardless of choice: GAME is never minted by playing.

## D17 — What is wagered? **SETTLED: a separate play-token**

▶ **A separate play-token with no cash value for v1, with the channel built
asset-agnostic underneath**, so moving to real value later is configuration
rather than a rewrite.

Worth prototyping on one server: wagering GAME itself, so that **your winnings
are more games**. Self-contained, no external value, no regulatory surface, and
the loop is genuinely elegant — a strong player accumulates play-time. Needs a
faucet or weak players get locked out.

**Settled: two tokens, and the single-token loop stays a server experiment.**

The single-token loop has one property worth recording, because it is a better
argument than the one originally made for it. If GAME is burned on open *and*
wagered, two bots playing each other are **strictly negative-sum**: the pot is
conserved but the burn is not. Bot-farming stops being prohibited by `E1` and
becomes unprofitable by construction — a sink instead of a rule, which is always
the stronger defence.

It was still not chosen for the base layer, for the symmetric reason. A player
who keeps losing eventually cannot play at all, so the loop requires a faucet,
and a faucet is a minting path that hands `E1` straight back to a sybil farm.
Trading a structural defence for a structural hole is not an improvement.

So: GAME is the consumable, a valueless play-token is the stake, and the channel
stays asset-agnostic underneath so real value is later a configuration change
rather than a rewrite. The GAME-wagered loop is tried on one server, where it
can fail without taking the protocol with it.

## D18 — How is style published?

▶ **Fingerprint first, model later.** Summary statistics — opening repertoire
distribution, aggression index, time-per-complexity curve, material-versus-
initiative bias — are cheap, barely leaky, and probably carry most of the useful
signal for matchmaking and preparation.

The full policy model needs the three-part stack (provenance by signature and
ZK, privacy by differential privacy, confidentiality by FHE-gated queries) set
out in `11-resources.md` L2. Note that **differential privacy, not homomorphic
encryption, is the tool for the stated problem** — the question is what the
output reveals, not who may compute on it. FHE's real home is selling queries
without selling weights.

## D19 — Is there a claim layer at all?

▶ **Yes, as attribution — never exclusion** (`E3`). A novelty is a
`(position, move)` pair in no prior public game; claiming it requires a
countersigned game record plus a Merkle non-inclusion proof against an earlier
registry root.

Both spam defences are structural rather than administrative: a claim costs a
real game against a real opponent, and claims earn nothing unless other games
play into them. Citation economy, not land registry.

**SETTLED: credit is the whole reward. No royalties.** The sub-question about
royalties (`11-resources.md` L3 option D) is closed, not deferred.

This is the one decision in this file that was **not** settled by argument. It
was marked **DISPUTED** in `docs/duality.md` — the analysis had pushed for
attribution and the intuition had not answered — and `E3` had meanwhile been
promoted to an invariant in `CLAUDE.md`, which meant one side of an openly
disputed question was being enforced as law while the other side's author had
not spoken. That is exactly the failure mode `docs/duality.md` exists to catch:
*the analytic voice writes the documents.*

Evan has now signed attribution, in his own name, having been shown the
alternatives — including a server-layer exclusion option and a compulsory-licence
royalty that would have honoured the original intuition. The ledger entry moves
to **CERTIFIED** because it was signed, not because the argument was loud, and
`E3` is legitimate law rather than a presumption.

## D20 — Measure the divergence before building on it ▶ **do this first**

`10-personality.md` §7 estimates `D(π_you ‖ π_pop) ≈ 0.02–0.10` nats per move
and concludes a game emits roughly four orders of magnitude more information
about the players than about the result. The conclusion is robust to large
errors in that estimate, but the estimate is still an estimate.

Measuring it is cheap: fit a population move model on a public game corpus, fit
per-player models on heavy users, and compute the cross-entropy difference. It
is a weekend, it needs no protocol, and it either grounds the entire Act II
thesis in a real number or kills it early.

**This is the highest-value-per-hour task in the project and it can be done
today, independently of everything else.**

### **SETTLED: do it now, minimally — and register the kill threshold first**

**The hole in the plan as previously written.** `10-personality.md` §7 says the
conclusion is "robust to large errors in that estimate." This file says the
measurement can *kill* Act II. Both cannot be true unless a number is named in
advance. Without a pre-registered threshold the result gets rationalised
whichever way it lands, and a measurement that cannot falsify anything is not
worth the weekend.

**Minimal, not full.** `π_pop` is **not** trained here — that is episode 19 and
it is weeks. Use a published, rating-conditioned human-move-prediction model as
the baseline and the Lichess open database for per-player histories. The
baseline is then someone else's published artefact rather than one of ours,
which is the `G1` posture. It lives in a separate research directory or
repository: no Python enters the Rust workspace.

**The threshold — proposed, and it needs Evan's signature before the
measurement runs, not after.** Working from `13`'s detection time
`≈ ln(1/α)/D_KL`, at α = 0.001:

| Measured `D(π_you ‖ π_pop)` | Detection time | Verdict |
|---|---|---|
| ≥ 0.02 nats/move (the estimate) | ~9 games | thesis holds as written |
| 0.005 – 0.02 | ~35–140 games | **amber** — Act II survives, every timescale in `10` and `13` is wrong and must be rewritten |
| < 0.005 nats/move | > 140 games | **Act II is dead.** Under 1 bit of identity per game; the cheat detector and the style asset both need hundreds of games to say anything, and neither is a product |

Rationale for the cut at 0.005: it is 4× below the low end of the existing
0.02–0.10 estimate, so it cannot be tripped by the estimate merely being
optimistic — only by it being wrong in kind.

---

# Episode 08 decisions

Five decisions that block the adjudicator's first line of code. None of them
were in this file before 2026-09-14, because all five only become visible when
`05-adjudication.md` is read against the actual crate layout rather than on its
own. All five are **SETTLED**.

---

## D21 — What does episode 08 run on? **SETTLED: PoW first, then adjudicate**

Episode 07 was built against `bc-channel::ledger`, a stub escrow, and that was
right: nothing in the channel cared whether the money was real. Episode 08 is
different in one specific way. Every deadline in `05-adjudication.md` is
`current_height + Δ`, and `P4` counts windows in blocks precisely so that a
halted chain cannot expire anyone's window.

| Option | For | Against |
|---|---|---|
| **PoW first, then adjudicate** ▶ | D4 already budgets the week; Milestone E becomes demonstrable rather than simulated; you get to watch a reorg eat a dispute, which is the argument episode 06 needs | one week before episode 08 starts |
| Stub the height oracle | fastest to a correct adjudicator; dispute logic is pure, so a driven counter tests it *better* than real blocks | Milestone E is claimed, not shown; episode 10 (censorship) stays unbuildable |
| Full consensus first (05 + 06) | the atlas's original dependency order | 4–5 weeks, and you would design BFT before knowing what the adjudicator demands of it |

The decisive argument is what Milestone E actually says: *"you can win against an
opponent who disconnects."* Against a stub height counter that sentence has not
been earned — it has been simulated. D4's throwaway week is spent before the
adjudicator, not after.

---

## D22 — Δ and τ: constants or negotiable? **SETTLED: a time-control class table**

`05-adjudication.md` marks two of its seven parameters as per-channel:
`DELTA_BLOCKS` and `TAU_MS` carry `(GameTerms)`. The other five —
`MIN_MOVE_BLOCKS`, `FLOOR_BLOCKS`, `MAX_BUDGET`, `MAX_PLIES`,
`FALSE_CLAIM_PENALTY` — bound what a validator must be able to afford or brake
griefing, and stay global. So this decision is about Δ and τ only.

**The attack.** Free-form negotiable Δ is Lightning's `to_self_delay` problem
imported wholesale: an opponent proposes Δ = 1 at open, a client that does not
check accepts it, and the victim now has two seconds to post a move on-chain or
forfeit the pot.

**Why consensus-enforced bounds do not fix it.** A single `[min, max]` band wide
enough to serve both bullet and correspondence is, by construction, wide enough
to contain a hostile value for either. The band cannot be tight and general at
the same time.

▶ **`GameTerms` carries a time-control class — Bullet / Blitz / Rapid /
Classical / Correspondence — and consensus holds one `(Δ, τ)` pair per class.**
No free-form number ever crosses the wire, so there is no hostile value to
propose, and a correspondence game still gets a correspondence-sized window.

This is consensus-visible, so unlike D15 it cannot be deferred and retrofitted:
whatever shape `GameTerms` has at first testnet is the shape signed channels
commit to.

---

## D23 — What is episode 08's oracle? **SETTLED: differential + model check**

`G1` is the rule that caught both bugs in `docs/build-log.md`, and it is the one
rule episode 08 cannot obviously satisfy, because nobody publishes adjudicator
test vectors. The episode splits, and only one half has an oracle available.

**The half that does.** The terminal-claim table — `FiftyMove`, `Threefold`,
`InsufficientMaterial` — is chess rules, and independent published
implementations of exactly those predicates exist (python-chess, shakmaty) to
differential-test against. This matters more than it looks: `perft` counts
nodes and says nothing about game endings, so insufficient material and
threefold are currently the thinnest-covered logic in `bc-chess` despite
`terminal.rs` having tests. Episode 08 closes that gap as a side effect.

**The half that does not.** Clock dilation, budget debits, `deadline_block`,
override-by-higher-ply and the halving penalty have no external referent
anywhere, because they are mechanisms this project invented. What they do have
is properties that must hold absolutely: budgets only decrease, the pot is
conserved, higher ply strictly wins, the process terminates.
`05-adjudication.md` already argues termination in prose — which is a proof
obligation written in English. Those are safety and liveness properties of a
small state machine, so they are **exhaustively model-checked** (stateright,
in-repo Rust).

Two standards, each honest about which half it covers. Neither is described as
an oracle where it is not one.

---

## D24 — Where does the adjudicator live? **SETTLED: a new `no_std` crate**

`P5` says the chess rules exist exactly once and are compiled for both client
and on-chain adjudicator. Today `bc-channel` depends on `bc-chess` and that is
the whole story; episode 08 adds a second consumer.

▶ **`bc-adjudicator`: one crate, depending only on `bc-chess`, exposing a pure
`fn(DisputeState, DisputeTx, Height) -> Result<DisputeState>`.**

The node links it as its state transition function. **`bc-channel` links it
too**, and that is the part worth choosing deliberately rather than falling
into: a client that links the adjudicator can run an entire dispute *locally
before spending any gas* — simulate posting its best certified state, see the
deadline it would get, see whether its mate claim survives refutation. That
turns the griefing analysis in `05-adjudication.md` from an argument into
something a client computes.

The second reason is determinism. Consensus code must be bit-identical across
every validator, so this crate wants `no_std`, no floating point, and no
hash-map iteration order — lints that are easy to impose at a fresh crate
boundary and painful to retrofit onto `bc-channel`, which is full of legitimate
client-side convenience. `P5` stops being a rule people remember and becomes a
dependency arrow.

---

## D25 — What is `adjudicator_ver`? **SETTLED: integer on the wire, hash in state**

D11 commits to keeping every adjudicator version forever, but never says what a
version *is*. A bare monotone integer is a promise with nothing behind it: two
builds can both claim version 3 and disagree about en passant, and the channel
that trusted the number cannot tell. A bare content hash is self-identifying but
illegible on the wire, unwritable in a spec before the build exists, and makes
reproducible builds load-bearing for consensus.

▶ **Both.** Channels negotiate a small integer; consensus holds a registry
mapping each integer to the hash of the ruleset it denotes. Readable where
humans read it, pinned where money depends on it.

The registry is also the concrete object D11's governance answer acts on — it
reduces "who controls upgrades" to "who may add a row to one table," which is a
much smaller question than it was.

---

# Working decisions

## D26 — Who writes the code? **SETTLED: split by layer**

Recorded in `docs/duality.md` as **HALF-SIGNED** with an empty intuition column:
the analysis asked whether it should keep writing the implementation or hand
over scaffolds and failing tests, on the grounds that a series about *learning*
this material may be poorly served by code its author did not type. The entry
sat unanswered.

▶ **Split by layer.** Evan writes the code that *is* the episode's subject — the
dilation arithmetic, the refutation check, the budget rules. Claude writes
plumbing, tests, serialisation, and the oracle harness. Roughly: the 20% that
gets filmed is typed by the person filming it; the 80% that does not is not.

Rejected: full handover of scaffolds only (slowest path to Milestone E, and it
risks stalling on borrow-checker fights in the exact month the project needs
momentum), and the status quo of Claude writing everything (fastest, and the
duality entry's objection stands).
