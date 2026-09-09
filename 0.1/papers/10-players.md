# 10 — Players: think time, style, and the modes of the whole body

*From a conversation with Evan on 2026-09-09. Three ideas, all his, written
down before they evaporate. None of them is measured yet and the paper says so
throughout — they are a research programme, not results.*

---

## 0. The scope decision that makes this tractable

**Pure data. No interviews, no commentary, no text.** What a player said
afterwards is not in any database and assembling it would be a different
project with different problems. What *is* in the data, for billions of games,
free and public: the moves, and **the clock reading at every one of them**.

That constraint is a gain, not a compromise. Interviews are what someone
reports about their thinking. The clock is a measurement of it.

---

## 1. Think time is the visible trace of a failed compression

`09-lineage.md` §5: Chase & Simon showed that a master's advantage is
*recognition*. Shown a real position, they see familiar chunks. Shown a random
one, the advantage largely goes.

Recognition is fast. Computation is slow. So:

> **A player moves quickly when the position is already in their vocabulary,
> and slowly when it is not. Time spent is where their internal compression
> failed.**

Which means Lichess's clock field is a per-move readout of one person's model
of chess breaking down — for billions of games, without a laboratory. The 1973
experiment is sitting in a public file, at a sample size Chase and Simon could
not have imagined.

### The experiment

Our encoder already prices every move in bits: few when the corpus made it
predictable, many when it was a surprise. The human spent some number of
seconds on that same move.

**Do bits and seconds track?**

If they do, a statistical model built from a million strangers and one person's
mind are getting confused in the same places — which would be a real result
about both. If they do not, that is at least as interesting, and it says the
compressor's notion of "surprising" is not the human one.

**Experiment X11.** Written up in `measure/EXPERIMENTS.md` with the controls it
needs, because the obvious confounds are severe: time pressure at the end of a
game, players moving instantly in known theory *because they memorised it
rather than understood it*, and the fact that a fast move and a bad move are
correlated for reasons that have nothing to do with recognition.

---

## 2. A compressor built from one player is a model of that player

Build the grammar from Kasparov's games alone and it stops encoding chess and
starts encoding *him*. His habitual structures get short codes. Everything he
never does gets long ones.

Now price a single move twice — once under the general model built from
everybody, once under his.

> **Style is the gap.** A move that is cheap under one player's model and
> expensive under everybody's is characteristically theirs.

That is a definition of playing style with a number attached, and it needs no
adjectives. Nobody has to agree on what "aggressive" means.

**Experiment X12.**

---

## 3. The distance between two players, and then the shape of all of them

Extend it: encode player A's games using player B's model. The extra bits it
costs is a **distance** — how surprised B's habits are by A's games. This is
cross-entropy, and it is a real quantity, not an analogy.

Do it for every pair and the result is a matrix. A matrix like that is a shape,
and the standard way to find the dimensions of a shape is its eigenvectors.

Here is the part worth being precise about, because Evan named it before
knowing it had a name. For a connected body, the eigenvectors of its Laplacian
are **literally called its modes of vibration**, ordered from lowest frequency
upward. The lowest mode of a drumhead moves the entire membrane together. High
modes are local ripples that do not propagate.

Build the graph of players, take its Laplacian, and:

| | in the graph | in the body |
|---|---|---|
| lowest non-trivial eigenvectors | the axes along which *everyone* varies together | the fundamental modes |
| high eigenvectors | one person's private quirks | local ripples |

> **"The clusters of dimensions that are the lowest frequency vibrations that
> connect the whole body" is not a metaphor for the computation. It is the
> name of it.** Spectral embedding of a graph is the eigendecomposition of its
> Laplacian, and its low modes are called low-frequency for exactly this
> reason.

So the fundamental temperaments of chess would fall out rather than being
chosen. You compute the modes, look at who sits at either end of each one, and
find out afterwards what that dimension turned out to be.

**Experiment X13.**

---

## 4. What this is not

Evan's framing, and it is a sharper one than "we are building a model":

> *"Its like a map of history. Like a model, you can make predictions, but
> that's your choice. It's more like trying to build a conscious
> understanding — a unit, dimension, group, relationship, geometry."*

A model is judged by whether its predictions come true. What is being built
here is judged by whether it is **faithful** — whether the symbols it carved
out correspond to real joints in the thing. Prediction is a side effect
available if you want it; the carving is the object.

That distinction has consequences rather than being philosophy. It is the
standing answer to "why not just use a neural network, it compresses better":
the network predicts well and hands you nothing you can point at. The grammar
hands you a list and says *these are the parts*. `04-permanence.md` §2 already
rejected the network for consensus reasons; this rejects it for a second,
independent reason, and the two do not depend on each other.

It also sets the honest ceiling. §5 of `09-lineage.md` holds: a vocabulary is
not a mind, and "conscious understanding" is a bigger claim than any of this
earns. The defensible version is that a hierarchy of discovered abstractions
over a corpus increasingly resembles how an expert *describes* the domain, and
whether it resembles how they *think* is a separate question — which, unusually
for a philosophical-sounding question, has an experiment attached to it. That
experiment is X11.

---

## 5. Order of attack

X12 before X13, because the distance matrix needs per-player models to exist
first. X11 is independent of both and is the cheapest, because the clock data
is already in the file — no extra machinery, just a join between two columns
the corpus already has.

**X11 is also the one that could fail most informatively.** If bits and seconds
do not correlate at all, §1 is wrong, and a large part of why this project
thinks compression has anything to do with understanding goes with it. Worth
finding out early, and it is a week of work rather than a year.
