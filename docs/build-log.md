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

---

## 14 — `no_std` was the easy half; the dependency list was the point

`D24` asked for the adjudicator as a `no_std` crate depending only on
`bc-chess`. Two things were more interesting than expected.

**`no_std` turned out to be nearly free, and that was informative.** Every
piece of `std` in `bc-chess` was in FEN parsing, `to_uci`, SAN, `render` and
`divide` — notation and debugging, all of it allocating, none of it on the
consensus path. `bc-hash` needed one `hex` helper gated. Nothing in
`make_move`, `is_move_legal`, `generate_legal`, `pack` or `unpack` allocates
at all.

That is not luck. Those functions were written to be cheap for `perft`, which
runs them 119 million times, and the shape that makes a move generator fast is
the same shape that makes it suitable for consensus: fixed buffers, no
allocator, no I/O, no ambient state. The optimisation and the determinism
requirement wanted the same thing.

**The dependency list needed a test, and writing it changed what it said.**
The obvious allow-list was `bc-chess` alone, per the decision. But a state
needs a hash, so `bc-hash` has to be there — and once that is written down,
the question "what else might reasonably creep in?" has an obvious answer:
`bc-sig`, the moment somebody wants to verify a resignation inside the
dispute machine.

It should not be there, and the reason is worth keeping: **deciding whether a
signature is good is the escrow's job; deciding what follows from it is the
adjudicator's.** The split is not about layering neatness — it is that the
adjudicator takes already-verified claims, so it cannot be tricked by a
signature check it got wrong, because it does not do any. `bc-sig` being
absent from that list is a security property, not tidiness.

So `tests/dependencies.rs` asserts the list, asserts there are no
dev-dependencies either (a dev-dependency is how a type quietly becomes
`pub`), and greps the source for `f32`, `f64`, `HashMap` and `HashSet` —
the two classic ways consensus stops being bit-identical.

**Lesson.** A boundary enforces nothing unless something checks it. The
comment in the manifest saying "no signatures here" would have survived
exactly until the first person with a good reason, and they always have one.

---

## 15 — Eighteen months of a wrong draw rule, found on the first oracle run

`D23` said to differential-test the terminal predicates against an
independent implementation, on the grounds that `perft` — the oracle episode
03 was verified against — counts nodes and **never asks how a game ends**. A
bug in `insufficient_material` cannot change a perft count, because perft
does not call it.

The first sweep against `shakmaty` disagreed at ply 387 of random game 303:

```
2B5/8/K7/5Bk1/8/8/8/8 b - - 0 194
  bc-chess: sufficient material     shakmaty: insufficient
```

Two white bishops, on c8 and f5, both light squares, against a bare king.
They cannot mate — a bishop confined to one square colour can never attack
the other — and the game is a dead draw the moment that material is reached.
`bc-chess` said play on.

The rule it was applying:

```rust
2 => knights == 0
    && white_bishops.count_ones() == 1
    && black_bishops.count_ones() == 1
    && (all bishops light || all bishops dark),
_ => false,
```

which is the same-colour-bishops case and nothing else. The comment above it
read *"two bishops of one colour mate easily"*, meaning two bishops on
opposite square colours — and the code then used `count_ones()` on the
*piece* count while the comment was about the *square* colour. The sentence
and the code were each half right about different things.

The correct rule has no counting in it at all:

```rust
if knights == 0 {
    (bishops & LIGHT) == bishops || (bishops & DARK) == bishops
} else {
    bishops == 0 && knights.count_ones() == 1
}
```

Any number of bishops, split between the players however you like, all on one
square colour. K vs K, K+B vs K and same-colour K+B vs K+B fall out as
special cases rather than being enumerated.

### Why this one costs money

An insufficient-material draw is not cosmetic here. `spec/05` gives each
disputing player a **block budget**, dilated once from their game clock. A
position the adjudicator wrongly believes is still playable keeps consuming
that budget, move after move, in a game neither side can win. Whoever runs
out first loses a pot they were entitled to split. The wrong answer is not
"the draw is declared late" — it is "the draw is declared for the other guy".

### What actually caught it

Not the eight hand-written tests in `tests/terminal.rs`, which include a test
named `bishops_on_one_complex_is_about_the_squares_not_the_colours` and which
passed before and after the fix. It tested one bishop each, because that is
the position its author pictured. Random play reached four-bishop endgames;
no human writing test cases reaches for those.

### The thing that made the test itself unsafe

While writing the case list, **four of the hand-written mate positions in it
were not mate.** `7k/8/8/8/8/8/8/R5K1 b` leaves the king g8, g7 and h7.
`7k/5Q2/6K1/8/8/8/8/8 b` is stalemate, not mate. `R5k1/8/6K1/…` *is* mate,
and had been written down as the counter-example. That is now five wrong mate
FENs across the project's history (`§03` has the others), from someone who
has been staring at chess positions for months.

So the case list was restructured: every row carries its expected `Outcome`
as data, `bc-chess` is asserted against the label, and only then is
`shakmaty` asked whether both of us are right. A label that is a comment can
be wrong forever. A label that is an assertion cannot.

**Lesson.** `G1` says an oracle must be published by someone else. The
sharper version, after this: an oracle must also be *reached* by inputs you
did not choose. A second implementation you only ever query with your own
hand-picked positions is a second implementation of your own blind spots.
1,075,744 random positions found in four seconds what eight careful tests
missed for eighteen months.

---

## 16 — The model check found the bound that only half existed

`D23`'s second half: the dispute machine has no oracle, because clock
dilation, block budgets and override-by-ply are mechanisms this project
invented and nobody else implements. So the properties `spec/05` argues in
prose get checked instead — exhaustively, over every reachable state, by
`stateright`.

The model drives the **real `Dispute`**. Every transition is an actual call
to `apply_move`, `refute` or `verdict`; what is abstracted is only the
environment — which of the legal moves gets tried, and at which of three
heights (as early as possible, exactly on the deadline, one block late).
A hand-written abstract model would have proved things about the
abstraction, and the abstraction is precisely where a re-derivation drifts
from the shipped code.

It failed on the first run, and it failed on the property that looked most
like a formality.

### The counterexample

```
Move   { when: OnTheDeadline,      claim: Some(Checkmate) }
Refute { when: OnTheDeadline }
Move   { when: AsEarlyAsPossible,  claim: Some(Checkmate) }
Refute { when: OnTheDeadline }
LetTheClockRun
→ ply: 4, max_plies: 4, budget: [0, 0]
```

Four plies against a cap of four, and the game had not ended at the cap. The
reason is one line long:

```rust
// apply_move
if self.ply >= self.max_plies { return Ok(MoveOutcome::Ended(Status::Draw)); }

// refute, ClaimKind::Checkmate
self.pos = self.pos.make_move(mv);
self.ply += 1;          // ← and nothing else
self.arm(height);
```

**Two code paths add a ply. One of them checked the cap.** A player
alternating false mate claims with their opponent's refutations advances the
ply indefinitely and never meets the bound.

### Why it is not "just a bound"

`max_plies` is not a sanity limit. It is in the signed `GameTerms`, it is
what bounds the worst-case replay a validator must be able to afford, and
`spec/02` sizes reserved dispute gas against it. A path that exceeds it is a
path where the chain does more work than the terms it committed to — and in
`bc-channel` it also means a `GameState` at a ply the channel signed a cap
below.

It is self-limiting in money terms: a false claim costs the claimant half
their remaining budget, so the cycle bankrupts the attacker after about five
turns. That is why nothing visibly broke, and it is exactly the shape of bug
that survives a test suite. The invariant was false; the exploit was merely
expensive.

### The fix is where the check belongs, not where it was missing

The tempting fix is to copy the `if` into `refute`. That gets this bug and
leaves the next one, because the property is *about states*, not about the
two functions that currently produce them. So the authoritative check moved
into `verdict`, which every path must pass through to settle:

```rust
pub fn verdict(&self, height: u64) -> Result<Status, DisputeError> {
    if self.ply >= self.max_plies { return Ok(Status::Draw); }
    ...
```

`refute` also reports it eagerly, as `Refutation::CapReached`, so a caller
learns at once rather than on the next block. But the eager checks are now
an optimisation. The total one is in `verdict`.

### The property that was wrong was mine, not the code's

The other first-run failure was my own. I wrote *"no player is ever given a
deadline they cannot meet"* and the checker produced a player at zero budget
with a zero-block window. That is not a bug — a player who has spent their
entire dilated clock is supposed to lose on time. The property had to be
restated as two weaker true ones:

- the window never exceeds the budget that pays for it, and
- a player with budget remaining always gets a nonzero window.

Which is the honest limitation of this whole technique, and worth writing
down next to the success: **a model check cannot tell you your properties
are wrong.** It agreed enthusiastically with a false one until it found a
state where it mattered. `G1` wants an oracle someone else published, and
this half of episode 08 does not have one; the model check is the best
available substitute and it is not the same thing.

### The measure

Termination is checked as a ranking function — a quantity that strictly
decreases on every non-settling transition and is bounded below:

```
2·(max_plies − ply) + budget_white + budget_black + (a claim is pending)
```

The weight of 2 on the ply term is load-bearing and was not obvious. A move
that *opens* a claim spends a ply (down one) and adds a pending claim (up
one); at weight 1 those cancel exactly, the measure stalls, and the checker
says so. Two plies are worth more than one claim because a claim can only be
opened by spending a ply and the ply is never refunded.

`spec/05` says "budgets only ever decrease, so the process terminates".
That sentence is true and it is not the proof, because budgets are not the
only thing moving.

**Lesson.** Writing the termination argument as an expression rather than a
sentence is most of the value. The English version had been read many times
by both authors and neither noticed it quantified over the wrong thing.
9,432 states, four seconds.

---

## 17 — The real chain did not confirm episode 08, it broke it

`D21` said Milestone E was *simulated* rather than earned, because the
adjudicator ran against a `BTreeMap` height counter, and that building a
real chain would fix that. The expectation was a confirmation: same
behaviour, better provenance.

The first run of `bc-node`'s reorg scenario gave the opposite.

A `BTreeMap` height counter has exactly one interesting property: **it only
goes up**. Every deadline argument in `spec/05` is written against a
monotone height, and against a monotone height every one of them is
correct. A real proof-of-work chain is not monotone. It reorganises, and
the blocks that leave the canonical chain take their transactions with
them.

So:

```
block 3   White moves on-chain; Black must reply by block 67
block 4   Black replies — 63 blocks early
block 10  Black's reply is 7 deep — settled, by convention
          …White publishes 65 blocks mined in private
published — reorg 7 blocks deep
block 68  Black's reply is GONE
verdict   WhiteWins — Black lost a game they defended
```

Black did everything the protocol asked, inside the window, and lost the
pot. The counter-intuitive part is the one worth saying out loud: **waiting
for more confirmations would not have helped.** The usual advice for
probabilistic finality is "wait longer for larger amounts", and it fails
here because the deadline is not waiting with you. By the time the reorg
arrived, block 67 had passed.

### The stub was not a weaker test, it was a different one

This is the part worth generalising. The gap document defended the stub on
the grounds that a driven counter tests the dispute machine *more*
thoroughly than a real chain — every deadline can be stepped over exactly,
which no real chain lets you do. That is still true.

But it tests the machine under an assumption it never states. The
assumption is "height is monotone", it is load-bearing for every deadline
in `spec/05`, and a stub that satisfies it perfectly can never surface it.
The substitute was not *less* faithful in degree; it was faithful in a
different shape, and the shape was where the bug lived.

The generalisable form: **a test double that satisfies an unstated
invariant of the real thing will never tell you the invariant exists.**

### What actually changed

Not the adjudicator. Not one line of it. The sentence:

> ~~You can win against an opponent who disconnects.~~
> You can win against an opponent who disconnects, **on a chain with
> deterministic finality.**

`spec/02` already argued for BFT on exactly these grounds, in prose, under
the heading *"Money under a deadline requires deterministic finality"*.
That argument was correct and had never been run. `Finality::Probabilistic`
and `ProofOfWork::is_final` returning `false` at every depth are that
paragraph turned into two lines a caller can branch on.

### A smaller one, from the BFT side

A node that heard a ⅔ prevote quorum **before** the proposal reached it —
the network reorders, and 24 of 24 seeds eventually produce this —
precommitted, acquired a lock, and then tried to prevote when the proposal
finally arrived. Steps only go forwards, and the missing guard was one
line. What is worth keeping is how it showed up: as a `todo!()` panic from
the `G0` hole, because the only path that reaches the locking rules at
round 0 is a node in a state round 0 should not have. **An unimplemented
function is a very effective assertion.**

**Lesson.** A stub is a hypothesis about which properties of the real thing
matter. Replacing it is worth doing even when you expect nothing to change,
because the value is not in confirming the behaviour — it is in finding out
which of your assumptions were being supplied by the stub.

---

## 18 — The deadline you are billed 8 blocks for and given 5 to meet

Episode 10 started as censorship work and found something a censor never
needed. Two functions in the adjudicator disagreed about what a move
costs.

```rust
// what a move is CHARGED
blocks_consumed(started, now) = max(now - started, MIN_MOVE_BLOCKS)   // ≥ 8

// what a move is GIVEN
arm() → window = min(Δ, budget)                                       // no floor
```

A budget falls by at least 8 per move, so it walks down in steps of 8 from
wherever it started — and where it started is `clock/τ + 32`, which is
whatever the clock happened to be. A budget of 41 goes 41 → 33 → 25 → 17 →
9 → **1**. A budget of 45 bottoms out at 5.

So the last window before a player flags is routinely smaller than the
minimum a move is billed for, and at a budget of 1 it is **one block**.
Two seconds, at a two-second block time, to get a transaction from a
laptop through a mempool and into a block. Nobody has to censor anything;
the deadline was simply not meetable.

A sweep of clocks from 0 to 6000 ms found the worst case at 7 ms and
288 ms, both of which give windows of 1.

### Why the model check had agreed this was fine

`bc-conformance`'s dispute model has a property called *"no player is given
a deadline they cannot meet"*, and it had been passing since episode 08.
It read:

```rust
window <= budget && (budget == 0 || window > 0)
```

A one-block window is greater than zero. The property was satisfied,
exhaustively, over every reachable state, and it was **wrong** — it
encoded "you get *a* window" when what matters is "you get a window you
can use". `build-log` §16 already said a model check cannot tell you your
properties are wrong. This is the second time that has cost something, and
the first time the property in question was one I had written two episodes
earlier while fixing a different instance of the same mistake.

It now reads: no budget → no window; budget → at least `MIN_MOVE_BLOCKS`
and at most Δ. The `window <= budget` clause is gone, deliberately, because
the fix breaks it and nothing is lost — budgets decreasing monotonically is
a separate property already checked.

### The fix gives nothing back

The obvious worry with raising a floor on the window is that it hands a
flagging player time they had spent. It does not, and the reason is worth
keeping: **the budget still falls by at least `MIN_MOVE_BLOCKS` per move.**
The number of on-chain moves a side gets is `ceil(budget / 8)` either way.
All the floor changes is whether the last of those moves is physically
possible. That is the same argument `FLOOR_BLOCKS` already makes once for
the whole game, applied per move — and `FLOOR_BLOCKS` exists in the code
with the comment *"so that even a flagging player can physically move"*,
which turns out to have been half-implemented for as long as it has
existed.

### And then the censorship consequence, which is the episode

With the floor in place, the smallest window any dispute can produce is
`MIN_MOVE_BLOCKS`. Minimise the arm expression over every live budget:

```text
    inf over budget ≥ 1 of  max( min(Δ, budget), MIN_MOVE_BLOCKS )
  = max( min(Δ, 1), MIN_MOVE_BLOCKS )
  = MIN_MOVE_BLOCKS
```

**Δ cancels.** The challenge window a channel negotiates is the *maximum*
it will ever get, and censorship safety is a question about the *minimum*.
A channel that agreed Δ = 2048 is defended, in its last few moves, by
exactly the same 8 blocks as one that agreed 64.

That inverts what `spec/02`'s third defence claims. "Generous Δ" reads as
the tunable knob for censorship resistance, and it is not a knob at all —
the binding quantity is a protocol constant that no channel negotiates.
The rotation bound has to be applied to the floor, which is why
`bc_bft::censorship::smallest_window` takes Δ as an argument and then
visibly discards it.

### A footnote, from making exactly the same mistake again

The first version of `smallest_window` guarded the Δ ≥ floor case with a
`debug_assert!` and a `#[should_panic]` test. The suite was green. The
**release** suite was not: `debug_assert!` compiles out, so the panic never
came and the test that demanded one failed.

That is the third instance of the pattern in this entry. Two descriptions of
one fact, and the disagreement invisible from either side alone — here, a
test that says "this input is rejected" and a check that only exists in one
build profile. The fix was to stop asserting and make both functions
**total**: `arm` floors the window at `min(Δ, move cost)` rather than at the
move cost, so it can never hand out more than the channel signed, and
`smallest_window` returns the same expression. No assertion, no profile
dependence, and the two now agree by construction rather than by a test
noticing.

**Lesson.** Two functions describing the same quantity — what a move costs
in blocks — from two directions, written weeks apart, and never compared.
The bug was not in either of them. It was in the absence of any place where
both appear, which is also why it survived a model check: the property was
written from the same side as the bug. The general fix is not a better
assertion, it is one expression that both sides call.
