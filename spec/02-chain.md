# 02 — The chain

## What actually needs consensus

Almost nothing. The chain stores:

- account balances,
- open channels and their funding,
- in-flight disputes,
- the server registry and bonds.

It does **not** store moves, games, ratings, or matchmaking. A 40-move blitz
game touches the chain twice. **The design target is not throughput — it is
finality and censorship resistance**, because both are load-bearing for money
held under a deadline.

## Account model, not UTXO

We use an account model. A channel is a long-lived, mutable, addressable object
that a dispute walks through a state machine; UTXO makes that awkward (you would
be threading a coin through each dispute step). Accounts also give a natural
replay-protection nonce.

```
Account {
  balance:  u128
  nonce:    u64
}
```

## State: a Sparse Merkle Tree

All chain state is one map `key: [u8;32] → value: [u8;32]`, committed as a
sparse Merkle tree of depth 256.

```
key = H_domain("BC/smt/key", kind_tag ‖ identifier)
```

`kind_tag` distinguishes accounts, channels, disputes, servers.

### The construction

A depth-256 binary tree has 2²⁵⁶ leaves, which we obviously do not store. The
trick: precompute the hash of an all-empty subtree at each level.

```
empty[0]   = H_leaf(⊥)
empty[i]   = H_node( empty[i-1] ‖ empty[i-1] )
```

There are only 256 such values. Any subtree containing no data has a known hash,
so we store only the ~n non-empty nodes and substitute `empty[i]` everywhere
else. A proof is 256 sibling hashes, but all but ~log₂(n) of them are the known
`empty[i]` constants and can be omitted from the wire format with a 256-bit
bitmap saying which levels were non-empty.

### Transaction and receipt roots use RFC 6962

`tx_root` and `receipt_root` commit to an *ordered list*, not a key/value map,
so they use a different tree: the Certificate Transparency one (RFC 6962).
Leaves are prefixed `0x00`, internal nodes `0x01`, and odd levels split at the
largest power of two below `n`.

Both details are load-bearing.

Without the leaf/node prefixes, the 64-byte concatenation of two child hashes is
simultaneously a valid internal node and a valid leaf, so an attacker can
present an internal node as a leaf and forge an inclusion proof — the
**second-preimage attack**.

Without the power-of-two split — Bitcoin pairs a trailing odd node with itself —
the lists `[A,B,C]` and `[A,B,C,C]` produce the *same root*, so two different
blocks get identical headers. That is **CVE-2012-2459**, a network-splitting
denial of service.

Using a published, deployed specification also means published test vectors,
which is worth more than an in-house design here.

### Why this and not Ethereum's Merkle-Patricia Trie

The MPT is a radix-16 trie with four node types, path compression, and RLP
encoding. It exists to save space in 2015 conditions and it is genuinely
unpleasant to implement correctly. The SMT above is roughly 150 lines, has one
node type, and gives you something the MPT does not give cheaply:

> **Free proofs of absence.** If the value at a key is `empty[0]`, the same
> proof that would show inclusion instead proves nothing is there.

This matters: "prove this channel has no open dispute" and "prove this key has
never registered a server" are both non-inclusion proofs, and we want them cheap
for light clients.

## Block structure

```
BlockHeader {                                    bytes
  version        u16                                2
  height         u64                                8
  parent_hash    [u8;32]                           32
  state_root     [u8;32]   SMT root AFTER this block 32
  tx_root        [u8;32]   Merkle root of txs       32
  timestamp_ms   u64                                8
  proposer       [u8;32]   validator pubkey         32
}                                                 ─────
                                                   146
```

The commit certificate (the set of validator precommit signatures) is stored
alongside the header, not inside it, because a header cannot contain signatures
over itself.

Target block time: **2 seconds**. This is a parameter, and it is the sampling
rate of the whole system — everything downstream that is measured in blocks
inherits its resolution from here.

## Consensus

### The recommendation: Tendermint-style BFT

Round-based, three steps per round: **propose → prevote → precommit**, with a
validator set weighted by stake. A block is final when it collects precommits
from more than ⅔ of stake. Safety holds while Byzantine stake `f < ⅓`.

### Why finality is the requirement here

This is the non-obvious argument and it is worth stating carefully.

Under Nakamoto (proof-of-work) consensus, confirmation is **probabilistic**. A
transaction six blocks deep is *very likely* permanent but never certainly so.
Normally that is fine — you just wait longer for larger amounts.

Here it is not fine, because our security depends on **deadlines**. The rule
"respond within Δ blocks or forfeit" assumes that once your response is
included, it stays included. With probabilistic finality, an adversary who can
reorg `k` blocks can un-include your dispute response *after* Δ has passed, and
you lose money that you correctly defended. The deadline turns a liveness
failure into a permanent financial loss.

> **Money under a deadline requires deterministic finality.**

That single sentence is why this project uses BFT rather than the more famous
design.

### FLP and why timeouts exist at all

Fischer–Lynch–Paterson (1985): in a fully asynchronous network, no deterministic
protocol can guarantee consensus if even one process may fail. There is no way
to distinguish "crashed" from "slow".

Every real consensus protocol escapes FLP by assuming **partial synchrony**:
after some unknown time, messages arrive within a known bound. Tendermint
implements this with per-step timeouts that increase each round. The system is
*always safe* (never two conflicting finalised blocks) and *eventually live*
(makes progress once the network behaves).

This is the same shape as clock-domain crossing in digital design: you cannot
sample an asynchronous signal with zero probability of metastability, so you
insert synchroniser flops and drive the failure probability down until the MTBF
exceeds the life of the product. You do not eliminate the impossibility; you
push its probability below the threshold that matters.

### The teaching path

Build proof-of-work first — one episode, ~300 lines, and it is the clearest
possible demonstration of "consensus as an economic race". Then show that its
probabilistic finality breaks the challenge-window argument above, and replace
it. Building the wrong thing on purpose and then explaining precisely why it is
wrong is better pedagogy than never building it.

### Difficulty adjustment is a control loop

If you do build the PoW stage: Bitcoin's retarget is a **proportional
controller** with gain 1, sampling every 2016 blocks, on a plant with enormous
delay. It is marginally stable and oscillates under hashrate steps. EIP-1559's
base fee is an exponential controller with a much shorter sampling period. This
is a control-systems problem wearing a costume, and the stability analysis is
the same one you would do for any feedback system.

## Censorship resistance

If validators can prevent your dispute transaction from being included for Δ
blocks, they can steal from you by timeout. Three defences, all of which we use:

### 1. Measure Δ in blocks, not seconds

This is the important one and it is easy to get wrong.

If Δ were wall-clock seconds, then a chain halt — for any reason, including an
honest one — expires everyone's windows simultaneously and every disputed game
resolves against whoever happened to be on move. If Δ is **blocks**, then no
blocks means no expiry. The window measures *opportunities to be included*,
which is the resource you actually need, rather than time, which is only a proxy
for it.

### 2. Reserved dispute gas

Every block reserves a fixed fraction of its gas limit (proposal: 25%) that only
dispute-family transactions may consume. A proposer stuffing the block with
ordinary transactions cannot squeeze disputes out; they must *explicitly* omit
them, which is detectable.

### 3. Generous Δ

Δ defaults to **256 blocks ≈ 8.5 minutes** for a single response, with the
per-game budget described in `05-adjudication.md`. Δ must exceed the worst
plausible censorship window, and censorship of a specific transaction by a BFT
set requires sustained coordination by more than ⅓ of stake — visible, and
slashable if it can be proven.

## Transactions

```
Tx {
  version    u16
  nonce      u64        replay protection, must equal account.nonce
  sender     [u8;32]
  fee        u128
  gas_limit  u32
  payload    Payload
  signature  [u8;64]
}
```

Payload variants:

| Payload | Purpose |
|---|---|
| `Transfer` | move tokens |
| `OpenGame` | fund a channel; carries **both** players' signatures |
| `CloseGame` | cooperative settlement; carries both signatures on a terminal state |
| `DisputeOpen` | post a signed state, start the clock |
| `DisputeMove` | play a move on-chain |
| `DisputeClaimTerminal` | assert mate / stalemate / repetition / 50-move |
| `DisputeRefute` | post the single move that refutes a terminal claim |
| `DisputeFinalize` | pay out after the window expires |
| `RegisterServer` | bond and publish a server |
| `SlashServer` | present proof of server equivocation |

Note the shape: **two of these are the happy path** (`OpenGame`, `CloseGame`).
Everything else exists only because people misbehave. That ratio is the honest
summary of what building financial protocols is like.

## Economic parameters (initial proposals, all governance-tunable)

```
BLOCK_TIME_MS            2000
DISPUTE_GAS_RESERVE      25%          of block gas limit
DELTA_BLOCKS             256          per-response challenge window
MIN_STAKE                dust limit, prevents spam channels
MAX_PLIES                600          hard cap on game length
```

## Explicitly not emitting block rewards to players

No token is minted for playing games. Ever. The moment playing pays, the
dominant strategy is to run two bots against each other and collect. Rewards go
to validators for securing the chain and (optionally) to servers via rake, both
of which are services with real costs. See `06-economics.md`.
