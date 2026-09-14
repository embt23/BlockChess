# 05 — Adjudication: what happens when someone lies or vanishes

## The single rule

> **On-chain, the game continues under the same rules, slowly.**

The adjudicator is not a special dispute-resolution mechanism with its own
logic. It is the same chess game, played at block speed, with the chain as
referee. If your opponent stops responding at ply 37, you post ply 37 and the
game resumes on-chain at ply 38.

That framing collapses an enormous amount of complexity. There is no "who was
right" arbitration, no evidence weighing, no appeal. There is only: it is your
move, here is the board, move or forfeit.

## Dispute state

```
Dispute {
  channel_id      [u8;32]
  ply             u16
  pos             [u8;26]    the packed position, IN PLAINTEXT
  clock_w_ms      u32
  clock_b_ms      u32
  budget_w        u32        remaining on-chain blocks for white
  budget_b        u32        remaining on-chain blocks for black
  deadline_block  u64        current responder must act by this height
  claim           Option<TerminalClaim>
  initiator       u8         0 = white, 1 = black
}
```

Note that the position goes on-chain **in plaintext** during a dispute. This is
a deliberate v1 privacy cost: disputes are rare, and the alternative (ZK proofs
of move legality) is a research project. See `08-privacy.md`.

## Opening a dispute

`DisputeOpen { channel_id, state, sig_opponent, [own_sig] }`

Requirements:

- `state.status == Ongoing`
- the state is signed by **the opponent** (this is what makes it evidence)
- `state.pos_hash` matches the supplied plaintext position
- the channel is `ACTIVE`

You post a state your *opponent* signed, because a state you signed yourself
proves nothing. The best state you can post is the highest-ply one they
countersigned. Since countersignatures ride along with moves (`04-channel.md`),
the player who is waiting always has one.

The initiator pays gas. This is the first griefing brake.

## Clock dilation

The problem: chess time controls are measured in seconds; blocks arrive every
2 seconds and confirmations take longer. A player with 400 ms left on a bullet
clock cannot physically move on-chain. But if we simply reset everyone to a
generous on-chain clock, then **a player about to lose on time can stall,
force the dispute, and get their time back** — converting a certain loss into a
fresh game.

Neither "keep the game clock" nor "discard the game clock" works. The answer is
to **change the time base while preserving the ratio**.

```
budget_blocks(player) = min( MAX_BUDGET,
                             ceil(clock_ms / τ) + FLOOR_BLOCKS )

τ            = 50 ms per block of budget   (GameTerms.budget_tau_ms)
FLOOR_BLOCKS = 32                          (≈ 64 s, enough to physically respond)
MAX_BUDGET   = 5400 blocks                 (≈ 3 hours)
```

This is a **total budget for the rest of the game**, not per move, and it is
consumed by the actual number of blocks each player takes. Each move must also
consume at least `MIN_MOVE_BLOCKS = 8`, so a player cannot be forced to respond
faster than ~16 seconds regardless of how little time they have.

Worked examples, at 2 s blocks:

| Certified clock | Budget blocks | Wall-clock budget |
|---|---|---|
| 180 s (3+2 blitz, fresh) | 3632 | ≈ 2.0 h |
| 30 s | 632 | ≈ 21 min |
| 5 s | 132 | ≈ 4.4 min |
| 0.4 s | 40 | ≈ 80 s |

The player with 400 ms still gets 80 real seconds to make a move — they can
physically play — but they hold 40 blocks against their opponent's 3632. They
are still overwhelmingly losing on time, and they will still run out first. The
*strategic meaning* of the clock is preserved while every deadline becomes
physically achievable.

This is a change of time base with the rate ratio held constant. Stalling no
longer buys time back; it only makes the game slower and costs the staller the
gas.

`deadline_block` for each response is `min(current_height + Δ,
current_height + remaining_budget)`, so a player is bounded both per-move and
in total.

## Responding

`DisputeMove { channel_id, ply, move, [new_status] }`

The adjudicator:

1. checks `msg.sender` is the side to move in `dispute.pos`
2. checks `current_height <= deadline_block`
3. runs `apply(pos, move)` — **this is the on-chain chess engine, and this is
   the only place it runs**
4. debits the mover's budget by `max(blocks_elapsed, MIN_MOVE_BLOCKS)`
5. updates `pos`, increments `ply`, flips the side to move
6. sets a new `deadline_block`

An illegal move is simply rejected (the transaction reverts); the clock keeps
running, so submitting garbage costs you both gas and time.

### Overriding with a newer state

`DisputeOpen` may be called again on a channel already in dispute, with a
**strictly higher ply**. This is how the "higher ply wins" rule is enforced. It
resets the deadline. A player who posts a stale state simply gets overridden.

Because each override resets Δ, and because Δ is bounded by the total budget,
the process terminates: budgets only decrease.

## Terminal claims and refutation

`DisputeClaimTerminal { channel_id, kind, evidence }`

| Kind | Evidence | Verification |
|---|---|---|
| `Resign` | signature | O(1), immediate, final |
| `DrawAgreed` | two signatures | O(1), immediate, final |
| `FiftyMove` | none | `pos.halfmove == 100`, immediate, final |
| `InsufficientMaterial` | none | popcounts, immediate, final |
| `Threefold` | three signed states | equal `pos_hash`, distinct plies, immediate, final |
| `Checkmate` | none | **optimistic** — refutable |
| `Stalemate` | none | **optimistic** — refutable |
| `Timeout` | none | budget check, immediate, final |

The first five and the last are decided on the spot. The two optimistic ones
open a refutation window of Δ blocks.

### `DisputeRefute { channel_id, move }`

The claimed-mated player posts **one move**. The adjudicator checks it is legal
and that the resulting position does not leave the mover in check. If it
verifies:

- the terminal claim is struck,
- the game resumes at the refuting move,
- **the false claimant's budget is halved** as a penalty.

If the window expires with no refutation, the claim stands and the game settles.

### Why this is safe

The obvious worry: can someone steal a win by falsely claiming mate against an
opponent who happens to be offline?

No — because that opponent was *already* going to lose. If they cannot post a
refutation within Δ blocks, they equally cannot post a legal move within Δ
blocks, and the timeout rule would have taken the game from them anyway.
The optimistic mate claim introduces **no new liveness requirement**. It rides
entirely on a liveness assumption the protocol already made.

That is the test to apply to any optimistic mechanism: *does this require the
honest party to be online at a time they didn't already need to be online?* If
the answer is no, the optimism is free.

### Cost asymmetry

| | Statement | Work |
|---|---|---|
| Claim | ∀ moves: still in check | up to 218 move generations + 218 check tests |
| Refute | ∃ a move: not in check | 1 move application + 1 check test |

Roughly two orders of magnitude. And crucially, the expensive side is the one
that is *usually true and unchallenged*, so the expensive computation is almost
never performed by anyone.

## Finalisation and payout

`DisputeFinalize { channel_id }` — callable by anyone once the deadline passes
with no valid response.

```
pot   = stake_white + stake_black
fee   = pot · rake_bps / 10000

WhiteWins  →  white receives pot − fee
BlackWins  →  black receives pot − fee
Draw       →  each receives their own stake, minus fee pro-rata
```

The rake goes to `server_pk`, or is burned if `server_pk` is zero.

**Rake is charged on draws too.** It is tempting to waive it as a courtesy, but
waiving it makes the draw the cheapest outcome and creates a small standing
incentive for two players to agree a quick draw and pay nothing. Charge
uniformly; make outcomes fee-neutral.

## Griefing analysis

The remaining attack: force a fast game on-chain purely to waste your opponent's
time. The attacker gains nothing financially and pays gas. Costs to them:

1. **Gas** — the initiator pays for `DisputeOpen`, and each `DisputeMove`
   is paid by its own sender.
2. **Their own budget** — the initiator's clock is dilated from the same
   certified state, so a player who initiates while low on time is initiating
   into a losing position.
3. **Reputation** — dispute initiation is public and on-chain. Servers at L4 can
   and should refuse to match accounts with anomalous dispute rates. This is the
   main defence, and it is deliberately pushed up to the server layer rather than
   encoded in consensus, because "anomalous" is a judgement call and judgement
   calls do not belong in a state transition function.

An honest player who is *forced* into dispute by a genuinely dead opponent bears
the gas. This is unfair but unavoidable — the chain cannot distinguish "crashed"
from "pretending to have crashed" (this is FLP again, in economic clothing). The
mitigation is that the cost is small and bounded, and servers can insure it.

## Parameter summary

```
DELTA_BLOCKS         256     per-response window        (GameTerms)
MIN_MOVE_BLOCKS        8     floor on per-move consumption
FLOOR_BLOCKS          32     added to every budget
TAU_MS                50     ms of game clock per block of budget
MAX_BUDGET          5400     ≈ 3 h cap
FALSE_CLAIM_PENALTY  50%     of remaining budget
MAX_PLIES            600     hard game-length cap → forced draw
```

At `MAX_PLIES` the game is a draw regardless of position. This bounds the
worst-case on-chain cost of a single channel, which matters because that bound
is what a validator must be able to afford.
