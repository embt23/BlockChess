# The `G0` holes

`spec/09` D26, recorded as `G0`: **the person filming types the code the
episode is about.** Evan writes the episode's subject — the arithmetic, the
rule, the check. Claude writes plumbing, tests, serialisation and oracle
harnesses.

This file is the list. Four functions are open across three episodes, each
the filmed subject of one. Everything around them is finished, and the tests
that specify them are written.

```sh
cargo test -p bc-pow -- --ignored     # episode 05
cargo test -p bc-bft -- --ignored     # episodes 06 and 10
cargo test -p bc-node -- --ignored    # episode 10's consumer
```

Both suites are red and are supposed to be. CI runs them in a
`continue-on-error` job so the open work is visible on every push rather
than living in somebody's memory. When a hole closes, delete the
`#[ignore]` attributes in the same commit as the implementation, and when
both are closed delete the `g0-holes` job.

---

## Episode 05 — difficulty as a control loop

**File:** `crates/bc-pow/src/retarget.rs`
**Function:** `next_target(prev: Target, observed_ms: u64) -> Target`
**Tests:** 6, all `#[ignore]`d

Bitcoin's retarget is a proportional controller with gain 1, sampling every
2016 blocks, on a plant with enormous delay. Written as control theory
rather than as folklore:

```text
error      = observed_ms / expected_ms
new_target = prev_target × error        (clamped to ×4 and ÷4)
```

It is marginally stable and it oscillates under a hashrate step, which is
not a flaw anyone introduced — it is what a P-controller with gain 1 and a
one-sample delay does. EIP-1559's base fee is the same shape with a much
shorter sampling period, which is why it tracks where Bitcoin's hunts.

### What the tests require

| Test | What it catches |
|---|---|
| `a_perfectly_paced_window_leaves_the_target_alone` | a loop that drifts at zero error |
| `slow_blocks_make_mining_easier_and_fast_blocks_make_it_harder` | the sign, which is one character |
| `the_target_is_monotone_in_the_observed_window` | a region where mining slower makes it harder |
| `a_single_absurd_timestamp_cannot_move_the_difficulty_far` | miner-supplied timestamps moving a consensus parameter arbitrarily |
| `the_target_never_leaves_the_mineable_range` | a zero target, which halts the chain forever |
| `a_hashrate_step_settles_instead_of_running_away` | the closed loop, with a 4× step partway through |

The last one is the episode. It requires convergence, **not** elegance — a
gain-1 P-controller passes it with visible ringing, and the ringing is the
thing worth discussing on camera.

### Already decided, so not in the hole

- `Target` arithmetic including the overflow behaviour of `scale`, which is
  the part that actually bites (`target.rs`).
- `WINDOW = 32` blocks, and what `observed_ms` means over it.
- The clamp bounds, `MAX_STEP_UP` and `MAX_STEP_DOWN`, both 4.
- `clamp_ratio`, because the clamp is a decision already made while the law
  is the episode.

`Difficulty::Controlled` panics with `"G0"` on crossing a window rather than
falling back to something reasonable. A silent fallback is how an episode's
subject stops being anyone's job.

---

## Episode 06 — the locking rules

**File:** `crates/bc-bft/src/locking.rs`
**Functions:** `prevote_for` and `lock_on_quorum`
**Tests:** 11, all `#[ignore]`d

### Why the seam is where it is

A single round of Tendermint needs no locking at all. Propose, collect a ⅔
prevote quorum, collect a ⅔ precommit quorum, commit — already safe by
quorum intersection, as long as it *completes*. `bc-bft` implements all of
that and four validators commit blocks over a reordering network with
`locking.rs` untouched.

Locking exists for one reason: **rounds can fail.** Every line in that file
is there because of that, which makes it exactly the content of the episode
— FLP says the failure cannot be ruled out, partial synchrony says you
survive it with timeouts, and this is what you must remember across one to
stay safe.

### The rule, in words

A validator holds a lock — a `(round, block)` pair recorded when it last
precommitted a block rather than nil. Having precommitted, it has told the
network that block may already have been committed by someone who saw a
quorum it did not. So it must not prevote a different block in a later
round; and it must be able to release the lock, or one crashed proposer
freezes the chain forever.

Tendermint releases it on evidence: a proposal carrying a ⅔ prevote quorum
from a round **strictly later** than the one the validator locked in.

### The two that matter most

- `a_locked_validator_never_prevotes_a_conflicting_block_without_proof` —
  the safety rule. If only one test passes, it should be this one.
- `a_prevote_quorum_from_before_the_lock_releases_nothing` — the one a
  plausible implementation fails. The condition is
  `valid_round > lock.round`, not `valid_round.is_some()`.

### Already decided, so not in the hole

- `Lock` and `Proposal`, their shapes and what they carry.
- That the proposer re-proposes its own locked value (`Node::start` does).
- That a released lock is *replaced*, never merely cleared.

---

## Episode 10 — the censorship bound

**File:** `crates/bc-bft/src/censorship.rs`
**Functions:** `max_byzantine_run` and `window_is_safe`
**Tests:** 7 here, plus 3 in `crates/bc-node/src/admission.rs`

### The two halves

```text
    the gas reserve  →  when an honest proposer arrives, there is ROOM
    this file        →  an honest proposer ARRIVES IN TIME
```

Neither is worth anything alone, and the reserve half is built: it is a
block-validity rule in `bc-block::gas`, enforced by both engines, and
`cargo run -p bc-node --bin censor` shows it defeating a flood that would
otherwise starve a dispute. It also shows, in run 3, that it does nothing
against a proposer who simply omits you — a block containing no disputes
breaks no rule. That is the half this file closes.

### The argument

Proposers rotate round-robin over the set. In any `w` consecutive blocks
you see `min(w, n)` distinct proposers, at most `f` of them Byzantine — and
an adversary choosing their own keys can make those `f` land
**consecutively**, so the worst case is a run, not a scatter. An honest
proposer is reached iff the window outlasts the longest run.

### The part that is the episode

**The window is not Δ.** `Dispute::arm` gives `min(Δ, budget)` floored at
`MIN_MOVE_BLOCKS`, and a budget runs down, so the infimum over a dispute's
life is the floor itself — Δ cancels (`build-log` §18). A channel that
negotiated Δ = 2048 is defended, in its last moves, by 8 blocks.

The consequence is sharp: with an 8-block floor, this scheme secures a
validator set only up to the size at which `f` reaches 8, and **no time
control rescues a larger one**. Raising the floor, weighting the rotation,
or a forced-inclusion queue are the ways out. None is built, and choosing
between them is a decision, not a cleanup.

### The two tests that matter most

- `a_window_equal_to_the_run_is_not_safe` — the off-by-one. A window of
  exactly `f` is spanned by `f` consecutive Byzantine proposers. This is
  the whole difference between a guarantee and a coin flip.
- `the_floor_and_not_delta_is_what_bounds_the_validator_set` — the result
  above, asserted rather than claimed.

### Already decided, so not in the hole

- `smallest_window`, and the derivation that makes Δ cancel.
- `Assessment` and `assess`, which compose the hole with the plumbing.
- `bc_node::admission`, which refuses a channel the set cannot secure —
  the `D22` instinct applied to censorship. It panics until the hole is
  filled, on purpose: a check that returns "safe" while the bound is
  unwritten is worse than no check.

---

## Not a hole, but still yours

The five `(Δ, τ)` pairs in `crates/bc-adjudicator/src/timecontrol.rs` are a
proposal, flagged `G0`. They are the rule rather than the plumbing.

Nothing currently depends on the values: the tests check the properties any
table must satisfy, not the numbers. D22's deadline is first testnet, not
today, because `GameTerms` is inside `channel_id` and its shape cannot
change once signed channels exist.

Now that there is a real chain with a real two-second block time, those
numbers could be re-derived from measured block-time variance and the
censorship bound rather than from judgement. That is unclaimed work and it
is not blocking anything.
