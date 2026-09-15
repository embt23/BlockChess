# D20 — the measurement

> `cargo run --release -p bc-style --bin measure -- <corpus-dir> [n_players]`

`docs/d20-calibration.md` built the instrument and measured its bias. This is
it pointed at humans.

## Verdict: **AMBER**

Against the threshold signed into `spec/09` D20 **before** the run:

| | 40 players | 150 players |
|---|---|---|
| `D̂` raw, mean | 0.0061 | **0.0079** |
| `D̂` raw, median | 0.0042 | 0.0058 |
| `D` corrected (÷0.60) | 0.0101 | **0.0132** |
| recovery interval | [0.0063, 0.0264] | [0.0081, 0.0343] |
| identification | 26/40 = 65% (chance 2.5%) | **73/150 = 48.7%** (chance 0.7%) |
| **verdict** | AMBER | **AMBER** |

Stable across both. The corrected estimate lands at **0.013 nats/move**,
between the `spec/09` cuts of 0.005 (dead) and 0.02 (holds), so:

> **Act II survives. Every timescale in `spec/10` and `spec/13` was written
> against `0.05` nats/move — the midpoint §7 actually computes with — which is
> roughly **four times** what this measures, and needs rewriting.**

The recovery interval spans the amber/holds boundary at its top end, so the
run flags itself as not unambiguous — a player at the best-recovered end of
the calibration would score as holding.

## The corpus

`rozim/ChessData`, `mega2600_part_*.pgn`. Chosen because `lichess.org` and
`database.lichess.org` are both refused by this session's egress policy (403
at the proxy) and `raw.githubusercontent.com` is not.

- **138,188 games** parsed of 138,348 present
- skipped: 157 too short, 3 missing a player name, **0 unparseable**
- **349 players with ≥200 games**, the density the calibration requires
- 12.3M decisions total; the run uses ~318k for `π_pop` and ≤400 games/player

The zero unparseable count is worth its own sentence. `samples_from_game`
rejects any move that is not legal in its position, so a SAN misparse fails
within a ply or two. Parsing 138,000 master games with no failures exercises
`bc-chess`'s move generator and the SAN reader against each other across
every opening, every underpromotion and every disambiguation a century of
master play contains. `perft` proves the generator counts correctly; this
proves it agrees with what humans actually wrote down.

## Three reasons this is a lower bound

All push the same way, and the third is a limitation of the calibration
itself rather than of the run.

**1. The population is masters, not people.** Stated in `spec/09` before the
number existed, because it cuts against the thesis. Everyone here is 2600+,
and masters resemble each other far more than a mixed-rating online field
does, so `D(π_you ‖ π_pop)` measured *within* this population is smaller than
the same quantity on Lichess. This is the conservative direction: clearing
the threshold here would have been strong evidence, and failing to clear it
is correspondingly weak evidence against.

**2. The feature set is 806 sparse indicators.** Piece, from-square,
to-square, victim, check, promotion, castling, phase. A policy does things
this cannot express — pawn structure, king safety, the shape of a plan — and
divergence the model cannot represent is divergence it cannot measure.

**3. The recovery factor is itself optimistic, and structurally so.** The 0.60
came from synthetic players generated *by the same log-linear family* the
estimator fits. Those players live inside the model's hypothesis class, so the
only losses measured were from regularisation and finite data. Real humans do
not live inside it, which adds **model misspecification** as a second source
of under-recovery — and one the calibration cannot see, because measuring it
would require a human whose true policy is known in closed form, which is
exactly the thing that does not exist.

So 0.60 is an upper bound on recovery for real data, and **0.0132 is a lower
bound on a lower bound.** The honest reading of AMBER here is "amber, and the
true figure is plausibly at or above the holds line."

## What is strong: identification

**48.7% correct out of 150 candidates, against 0.7% chance. Seventy times
chance**, from held-out games, using only `δ`.

This is the cheap reproduction of McIlroy-Young et al. (KDD 2022) and it is
the result that matters most for Act II, because the products in `spec/11`
are identity products. Picking one player out of 150 from games the model
never saw is the operation a cheat detector and a style asset both perform.

It also sharpens what AMBER means. Identification being excellent while
per-move divergence is middling is not a contradiction — they measure
different things. Identification is *comparative*: it only needs each `δ` to
be more like its owner than like 149 others, and small distinctive
differences suffice. Divergence is *absolute*, in nats, and is what sets
detection time. The information is demonstrably there; the question the
threshold asks is how fast it accumulates.

## Detection time, and an error in the table

`spec/13`'s detection time is `≈ ln(1/α)/D_KL`. At α = 0.001 and ~39 of a
player's own moves per game — the convention `spec/06` §5 already used:

| `D` nats/move | own-moves | games |
|---|---|---|
| 0.0200 (holds cut) | 345 | 9 |
| **0.0132 (measured)** | **523** | **13** |
| 0.0079 (raw, uncorrected) | 874 | 22 |
| 0.0050 (dead cut) | 1382 | 35 |

At the α = 10⁻⁶ that `spec/06` §5 uses to *accuse* rather than flag, the
threshold is 13.8 nats instead of 6.9, and the measured figure gives **1,043
moves ≈ 27 games**. Those are two different products: 13 games is what a
flagging pipeline runs on, 27 is what a banning decision needs.

**So a cheat detector needs ~13 games at α = 0.001**, not the ~35–140 the
`spec/09` table's amber row claimed. That column was wrong: its range spans
`D` from 0.005 down to 0.0013, and 0.0013 is *inside* the dead band, not at
the edge of amber. Corrected in `spec/09` with the derivation shown. The
verdict is unaffected — the cuts are on nats/move, not on the game counts —
but the practical conclusion changes a good deal, because 13 games is a
product and 140 is not.

## What did not work

Six of the 40 players and a similar fraction of the 150 measure **negative**:
Carlsen −0.0030, Aronian −0.0044, Ivanchuk −0.0043, Yu Yangyi −0.0057. A
negative `D̂` means the adapted model did worse on held-out games than the
population model did, having beaten it on validation.

That is variance, not a bug: ~3,500 test decisions per player, and the
selection of `λ` on a validation split of similar size. Three more players
measured exactly 0.0000, which is the estimator declining to claim a
personality at all — `δ = 0` winning on validation. Both behaviours are the
honest ones and both were present in calibration.

It does mean the per-player figures are noisy and only the aggregate should
be read. Widening to 150 players moved the mean *up* (0.0061 → 0.0079), which
is the direction more data should move it if the negatives are noise around a
positive mean rather than a real cluster of styleless players.

## What this does to the rest of the spec

`spec/06` §5 now carries the measured row beside its three assumed ones, and
labels which is which. The consequence worth stating on its own:

> **The cheat detector and the style asset are the same object, and not the
> same speed.** "You are consulting an engine" rides ~0.10 nats/move
> (assumed); "you are not who you say you are" rides 0.0132 (measured). Eight
> times apart. A design that assumes account-sharing is caught as fast as
> engine use is wrong by that factor.

Only one of those two numbers has been measured, which points at the cheapest
experiment left in this area: `bc-style` can measure the engine rows exactly
as it measured the identity row — generate games from an engine at fixed
depth, treat it as a player, read off `D(π_engine ‖ π_pop)`. Estimator,
corpus and harness all exist.

## What would sharpen it

In rough order of value per hour:

1. **A mixed-rating corpus.** The single largest known bias. Lichess would
   settle it; any corpus spanning ratings would help.
2. **A richer feature set**, or Maia as a cross-check. Addresses reason 2 and
   partly reason 3.
3. **More games per player.** The calibration showed recovery still climbing
   at 5,000 decisions and not plateaued; the cap here is 400 games.
