# 07 — The server layer

> **REPURPOSED — see `METAPLAN.md`.** A server is a **corpus**, therefore a
> **lens**: it fits its own basis and so legitimately judges the same player
> differently from any other server (`METAPLAN` N10, N16). The trust ladder's
> custody tiers (T3/T4), bonds and slashing are cut with finance
> (`METAPLAN` N14, N17); registration and the VRF work survive.

Servers are the part of the system that most resembles a normal product. They
are permissionless: anyone can run one, and none of them are privileged by
consensus.

## The trust ladder

The important question about any server is **what can it take from you if it is
malicious?** Arrange servers by that answer.

| Tier | Name | Holds | Worst case | Defence |
|---|---|---|---|---|
| **T0** | Relay | nothing | refuses to match you | run another; `expiry_block` on offers |
| **T1** | Bonded relay | a bond | equivocates on pairings, loses bond | on-chain `SlashServer` with proof |
| **T2** | Rating authority | a reputation | lies about ratings | ratings recomputable from public games |
| **T3** | Custodial | **your money** | takes your money | proof-of-liabilities; only risk what you deposit |
| **T4** | Public-history | your privacy | publishes your games (that's the point) | opt-in by depositing to a T4 account |

The default and the recommended configuration is **T0**. A T0 server sees your
signed offer, finds you an opponent, collects both signatures, and submits one
`OpenGame` transaction. Funds move from the players' own accounts under the
players' own signatures.

> **The server is a relay of signatures, not a custodian of funds.**

This is the structural property that distinguishes this design from every
existing money-chess platform, all of which are T3.

### When T3 is nonetheless correct

Custody is not always wrong. If you want free instant games with sub-cent stakes
and no on-chain transaction per game, you *must* have something batching — and
that is a custodial internal ledger. The right response is not to forbid it but
to **make the risk explicit and bounded**:

- users move a small float to the server account, deliberately,
- the server publishes a periodic **proof of liabilities**: a Merkle tree over
  all user balances, root published on-chain, so each user can verify their own
  balance is included in the total the server claims,
- clients display "T3 — this server holds N tokens of yours" permanently.

Proof of liabilities is a nice, small Merkle exercise: each leaf is
`H(user_id ‖ balance)`, internal nodes carry subtree sums, and a user's
inclusion proof also proves the sums above them are consistent. It proves the
server is not *understating* its debts. It does not prove solvency without a
matching proof of reserves.

## Registration

```
RegisterServer {
  server_pk       [u8;32]
  bond            u128
  metadata_uri    [u8;64]     where to find the endpoint & policy document
  tier            u8
}
```

The bond does two things: it makes slashing possible, and it is a sybil filter —
a client can require "only show me servers bonded above X".

## Slashing: equivocation is the only provable crime

Consensus can only punish what it can verify. The verifiable server crime is
**equivocation**: signing two contradictory statements.

```
SlashServer {
  server_pk
  statement_a, sig_a
  statement_b, sig_b
}
```

Valid when both signatures verify under `server_pk`, both statements share the
same `(context, epoch)` tag, and they differ. The classic cases:

- two different pairings for the same player in the same tournament round,
- two different VRF seeds for the same round,
- two different rating attestations for the same player at the same epoch.

Anything requiring *judgement* ("this server was slow", "this server is
unfriendly") is deliberately not slashable. **Only mechanical contradiction is
punished on-chain.** Everything else is reputation, and reputation lives in
clients, not in consensus.

## Verifiable matchmaking with a VRF

A server that controls pairings can rig a tournament — put its own account
against the weakest opponents, or arrange a specific final. The fix is a
**verifiable random function**.

A VRF is a keypair where

```
(output, proof) = VRF_prove(secret_key, input)
       bool     = VRF_verify(public_key, input, output, proof)
```

The output is **unique** for a given key and input — the server cannot choose
among several — and **pseudorandom** to anyone without the secret key, but
**verifiable** by everyone once revealed. It is a signature that also happens to
be a uniformly random string.

Protocol for a tournament round:

1. Before entries close, the server publishes `commit = H(seed)`.
2. Entries close; the entry list `L` is fixed and published.
3. The server computes `(r, π) = VRF_prove(sk, seed ‖ round ‖ H(L))` and
   publishes `seed, r, π`.
4. Pairings are derived deterministically from `r` — e.g. Fisher–Yates shuffle
   of `L` seeded by `r`.
5. Anyone can verify `π` and recompute the pairings.

The server cannot bias `r` because the VRF output is a deterministic function of
inputs it committed to before seeing the entry list. It could only refuse to
publish, which is visible and (if bonded) slashable as failure to produce.

Why not just `H(seed)` with a commitment? Because with a plain hash commitment,
the server can *grind*: try many seeds offline, commit to the one giving
favourable pairings. A VRF removes the grinding freedom, because for a given key
and input there is exactly one valid output.

> **A commitment stops you changing your mind. A VRF stops you shopping for a
> mind to have.**

## Ratings

Servers may run any rating system. Recommended progression:

**Elo** — one number, one update rule, `K` controls responsiveness. The
weakness: it treats a new player's rating as if it were as certain as a
10 000-game veteran's.

**Glicko-2** — keeps `(R, RD, σ)`: rating, rating deviation, volatility. `RD` is
the standard deviation of a Gaussian posterior over your true strength. It grows
while you are inactive and shrinks as you play. Updates are a Bayesian posterior
update, and the effective `K` becomes a function of `RD` — an uncertain player
moves fast, a well-measured one moves slowly. This is the Kalman filter idea
applied to skill.

**TrueSkill** — the same Bayesian model as a factor graph, solved with
expectation propagation. Generalises to teams and free-for-alls. Overkill for
two-player chess but the right tool for an arena with many-way events.

### Rating attestations

A server signs:

```
RatingAttestation {
  server_pk, player_pk, rating u16, rd u16, epoch u32, games u32
}
```

Clients use these for handicap odds (`06-economics.md` §2) and for pool entry.
Attestations from a T4 public-history server are the strongest, because anyone
can recompute the rating from the published game record and catch a lie.

Note the interaction with money: **if ratings set odds, ratings become worth
manipulating.** Sandbagging — deliberately losing to lower your rating, then
farming favourable handicap odds — is the money-chess version of smurfing, and
Glicko's `RD` partly defends against it (a player with a suspiciously volatile
history has high `RD`, which clients can require to be low before offering
generous odds).

## Server archetypes

| Archetype | Tier | Notes |
|---|---|---|
| **Friends lobby** | T0 | invite-only, custom time controls, arbitrary stakes |
| **Bot arena** | T0/T1 | engines welcome, no anti-cheat needed, high-capacity edges |
| **Public free-for-all** | T1 | random pairing within stake tiers |
| **Rated ladder** | T2 | proprietary matchmaking, Glicko, low or zero stakes |
| **Glass house** | T2/T4 | every game published; the anti-cheat training corpus |
| **Fast lobby** | T3 | custodial, sub-cent stakes, instant, proof-of-liabilities |
| **Teaching server** | T0 | play a ladder of engines at increasing depth; the engine explains itself |

The teaching server deserves a note: it is the one archetype with no wagering
requirement at all, and it is probably the best on-ramp for new users. A ladder
of opponents from "random legal move" through "1-ply greedy" to "full search"
makes the *structure* of computer chess legible by playing against each rung of
it, which is a much better introduction than reading about minimax.

## Jurisdiction

Real-money wagering on games of skill is regulated, and the rules differ sharply
by country and, in the United States, by state. Chess is unambiguously a game of
skill, which matters in many jurisdictions but not all.

The architecture responds to this the way DeFi does: **the base layer is
non-custodial, permissionless, and holds no user relationships, so the
compliance surface sits with server operators**, who choose their own
jurisdiction, KYC posture, and geofencing. That is a real property of the design,
not a loophole — a T0 relay genuinely never touches user funds.

For the project's own public deployment, the low-risk path is a **play-token
testnet**: the full protocol, real cryptography, real disputes, tokens with no
purchase path and no cash-out. Everything is technically identical and nothing
is a regulated transaction. Get legal advice before that changes.
