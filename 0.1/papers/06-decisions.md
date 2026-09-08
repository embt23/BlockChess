# 06 — Decisions

*Every fork that meaningfully changes the project. `▶` marks the
recommendation. **SETTLED** means the reasoning is in the papers and the
decision should not be reopened without new measurement.*

---

## D1 — What is the thing being stored? **SETTLED: games, not positions**

`01-position-space.md` §4. A position costs ~150 bits; a move costs ~5. Storing
positions is 30× worse and no refinement of position encoding closes it.

Positions appear as *derived* index keys (`05-index.md`) and optionally as
sparse seek checkpoints inside a game. Never as the primary record.

---

## D2 — Does the chain hold the moves, or only a commitment to them?

| Option | For | Against |
|---|---|---|
| **Chain holds the moves** ▶ | it is a public chess database, which is the entire point; readers need no permission and no second system; ~364 GiB at six billion games is large but not absurd | the chain is now a bulk data store, and bulk data storage is the expensive thing about chains |
| Hash on chain, data elsewhere | 32 bytes a game; the chain stays small | whoever holds the data decides who reads it, which reintroduces exactly the gatekeeper this project exists to remove; a hash of a game nobody can fetch is not a database |

▶ **The chain holds the moves.** This is the decision that makes 0.1 a different
project rather than a variation on existing ones. It is also what makes the
compression work in `02`–`04` load-bearing rather than academic: the reason to
care about 49 versus 160 bytes is that the difference is multiplied by six
billion and stored by every full node forever.

The commit-only design is the right answer for *annotations* (`05-index.md` §6),
where the content is unverifiable and 100× larger. Not for games.

---

## D3 — The move encoding **SETTLED: E7, legal-move index, arithmetic-coded**

`02-encodings.md`. Roughly 4.9 bits/ply, ~49 bytes/game, decoder is a move
generator plus an arithmetic coder, both fully specifiable in prose.

Rejected: E3 (16-bit, 3.3× larger for no gain once a move generator exists —
and any chess project has one); E8 (a heuristic table is defensible but buys
~9 bytes/game and adds a permanent table); E9/E10 (D4 below).

Consequences accepted, all from `04-permanence.md` §5:

1. The move generator, **including its enumeration order**, is permanent
   consensus and must be specified to the level of iteration order.
2. Every game record names a `RuleSetId` in plaintext, outside the coded
   stream, so a reader who cannot decode can still say why.
3. A rules bug fix is a new `RuleSetId`, not an edit. Old games keep pointing at
   the old one and keep decoding correctly.

Point 3 is what makes this survivable, and it is also what makes variants a
first-class feature rather than an afterthought — see D9.

---

## D4 — A learned model in the decoder? **SETTLED: no**

`04-permanence.md` §2. Costs 3.2 bits/ply, ~32 bytes/game, and at per-game
attestation that is 13% of the record.

The criterion, which generalises past this one decision:

> **Can an independent reader reconstruct the decoder from the written
> specification alone?**

A move generator and an arithmetic coder pass. A heuristic ranking table passes.
A weights file does not, and a decoder that requires bit-exact floating-point
inference forever is a permanent liability accepted for a 13% saving.

Revisit only if D5 lands and the measured E8/E9 gap (**X3**) is much larger than
the literature figures suggest.

---

## D5 — How are games attested? ▶ **batched, with one aggregate signature**

`04-permanence.md` §§3–4. This is the largest single saving available anywhere
in the project and it is not a chess decision at all.

| Option | envelope bytes/game | For | Against |
|---|---|---|---|
| Two signatures per game | 200 | each game independently verifiable in isolation | the signatures are 80% of the record |
| **Batched, aggregate signature** ▶ | ~17 at batch 100 | 12× on the dominant term; makes move compression worth doing again | verifying one game requires the batch's Merkle path; needs an aggregatable scheme |

▶ **Batch.** Order of operations matters and is easy to get wrong: compress
first and the envelope swamps the gain (E7→E9 saves 13% of the record); batch
first and the same compression saves 49%. **Batch, then compress, then
re-evaluate the encoding.**

Open: which aggregation scheme. BLS aggregates arbitrary signers at 48 bytes but
needs pairings; Schnorr half-aggregation is simpler and saves less. This is a
cryptography decision that does not block the chess work and should be made
after X1–X3.

---

## D6 — Commit the index roots on chain? ▶ **not yet**

`05-index.md` §5. Committed epoch roots enable verified proofs of presence and,
more valuably, **proofs of absence** — the honest form of "nobody has played
this before".

Against, for now: committing the index root drags a derived, freely-changing
structure partway into consensus, and the index shape is going to change
repeatedly over the next year. Commit it once it has stopped moving.

Nothing is lost by waiting. The commitment can be added later without touching
any stored game, because the index is recomputable from the games.

---

## D7 — What stops the database being flooded with fabricated games? **OPEN**

The hardest problem in 0.1, and it is hard *because* there is no economy.

Anyone can generate a legal chess game between two keys they own, at millions of
games per second. A public chess database whose contents can be manufactured for
free is worthless for study: the statistics are the product, and unfiltered
statistics over fabricated games are noise. This is the one place where removing
the money from 0.0 removed a defence and did not replace it.

The candidate answers, and none of them is complete:

| Mechanism | Stops | Costs |
|---|---|---|
| Both players must sign | one party inventing a game about someone else | nothing, and it stops nothing else — a flooder owns both keys |
| Proof of work per batch | volume | a permanent energy cost, and an attacker with a GPU still outruns honest players |
| Bonded writers | volume, credibly | that is an economy, which is out of scope for 0.1 |
| Rate limit per identity | volume per key | identities are free, so it stops nothing without D8 |
| Permissioned writers | everything | that is a gatekeeper, which is the thing this project exists to remove |
| **Post everything, filter in the index** ▶ | nothing at write time | the filter becomes the valuable and contested object |

▶ **Provisionally: accept everything, and make provenance a first-class field.**
Reframe the problem. The chain's job is to record *that this identity claims
this game happened*, permanently and publicly. It is not the chain's job to
decide which games are worth studying. That judgement belongs to the index
layer, where it is contestable, versionable, and where several people can
disagree in public — which is nearer to the "public discourse" this version is
for than any consensus rule would be.

This is provisional because it moves the problem rather than solving it, and it
has a real failure mode: if the filter is the valuable object and there is only
one filter, its maintainer is the gatekeeper again, wearing a different hat.
Two things make it defensible rather than a dodge — filters are cheap to build
because the index is derived, so *several* can exist; and a game signed by two
identities that people have reason to trust is self-evidently different from one
signed by two keys created that morning, without anyone having to rule on it.

**This decision needs the most work and it should not be closed quietly.**
It is the one that determines whether the thing is useful.

---

## D8 — What are identities? **OPEN, and coupled to D7**

Free identities make D7's rate limits meaningless. Non-free identities are an
economy or a registry.

Middle paths worth exploring: identities that accumulate a public history and
are therefore costly to abandon; optional attestations linking a key to an
existing rated identity, where the linkage is a claim anyone may verify or
ignore; a web-of-trust among keys. All of these are *index-layer* facts under
D7's framing, which is consistent, but none is worked out.

Do not decide this before D7.

---

## D9 — Variants **SETTLED: `RuleSetId`, registered, immutable**

Falls straight out of D3. Under a legal-move-index encoding, the rule set *is*
the decoder, so a variant is a different format, and there is no way to make it
anything else short of abandoning Group B and paying 3× the storage.

So make it explicit and cheap: a registry of immutable, published rule-set
specifications, each with a numeric id; standard chess is 1; a bug fix or a
variant gets the next number. Costs a couple of bytes per game.

This turns the encoding's most awkward constraint into the feature that supports
the discourse the project wants — new play structures are first-class objects
with permanent ids and their own game corpora, comparable side by side.

---

## D10 — What gets built first? ▶ **measurement, then a codec, then a chain**

The papers rest on three literature figures and one unmeasured quantity. Until
X1–X3 (`measure/EXPERIMENTS.md`) are done, D3 and D4 rest on other people's
numbers, and 0.0's standing rule was that an external oracle beats a
self-written check — the corollary being that a number nobody has checked is not
a result.

▶ Order:

1. **X1–X3.** A corpus, a move generator, branching and rank histograms. This is
   also the fastest way to find out whether E8 is closer to E9 than assumed,
   which is the one measurement that could reopen D4.
2. **The codec.** Encoder and decoder for D3, with the round-trip property
   `decode(encode(g)) == g` on every game in the corpus. That property is the
   perft of this project: an exact, external, adversarial check that either
   passes on millions of cases or does not.
3. **The index.** Trie and inverted index over the corpus. At this point it is
   already a useful chess database with no chain in it at all, and it is worth
   stopping to notice that.
4. **The chain.** Last, and D7 must be answered before it means anything.

Note what stages 1–3 are: a complete, useful, studiable chess database, with the
blockchain still absent. If the chain never arrives, that is still a real thing.
0.0's equivalent milestone was ten months out and had money on it.

---

## D11 — Implementation language ▶ **Python to measure, Rust for the codec**

The measurement layer is a data-processing job over a corpus and it is
throwaway; Python is correct for it and `measure/` is already Python.

The codec is different: it is the permanent object from D3, it needs
bit-exactness, it will be reimplemented by other people from the spec, and the
reference implementation should be the one that is hardest to get subtly wrong.
Rust, as in 0.0.

The two are cleanly separated by the round-trip test, which either passes on the
whole corpus or does not, and which can be run against both implementations
independently.
