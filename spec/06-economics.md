# 06 — The economics and the mathematics of the wager

> **PARTLY DEFERRED — see `METAPLAN.md`.** §2–4 and §7–8 (handicap odds, rake,
> Kelly, where the money is) are finance and are deferred with `METAPLAN` N17.
> §1 survives and is now load-bearing for a different reason: Elo as a Boltzmann
> distribution is the observation that **a rating is already a lossy compression
> of a player**, which is what `spec/10-personality.md` generalises. §5 (cheat
> detection as a hypothesis test) survives as attribution failure.

This file contains the parts you will want on one page in your notes.

---

## 1. Elo is a Boltzmann distribution

The Elo expected score:

```
        E_A  =  ───────────────────
                 1 + 10^(−ΔR/400)
```

Rewrite base 10 as base e:

```
        E_A  =  ─────────────────────────           ΔR = R_A − R_B
                 1 + exp(−ΔR·ln10/400)
```

Now compare a two-state Boltzmann distribution at temperature `T`:

```
        p_A  =  ─────────────────────────
                 1 + exp(−(E_B−E_A)/kT)
```

They are the same function. The identification is:

> **Rating is negative energy. The temperature of chess is `kT = 400/ln 10 ≈ 173.7` Elo.**

A player 173.7 points stronger is favoured by exactly a factor of `e`, the same
way a state one `kT` lower in energy is `e` times more populated. The number 400
was chosen by Arpad Elo so that 400 points means 10:1 odds; it is a units
choice, exactly like choosing to measure energy in kcal/mol.

Corollaries worth writing down:

- Rating differences are **log-odds**. One Elo point = `ln10/400 = 0.005756`
  nats = `0.0083` bits.
- The logistic curve is not an empirical fit that happens to work; it is what you
  get from any model where "strength" is additive on a log-odds scale.
- **Logistic regression** is exactly the problem "given outcomes, infer the
  energies". Elo's update rule `R ← R + K(S − E)` is one step of stochastic
  gradient descent on the log-likelihood of that model, with learning rate `K`.

That last point means Elo is not an ad-hoc heuristic — it is online gradient
descent, invented in 1960 by a physics professor, which is not a coincidence.

---

## 2. Fair odds for unequal players

Two players stake `s_A` and `s_B`. Winner takes the pot; a draw returns stakes.
Let `w_A`, `w_B`, `d` be the win/win/draw probabilities, `w_A + w_B + d = 1`.

```
EV_A  =  w_A·(+s_B)  +  w_B·(−s_A)  +  d·(0)
```

Setting this to zero:

```
        s_A       w_A
        ───   =   ───            ← the draw probability cancels out entirely
        s_B       w_B
```

**The draw rate does not affect fair odds.** Draws return stakes, so they carry
no money; only the win:loss ratio matters. This is a small, satisfying result and
it makes the handicap rule clean.

Under the Davidson model (Bradley–Terry extended with ties), the win:loss ratio
is exactly the Elo ratio:

```
        s_A                                 ΔR = R_A − R_B
        ───   =   10^(ΔR/400)
        s_B
```

> **The favourite stakes 10× more per 400 Elo.**

| ΔR | stake ratio | e.g. underdog stakes 100 |
|---|---|---|
| 0 | 1 : 1 | 100 |
| 100 | 1.78 : 1 | 178 |
| 200 | 3.16 : 1 | 316 |
| 400 | 10 : 1 | 1000 |
| 800 | 100 : 1 | 10 000 |

Note the honesty problem this creates: the odds depend on ratings, and ratings
come from a **server** (`07-servers.md`), so handicapped games inherit that
server's trustworthiness. Equal-stake games do not. This is the main reason to
keep equal-stake pools available as the trust-minimal default.

---

## 3. What the rake costs you, in Elo

Equal stakes `s`, rake `r` on the pot, draws refunded net of pro-rata rake:

```
EV_A  =  s·[ w_A(1−2r) − w_B − d·r ]
```

Break-even requires `w_A(1−2r) = w_B + d·r`. With no draws this simplifies to

```
        w_A          1
        ───   =    ─────
        w_B        1−2r
```

Converting to Elo, `ΔR* = 400·log₁₀(1/(1−2r))`:

| Rake | ΔR* at d = 0 | at d = 0.3 | at d = 0.5 |
|---|---|---|---|
| 1% | 3.5 Elo | 5.1 | 7.1 |
| 2% | 7.1 Elo | 10.1 | 14.2 |
| 5% | 18.3 Elo | 26.1 | 36.4 |

Two things to take from this table:

1. **A 2% rake costs about 7–14 Elo.** Any player whose edge over the field is
   smaller than that is a net donor, no matter how well they play.
2. **The more drawish the level, the worse the rake hurts.** You pay a fee on
   games that transfer no money. At grandmaster draw rates a 5% rake costs 36
   Elo, which is the difference between two serious titled players. This is why
   real chess prize structures are not per-game rakes.

Design consequence: cap `rake_bps` in consensus. The spec sets the maximum at
**500 bps (5%)** and expects competitive servers to sit near 50–100 bps.

---

## 4. Kelly, and why wagering is literally a communication channel

You have an edge `p` (probability of winning a even-money bet). What fraction
`f` of your bankroll should you stake?

Maximise the expected log growth rate:

```
G(f) = p·log₂(1+f) + (1−p)·log₂(1−f)
```

`dG/df = 0` gives the **Kelly fraction**:

```
f* = 2p − 1        (= your edge, doubled)
```

Substituting back:

```
G(f*) = 1 − H₂(p)          bits per game

where  H₂(p) = −p log₂ p − (1−p) log₂(1−p)
```

Now the punchline. The capacity of a binary symmetric channel with crossover
probability `ε` is `C = 1 − H₂(ε)`. These are the same expression.

> **Your maximum bankroll growth rate, in bits per game, equals the information
> capacity of the channel between your skill and the outcome.**
>
> Money is the physical instantiation of information you have and the market
> doesn't. If you know nothing (`p = 0.5`) the channel capacity is zero and no
> betting system can help you. Kelly (1956) proved this in a paper published in
> the *Bell System Technical Journal*, one office down from Shannon.

### The table that should govern your expectations

| Win rate `p` | Edge in Elo | Kelly `f*` | Growth (bits/game) | Games to double |
|---|---|---|---|---|
| 51.0% | 7 | 2% | 0.00029 | **≈ 3 500** |
| 51.02% (2% rake break-even) | 7.1 | 2% | 0.00030 | ≈ 3 300 |
| 55% | 35 | 10% | 0.0072 | ≈ 138 |
| 60% | 70 | 20% | 0.029 | ≈ 34 |
| 70% | 147 | 40% | 0.119 | ≈ 8.4 |
| 75% | 191 | 50% | 0.189 | ≈ 5.3 |

Read the first row carefully. A player who is 7 Elo better than the field —
which is a real, genuine edge — needs **thousands of games** to double their
money at optimal sizing. Near-even wagering is an almost-zero-capacity channel.

This is the single most important economic fact about the whole project, and it
has a direct design consequence: **the interesting games are not the even ones.**
Value comes from mismatches priced by handicap odds, from tournaments with
structured payouts, and from bot-vs-bot arenas where edges are large. A platform
of evenly-matched players paying rake is a machine that slowly converts everyone's
money into rake.

### Risk of ruin

Betting more than Kelly is worse than betting less: `G(f)` is concave with
`G(2f*) = 0`. Betting exactly double the Kelly fraction has **zero** long-run
growth despite a positive edge per bet. Most practitioners use half-Kelly, which
gives 75% of the growth for half the variance.

Client software should show the Kelly fraction and refuse, by default, to let a
user stake more than half-Kelly of their balance on one game. This is a UX
decision with a theorem behind it.

---

## 5. Detecting cheating is a hypothesis test

**Nothing cryptographic prevents a human from consulting an engine.** The human
is outside the protocol. Detection is therefore statistical, and the right frame
is Wald's sequential probability ratio test.

For each move, compare two models:

- `P₀` — the move distribution of an honest player at this rating, in this
  position type, with this time remaining.
- `P₁` — the move distribution of an engine-assisted player.

Accumulate the log-likelihood ratio:

```
Λ  =  Σ  ln [ P₁(move_i) / P₀(move_i) ]
```

Under `H₁` this drifts upward at an average rate equal to the Kullback–Leibler
divergence `D_KL(P₁ ‖ P₀)` per move. Accuse when `Λ` crosses
`ln((1−β)/α)`.

```
                    ln((1−β)/α)
   moves needed  ≈  ────────────
                      D_KL
```

With `α = 10⁻⁶` (one false accusation per million clean players) and `β = 0.05`:
threshold `≈ 13.8` nats.

| Cheating behaviour | `D_KL` per move | Moves to detect | ≈ games |
|---|---|---|---|
| Engine every move | 0.10 | 138 | 3–4 |
| Engine 20% of moves | 0.020 | 690 | ~17 |
| Engine on 3 critical moves per game | 0.004 | 3 450 | ~86 |

> **Detection time scales as 1/D_KL.** A cheater who uses the engine on a
> fraction `f` of moves takes roughly `1/f` times longer to catch. This is why
> selective cheating is the hard case, and why any platform claiming to reliably
> catch occasional cheaters is overstating.

The signal is not only move choice. **Time per move is at least as informative**:
honest players think longer in complex positions, and the correlation between
position complexity and time spent is very hard to fake. This is the same
observation that makes side-channel attacks work in hardware.

Practical note: `P₀` must be conditioned on rating, time control, and position
type, which means you need a large corpus of honest games to fit it. This is
exactly what the user's *public-history servers* provide, and it is the strongest
argument for them: **transparency is not a nice-to-have here, it is the training
set for the detector.**

---

## 6. Sybil resistance and identity bonds

Detection takes `N` games. In those games a cheater with edge `e` extracts, at
stake `s`:

```
profit  ≈  N · s · 2e
```

Deterrence requires the cost of a fresh identity to exceed that:

```
B  >  N · s · 2e
```

With `N = 20`, `s = 10` tokens, `e = 0.25` (an engine against a club player is a
huge edge): `B > 100` tokens. **A cheat-resistant pool must bond identities at
roughly ten times the typical stake.** That is an unpleasant number and it is
the real reason open-entry money chess is hard.

Alternatives that change `N`, `s`, or `e` instead of `B`:

- Lower `s` — micro-stakes pools are barely worth cheating.
- Lower `e` — segregate by rating so an engine's edge over the pool is smaller
  (this does not work; an engine's edge over anyone is enormous).
- Lower `N` — better detection, which is `06.5` above.
- **Set `e ≈ 0` by allowing engines.** This is the bot arena, and it is the only
  option on this list that fully works.

---

## 7. The rule that prevents most of the abuse

> **Never mint tokens for playing games.**

No play-to-earn emissions, no per-game rewards, no "activity mining". The moment
playing pays, the optimal strategy is to run two accounts against each other and
harvest, and every such scheme in history has been farmed within days.

Tokens enter circulation only through whatever issuance the chain uses for
validator rewards. Value moves between players and to servers as rake. The
system is a **zero-sum transfer layer with a small negative-sum fee**, and it is
much healthier for being honest about that.

---

## 8. Where the money actually is

Given §4, the honest answer about where sustainable value exists:

1. **Tournaments.** Structured payouts concentrate variance into a top-heavy
   distribution, which is what makes entry fees psychologically and
   mathematically worth paying.
2. **Handicap markets.** The odds mechanism in §2 lets a strong player and a
   weak player both take a fair bet. Mismatched games have high channel capacity
   for whoever is correctly priced.
3. **Bot arenas.** Engine-vs-engine with real stakes, where the edge between two
   engine authors can be hundreds of Elo. Legitimately high capacity, zero
   cheating problem by construction, and a real market for engine development.
4. **Servers.** Rake on volume is a real business; playing is not.

The bot arena is the one to build first. It has no anti-cheat problem, its
participants are technical, its edges are large enough for the mathematics to
actually work, and the games run themselves at any hour.
