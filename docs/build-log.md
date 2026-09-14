# Build log

Bugs worth keeping. Each one is an episode beat: the failure mode is more
instructive than the working code.

---

## 01 — The hash that passed every official vector and was still broken

**Symptom.** All FIPS 180-4 vectors for SHA-256 and SHA-512 passed. `H("abc")`,
`H("")`, the million-'a' test — all exact. But feeding the same bytes through
`update()` in small chunks gave a different digest than feeding them at once.

**Cause.** `update()` topped up a partial block, then fell through to a tail
that stored the leftover input:

```rust
self.buf[..data.len()].copy_from_slice(data);
self.buflen = data.len();
```

When the top-up did *not* fill a block, `data` was already empty by then, so
this set `buflen = 0` and silently discarded the bytes just buffered.

**Why the vectors missed it.** Every official vector is a single `update()`
call. The bug only exists across calls.

**Lesson.** A hash implementation's compression function is the part that gets
the attention and the test vectors; the buffering is the part that gets the
bugs. Test the *interface*, not only the algorithm: the streaming-equals-oneshot
property found this instantly and is three lines long.

This matters here specifically because the protocol hashes structured records
field by field — `tagged_parts` makes many small `update` calls — so every
state hash in the system would have been wrong.

---

## 02 — The elliptic curve bug that kept points on the curve

**Symptom.** RFC 8032 test vectors failed: public keys came out completely
wrong. But nearly everything else passed. Field arithmetic matched a reference
implementation byte for byte on `+`, `−`, `×`, `x²`, `−x` and `x⁻¹`. Point
doubling produced projective coordinates identical to the reference. Repeated
addition — `B`, `2B`, `3B` … `8B` — matched published values exactly. Every
computed point satisfied the curve equation.

Only `mul_scalar` was wrong, and it was wrong even for `[1]B`.

**Cause.** Field elements are kept *lazily reduced*: a value is any
representative in `[0, 2^256)`, canonicalised only when compared or encoded.
Subtraction that borrows past the top limb has implicitly added `2^256 ≡ 38`,
so it must subtract 38 back. That subtraction can itself borrow — when the
intermediate is below 38 — and then the result is wrong by another `2^256`.
The code corrected once and dropped the second borrow.

**Why it hid so well.** The trigger is narrow: only representatives below 38.
The identity element's doubled form, `(0 : −1 : −1 : 0)`, is exactly such a
case, so it fired on `IDENTITY.double()` — which happens 255 times at the top of
every scalar-multiplication ladder and nowhere else in the tests. Direct point
addition never went near it.

Worse, the wrong answer was still a *valid curve point*. An on-curve check,
which is the obvious sanity test, passes.

**Lesson.** Lazy reduction trades correctness-by-construction for speed, and the
bill comes due at the boundaries. When a representation admits many spellings of
the same value, test the boundary spellings explicitly — `0`, `1`, `37`, `38`,
`p`, `2^256−1`, `2^256−38` — not just random values, which will essentially
never land there.

And: **an external oracle is worth more than any number of self-written sanity
checks.** Twelve hand-written tests passed. RFC 8032 caught it.

**Regression tests:** `field_sub_handles_repeated_borrow`,
`field_identities_over_many_values`.

---

## 03 — The reference implementation was the thing that was wrong

While chasing bug 02, a throwaway Python Ed25519 reference disagreed with the
Rust about `[2]B`. The Rust was right. The Python's point functions returned
`[X, Y, T, Z]` while its encoder unpacked `x, y, z, t` — swapping the last two.

Half an hour went into suspecting correct code.

**Lesson.** A reference you wrote five minutes ago is not an oracle, it is a
second implementation with its own bugs. A real oracle is one you did not write:
the RFC's vectors, the published `perft` counts. Check the reference against the
oracle before trusting it against your code.

---

## 04 — perft passed first run

Worth noting for balance: the move generator hit all six standard positions on
the first execution, including `perft(6) = 119,060,324` and Kiwipete
`perft(5) = 193,690,690`.

The reason is not luck. Generating pseudo-legal moves and filtering by
"make the move, is my king attacked?" makes the two classic bug sources —
pinned pieces, and en passant discovering a check along a rank — fall out for
free rather than needing to be handled. The slower, more obviously correct
algorithm was correct.

51 Mnps in release, which is ample: the on-chain adjudicator evaluates *one*
move per dispute.
