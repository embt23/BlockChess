# 04 — The game channel

This is the heart of the protocol. Everything else is scaffolding around it.

## The state object

```
GameState {
  channel_id      [u8;32]
  ply             u16        strictly increasing, 0 = initial position
  prev_hash       [u8;32]    hash of the state at ply-1  (zero at ply 0)
  move            u16        the move that produced this state (0 at ply 0)
  pos_hash        [u8;32]    H_domain("BC/pos/v1", packed_position)
  clock_w_ms      u32        white's remaining time AFTER this move
  clock_b_ms      u32        black's remaining time AFTER this move
  status          u8         Ongoing | WhiteWins | BlackWins | Draw
}
```

```
state_hash = H_domain("BC/state/v1", encode(GameState))
```

Fixed size: 32+2+32+2+32+4+4+1 = **109 bytes**, hashing to 32.

### The state is a hash chain

`prev_hash` makes the sequence of states a hash chain, exactly like the block
chain one layer down. This has a consequence worth stating on its own:

> **Signing state `n` is signing the entire game up to ply `n`.**

You cannot sign ply 40 while disputing ply 12, because ply 40's hash commits to
ply 39's hash, which commits to ply 38's, all the way down. There is no need for
a separate "history" object and no need to keep old signatures around to prove
the past — one signature at the current ply is a proof about everything before it.

This is the same argument that makes a blockchain a chain, applied one level up.
The structure repeats at every scale in this system, which is worth noticing:
**a chain of hashes is how you turn a statement about one thing into a statement
about everything that led to it.**

## Certified states

- A state is **half-signed** if the player who made the move signed it.
- A state is **certified** if both players signed it.

Certification is what lets the *other* player use a state as evidence.

## Channel lifecycle

```
                     ┌──────────────────────────────┐
                     │                              ▼
   ─── OpenGame ──▶ ACTIVE ─── CloseGame ──▶ SETTLED
                     │                              ▲
                     └── DisputeOpen ──▶ DISPUTED ───┘
                                            │  ▲
                                            └──┘
                                    DisputeMove / ClaimTerminal
                                    / Refute  (each resets Δ)
```

### Opening — one transaction, two signatures

Both players sign a `GameOffer`; whoever is submitting (usually a matchmaking
server) puts both signatures into a single `OpenGame` transaction.

```
GameOffer {
  white_pk        [u8;32]
  black_pk        [u8;32]
  stake_white     u128
  stake_black     u128
  terms           GameTerms
  server_pk       [u8;32]     zero if none
  rake_bps        u16         basis points, 0-500 (max 5%)
  expiry_block    u64         offer is void after this height
}

channel_id = H_domain("BC/chanid/v1",
                      white_pk ‖ black_pk ‖ encode(terms)
                      ‖ stake_white ‖ stake_black ‖ open_nonce)
```

```
GameTerms {
  start_pos_hash    [u8;32]   usually the standard opening position
  base_time_ms      u32
  increment_ms      u32
  max_plies         u16       ≤ 600
  delta_blocks      u32       Δ, ≥ 64
  budget_tau_ms     u32       clock dilation constant, see 05
  rules_mask        u8        which draw rules are enabled
  adjudicator_ver   u16
}
```

**The critical property of this design:** the server assembles and submits the
transaction, but the funds move from the players' own accounts under the
players' own signatures. The server is a **relay of signatures, not a custodian
of funds**. The worst a T0 server can do is refuse to submit, or submit late
(hence `expiry_block`). It can never take the money. This one structural choice
removes the entire category of "the exchange ran off with the deposits", which
is the dominant failure mode of real money-gaming platforms.

`stake_white` and `stake_black` need not be equal — see the handicap odds in
`06-economics.md`.

### Playing — the happy path

Both clients hold the current certified state. Suppose it is White's turn at
certified ply `n`.

```
White                                          Black
  │                                              │
  │  compute move m, apply, build S(n+1)         │
  │  clock_w -= elapsed;  clock_w += increment   │
  │                                              │
  │────  Move { m, S(n+1), sig_W(S(n+1)) }  ────▶│
  │                                              │  verify:
  │                                              │   • ply == n+1
  │                                              │   • prev_hash == H(S(n))
  │                                              │   • m legal in S(n).position
  │                                              │   • pos_hash == H(apply(...))
  │                                              │   • clock debit plausible
  │                                              │   • sig valid
  │                                              │
  │◀── Move { m', S(n+2), sig_B(S(n+2)),   ──────│
  │           sig_B(S(n+1)) }                    │
  │                                              │
  S(n+1) now certified.  S(n+2) half-signed.
```

**One round trip per move pair.** Black's countersignature on `S(n+1)` rides
along with Black's own move. The protocol never waits for a bare acknowledgement.

Consequence: at any moment the player *not* to move holds a certified state, and
the player to move holds a certified state one ply behind plus their own
half-signed state. That asymmetry is exactly right — the person who needs
evidence is the person waiting.

### Clock accounting

The mover debits their own clock and asserts the result. The opponent decides
whether to accept it. There is no trusted time source and there does not need to
be one, because **the clock is enforced by refusal to countersign.**

Recommended client policy: accept the opponent's asserted debit if it is at
least `(locally_measured_elapsed − grace)`, where `grace ≈ 300 ms` absorbs
network jitter and honest clock skew. If the opponent claims to have used less
time than you observed, that is in your favour and there is no reason to object.
If they claim implausibly less, refuse and go to chain.

Increment is added *after* the move, per Fischer rules. A move that would take
the mover's clock below zero produces `status = opponent wins` — a self-declared
flag, which an honest client does and which a dishonest client simply will not
send, at which point you go to chain.

### Closing — cooperative

The overwhelmingly common case. One player sends a terminal state
(`status != Ongoing`), the other countersigns, and either submits `CloseGame`
carrying both signatures. The chain verifies two signatures, checks the status
field, and pays out. **It does not verify any chess.** Two people who agree
about the result are allowed to be wrong about it — that is their money.

This is why the expensive on-chain chess engine almost never runs.

### Closing — uncooperative

See `05-adjudication.md`.

## Why this needs no revocation machinery

Restating the argument from `00-overview.md` in protocol terms.

In a payment channel, state `n` and state `n−1` differ in the *split of funds*,
and either party might prefer an older split. So Lightning needs revocation
secrets: publishing an old state hands your entire balance to your counterparty
as a penalty.

Here:

1. Payout is a function only of `status`, and `status` is `Ongoing` for every
   non-terminal state. **Non-terminal states pay nobody.** Posting one cannot
   move money.
2. `ply` is strictly increasing and the adjudicator accepts strictly higher plies
   over lower ones.
3. Both players hold signatures for the highest ply reached.

So posting an old state accomplishes nothing except paying gas and giving your
opponent a free opportunity to post a newer one. There is no profitable
old-state attack, therefore no penalty is needed, therefore no revocation
secrets, therefore no watchtower needs to hold a secret on your behalf.

**Consequence:** a BlockChess watchtower is a purely *public* service. It only
needs to know "if channel X enters dispute, here are signed states at plies up to
N, post the highest". It cannot steal from you, so you can use several, run by
strangers, for free. Lightning watchtowers are a hard problem; ours are a cron
job.

## Message set (P2P, over Noise)

| Message | Direction | Contents |
|---|---|---|
| `Hello` | both | protocol version, pubkey, capabilities |
| `Offer` | either | signed `GameOffer` |
| `Accept` | either | countersignature on `GameOffer` |
| `Start` | either | `OpenGame` txid + inclusion proof |
| `Move` | mover | move, new state, own signature, countersig on previous |
| `Resign` | either | signed resignation at ply |
| `DrawOffer` | either | ply |
| `DrawAccept` | either | signed draw agreement at ply |
| `Sync` | either | "my highest state is N", for reconnection |
| `SyncResp` | either | states from N to current, with signatures |
| `Close` | either | terminal state with both signatures |

`Sync` handles the common non-malicious case of a dropped connection. A client
that reconnects within the challenge window resumes without touching the chain.
This should be the *normal* recovery path; disputes are for actual adversaries,
not for flaky wifi.
