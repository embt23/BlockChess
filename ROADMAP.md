# Roadmap

## Where things stand

| | Status |
|---|---|
| Chess engine (movegen, SAN, PGN, SEE, negamax) | Done · `./tools/verify.sh` |
| Eight visual layers over one animated canvas | Done · `index.html` |
| Readings: frontier, tension, at-risk, symmetry, entropy, 4-bit encoding | Done |
| Story mode (11 stages) + Explore mode | Done |
| Search map with hover-preview | Done |
| Presentation | Done · `docs/presentation/` |
| Glyph alphabet | Done — six marks, one rotation rule (`NOTES.md`) |
| Page workbench | Done · `workbench.html` |
| Position packing to 256 bits | Done · `tools/tests/pack.mjs` |
| **Running on the author's own games** | **Blocked — needs the page tagged** |
| Felt-border capture | Not started |
| Pattern web + gap map | Specified below |

Verification is one command: `./tools/verify.sh` — perft to depth 4 on three
standard positions, both embedded games parsed to their true final mate, and
eight hand-worked SEE cases.

---

## Phase 1 · Tag the page

The moves are on the journal page, written in the author's own glyphs — about
75% legible, so some squares have more than one reading and the game may fork
into several candidate interpretations.

`workbench.html` is the instrument: drop the photo in, box each grid, line the
overlay up with the hand-ruled lines, and click the squares. Every board is
checked against the rules of chess as it is filled in, so an illegal reading is
caught on the spot — the redundancy in chess does the error correction.

Computer vision is deliberately **not** the tool here: with one page it would
mean training a classifier on a dataset of one.

### Handling the author's files

Never glob for them. A pattern like `find ~ -iname '2026-06-13-*.jpg'` matches
everything taken that day, not the one file intended, and a personal machine has
things on it that must not end up in a repository. The rule: locate candidates,
show the list, have the author confirm which one is wanted, then copy that single
path **by name**. Nothing is copied or committed on a pattern match.

Git history is effectively permanent, so this is not a tidy-up-later problem —
removing a pushed file needs a history rewrite, and orphaned objects stay
reachable by SHA until the host garbage-collects them.

**Verified end to end:** tagging a board of the author's glyphs produces
`rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR` — the standard opening position.
The alphabet reads correctly.

## Phase 2 · The felt border

A capture layer: click squares to draw where the border *felt* like it was, at
any ply, then overlay that against the computed zero contour.

The output is the disagreement — ground conceded that was actually held, and
borders believed quiet that were already underwater. This is the only layer
whose data cannot be computed; it has to come from the player.

## Phase 3 · The pattern web  ← the next build

**Goal:** show the web of patterns *around* the path actually played, so the
unvisited regions become visible as things to learn.

1. **Path.** The positions actually played, `P₀…Pₙ`.
2. **Shoulder.** At each ply, the positions one legal move away that were *not*
   taken — filtered to those that move the frontier or the risk materially, so
   the web shows real forks rather than noise.
3. **Feature vector, in the page's own vocabulary.** Every node described by:
   frontier length · territory split · contested count · tension stake ·
   material at risk per side · mirror symmetry · field entropy · the nine-bin
   histogram of 4-bit direction symbols · centre of mass of each side's
   territory and the distance between them.
   This is the point of the whole notation: it makes a position a vector, and
   vectors can be compared.
4. **Graph, laid out by similarity.** Nodes are positions, edges are moves, but
   position on screen comes from feature similarity — so geometrically similar
   positions sit together and it reads as a **web**, not a game tree.
5. **Motif families.** Cluster the nodes and name each family in chess terms —
   locked centre, open-file fight, opposite-wing race, king hunt, simplified
   endgame. Naming is done against the features, then checked by eye.
6. **Gaps.** Families that appear in the shoulder but never on the path: the
   positions repeatedly available and never entered. Rank by how often they were
   reachable and how far the path stayed away.

**Output:** an interactive web. The played path is a bright line through it;
motif families are regions; unvisited families sit at the edge, labelled with
what they are and what to study to reach them.

## Phase 4 · Playing yourself

Self-play means one mind chose both sides, so any preference shows up twice in
the same game.

Compare the feature distribution of the White choices against the Black choices.
If they favour the same geometry — the same frontier lengths, the same tension
levels, the same time to commit — that is a signature, not a coincidence, and it
is invisible in ordinary games against an opponent.

The likely finding, worth testing rather than assuming: a player against
themselves converges on positions they find *legible*, and the gaps in Phase 3
are exactly the positions they avoid steering both sides into.

## Phase 5 · The territory variant

A chess variant scored by ground rather than material — the frontier as the win
condition — first playable in the atlas, then settled on-chain. `fields()`
already returns the score. The risk is whether the game is any good, which is
answerable off-chain in a day.

A position in the author's alphabet is 13 states a square, 4 bits, 64 squares:
**256 bits, exactly one EVM storage word.** Verified by `tools/tests/pack.mjs`
over 80 positions including castling and en passant. On-chain chess is not new
and nothing here should claim a first; a territory-scored variant may be
unoccupied, but that is an unverified belief, not a fact.

## Phase 6 · Close the loop

Feed the gap list back into the atlas as a practice mode: load a position from
an unvisited family, hide the layers, and ask where the border is before showing
the answer.

---

## Open questions carried from the page

Tracked in `NOTES.md`. The two that would change the work most:

- **`8 geometry`** — boxed and circled, never explained.
- **The large glyph field** — drills, or a transcription? If it is a
  transcription, Phase 1 is already solved and the game is on the page.
