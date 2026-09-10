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

---

## 05 — The sign that was arbitrary, caught before it bit

**Not a bug found. A bug prevented, by a lesson already in this file.**

An eigenvector has no canonical sign. If `v` is an eigenvector then so is `−v`,
with the same eigenvalue, and every numerical routine is free to return either.
Both are correct. Both satisfy every test you would think to write: the
reconstruction `VΛVᵀ = A` holds, orthonormality holds, the trace matches.

But a medal is `H(quantised coordinates)`, and coordinates are projections onto
those eigenvectors. A flipped axis negates a coordinate, which changes the
hash. The same player, the same corpus, the same code — a different identity,
depending on the order Jacobi happened to visit the rotations in on that
machine.

That is exactly bug 02's lesson, arriving in different clothing:

> When a representation admits many spellings of the same value, test the
> boundary spellings explicitly.

There the many spellings were lazily-reduced field elements, and `0`, `37`,
`38`, `2^256−38` were the ones that hurt. Here the many spellings are `±v`, and
the value that hurts is the one your platform's floating-point rounding happens
to produce today and not tomorrow.

**The fix** is one rule, in `linalg::symmetric_eigen`: flip each eigenvector so
its largest-magnitude entry is positive, ties broken by lowest index. Three
lines, and `eigenvector_signs_are_canonical` is the regression test.

**Lesson.** *Correct* and *canonical* are different properties, and only the
first one has an oracle. Any value that gets hashed needs the second, and
nothing in the mathematics will tell you it is missing — the mathematics is
perfectly happy with both answers. Ask of every committed value: **how many
ways can this be spelled?** If the answer is more than one, pick one in code and
write the test that pins it.

**Regression test:** `eigenvector_signs_are_canonical`, `medals_are_deterministic`.

---

## 06 — The medal minted from one game

**Symptom.** The lab was pointed at real games for the first time — four famous
master games, the Opera Game and the Immortal among them. The parser handled
them perfectly: four games, zero rejected, castling and mate markers and all.

Then it printed this, in exactly the same typeface as everything else:

```
  axis 0    71.4% of variance      take_rate  ←→  mean_material
  dispersion   0.00  over 1 games
  medal        5308f402e33e4f5b2b525349459e88460a09aa510acf536f43ffdea696c7ebca
```

Eight game-sides. Eighteen features. A confident 71.4%, a personality
commitment, and a dispersion of exactly zero — which reads as *this player is
perfectly consistent* and actually means *we have one sample*.

**Cause.** A covariance matrix estimated from `n` observations has rank at most
`n − 1`. With eight samples in eighteen dimensions, at most seven directions
carry any information at all; the rest have eigenvalues that are zero to within
rounding, and `symmetric_eigen` dutifully sorts them by whatever the floating
point noise happened to be. Those axes have poles. They have loadings. They
have a share of variance. They will mint you a medal.

**Why it hid.** Every test in the crate used the synthetic corpus, which has
240 game-sides and comfortably clears the bar. The failure needs a *small*
corpus, and small corpora had only ever appeared in unit tests that checked
mechanics — does the hash change, is the JSON balanced — never in a test that
asked *should this have produced an answer at all*.

And nothing about the output looks wrong. That is the whole problem. A
rank-deficient basis is not an error state; it is a well-formed object
containing nothing. The same shape as bug 02, where a miscomputed point was
still a valid curve point, and bug 05, where a flipped eigenvector was still a
correct eigenvector.

**The fix.** `Basis::adequacy()` computes two bounds and the crate refuses to
be quiet about either: the hard one, `rank = min(samples − 1, D)`, and the soft
one, ten samples per dimension, which is the usual rule of thumb for factor
analysis. The warning goes to the terminal, into the exported JSON, and onto
the published page — the page especially, because it is the most
authoritative-looking artifact here and therefore the most dangerous.

**Lesson.** This project's standard has been *check against an oracle you did
not write*. That is necessary and it is not sufficient, because an oracle tells
you whether an answer is right and says nothing about whether you were entitled
to an answer. Sample size is not a statistical nicety; it is a precondition for
the question being meaningful, and code that computes an estimate should
compute the bound alongside it and carry the two together.

The generalisation, and it is the one worth keeping: **an estimator that cannot
say how much it should be believed will be believed exactly as much as
everything printed next to it.**

**Regression tests:** `asking_for_more_axes_than_the_data_supports_is_caught`,
`a_thin_corpus_is_caught_separately`, `the_warning_travels_into_the_json_and_the_page`,
`the_synthetic_demo_corpus_is_adequate`.

---

## 07 — Every player was about to be named after their rating

**Symptom.** None, and that is the entry. The parser had been given four famous
games typed out by hand, and it read them perfectly. Then it was given a
round-trip oracle — render a generated game to SAN, read it back, demand the
same moves — and one of the first tests came back:

```
  left: "1523"
 right: "alice0"
```

**Cause.** Reading a tag was `line.strip_prefix('[')?.strip_prefix(key)?`. The
tag `[WhiteElo "1523"]` begins with the characters `White`, so asking for the
`White` tag matched it, found the first quoted string after the prefix, and
returned the rating. Tags are read in file order, so `WhiteElo` — which comes
*after* `White` in every real export — overwrote the name every time.

On a Lichess file, **every player would have been named after their rating.**
Two people at 1523 would have merged into one player. One person whose rating
moved would have become several. Every medal in the corpus would have been
minted for a number.

**Why it hid.** The four hand-written classics carry `[Event]`, `[Site]`,
`[Date]`, `[White]`, `[Black]`, `[Result]` and nothing else. No ratings, no
`[Variant]`, no clock comments — none of the tags a real export carries. The
test corpus and the target corpus had different *shapes*, and every test passed
because the bug needs a tag that the fixtures did not contain.

**The fix.** One condition: after the key is stripped, the remainder must begin
with whitespace, so `WhiteElo` no longer answers to `White`.

**Lesson.** Four hand-picked examples are not a sample, they are an
illustration. The bug was not in the hard part — SAN disambiguation, en
passant, promotion, castling all worked first time — it was in the boring part,
reading a header, which nobody thinks to test because nobody thinks it can be
wrong. That is the same shape as bug 01, where the hash compression function
was right and the buffering was wrong.

The general form, and it now has three instances in this log: **build the
generator, not just the examples.** A round-trip oracle over thousands of
generated games found in one run what a hand-written fixture set had concealed
completely, and it did it because the generator produces shapes nobody would
think to type.

**Regression tests:** `a_rating_tag_does_not_become_the_player`,
`tag_order_does_not_matter`, `many_games_in_one_file_keep_their_identities`.
