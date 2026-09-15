# Episode 08: what was decided, and what shipped

`spec/09` D21–D25 were settled on 2026-09-14. Episode 08 was written and
merged on the same day, without them. Four of the five decisions are not met
by the code on `main`.

This file exists so that the disagreement is written down somewhere rather
than discovered by whoever next opens `bc-channel::dispute`. **No code has
been changed to resolve it** — that is a decision, not a cleanup, and it is
Evan's.

---

## The gap

| | Decided | Status | Cost of fixing later |
|---|---|---|---|
| **D21** | PoW first, so Milestone E is earned | ⬜ stub `BTreeMap` height counter | low — the dispute logic is pure in `height: u64`, so a real chain substitutes underneath it |
| **D22** | Δ, τ from a time-control class table | ✅ **done** — `crates/bc-channel/src/timecontrol.rs` | — |
| **D23** | differential-test the terminal predicates; model-check the state machine | ⬜ spec's worked table, hand-chosen positions | low — additive |
| **D24** | `bc-adjudicator`, `no_std`, depending only on `bc-chess` | ⬜ `bc-channel::dispute` + `ledger::adjudicate` | moderate — a move, not a rewrite |
| **D25** | `adjudicator_ver` = integer + ruleset hash in state | ✅ **done** — `crates/bc-channel/src/ruleset.rs` | — |

**The `P3` cost bug below is also fixed** (`build-log` §12): the chain no
longer calls `Position::outcome()` on every on-chain move. Mate and stalemate
are claimed by the mover, optimistically and refutably, with the claim riding
along on `DisputeMove` exactly as `spec/05` always allowed.

## D22 — done

Kept because the reasoning is the transferable part.



Every other row can be fixed whenever. D22 cannot, and D22's own text says so:

> This is consensus-visible, so unlike D15 it cannot be deferred and
> retrofitted: whatever shape `GameTerms` has at first testnet is the shape
> signed channels commit to.

`GameTerms` is inside `channel_id`, which is inside every `GameState`, which
is what both players sign every ply. Changing its shape after channels exist
invalidates every signature over the old shape. Before a testnet this costs
one afternoon; after one it is a migration with money in it.

What shipped:

```rust
pub struct GameTerms {
    pub delta_blocks: u32,    // free-form
    pub budget_tau_ms: u32,   // free-form
    ...
}
```

`offer.rs` enforces `delta_blocks >= MIN_DELTA_BLOCKS` (64) and nothing else,
which is exactly the single-band defence D22 rejects: a band wide enough for
correspondence contains hostile values for bullet.

The attack *was* live: an opponent proposes Δ = 64 for a bullet game, a client
that does not check accepts it, and the victim has ~2 minutes to get a move
on-chain or forfeit the pot. `MIN_DELTA_BLOCKS` did not help — 64 *was* the
minimum, and it is hostile at bullet time controls.

Closed by `TimeControl`: the class is named on the wire and checked against
`base_time_ms + 40·increment_ms`, so relabelling a bullet game as
correspondence is not a valid offer. Tests
`a_bullet_game_cannot_be_dressed_as_correspondence` and
`a_relabelled_offer_never_opens_a_channel`. The five `(Δ, τ)` pairs are a
proposal awaiting Evan under `G0`.

## D21 and the Milestone E claim

D21's argument is correct and the status table has been changed to match:

> Milestone E says *"you can win against an opponent who disconnects."*
> Against a stub height counter that sentence has not been earned — it has
> been simulated.

`cargo run --bin dispute` passes `1_000`, `1_001`, and a large number as
block heights to a `BTreeMap`. Nothing produces those heights, nothing can
reorg them, and nothing can censor a transaction at them. The dispute logic
is right; the thing it is right *about* is a driven counter.

`CLAUDE.md` now reads **"Milestone E demonstrated, not earned"**. That is the
only change made in response to this gap.

Worth keeping, because it is the counter-argument D21 weighed and rejected:
the dispute state machine is pure in its `height` parameter, so a driven
counter tests it *more* thoroughly than a real chain — every deadline can be
stepped over exactly, which no real chain lets you do. The code is better
tested for having been built this way. It is the *claim* that is unearned,
not the implementation.

## D24, and the thing it buys

Moving the adjudicator to its own crate is not tidying. D24's second
paragraph is the point:

> a client that links the adjudicator can run an entire dispute *locally
> before spending any gas* — simulate posting its best certified state, see
> the deadline it would get, see whether its mate claim survives refutation.

The pieces already exist — `Dispute` is a pure state machine, and
`Channel::evidence()` returns exactly what `DisputeOpen` takes. What is
missing is the crate boundary that makes the determinism lints (`no_std`, no
float, no hash-map iteration order) enforceable rather than remembered.

Note one thing that has to move with it: `Dispute::apply_move` calls
`Position::outcome()`, which calls `generate_legal()`. That is the `∀` `P3`
says the chain must never compute. It is currently reached on every on-chain
move to detect mate-on-the-board, and in a `no_std` consensus crate it should
be reached only through an optimistic claim. This is a real finding and it is
not in any of D21–D25: **the shipped adjudicator computes the quantifier P3
exists to avoid.** It is correct, it is just not cheap, and on a real chain
it is the difference between a dispute move costing one check test and
costing 218.

## Remaining order

1. ~~**D22**~~ — done.
2. ~~**The `P3` violation**~~ — done.
3. ~~**D25**~~ — done.
4. **D24** — the move, with the lints, now that `GameTerms` is settled.
5. **D23** — the split oracle: `shakmaty` for the terminal predicates,
   `stateright` for the dispute machine. Both are on crates.io and reachable.
6. **D21** — the PoW week, which converts "demonstrated" into "earned".
