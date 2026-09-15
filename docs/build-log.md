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

## 05 — The repetition key that could never match

**Symptom.** A threefold-repetition test could not be made to pass. Shuffling a
rook a1–a2–a1 while the enemy king shuffled h8–g8–h8 returns the board to a
position it has demonstrably occupied before, and the channel counted one
occurrence every time. Not off by one — the count never rose at all.

**Cause.** `spec/04` puts `pos_hash` in every state, and `spec/03` defines it
over the packed position, which includes the halfmove clock. Two occurrences of
a position *always* differ in that clock: plies happened in between, which is
what a repetition is. So the hashes could never be equal, and the comparison
`spec/03` prescribes — "three signed states with equal `pos_hash`" — detects
nothing, ever.

**Why it hid.** Because it looks right, and because nothing else notices. The
clock has to be in the packed form: the adjudicator replays moves from it, and
the fifty-move rule reads that exact field. Every other use of `pos_hash` is
correct. Only the repetition comparison wants a *coarser* notion of sameness
than the one the encoding provides, and nothing in the type system says so.

**The real distinction.** Position identity and repetition identity are
different relations. FIDE's is the coarser one: pieces, side to move, castling
rights, en-passant availability — and it says nothing about the fifty-move
counter. We had been treating one hash as answering both questions.

**Fix.** Two domain-separated hashes over the same bytes. `pos_hash`
(`BC/pos/v1`) commits to everything, because move replay needs everything;
`rep_hash` (`BC/rep/v1`) is the same encoding with the halfmove field cleared,
and repetition compares that. `spec/03` and `spec/04` amended.

**What it costs the chain.** A threefold claim must now carry the three packed
positions, not just the three states, so the adjudicator can recompute both
hashes. Three unpacks and six hashes instead of three comparisons. Still O(1),
and still paid for by the opponent's own past signatures rather than by
replaying the game.

**Lesson.** When one hash is asked to answer two questions about sameness,
check that they are the same question. They were not, and the way this surfaced
is worth keeping: not by inspection of the spec, which reads perfectly, but by
a test of the *behaviour* the spec was describing. Episode 01's bug was found
the same way — by testing the interface rather than the algorithm.

**Regression test:** `threefold_repetition_is_three_signatures_not_a_history_replay`.

---

## 06 — The en passant that three different checks all miss

Not a bug that shipped — `spec/03` called for this before the code existed —
but the third case is worth recording, because two obvious implementations
handle only the first two.

An en-passant target must not be recorded in the canonical position unless a
*legal* capture onto it exists. Three ways for one not to exist:

1. **No pawn beside it.** After `1.e4` the target is e3 and there is no black
   pawn within three files. Handled by any implementation that looks.
2. **The capturing pawn is pinned.** `4k3/8/8/8/3Pp3/8/8/K3R3 b - d3` — the e4
   pawn cannot leave the e-file. Handled by anything that tests legality.
3. **Both pawns leave the rank at once.** `8/8/8/8/k2Pp2Q/8/8/3K4 b - d3` —
   `exd3` e.p. removes the capturer *and* the captured from the fourth rank,
   opening the line from h4 to the king on a4. Neither pawn is pinned; no
   single-piece pin test finds it.

The third is why `ep_effective` asks the move generator rather than reasoning
about pins. There is exactly one implementation of the rules (`P5`), and the
cost of consulting it is one `make_move` per candidate pawn, once per ply.

Getting this wrong is not a movegen bug — legality is unaffected either way.
It is a *hashing* bug: the same position acquires two encodings, so the
threefold comparison silently stops working. Same failure as bug 05, reached
from the opposite direction.

**Regression tests:** `ep_by_a_pinned_pawn_is_not_recorded`,
`ep_that_would_expose_the_king_sideways_is_not_recorded`.

---

## 07 — The dispute nobody could open

**Symptom.** Building episode 08's tests, every scenario that started from a
fresh channel failed at `DisputeOpen` with `BadSignature` — including the
simplest one imaginable: two players fund a game, one of them never shows up.

**Cause.** `spec/05` admits a dispute only on a state signed by your
*opponent*, which is the right rule and the reason the whole mechanism works:
a state you signed yourself proves nothing. But **at ply 0 nobody has signed a
state**. The opening position is not a move; it is the thing moves start from.
There is no signature on it and there never will be.

So a player whose opponent vanished before making their first move could
satisfy no version of the requirement. Both stakes sit in escrow and no
sequence of messages by anyone ever releases them. Not a griefing vector — a
permanent lock, reachable by accident, on a game that never started.

**Fix.** The opening state does not need a signature of its own, because it
already has two. The `OpenGame` transaction carries both players' signatures
over a `GameOffer` that commits to `start_pos_hash` and `base_time_ms`, which
is every field of the ply-0 state. The escrow can reconstruct it and compare.
`spec/05` amended with a "ply-0 exception" section.

**Lesson.** An authorisation rule of the form *"admit this object if the right
party signed it"* has a hole at the first object, because the first object is
authorised by whatever created the sequence rather than by anything inside it.
Genesis is not the zeroth element of the chain; it is the thing the chain
hangs from. This is the same shape as bug 01 — the compression function was
fine and the *buffering around it* was wrong — and it showed up the same way,
by testing the interface rather than the algorithm.

**Regression test:** `you_can_win_against_an_opponent_who_disconnects` reaches
the adjudicator at all because of this; `the_board_checkable_draws_need_no_evidence_but_must_be_true`
opens at ply 0 directly.

---

## 08 — Refuting a stalemate is not the same as refuting a mate

Not a bug that ran, but a line of spec that would have become one.

`spec/05` said a successful refutation strikes the claim and **the game
resumes at the refuting move**. True for checkmate: the claim is "you have no
move", the refuter is the allegedly mated player, and the move they post is
their own. Playing it is exactly right.

Stalemate inverts every one of those. The claimant *is* the side to move — "I
have no move" — so the refuter is their **opponent**, and the move being
exhibited belongs to the claimant. Resuming at it would let your opponent
choose your move for you. That is not a penalty for a false claim; it is a
different game.

So the two cases share a check and not a consequence: both verify one legal
move and both halve the liar's budget, but mate resumes at the move and
stalemate merely strikes the claim and makes the claimant move.

**Lesson.** Two mechanisms that verify the same predicate are not therefore
the same mechanism. The tell here was the word "the refuting move" quietly
assuming the refuter owned it — an ownership question the shared predicate
does not ask.

**Regression test:** `a_stalemate_refutation_does_not_let_the_opponent_pick_your_move`.

---

## 09 — The experiment that confounded itself

**Symptom.** The personality estimator recovered strong styles poorly and weak
ones not at all, with no clear pattern.

**Cause.** Not in the estimator. The synthetic corpus generator emits games
grouped by player — all of player 0's, then all of player 1's — and the
train/validation/test split was taken by index. So player 0 got almost all of
the training data and player 9 almost all of the test data. Personality strength
was perfectly confounded with sample size, because the players were created in
order of increasing strength.

**How it surfaced.** Not from the results, which looked plausible. From the
`train` and `test` columns printed beside them: 7426/1436 on the first row and
2600/5140 on the last. The diagnostic was in the table by accident.

**Lesson.** Print the size of every split next to every result. It costs one
column and it is the cheapest possible check that an experiment is measuring
what it claims. A result table that only shows results cannot be audited.

---

## 10 — The ground truth was wrong, not the estimator

**Symptom.** Players constructed with *zero* personality measured +0.010
nats/move of divergence. Read as estimator bias — the exact failure the whole
calibration exercise exists to detect.

**Cause.** The ground truth compared each player against the **base policy** the
synthetic world was built from. But the estimator measures divergence from the
**fitted population model**, and those are not the same distribution: the fitted
population is the average of everyone actually playing, and it is pulled off the
base by whichever eccentric players are in the corpus.

So a personality-free player genuinely *does* diverge from the population — by
however much the population as a whole is skewed. The estimator was right. The
oracle was wrong.

**Lesson.** When an estimator disagrees with ground truth, the ground truth is a
suspect too. This is the third time in this project that a reference
implementation turned out to be the broken half (see 03). The habit to build:
before believing a discrepancy, ask what *exactly* each side is computing, and
whether they are the same quantity.

It also changed the design of the null control. "A player with no personality"
is not testable against a skewed population. What is testable is a **chimera** —
a player-shaped pile of moves each drawn from a random member of the
population — whose true policy *is* the population mixture. That reads +0.0006
nats/move, which is the control that was actually wanted.

---

## 11 — The PGN parser that silently deleted every other move

**Symptom.** A hand-written test parsing Morphy's Opera Game failed immediately,
while a nearly identical Scholar's Mate test passed.

**Cause.** The tokeniser discarded any whitespace-separated token that started
with a digit and contained a dot, treating it as a move number. PGN writes
`1. e4` and `1.e4` interchangeably — so for the second spelling the move was
thrown away along with its number. Scholar's Mate had been typed with spaces;
the Opera Game had not.

**Why it could not have hidden.** SAN is parsed as a *filter* over the legal
move list, so dropping White's move makes Black's move illegal for White on the
very next token. A game that parses to the end is almost certainly parsed
correctly — and the Opera Game ends in mate, so reaching the end validates the
parser and `bc-chess` against each other in one assertion.

**Lesson.** Prefer formats where a parse error propagates immediately over
formats where it degrades quietly. Had SAN been parsed generatively — render
each legal move to a string and compare — the same bug would have produced a
plausible but wrong move sequence and quietly poisoned a corpus.

---

## 12 — The adjudicator computes the quantifier it exists to avoid

Found while writing `docs/episode-08-gap.md`, by reading episode 08's code
against `P3` rather than against its own tests. Not a wrong answer — a wrong
cost, which is the kind that passes every test you thought to write.

**The invariant.** `P3`: *never verify checkmate; assert it and allow
refutation by a single move.* The whole of `spec/05`'s cost argument rests on
it — claiming mate is ∀ over ~218 moves, refuting it is ∃ over one, and the
expensive side is the one that is usually true and therefore never checked.

**What shipped.** `Dispute::apply_move` calls `settled_on_the_board`, which
calls `Position::outcome()`, which calls `no_legal_moves()`, which calls
`generate_legal()`. So **every on-chain move generates the full legal move
set** to notice a mate that has appeared on the board.

**Why it looked right.** It is right, in the sense of returning correct
answers, and it is genuinely convenient: a game that ends by mate during a
dispute settles immediately rather than waiting for someone to claim it. The
test `the_game_continues_on_chain_under_the_same_rules` asserts exactly that
and passes. Nothing about the behaviour is wrong.

The mistake was reasoning about `P3` as a rule concerning *claims* — never
accept "this is mate" without allowing refutation — when it is a rule
concerning *computation*. The client-side code has the same call and there it
is correct, because `docs/duality.md`'s "Where the expensive check belongs"
says the expensive check belongs where it is cheap. The adjudicator is the
other side of that entry, and the same line of code changes meaning when it
crosses the boundary.

**What it costs.** On a real chain, one dispute move is either one check test
or two hundred and eighteen move generations plus two hundred and eighteen
check tests. Per move, for every disputed game. `P3` exists because somebody
pays gas for that.

**The fix is not local.** Mate during a dispute should be reached the way
every other terminal condition is — through `DisputeClaimTerminal`, which is
already optimistic and already refutable. That means `apply_move` returns
`Continues` unconditionally and a player who has just delivered mate claims
it. It belongs with D24's extraction of `bc-adjudicator`, where a `no_std`
crate boundary makes "this is consensus code" a property of the compiler
rather than of the reader's memory.

**Lesson.** An invariant stated as a rule about *what you may believe* will be
read that way, and this one is a rule about *what you may spend*. Both
readings license the same behaviour at the client and different behaviour at
the chain, and the code that differs sits in two crates that currently share
a module. Boundaries that exist only in prose do not enforce anything — which
is the argument for D24 restated as a bug.

---

## 13 — What the ∀ cost, once it was actually removed

Follow-up to §12, which found that `Dispute::apply_move` called
`Position::outcome()` on every on-chain move and so generated all ~218 legal
moves to notice a mate — the quantifier `P3` exists to keep off the chain.

**The fix was not "stop calling it".** Deleting the call leaves a game that
reaches mate on-chain and then just sits there, because nothing notices. What
was missing was the other half of the optimistic design: somebody has to
*say* it is mate. `spec/05` had this all along —
`DisputeMove { channel_id, ply, move, [new_status] }` — and the implementation
had silently dropped the optional field and replaced it with a search.

So the claim now rides along with the move. One transaction, no quantifier,
and the claim is refutable like any other.

**What the tests had to become, which is the instructive part.** The old test
was `the_game_continues_on_chain_under_the_same_rules`, and it asserted that
playing Fool's mate through the adjudicator settled the game. That assertion
was the bug, written down as an expectation and passing. The replacement
asserts the opposite and says why:

```rust
assert_eq!(dispute_move(…, mate, None), None, "the chain does not notice mate by itself");
assert!(dispute(&id).unwrap().pos.is_checkmate(), "…even though it is, in fact, mate");
```

Two assertions that look contradictory and are not. The first is the
invariant; the second is what makes the first surprising enough to need
stating.

**Lesson.** A test that encodes convenient behaviour will defend it. This one
had been green since the day it was written and was the reason the cost bug
survived review — not because anybody argued for computing the ∀, but because
nobody was looking at cost, and the only thing watching was a test that
preferred the expensive answer.
