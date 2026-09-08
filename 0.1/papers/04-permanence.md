# 04 — What immutability costs, and where compression should stop

*The main result of version 0.1. Computed figures come from
`measure/envelope.py` and `measure/plyrate.py`.*

---

## 1. The claim

Ordinary compression asks for the smallest output. A permanent public log asks
for the smallest output **that can still be read in fifty years by someone who
was not there when it was written**. Those are different optimisation problems
and the second one has a computable price.

Two results, and they point the same way.

> **Result 1. Immutability costs about 3 bits per ply — roughly 30 bytes per
> game — and that is the price of not putting a model in consensus.**
>
> **Result 2. It does not matter, because below about 50 bytes per game the
> chess is no longer the big term. The signatures are.**

Result 2 is the one that decides the architecture, and it is the reason not to
spend the project on Result 1.

## 2. Result 1 — the price of a model-free decoder

From `02-encodings.md`:

| decoder needs | scheme | bits/ply | bytes/game |
|---|---|---|---|
| the rules only | E7, index + arithmetic coding | ~4.9 | ~49 |
| the rules + a fixed heuristic table | E8 | ~4 `[lit]` | ~40 |
| the rules + a learned model | E9 | ~1.7 `[lit]` | ~17 |

The premium for refusing a learned model is **3.2 bits/ply, about 32 bytes per
game**, or a factor of 2.9 on the move stream.

What you buy for it is worth stating in full, because "we could compress 3×
better" will come back every few months and it needs an answer on file.

**A model in consensus can never be improved.** The chain will run for years.
Better policy models will exist. Adopting one means old games decode under the
old model and new games under the new one, so the chain now carries two
decoders. Then three. Every improvement is permanent baggage, and the marginal
gain shrinks each time while the baggage does not.

**A model in consensus can never be fixed.** If the model file has a bug — a
mis-serialised weight, a platform-dependent floating-point rounding — you cannot
correct it, because correcting it makes the existing corpus unreadable. You must
ship the bug forever, bit-exactly.

**Floating point is not deterministic across platforms, and an arithmetic coder
is exactly where that matters.** A neural policy prior means every node must
reproduce identical probabilities to the last bit or the decoder desynchronises
and every subsequent game is garbage. This is solvable — fixed-point
quantisation, one specified evaluation order — but it means writing a
bit-exact-by-specification inference engine, which is a serious project in its
own right and a permanent one.

**The model is a barrier to entry.** "Anyone can read the chess database" is the
point of this version. If reading requires downloading and correctly executing a
specific 200 MB network, most people cannot, and a chess database nobody can
independently read is a database with a maintainer.

Against that: 32 bytes a game. **▶ Pay the 3 bits.** The recommendation is
formalised as D4 in `06-decisions.md`.

### The same argument applies, more weakly, to E8

E8's ranking heuristic is also permanent consensus state — but it is a hundred
lines of integer comparisons that fit in a specification document, it has no
floating point, and a competent reader can reimplement it from the spec. That is
a categorically different object from a weights file, even though both are
technically "a model in consensus". The line to draw is not *model / no model*;
it is **can an independent reader reconstruct the decoder from the written
specification alone?** E8 passes. E9 does not.

That is the criterion. It is worth naming, because it is the criterion, not the
ratio, that decides every scheme in `02-encodings.md`.

## 3. Result 2 — the chess is not the big term

A game record is not a move stream. It is a move stream inside an envelope: who
played, what the result was, and evidence that it happened. If every game is
independently attested by both players:

| | bytes |
|---|---|
| two public keys | 64 |
| two signatures | 128 |
| result, rule set id, clock, date | 8 |
| **envelope total** | **200** |

Now put the move streams next to it:

| move stream | bytes | + envelope | chess % |
|---|---|---|---|
| PGN text | 400.0 | 600.0 | 66.7% |
| 16-bit moves | 160.0 | 360.0 | 44.4% |
| legal index, b = 30 | 49.1 | 249.1 | **19.7%** |
| model-based, 1.7 bits/ply | 17.0 | 217.0 | **7.8%** |

Read the last two rows. Going from E7 to E9 makes the move stream **2.9× smaller
and the record 13% smaller.** The entire model-versus-no-model argument of §2,
with all of its permanent consequences, is worth thirteen percent.

> **Once you are below ~50 bytes of chess, you are no longer storing chess. You
> are storing signatures, and further compression of the move stream is
> optimising the wrong term.**

This inverts the project's priorities, and it inverts them in a useful
direction, because the envelope compresses far better than the chess does — not
by encoding it more cleverly, but by **not repeating it**.

## 4. Where the real saving is: attest in batches

An Ed25519 signature is 64 bytes and irreducible. Two of them per game is 128
bytes of the 200. But signatures amortise: sign a Merkle root over a thousand
games and there is one signature for the batch, plus a compact account
reference per game.

| batch size | envelope bytes/game | chess % at 49 bytes |
|---|---|---|
| 1 | 200.0 | 19.7% |
| 10 | 26.4 | 65.0% |
| 100 | 17.0 | 74.2% |
| 1,000 | 16.1 | 75.3% |
| 10,000 | 16.0 | 75.4% |

Batching by 100 does more than every move-encoding decision combined: envelope
200 → 17 bytes, a 12× reduction on the dominant term. Beyond ~100 the curve is
flat; the residual 16 bytes is two account ids and the per-game metadata, and
that is a data-modelling question, not a cryptographic one.

And notice what batching restores: **it makes compressing the moves worth doing
again.** At batch 1, E7 → E9 saves 13% of the record. At batch 1000, it saves
49%. The two optimisations are multiplicative in the wrong order — compress
first and the envelope swamps you, batch first and the compression matters
again. So: **batch, then compress, and re-evaluate.**

### At corpus scale

Six billion games, which is the order of magnitude of the large public
databases:

| move stream | per-game attestation | batched by 1,000 |
|---|---|---|
| PGN text | 3,353 GiB | 2,325 GiB |
| 16-bit moves | 2,012 GiB | 984 GiB |
| legal index, b = 30 | 1,392 GiB | **364 GiB** |
| model-based, 1.7 bits/ply | 1,213 GiB | **185 GiB** |

The two decisions together — batch the attestations, index-code the moves — take
3.3 TiB to 364 GiB, a 9.2× reduction. The model-based row then takes it to 185
GiB, and *that* is the honest statement of what §2 is arguing about: 364 versus
185 GiB, against a permanent consensus model. Still no. But it is a real number
and it deserves to be argued with rather than waved away.

## 5. The rule set is part of the encoding

The deepest permanence problem is not the model. It is the rules.

Every Group B scheme in `02-encodings.md` encodes a move as *an index into the
list of legal moves*. To decode, you must generate the same list, in the same
order. Which means the permanent consensus object is not "the rules of chess"
but **one specific move generator, including its enumeration order.**

Three consequences, and the third is the one that connects to what this project
is for.

**A bug fix is a hard fork of the past.** If the generator mishandles some en
passant edge case, fixing it changes the move list in affected positions, which
changes the decoding of every stored game that passed through one. The bug
becomes part of the format.

**Enumeration order is consensus.** Not just which moves are legal — what order
they come in. Two implementations that agree perfectly on legality and differ in
ordering decode different games from the same bytes. This must be pinned in the
specification at the level of "iterate piece types in this order, squares in
this order, promotions in this order", and it must never be tidied up.

**Variants are not an extension; they are a new format.** This project wants
public discourse about variations in play structure. Chess960, atomic, a
variant somebody invents next year — each has a different legal-move function,
so each is a different decoder. Under Group B, a variant is not a flag on a
game, it is a whole encoding.

The design that follows:

```
GameRecord.rules : RuleSetId
```

where a `RuleSetId` names an immutable, published, versioned move generator
specification. Standard chess is `RuleSetId(1)`. A bug fix issues
`RuleSetId(2)`, and games already stored keep pointing at `1` and keep decoding
correctly forever. A variant is `RuleSetId(n)`, registered the same way.

This costs a couple of bytes per game — already counted in the 8-byte metadata —
and it is the difference between a chain that can host the discourse this
project is for and one that is frozen to one interpretation of one rule set on
the day it launched.

It also gives back the fallback that Group B otherwise removes. A game under an
unknown `RuleSetId` is undecodable by an old reader. So the registry entry for
every rule set must include the rule set's **own** specification, and a reader
that does not have it should be able to say "this is a game under rules 7, which
I do not implement" rather than silently producing wrong moves. That is a
one-line requirement with real teeth: **the rule set id must be outside the
arithmetic-coded stream, in plaintext, where a reader who cannot decode the game
can still read the id.**

## 6. Summary

| Result | Value |
|---|---|
| Premium for a model-free decoder | 3.2 bits/ply, ~32 bytes/game |
| Value of that premium once the envelope is counted | 13% of the record |
| Envelope, per-game attestation | 200 bytes, 80% of the record |
| Envelope, batched by 100 | 17 bytes |
| Best single decision available | **batch the attestations**, 12× on the dominant term |
| Corpus at 6e9 games, batched + index-coded | 364 GiB |
| The criterion for a decoder | reconstructible from the written spec alone |
| The permanent object nobody expects | the move generator, enumeration order included |
