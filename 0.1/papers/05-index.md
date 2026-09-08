# 05 — Making it findable

*Storage is settled by `02`–`04`. This paper is about the question the project
actually exists to answer: "if someone has played this opening, I can go find
it."*

---

## 1. The separation

The chain stores games. It does not store the index. The index is **derived**:
any reader can rebuild it by replaying the chain, and two readers who rebuild it
independently get the same answer, so nothing about it needs to be agreed by
consensus.

That separation is the single most important structural decision in this version
and it is what makes the aggressive storage choices safe.

| | the record | the index |
|---|---|---|
| lives | on chain | anywhere |
| must be | canonical, permanent, adversary-proof | fast, rebuildable, disposable |
| may use | only what the spec pins forever | any model, any heuristic, any version |
| changes | never | whenever someone has a better idea |

The consequence worth stating loudly: **every technique rejected in `02` and
`04` for being unfit for consensus is welcome in the index.** Learned models,
symmetry canonicalisation, corpus statistics, position-keyed DAGs, aggressive
lossy summaries — none of them can corrupt anything, because all of them can be
recomputed from a record that does not depend on them.

`04-permanence.md` says the model may not go in the chain. It does not say the
model may not exist. It says the model belongs on this side of the line.

## 2. The four questions a chess database is asked

| # | Question | Structure that answers it |
|---|---|---|
| Q1 | "Show me games in this opening" | trie of move sequences |
| Q2 | "Show me games that reached this position, any move order" | inverted index on position key |
| Q3 | "What is usually played here, and how does it score" | edge counts on the position graph |
| Q4 | "Has this position ever occurred at all" | filter over positions per epoch |

Q1 and Q2 are different questions and they need different structures. A study
tool that only answers Q1 will insist the Sicilian Najdorf reached by a
transposition is a different opening, which is wrong in the way that matters
most to a person studying it.

### Q1 — the trie

Keyed on move sequence from the initial position. A node is one exact move
order. This is the opening book, and `03-corpus.md` established that it is
excellent at being an index and useless as a compressor.

Cost is bounded by the branching of *played* moves, not legal ones. Most nodes
past move 15 have one child, so the trie should be path-compressed: collapse
chains of single-child nodes into one edge labelled with a run of moves. A
radix trie, not a trie.

### Q2 — the inverted index

Keyed on a position key, valued by the list of (game, ply) pairs that reached
it. This is the structure that makes transpositions work, and it is the larger
of the two by far: 80 entries per game, one per ply, against one leaf per game
in the trie.

At six billion games that is 4.8e11 postings. This is a serious systems problem
and it is where the engineering effort in a later version should go. Two knobs
make it tractable and both are honest lossiness *in the index only*:

- **Ply sampling.** Only index every position from ply 6 to ply 40. Openings and
  early middlegames are what people study; a 90-ply endgame grind is not
  searched by position. Cuts postings by ~2×.
- **Popularity thresholding.** Positions reached by exactly one game in the
  whole corpus are the overwhelming majority of distinct keys and are almost
  never searched. Store them in a second, colder tier or behind a filter.

### Q3 — the position graph

Q2's index, aggregated: for each position, the distribution of continuations and
the results that followed. This is what a person actually looks at — "the main
line scores 54% over 12,000 games, this sideline scores 61% over 300" — and it
is a fold over Q2, not a separate structure.

### Q4 — the filter

"Has anyone ever reached this?" is answered by a Bloom or cuckoo filter per
epoch: a few bits per position, no false negatives, tunable false positives, and
a definite answer available by then consulting the real index. This is what
makes a light client useful: a reader with 100 MB can answer Q4 over the whole
corpus and fetch the details only when the answer is yes.

## 3. The position key

The key for Q2/Q3/Q4 must be:

- **Order-independent** — the point is to collapse transpositions.
- **Collision-resistant against an adversary** — anyone can write games to this
  chain, so anyone can grind for a collision. A collision means one position's
  games appear under another's, which is an attack on the study database even
  though no money is involved.
- **Cheap to update incrementally** — it is computed 80 times per game, 4.8e11
  times over the corpus.

`01-position-space.md` §7 gives the construction: a homomorphism from board
edits into a group where discrete log is hard. Zobrist gives properties 1 and 3
and fails property 2 at the `2^32` birthday bound, which is grindable on a
laptop. A cryptographic hash of the canonical packed position gives 1 and 2 and
fails 3, costing a full rehash per ply.

**▶ Use the cryptographic hash of the canonical packed position, and drop the
incremental requirement.** 4.8e11 hashes of a 20-byte input is a few
CPU-days — a one-off cost for a full index build, entirely affordable, and it
avoids a homomorphic-commitment construction whose security would need its own
analysis. Revisit if index build time ever becomes the bottleneck; the group
construction is the upgrade path and it is written down.

This requires a **canonical** packed position, in the strict sense: exactly one
byte string per position, no spare bits, no alternative orderings, and — the
subtle one — the en passant field set only when an en passant capture is
actually available. Otherwise the same position hashes two ways and the
transposition index silently splits it.

### Whether to include the halfmove clock

Two different keys are wanted for two different purposes:

- **With** the clock: the true position, acyclic (`03-corpus.md` §5), correct
  for repetition reasoning.
- **Without** it: what a person means by "the same position". Someone studying a
  structure does not care that one game arrived with the clock at 4 and another
  at 11.

Build the index on the clock-free key and keep the clock in the posting. This is
the one place where the study tool and the rules disagree, and the study tool
should win, because that is what this version is for.

## 4. Symmetry, finally earning its keep

`01-position-space.md` measured symmetry at 0–1 bit and rejected it as
compression. Here it pays.

A line and its file-mirror image are the same idea. So are a position and its
colour-swapped counterpart with the result negated. A database that shows them
separately is asking the student to do the reflection in their head, twice, for
every query.

So the index — not the record — should be keyed on **canonical form**:

```
key(P) = min over g in G(P) of packed(g · P)
```

with `G(P)` the surviving group from `01-position-space.md` §5: order 4 with
pawns and no castling rights, order 2 with castling rights, order 16 pawnless.
Store the group element alongside the posting so the display can un-mirror it.

The measured effect is exactly the table in `01` §6, read as a *merge* rate
rather than a saving: the colour-closed families merge at the full group order
(4.0 bits, i.e. 16 positions to an orbit, pawnless; 2.0 bits, 4 to an orbit,
with pawns) because the index *is* closed under colour swap even though a single
game's material is not. That is the row that could not pay in `01` and does pay
here — a factor of four fewer distinct keys in the pawn regime, which is most of
the corpus.

A factor of four on 4.8e11 postings is worth having on its own terms, and it is
a better study tool. Both, for once, in the same direction.

## 5. Proving things about the corpus without holding it

The index is derived, so it needs no consensus. But a *light* reader wants
answers they can check without downloading 364 GiB, and there is a cheap way to
give them one.

Commit the index. Each epoch, publish in the block header the root of a Merkle
structure over that epoch's index. The index remains derived — anyone can
recompute it and check the root — but now a server can answer "12,347 games
reached this position" with a proof, and a light client can verify it against a
header it already trusts.

Two things become possible that are otherwise not:

- **Proof of presence.** "Here is a game reaching this position, and here is its
  path to the epoch root."
- **Proof of absence.** "No game in this epoch reached this position" — a
  sparse-Merkle non-membership proof. This is the interesting one, because a
  novelty is defined by absence, and "nobody has played this before" is the
  claim a study database most wants to be able to make honestly.

Cost: one hash in each block header, and index builders must agree on the
index's exact shape — which drags a *derived* structure back toward consensus,
partially. The honest framing: this is a real trade, it converts a rebuildable
convenience into a semi-committed artefact, and it should be a later version's
decision once the index shape has stopped changing. Recorded as D6 in
`06-decisions.md`, recommended **not yet**.

## 6. The other data type

The project wants public discourse about variations, not just a pile of games.
That means annotations: a comment, a variation, an assessment, anchored to a
node.

An annotation is a signed statement anchored at a position key or a trie node.
It is a different object from a game in every way that matters here:

| | game | annotation |
|---|---|---|
| size | ~49 bytes | unbounded prose |
| compressible | 8× | barely |
| canonical | exactly one encoding | no canonical form |
| verifiable | legality is checkable | opinion is not |
| adversary | can fabricate valid games | can post anything |

Which means the storage answer is different, and the difference is not a
compromise. Games go on chain, because they are small, canonical, and mechanically
checkable. Annotations go **content-addressed off chain, with the hash and the
anchor on chain** — the chain records that *this* identity said *something* about
*this* position at *this* time, and the something is fetched separately.

This is the only place in 0.1 where E14 ("commit only") from `02-encodings.md`
is the right answer, and it is right for exactly the reason it was wrong for
games: nobody can verify prose, so putting it in consensus buys nothing, and
prose is 100× larger than a game so it costs everything.

## 7. Summary

| Decision | Choice |
|---|---|
| Index in consensus? | No. Derived, rebuildable, free to change. |
| Structures | Radix trie (Q1), inverted index (Q2), graph fold (Q3), filter (Q4) |
| Position key | Cryptographic hash of the canonical packed position |
| Incremental hashing | Not needed; group-homomorphic commitment is the upgrade path |
| Halfmove clock in the index key | No; keep it in the posting |
| Symmetry | Canonicalise **here**, ~4× fewer keys in the pawn regime |
| Committed index roots | Not yet — D6 |
| Annotations | Off chain, content-addressed, anchored on chain |
