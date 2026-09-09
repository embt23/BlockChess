# 09 — The lineage: where these ideas already live

*Written because the ideas in `08-layers.md` were arrived at from scratch, and
they have names, a literature, and — in one branch — sixty years of
experiments done specifically on chess players. Knowing that changes what to
build and what to claim.*

*Every citation here is marked `[lit]` and should be read before being relied
on. `07-sources.md` sets the standard: a number nobody has checked is not a
result, and neither is a paper nobody has read.*

---

## 1. Why this file exists

`08-layers.md` claims that compressing a corpus of chess games and deriving
chess theory are the same search. That is a strong claim, arrived at
independently, and the first responsible thing to do with a strong claim is
find out who got there first.

The answer is: several people, from three different directions, and they did
not entirely know about each other. One of those directions ran its
experiments **on chess players**, which is startlingly lucky for this project.

---

## 2. Compression is prediction is induction

The oldest branch, and the most abstract.

**Solomonoff (1964)** `[lit]` asked how an ideal agent should predict the next
symbol of a sequence. His answer: consider every program that could have
produced what you have seen, and weight each by `2^−(its length)`. Shorter
explanations get exponentially more weight. This is the formal version of
Occam's razor and it is, under its assumptions, the optimal inductive method.

**Kolmogorov (1965)** and Chaitin, independently, defined the complexity of a
string as the length of the shortest program that outputs it. A string is
random exactly when nothing shorter than itself produces it.

Put together: **the shortest description of your data is the best explanation
of it, and having it is the same as being able to predict.** Compression,
prediction, and induction are three views of one object.

Both are uncomputable, which sounds fatal and is not. It means no algorithm
finds the shortest description in general, so every practical method is a
heuristic search — and Re-Pair, entropy coding, and a neural policy model are
all heuristic searches for the same thing, differing in what part of the space
they can reach.

**Rissanen (1978)** `[lit]` made it usable. **Minimum Description Length** says:
choose the model minimising

```
length(the model) + length(the data, encoded using the model)
```

The first term punishes complicated models, so MDL will not let you "explain"
a corpus with a model as big as the corpus. **Wallace & Boulton (1968)** `[lit]`
had the same idea a decade earlier as Minimum Message Length.

This is exactly what `bc-grammar` reports as `total_size` — rewritten
sequences *plus two symbols per rule. A grammar that shrinks the corpus by
growing itself has achieved nothing, and MDL is the reason that term is in
there rather than being an accounting nicety.

### What this settles for us

`08-layers.md` §3's claim — that the shortest encoding is the one that found
the most structure — is not a conjecture. It is what MDL asserts, and the
project inherits both the claim and its known limits.

---

## 3. "Compression is intelligence"

**Hutter's AIXI (2005)** `[lit]` builds an optimal agent on Solomonoff
induction, and the **Hutter Prize (2006–)** `[lit]` turned the thesis into a
competition: compress 1 GB of Wikipedia, with the explicit argument that doing
it well requires understanding the text. The prize is for compression and the
claim is about intelligence.

**Schmidhuber's compression-progress theory (2009)** `[lit]` is the one worth
reading for this project, because it is about **layers**, and it answers a
question `08-layers.md` left open.

His proposal: what an agent finds *interesting* is not what is complex, and not
what is simple, but what it is **currently getting better at compressing**.
Interestingness is the derivative of compression. Noise is boring because it
never compresses; a solved problem is boring because it already compressed;
the interesting zone is the moving frontier in between.

Map that onto chess and it is immediately recognisable. Random legal moves are
boring — no structure to find. A memorised opening is boring — already
compressed, it is "theory" precisely because the compression is finished. What
is interesting is the region where compression is currently improving: novelties,
disputed lines, the sharp middlegame where nobody has settled what the pattern
is.

> **This gives `08-layers.md` a mechanism it was missing. "Which lines deserve
> discourse?" has an answer that is computable rather than editorial: the ones
> where the grammar is still changing.**

That is worth building. A line whose compression has stabilised is settled
theory. A line where each new batch of games shifts the encoding is where the
argument is. `05-index.md` wanted a way to surface what is worth discussing
without a person curating it; this is one, and it falls out of the same
machinery.

---

## 4. Grammar induction as structure discovery

**Nevill-Manning & Witten's SEQUITUR (1997)** `[lit]` is the direct ancestor of
what `bc-grammar` does, and they were explicit that the compression was not
the point: they ran it on DNA, on music, on the Bible, and argued the grammar
it produced was a *structural analysis*. **Larsson & Moffat's Re-Pair
(1999)** `[lit]` is the algorithm actually implemented here — offline,
greedy-by-frequency, and the one whose output is easiest to read.

So the "run a compressor and look at what it named" move has form. It has been
done on genomes and on music, and in both cases the symbols it invented lined
up, imperfectly, with units the field already had names for.

**What has apparently not been done is the chess version at scale with the
opening names as ground truth** — which is what `blockchess grammar` now does,
and why experiment X7 is worth running even though every ingredient is old. It
is a new measurement of an old idea in a domain with an unusually complete
answer key: 3,810 named lines, agreed by humans over a century, sitting in
`data/eco.tsv`.

---

## 5. The branch that studied chess players

This is the one that changes how ambitious the project is allowed to be.

**De Groot (1946/1965)** `[lit]` showed masters and weaker players search a
similar number of moves — masters simply consider better ones. Expertise was
not brute force.

**Chase & Simon (1973)** `[lit]`, "Perception in chess", found the mechanism.
Shown a real position for five seconds, masters reconstruct it far better than
novices. Shown a **randomly arranged** position, that advantage largely
collapses. So the master is not remembering pieces — they are remembering
*chunks*: familiar configurations, a castled king with its pawns, a known pawn
structure. Estimates of a master's vocabulary run from tens of thousands to a
few hundred thousand chunks `[lit]`.

**Gobet & Simon (1996)** `[lit]` refined this in a way worth carrying, because
it is a correction rather than a confirmation: masters retain *some* advantage
even on random positions, so pure chunking is not the whole story. Their
**template theory** proposes larger structures with slots — a schema like "the
Najdorf pawn structure, with these squares variable" — which is a strictly
richer object than a fixed chunk.

Now put that beside §4.

> **Chunking theory says chess expertise is a learned vocabulary of recurring
> configurations. Grammar induction is an algorithm that learns a vocabulary of
> recurring configurations. These are the same shape of object, arrived at from
> psychology and from information theory independently.**

Evan's intuition — that layers of abstraction stack up toward something like a
representation of the game — is the chunking theory of expertise, restated from
the compression side. It is not a metaphor and it is not new, and both of those
facts are good news: it means there is sixty years of experimental work to
check against, and a ready-made criticism to answer.

### The honest limits

Three, and they matter.

**Chunks are about positions; Re-Pair here works on move sequences.** A chunk
is a spatial configuration on a board. What `bc-grammar` currently finds is a
temporal pattern in a move stream. These are related but not the same, and the
project should not claim the psychology result until it works on positions.
That is a real, buildable next step — grammar induction over the *position*
stream rather than the move stream — and it is now on the plan as X9.

**Template theory says fixed sequences are not enough.** A master's unit has
slots. Re-Pair's symbols are rigid: exactly this sequence, or no match. So the
right expectation is that Re-Pair recovers the *rigid core* of theory —
forced sequences, main lines — and misses the flexible schemas, which are
where much of the real expertise lives. If X7 comes back with high agreement
on long forced lines and poor agreement elsewhere, that is not a failure, it is
this prediction confirmed.

> **CONFIRMED, and quantified.** Against a control of real game prefixes, the
> lift is 0.41× at two plies, crosses 1.0 at four, and reaches 3.05× at five
> and 5.83× at seven. It wins exactly where lines are forced and loses where
> they are not. What it recovers at 5–7 plies is the Scotch, the Sicilian Open,
> the Giuoco Piano, the Ruy Lopez Steinitz — sequences where each move is close
> to compelled by the last. `papers/11-results.md` §3c.

**"Conscious representation" is a bigger word than anything here earns.**
Hierarchical predictive models are a live theory of what brains do `[lit]`, and
the resemblance is real and worth noticing. But a Re-Pair grammar over chess
games is a vocabulary, not a mind, and the distance between the two is not
something this project can close or should claim to. The defensible statement
is the interesting one anyway:

> Building a hierarchy of discovered abstractions over a corpus produces
> something that increasingly resembles how an expert *describes* the domain.
> Whether it resembles how an expert *thinks* is a separate question, and
> Chase & Simon is where you would go to test it.

---

## 6. What this changes

| | before | after |
|---|---|---|
| The MDL accounting | felt like a technicality | is the entire reason the claim is falsifiable — `08` §6's prediction is an MDL prediction |
| "Which lines deserve discourse?" | editorial, unsolved | computable: where compression is still improving (§3) |
| Grammar over moves | the obvious thing to do | the *easier* thing; positions are what the psychology is about (§5, X9) |
| Rigid symbols | a limitation to apologise for | a testable prediction: recovers forced lines, misses schemas |
| "Conscious representation" | the goal | out of scope, and the in-scope version is sharper |

Two new experiments come out of this file, and they are on the plan.

**X9 — grammar over positions, not moves.** Induce over the sequence of
canonical position keys instead of move ids. Slower and much closer to what
chunking theory is actually about. `bc-index` already computes the keys.

**X10 — compression progress as an interestingness measure.** Induce a grammar
on the corpus up to time *t*, then at *t+1*, and diff. Lines whose encoding
changed are where the theory is moving. Schmidhuber's proposal, applied to a
database that has the timestamps to do it.

---

## 7. Reading order

If only one: **Chase & Simon (1973)**. It is about chess, it is short, and it
is the experimental floor under the whole idea.

Then **Nevill-Manning & Witten (1997)** for what a grammar over sequences
actually gives you, since that is the code that now exists.

Then **Schmidhuber (2009)** for the layers and for §3's mechanism.

Kolmogorov and Solomonoff are the foundations and can wait — they say the
target exists and is unreachable, which is good to know and does not change
what to type tomorrow.

**Grünwald, *The Minimum Description Length Principle* (2007)** `[lit]` is the
standard book-length treatment if MDL turns out to be load-bearing, which on
current evidence it is.
