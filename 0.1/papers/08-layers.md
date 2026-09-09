# 08 — Layers, and why compressing the corpus *is* deriving chess theory

*This paper started as a note from Evan, and the core claim is his: that the
questions stack — one game, then all games, then structures found in the pile
of games — and that each layer hands the next one new tools. Written up here
because it is right, and because it changes what the project is for.*

---

## 1. The three questions, in the right order

> "It's like multiple questions. The smallest way to write one chess game.
> What is the smallest way to stack all of these games on top of each other in
> the most information-dense way. […] As we build up more and more layers of
> games, we will find new abstraction and compression tools that work on the
> previous layers that we have built."

`00-problem.md` names the first two (P1, one move; P3, one game given the
corpus). The third is new, and it is the interesting one:

| | question | the unit | the tool |
|---|---|---|---|
| **L0** | write one move | a ply | the legal-move index — `02-encodings.md` |
| **L1** | write one game | a move sequence | mixed-radix coding — built, `bc-codec` |
| **L2** | write the corpus | a game, given the others | prefix/predictive coding — `03-corpus.md` |
| **L3** | write the *structure* | a repeated pattern | **this paper** |

L3 is not "compress harder". It is a different kind of object: at L0–L2 the
units are given to you by the rules of chess, and at L3 the units are
**discovered from the data**.

## 2. What a layer actually is

The formal name for the L3 idea is **grammar-based compression**. The
procedure, in one sentence: find a substring that occurs often, give it a name,
replace every occurrence with the name, and repeat on the result.

Run that on a corpus of chess games and watch what the names turn out to be.
The first symbol it invents will be something like

```
A → 1.e4 c5 2.Nf3 d6 3.d4 cxd4 4.Nxd4 Nf6 5.Nc3 a6
```

which is the Najdorf. Nobody told it about the Najdorf. It found a
twelve-ply string that recurs tens of thousands of times and decided it
deserved a name, for no reason except that naming it makes the file smaller.
Then it runs again on the rewritten corpus, where games are now sequences of
symbols rather than moves, and finds patterns *among the patterns*.

That is the layering, and it is the same operation each time. What changes is
the alphabet.

| layer | alphabet | what a symbol means to a player |
|---|---|---|
| L1 | moves | a move |
| L2 | move sequences | an opening line |
| L3 | sequences of lines | a system, a plan, a structure |
| L4 | ? | nobody knows, which is the point |

L4 is speculative and should stay marked as such. But there is no reason the
procedure stops, and no reason its output at every level should not be
recognisable to a chess player, because the thing it is picking out is
*whatever recurs*, and recurrence is what makes something worth having a name
for in the first place.

## 3. The claim this leads to

Chess theory is a list of things that happen often enough to be worth naming.
Openings, structures, standard plans, typical endgames. Nobody derived them —
they accreted, over centuries, from people noticing repetition.

Compression is a procedure for finding what recurs, and the shortest
description of a body of data is the one that has found the most of it. This
is **minimum description length**: the best model of a dataset is the one that,
counting the model itself, encodes the data in the fewest bits.

Put those together:

> **Searching for the shortest encoding of the corpus is the same search as
> searching for chess theory. The compression is not a way to afford the
> database — it is the method by which the database produces knowledge.**

That is the strongest justification the project has for being about
compression at all, and it is much better than "storage is expensive". It also
answers, without any special pleading, why this database would be worth
studying rather than merely worth having: the act of storing it efficiently
*is* the act of finding the structure in it, and the structure is what a
student wants.

## 4. The hashing intuition, made precise

> "This in my head looks a lot like hashing functions."

There is a real connection and it is worth stating exactly, because the
surface similarity — both make things smaller — is the wrong half.

A hash is lossy and one-way; compression is lossless and reversible. But look
at their *outputs*. A hash function is designed so its output has no
exploitable pattern: it must look like random bits, or it is broken. And an
optimal compressor produces output that also looks like random bits — because
any pattern remaining in the output is redundancy the compressor failed to
remove, and could have used to go smaller.

> **The output of an optimal compressor is indistinguishable from random. That
> is the same property a hash function is built to have. They arrive at it from
> opposite directions: the hash by destroying structure, the compressor by
> spending it.**

This is not a metaphor, and it gives a **free diagnostic**: take the compressed
output and try to compress it again with a general-purpose tool. If it shrinks,
there is structure you missed.

Run on the four games in `corpus/`:

| file | bytes | gzipped | |
|---|---|---|---|
| PGN | 1,237 | 718 | shrinks **1.72×** — full of structure |
| `.bcg` under E3 (16-bit moves) | 304 | 321 | **grows** |
| `.bcg` under E7 (legal index) | 145 | 167 | **grows** |

gzip cannot find anything in our output. It grows the file, because all it
manages to add is its own header.

Reproduce it:

```sh
./target/release/blockchess pack corpus/classics.pgn out.bcg
gzip -9 -c out.bcg | wc -c        # bigger than out.bcg
```

Two honest caveats. These files are small enough that gzip's header is a real
fraction of them, so the *direction* is meaningful and the exact ratio is not;
and gzip only looks for repeated byte strings, so passing this test rules out
one family of leftover structure, not all of them. The strong version of the
test needs a corpus, and it is worth running there — **experiment X6**.

## 5. Two ways to be "a point that represents the group"

> "The blockchain could potentially live both as a history of all of the
> individual games and/or be a point in space that represents the group of
> games and the repeating patterns in the least amount of moves."

The "and/or" is right, and it is in fact the oldest structural distinction in
blockchains: a chain holds a **log** (everything that happened, in order) and a
**state** (a summary of where that leaves you). Bitcoin has transactions and
balances; 0.0 had transactions and a state tree. Here:

- **The log** is the games. Every one, forever, exactly as played.
- **The state** is the discovered structure — the grammar, the opening book,
  the frequency model. Everything L3 finds.

The state is *derived*: anyone can recompute it from the log, so it never needs
to be voted on, only published and checked. That is what makes it safe to be
ambitious with it, and it is the same argument as `05-index.md` §1.

But "a point in space" is two different objects depending on what you want from
it, and conflating them would cause real trouble:

| | a set commitment | an embedding |
|---|---|---|
| what it is | one group element committing to the whole set | a vector in a learned space |
| built from | `01-position-space.md` §7.2, a homomorphic hash | a model |
| answers | "is this game in the set?" — with proof | "what is this game *like*?" |
| lossy | no | yes |
| adversary-proof | yes | no |

Both are one small object standing for a large collection. The first is for
**proving**, the second for **understanding**. The project wants both, in
different places: the commitment goes on chain where it must be checkable, the
embedding lives in the index where being approximately right is fine.

## 6. What this does not escape

`03-corpus.md` proved that prefix sharing and predictive coding are one saving
and cannot be banked twice. That result still holds here, and it constrains
L3 more than it first appears: a grammar is a model, so **the layers will not
beat a good predictive coder on bits.** Anything a discovered grammar knows,
a sufficiently good policy model also knows, implicitly.

So if L3 is judged purely as compression, the honest prediction is that it
roughly ties E9 and loses on permanence, exactly as E10 did. It should be
written down now, before measuring, so the measurement can contradict it:

> **Prediction, on file: grammar-based coding lands near the model-prior
> figure, ~1.5–2 bits/ply, and does not beat it.**
>
> **WRONG — right in direction, badly wrong in magnitude.** Measured on
> 121,332 games: **10.17 bits/ply**, against E7's 4.625 on the same corpus.
> Re-Pair is more than twice as bad as the codec already in the repo, not
> "near a learned model". It is not near anything. `papers/11-results.md` §3a.
>
> The reason is one the paper missed entirely: induction **grows the
> alphabet**, so every surviving symbol costs more bits than the ones it
> replaced. 1,117 new symbols widened it from 1,962 to 3,079, and the savings
> do not pay for the width.

**And it does not matter, because bits were never the reason to build it —
which is fortunate, because the discovery half survived and the compression
half did not.** On the same run, the sequences it chose to name are, past four
plies, named by humans 3–6× more often than the sequences it did not choose.
`papers/11-results.md` §3b, with the control that makes that statement mean
something.
A policy network and a discovered grammar might spend the same bits, but only
one of them hands you a *list*. The grammar's symbols are inspectable, nameable,
countable, and arguable-about; the network's weights are not. What L3 produces
that nothing else does is an explicit, discovered vocabulary of chess ideas —
and that vocabulary is the thing people would come to discuss.

Which lands where the project started. `README.md` says 0.1 is for public
discourse about variations in play structure. This is the mechanism: the
compressor proposes the vocabulary, and people argue about it.

## 7. What to do about it, and when

Not yet. L3 needs a corpus, and it needs L1 and L2 measured first, or there is
no baseline to claim an improvement against. It is also the most seductive
thing in these papers and therefore the easiest to start too early.

The order stands as `06-decisions.md` D10 has it: corpus, then the index. But
the index is now doing double duty, and that is worth knowing while building
it — a trie of openings with occurrence counts **is the first layer of the
grammar**, already. Every node visited more than *k* times is a candidate
symbol. The structure that answers "who has played this line" is the same
structure that proposes "this line deserves a name."

So building the index is not a detour from this idea. It is step one of it.

## 8. Open experiments this adds

**X6 — the incompressibility check, at scale.** Pack a real corpus and try
gzip, xz and a general-purpose neural compressor on the output. Anything that
shrinks it is structure L1/L2 left behind, and the amount is a direct measure
of how much room L3 has. This is the cheapest experiment in the whole
programme and it bounds the value of the most expensive one.

**X7 — grammar induction on the corpus.** Run Re-Pair or SEQUITUR over the
move streams. Report: bits/ply against E7 and E9; the number of symbols
discovered; and — the part that is not a compression result — **how many of the
top 100 discovered symbols correspond to named openings.** That last number is
the real test of §3, and if it is high, it is the most interesting single fact
this project could produce.

**X8 — do the layers keep paying?** Run the grammar procedure again on its own
output, and again. Report bits saved per layer. The question is whether it
converges after two rounds or keeps finding structure, and nobody knows.
