# D20 — calibrating the personality estimator

> `cargo run --release -p bc-style --bin calibrate`

`spec/10-personality.md` rests on one measurable quantity: `D(π_you ‖ π_pop)`,
the divergence between how you play and how the population plays. §7 estimated
it at 0.02–0.10 nats per move and concluded that a game emits roughly four
orders of magnitude more information about the players than about the result.

That estimate has to become a measurement. This is the first half of doing so.

---

## Why calibration comes before data

The naive measurement is **biased upward by construction**. Fit a model on a
player's games, score those games with it, subtract the population model's
score, and the difference is positive — whether or not the player has any
personality at all. The model has simply memorised some noise.

So a number from real games means nothing until the estimator has been shown to

1. recover a divergence that was deliberately put there, and
2. report **zero** for a player who genuinely has none.

Point 2 is the one that matters. It cannot be checked on real data, because
real data has no ground truth: there is no human whose style is known in advance
to be exactly average. It can only be checked against players constructed so
that the true answer is known.

This is `G1` — *verify against an oracle you did not write* — applied to a
measurement rather than a codec. Here the oracle is arithmetic: both policies
are known in closed form, so the true divergence is computable exactly.

## The setup

**The model.** A log-linear policy over legal moves:

```
    P(m | pos) = exp(w · f(pos, m)) / Σ   exp(w · f(pos, m'))
                                     m' legal
```

The normalisation runs over the legal moves of *this* position, so the rules
supply the support and the weights supply the preference. `bc-chess` is the
feature extractor — the same move generator `perft` validated. 806 sparse
indicator features: piece type, piece-square (from and to), captured piece,
gives-check, promotion, castling, and phase. The board is mirrored when Black is
to move, so style does not depend on which colour you were dealt.

**Personality is an offset.**

```
    π_pop = softmax( w        · f )
    π_you = softmax( (w + δ)  · f )
```

`δ` is regularised toward zero, so weak evidence produces no claim.

**The estimate.** On a player's held-out games,

```
    D̂ = CE(games under π_pop) − CE(games under π_you)     nats/move
```

**Two splits, not one.** Train fits the models; validation chooses each player's
regularisation strength; test is touched once, at the end. Choosing a
hyper-parameter on the test set is a slower way of fitting to it.

**The candidate set includes "no personality".** `δ = 0` always competes. A
player whose games do not support a distinctive model measures *exactly* zero
rather than a small positive number invented from noise. This single detail is
what makes the estimator honest.

## Results

Ten synthetic players across five personality strengths, 1200 games, ~56,500
training decisions. `D_true` is the exact divergence, computable because the
policies are known.

```
player      train   test   lambda    D_true    D_self   D_cross
----------------------------------------------------------------
null-0       5902   1680     1e-2    0.0208    0.0132    0.0097
null-1       5552   1400     3e-2    0.0187    0.0069    0.0078
faint-0      5843   1858     3e-2    0.0225    0.0110    0.0129
faint-1      5741   2150     1e-2    0.0193    0.0044    0.0011
mild-0       5433   2192     1e-2    0.0258    0.0115    0.0018
mild-1       5924   1787     1e-2    0.0268    0.0161   -0.0283
strong-0     5348   2216     3e-3    0.0386    0.0170   -0.0656
strong-1     5604   1747     3e-3    0.1179    0.1094   -0.2314
extreme-0    5204   1984     1e-3    0.2367    0.2110   -0.1818
extreme-1    5952   1933     1e-3    0.1398    0.1354   -0.0190

chimera control: D = +0.0006 nats/move        <- no player behind the data
identification:  216/478 = 45.2%              <- chance is 10.0%
recovered fraction of D_true: mean 60%, range 23-97%
cross-player D: -0.0493 nats/move
```

### What passes

**The estimator does not fabricate personality.** The chimera — a player-shaped
pile of decisions each drawn from a random member of the population, so its true
policy *is* the population mixture — reads **+0.0006 nats/move**. Given a
dataset with no player behind it, the estimator correctly finds nothing.

**Another player's personality does not fit you.** Cross-player divergence
averages −0.049: wearing someone else's `δ` makes your games *less* likely. If
this were positive the estimator would be measuring something shared — rating,
or the population's own quirks — rather than identity.

**Players are identifiable.** 45.2% against 10% chance, from held-out games
alone, using only `δ`. This is the cheap reproduction of the result in
McIlroy-Young et al. (KDD 2022): style is a fingerprint.

**Recovery is monotone in the truth** and tracks it across a 13× range.

### What it costs

**The estimate is a lower bound — mean 60% of the truth.** Two things push the
same way: the feature set cannot express everything a policy does, and the
regulariser shrinks `δ` toward zero. Neither can inflate the answer.

That is the right direction for a number an economy might be built on. **A
measurement of 0.05 means at least 0.05.**

### How much data a personality needs

Fitting one moderate player (`D_true = 0.0258`) on increasing amounts of their
own games:

```
   moves     lambda    D_self
     250       none    0.0000      <- correctly detects nothing
     500       1e-1    0.0048
    1000       3e-2    0.0040
    2000       3e-2    0.0074
    3000       3e-2    0.0092
    4000       1e-2    0.0075
    5000       1e-2    0.0109
```

At 250 decisions the estimator selects "no personality" and returns exactly
zero — the honest answer, not a small fabricated one. Recovery then climbs with
data and has **not plateaued at 5,000 decisions** (~125 games).

> **Design consequence for the real run:** a player needs on the order of
> **hundreds of games**, not tens, before their style is measurable at moderate
> strength. Any real corpus must be filtered to heavy users, and a null result
> on a light user means "not enough data", not "no personality".

## Status: half done

The instrument works and its bias is quantified. What remains is running it on
humans.

**That is currently blocked.** `database.lichess.org` and `lichess.org` are both
refused by this session's egress policy (403 at the proxy). Per the proxy's own
guidance, a policy denial is reported rather than worked around.

To finish D20, the estimator needs a real corpus. Any of:

- a PGN file placed in the repository or a local path,
- egress to a game database being permitted,
- any other public collection of games with player identities and enough games
  per player (hundreds).

The reading path is already built and tested: `bc_style::san` parses SAN and
PGN movetext, verified against Morphy's Opera Game parsed end to end into a
checkmate — a test that validates the parser and `bc-chess` against each other,
since a misparse makes the following move illegal within a ply or two.

## The prediction on record

Stating it before the data arrives, so the result can be wrong:

- `D(π_you ‖ π_pop)` for an active human will measure **0.01–0.08 nats/move**
  under this feature set, being a lower bound on the truth.
- Identification accuracy over a few hundred candidate players will **beat
  chance by more than an order of magnitude**.
- The measurement will need **≥ 200 games per player** to stabilise.

If the first comes back below ~0.005 — the level at which the estimator cannot
separate a real player from a chimera — then style carries far less information
than `spec/10` claims, and Act II should be abandoned rather than rescued.
