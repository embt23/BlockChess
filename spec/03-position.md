# 03 — Positions, moves, and terminal conditions

## What a position must contain

A "position" for our purposes is exactly the information needed to decide
(a) which moves are legal and (b) whether two positions are the same for the
threefold repetition rule. That is the FEN fields minus the move counter:

```
Position {
  pieces          who stands where
  side_to_move    1 bit
  castling        4 bits    (K, Q, k, q)
  ep_file         4 bits    (0-7, or 8 = none)
  halfmove        7 bits    (0-100, for the fifty-move rule)
}
```

The fullmove number is **not** included — two positions differing only in move
number are the same position for repetition purposes.

## Two representations

### Working representation: bitboards

Twelve `u64`s, one per piece type per colour. Bit `i` set means that piece
occupies square `i`, with `i = 8·rank + file`, a1 = 0, h8 = 63.

```
u64 pawns[2], knights[2], bishops[2], rooks[2], queens[2], kings[2];
```

Why: the board is 64 squares and the machine word is 64 bits. This is not a
coincidence anyone planned, but it means set operations on the board are single
instructions. "All squares attacked by white knights" is a few shifts and ORs.
"Is the king in check" is one AND against zero.

Knight attacks from square `s`, for example, are eight shifted copies of the
knight bitboard masked to prevent file wraparound:

```
n = knights;
attacks =  ((n << 17) & ~FILE_A)
        |  ((n << 15) & ~FILE_H)
        |  ((n << 10) & ~(FILE_A|FILE_B))
        |  ((n <<  6) & ~(FILE_G|FILE_H))
        |  ((n >> 17) & ~FILE_H)
        |  ((n >> 15) & ~FILE_A)
        |  ((n >> 10) & ~(FILE_G|FILE_H))
        |  ((n >>  6) & ~(FILE_A|FILE_B));
```

The hard case is sliding pieces (bishop, rook, queen), where the attack set
depends on which squares are occupied. The standard solution is **magic
bitboards**: for each square, find a multiplier `M` such that
`(occupancy & mask) * M >> shift` maps every relevant occupancy pattern to a
distinct index into a precomputed table. It is a perfect hash function found by
brute-force search. Start with the simpler "classical" ray-scanning approach and
upgrade later; correctness first.

### Wire/storage representation: packed, 26 bytes

Bitboards are 96 bytes, which we do not want to sign or transmit per ply. Pack:

```
occupancy     u64        8 bytes   which squares are occupied
nibbles       4 bits × popcount(occupancy)   ≤ 16 bytes
flags         2 bytes    side | castling | ep_file | halfmove
                        ─────────
                        ≤ 26 bytes
```

Read the nibbles in ascending square order, one per set bit in `occupancy`, each
encoding one of 12 piece types. Maximum 32 pieces → 16 bytes. A typical
middlegame position is ~20 bytes.

**Canonicality requirement.** The packed form must be the *only* valid encoding
of a position — no spare bits, no alternative orderings — because we hash it for
repetition detection. Reject any encoding with `ep_file` set to a square where
no en-passant capture is actually possible; otherwise two representations of the
same position hash differently and threefold repetition silently breaks.

## Position hash, and the repetition key

```
pos_hash = H_domain("BC/pos/v1", packed_position)
rep_hash = H_domain("BC/rep/v1", packed_position with halfmove zeroed)
```

Two hashes over the same bytes, because there are two different questions.

`pos_hash` goes inside the state hash and must commit to **everything**: the
adjudicator replays moves from the packed form, and the fifty-move rule reads
the halfmove field. Nothing may be left out of it.

`rep_hash` is what a repetition claim compares, and it must **not** include the
halfmove clock. Two occurrences of a position always differ in that clock —
plies happened in between, which is what a repetition is — so comparing
`pos_hash` detects a repetition exactly never. This is FIDE's definition:
pieces, side to move, castling rights, en-passant availability, and nothing
about the fifty-move counter.

The cost on-chain: a threefold claim carries the three **packed positions**,
not only the three states, so the adjudicator can check each against its
state's `pos_hash` and then compare the three `rep_hash`es. Three unpacks and
six hashes — still O(1), and still without replaying a single move.

Both are *cryptographic* hashes: a collision in either would forge a repetition
claim.

> This was wrong in the first draft of the spec, and the way it was found is
> worth keeping. The text read perfectly; a test of the behaviour it described
> could not be made to pass. `docs/build-log.md` §05.

### Zobrist hashing (engine-internal only)

Inside an engine you want an *incrementally updatable* hash. Zobrist: draw a
random `u64` for every (piece, square) pair, plus keys for side-to-move,
castling rights, and ep file. The position hash is the XOR of the keys for
everything present.

```
Z(position) = ⊕ { key[p][s] : piece p on square s }  ⊕  key_side  ⊕ …
```

Moving a piece is `Z ^= key[p][from]; Z ^= key[p][to];` — two XORs, no rehashing.

This works because `(GF(2)⁶⁴, ⊕)` is a group, XOR is its operation, every
element is its own inverse, and `Z` is a **homomorphism** from the free abelian
group of board edits into it. Undoing a move is applying the same key again.
It is the cleanest piece of group theory in the entire project and it is hiding
inside a chess engine.

Zobrist is 64-bit and therefore *not* collision-resistant against an adversary
(birthday bound ≈ 2³²). Use it inside your engine's transposition table; use the
cryptographic `pos_hash` for anything a counterparty could exploit.

## Move encoding

```
Move : u16
  bits  0-5    from square
  bits  6-11   to square
  bits 12-13   promotion piece (0=N, 1=B, 2=R, 3=Q)
  bits 14-15   flag (0=normal, 1=promotion, 2=en passant, 3=castle)
```

Two bytes per move. Castling is encoded king-from/king-to (and the rook is
implied), which handles Chess960 uniformly if we ever support it.

## Notation

Two readers, both in `bc-chess` beside each other:

- **UCI** (`e2e4`, `e7e8q`) — what the protocol and our own game files speak.
  Unambiguous, fixed width, no board context needed to write it.
- **SAN** (`Nf3`, `exd5`, `O-O-O`, `Qxe7+`) — what published games and PGN
  archives speak. It is context-dependent and states only as much of the origin
  square as disambiguation requires.

SAN is parsed as a **filter over the legal move list** rather than by rendering
each legal move to a string and comparing. That makes a parse self-validating:
a misread picks the wrong move, which makes the *next* move illegal, so an error
surfaces within a ply or two instead of silently corrupting a corpus. A game
that parses to a checkmate has validated the reader and the move generator
against each other.

## The move function

```
apply(Position, Move) -> Result<Position, IllegalMove>
```

This function is **consensus-critical**. It must be:

- **Total** — every input either produces a position or a rejection, never a panic.
- **Deterministic** — no floats, no hash-map iteration order, no allocation-address dependence.
- **Bit-identical across implementations** — because two nodes that disagree
  about legality disagree about who won.

It is the same function off-chain (in your client, thousands of times per game)
and on-chain (in the adjudicator, at most a few times per dispute). **Compile
the same source for both.** Do not write it twice. Divergence between a fast
client implementation and a careful on-chain implementation is exactly how you
get a chain split with money on it.

## perft — the only acceptable correctness test

`perft(n)` counts leaf nodes in the move tree to depth `n`. From the starting
position:

```
perft(1) =            20
perft(2) =           400
perft(3) =         8,902
perft(4) =       197,281
perft(5) =     4,865,609
perft(6) =   119,060,324
perft(7) = 3,195,901,860
```

If your generator matches these, it is almost certainly correct. If it is off by
one at depth 5, you have an en-passant, castling-through-check, or pinned-piece
bug, and the standard "Kiwipete" and position-3/4/5 test suites will localise it.

**Do not proceed past this milestone with a failing perft.** Everything above —
the channel, the adjudicator, the money — assumes the rules are right. This is
the one place in the project where correctness is cheap to verify exactly, so
verify it exactly.

## Terminal conditions and how each is proven on-chain

This table is the core of the adjudicator's design. Read the third column as
"what the chain has to compute in the worst case".

| Condition | Claimed by | On-chain cost | Mechanism |
|---|---|---|---|
| **Resignation** | loser | O(1) | verify one signature over `("BC/resign/v1", channel, ply)` |
| **Draw agreed** | either | O(1) | verify two signatures over `("BC/draw/v1", channel, ply)` |
| **Fifty-move** | either | O(1) | `halfmove == 100` — it is a field in the position |
| **Insufficient material** | either | O(1) | popcounts on the bitboards |
| **Threefold repetition** | either | O(1) | three signed states at distinct plies whose packed positions share a `rep_hash` |
| **Stalemate** | either | **optimistic** | asserted; refuted by one legal move |
| **Checkmate** | winner | **optimistic** | asserted; refuted by one legal escaping move |
| **Loss on time** | either | O(1) | budget exhausted — see `05-adjudication.md` |

Note how much work the *signatures* do. Threefold repetition would normally
require replaying the whole game to check history. It does not here, because
each of the three positions was **countersigned by the opponent at the time**.
Your opponent's past signature is a compressed proof of history, and it turns an
O(n) history scan into three signature checks.

> **General principle: a signature from your adversary is the cheapest evidence
> that exists.** Structure protocols so that the thing you will need to prove
> later is something your opponent had to sign earlier.

## Insufficient material

Draw by insufficient material when neither side can possibly mate:

- K vs K
- K+B vs K
- K+N vs K
- K+B vs K+B with both bishops on the same colour complex

Not included (these are draws by the *fifty-move* rule in practice, not
insufficient material): K+N+N vs K, which can mate with cooperation.

Checking "same colour complex" is `(bishops & LIGHT_SQUARES) == bishops` for
both sides, or the dark equivalent — two masks and a compare.
