# 10 — Personality: the compression

*The manual for `bc-style`. This is the new centre of the project; everything
in `00`–`09` is either foundation for it or deferred behind it.*

## The object

A player is a **policy** — a function from positions to moves:

```
π_p : Position → Distribution over Moves
```

We never observe `π_p`. We observe *samples* from it: the moves actually played,
each paired with the position it was played in. A game between `p` and `q` is a
sequence of alternating samples from two policies, each conditioned on the state
the other produced.

**Compressing a player means finding a short description of `π_p`.**

Two consequences follow immediately and shape everything below.

1. **Elo is already a compression** — to about ten bits, discarding everything
   but strength. Personality is the *residual*: what is left after strength is
   accounted for. This is why the encoding is relative rather than absolute.
2. **A game is not two independent samples.** `π_p(move | position)` is the
   player alone; what a game gives you is `π_p(move | position, opponent)`. The
   gap is the interaction term (`METAPLAN` N12) — real, measurable, and the
   subject of episode 18.

## The pipeline

```
   PGN ──▶ features ──▶ standardise ──▶ basis ──▶ project ──▶ profile ──▶ medal ──▶ chain
          (per game)     (per corpus)   (per corpus)  (per game)  (per player)
```

Each arrow is one episode. Note where the *corpus* enters: standardisation and
the basis are fitted to a corpus, so **a point only means something relative to
the corpus it was projected under.** That is `METAPLAN` N10 in one sentence, and
it is why an identity is the pair `(corpus_root, position)`.

---

## Stage 1 — Features

A game yields **one feature vector per side**. Features are computed from the
game record alone: no engine, no evaluation function, no external oracle. This
is deliberate — an engine would import somebody else's idea of what chess *is*,
and the whole point is to discover the dimensions rather than assert them.

The v1 feature set, all measured over the moves that player actually made:

| # | Feature | Reads as |
|---|---|---|
| 0 | `capture_rate` | share of own moves that capture |
| 1 | `check_rate` | share that give check |
| 2 | `pawn_share` | share that move a pawn |
| 3 | `knight_share` | share that move a knight |
| 4 | `bishop_share` | share that move a bishop |
| 5 | `rook_share` | share that move a rook |
| 6 | `queen_share` | share that move the queen |
| 7 | `king_share` | share that move the king |
| 8 | `enemy_half_rate` | share landing in the opponent's half |
| 9 | `center_rate` | share landing on the sixteen central squares |
| 10 | `edge_rate` | share landing on an edge file or rank |
| 11 | `own_mobility` | mean legal moves available at own turn |
| 12 | `opp_mobility` | mean legal moves the opponent then had — **low means prophylactic** |
| 13 | `take_rate` | of turns where a capture was legal, how often one was made |
| 14 | `queen_dev` | normalised ply of first queen move (1.0 if never) |
| 15 | `castle_ply` | normalised ply of castling (1.0 if never) |
| 16 | `mean_material` | mean total material on the board — low means endgame-seeking |
| 17 | `length` | normalised game length |

Feature 12 is the one worth defending: it measures whether you *restrict* your
opponent, which is the difference between attacking chess and prophylactic
chess and is invisible in any purely self-referential statistic.

> **This list is `METAPLAN` O2 and is not settled.** It is an *asserted*
> compression — a human chose these axes — which is precisely why it is built
> first and then used as the oracle against which a learned embedding is
> checked (`METAPLAN` N8). If the learned model and this list disagree about a
> player, one of them is broken and the disagreement is the finding.

## Stage 2 — Standardise

Raw features have incompatible units. Each column is z-scored against the
corpus:

```
z_ij = (x_ij − μ_j) / σ_j          σ_j = 0  ⟹  z_ij = 0
```

`μ` and `σ` are properties **of the corpus**, not of the player, and must be
published with the basis for anyone to reproduce a projection.

## Stage 3 — The basis

Discovered, not chosen (`METAPLAN` N6). Form the covariance of the standardised
corpus and take its leading eigenvectors:

```
C = Zᵀ Z / (n − 1)                 C is symmetric, d × d
C = V Λ Vᵀ                          Jacobi rotation
basis = first k columns of V, ordered by descending eigenvalue
```

The eigenvectors are the **axes of chess personality as that corpus expresses
them**. Each is signed, so each has two poles, and both poles are meaningful —
`METAPLAN` N7. There is no "anti-" problem: an axis is a *tension*, and being at
the negative end of `sharp ↔ prophylactic` is not the absence of a style, it is
the other style.

Eigenvector sign is arbitrary in general, which would make the medal
non-deterministic. **Canonical rule:** flip each eigenvector so that its
largest-magnitude loading is positive. Ties broken by lowest feature index.

## Stage 4 — Naming the poles

An axis is named by its extreme loadings: the feature with the largest positive
loading names the `+` pole, the largest negative loading names the `−` pole. So
an axis reads `capture_rate ↔ opp_mobility`, which a human then relabels
`tactical ↔ prophylactic`.

Naming an axis after a *player* — the "chess gods" — requires that player's
games to be in the corpus. With a historical corpus, the name of a pole is
whichever player sits furthest along it. That is the intended end state; it is
a property of the corpus, not of the code.

## Stage 5 — The profile

A player's games are points. Their profile is the centroid, with the spread
retained because it is meaningful on its own:

```
profile_p    = mean over that player's game points
dispersion_p = mean distance from the centroid
```

Dispersion is *range* — a player who plays the same way every game and a player
who plays wildly differently have the same centroid and are not the same
player. Glicko's `RD` is the same quantity for the strength axis alone.

## Stage 6 — The medal

A hash must be over exact bytes, so coordinates are quantised onto a fixed grid
before commitment:

```
q_i    = round(coord_i / STEP)             STEP = 0.05, saturating at ±127
medal  = H_domain("BC/style/medal/v1", corpus_root ‖ k ‖ q_0 … q_{k−1})
```

The grid is not an implementation detail — **the quantisation step is the bit
budget of a personality**, and therefore sets how many distinct medals exist.
That makes `METAPLAN` O3 ("how many bits is a personality?") a question about
this constant, and it should be chosen from measured data rather than taste.

`corpus_root` is inside the hash. The same player under a different lens has a
different medal, by construction (`METAPLAN` N10).

## Stage 7 — The chain

A link is earned by **changing**, not by playing (`METAPLAN` N4):

```
emit a link  ⟺  ‖ profile_now − profile_at_last_link ‖ > DRIFT
```

Links form a hash chain, using `bc_hash::HashChain`:

```
link_n = H_domain("BC/style/link/v1", link_{n−1} ‖ medal_n ‖ n_games ‖ corpus_root)
```

So the head commits to the entire history of who you have been, which is
`spec/04`'s argument one level up: *a chain of hashes turns a statement about
one thing into a statement about everything that led to it.*

Properties this gives, and they are the whole point:

- **Unforgeable growth.** A long chain means many genuine changes of style.
  Playing a thousand games without changing earns exactly one link.
- **A readable narrative.** The sequence of profiles is a trajectory through
  personality space — the record of learning chess.
- **Verifiable by anyone.** Given the corpus and the published basis, any
  observer recomputes every medal and every link.

## What this deliberately does not do

- **It does not authenticate.** The medal proves nothing about who holds a key.
  Authentication is Ed25519 and is a separate mechanism (`METAPLAN` N2).
- **It does not use an engine.** No evaluation, no "blunder rate", no centipawn
  loss. Those import an external notion of chess quality; this layer measures
  *behaviour*, not correctness.
- **It does not resist a determined impersonator yet.** Whether you can forge a
  medal by playing like someone is exactly `METAPLAN` O3, and it is unanswered
  until the pipeline is run over real human games at scale.

---

## Stage 8 — Identification and forgery

Two attacks decide whether any of the above is worth building, and both are
empirical. The design cannot settle them; only a corpus can.

### The protocol

Attribution accuracy is the easiest number in this project to inflate by
accident, so the split happens **before the lens is fitted**:

```
   train games ──▶ standardise · basis · player centroids
   test games  ──▶ projected under the train basis, then attributed
```

The axes, the column statistics and the centroids are all functions of the
training half alone, so a held-out game is scored exactly as a genuinely new
game would be. `Lab::nearest_neighbour_accuracy` does **not** do this — it
scores every point against every other, including other games by the same
player, on data the basis already saw. It is a useful smoke test and a bad
measurement, and the honest estimate must always come out lower. There is a
test that fails if it ever does not.

### "I'll just start a fresh account"

If identity is derived from play, a new name converges back to the old
fingerprint after some number of games. That number is the entire content of
the claim that you cannot hide, and it is measured by drawing `n` held-out
games, averaging them into a provisional profile, and attributing it.

On the four constructed archetypes:

| Games observed | Identified correctly |
|---|---|
| 1 | 71% |
| 3 | 90% |
| 5 | 93% |
| **7** | **97%** |
| 10 | 99% |

Seven games. The confusion is not uniform, and the shape is the interesting
part: Petrosian — the only archetype rewarded for *restricting the opponent* —
is never once mistaken for anybody, while Tal, Morphy and Capablanca trade
misattributions freely. Three attacking players look alike; the prophylactic
one is unmistakable. That is a statement about which features carry identity,
and it is the argument for keeping `opp_mobility` in the set.

### "I'll play like you and steal your medal"

A medal is a commitment to *quantised* coordinates, so forging one means
landing in the same grid cell. Two players inside one cell mint the same medal
and **the collision happens before the hash**, where no cryptography helps.

So the security parameter is the distance between players in units of `STEP`.
On the constructed corpus the closest pair — Morphy and Capablanca — sit **39
cells** apart, so there is no collision at `STEP = 0.05`. That is a property of
this corpus, not of chess.

### How many bits is a personality?

The number of distinct medals is bounded by the product of the cells each axis
spans. On this corpus that is **30.7 bits**.

Treat that as a method rather than a result. It is an *upper* bound and a loose
one: it counts the cells the cloud spans, not the cells anyone occupies, and it
says nothing about how a real population distributes inside them. Four
synthetic players cannot speak for humanity. The number that matters is this
same measurement over a real corpus, and until it exists every claim about
medal forgeability is provisional — `METAPLAN` O3.
