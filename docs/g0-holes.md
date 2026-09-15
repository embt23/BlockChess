# The `G0` holes

`spec/09` D26, recorded as `G0`: **the person filming types the code the
episode is about.** Evan writes the episode's subject — the arithmetic, the
rule, the check. Claude writes plumbing, tests, serialisation and oracle
harnesses.

This file is the list. Two functions are open, both the filmed subject of an
episode. Everything around them is finished, and the tests that specify them
are written.

```sh
cargo test -p bc-pow -- --ignored     # episode 05
cargo test -p bc-bft -- --ignored     # episode 06
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
