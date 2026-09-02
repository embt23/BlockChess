# 09 — Decisions

Every decision that meaningfully forks the project. `▶` marks my recommendation
and the reasoning. Decisions marked **SETTLED** are already made.

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

**Open question I cannot decide for you:** who has the authority to ship an
upgrade? This is a political question, not a technical one, and it is worth
answering in public early.

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

▶ **Apache 2.0** for the protocol and node (permissive, includes an explicit
patent grant, which MIT lacks); **CC BY-SA 4.0** for the specification prose and
diagrams. If you would rather force servers to open-source their modifications,
AGPL is the tool — but it will reduce adoption, and adoption is what makes the
anti-cheat corpus valuable.

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
