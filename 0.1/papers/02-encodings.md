# 02 — The encoding zoo

*Fifteen ways to write down a chess game, scored on ratio, decode cost,
permanence and reach. Computed figures come from `measure/plyrate.py`.*

---

## The scoring axes, restated

| Axis | Bad | Good |
|---|---|---|
| **Ratio** | bits per ply, lower better | |
| **Decode** | what a reader must run | nothing / a rules engine / a model |
| **Permanence** | what must never change | nothing / the rules / a model file |
| **Reach** | what it forbids | nothing / non-standard rules / anything unusual |

`Permanence` is the axis that does not appear in ordinary compression papers
and it is the one that decides this. A scheme whose decoder is a 200 MB neural
network is not "1.5 bits per ply"; it is 1.5 bits per ply plus a 200 MB
consensus object that may never be corrected.

---

## Group A — schemes that need no chess knowledge

The decoder needs to know nothing about chess. It cannot check legality, and it
will happily decode nonsense, but it will decode *forever*.

### E1 — SAN text, as in PGN
`e4 e5 Nf3 Nc6 Bb5` — 40 bits/ply, **400 bytes/game**.

The reference point, and the format every existing database uses. Its virtue is
that it is the interchange format of the entire chess world and a human can
read it. Its vice is that it is ambiguous without the position (`Nf3` requires
knowing which knight can go there), so it is *not* actually decode-free — it
needs a rules engine to disambiguate, while paying full text price. Worst of
both groups. Keep it as an import/export format, never as storage.

### E2 — UCI text
`e2e4 e7e5 g1f3` — 40 bits/ply, **400 bytes/game**. Unambiguous, decode-free,
and exactly as large. Strictly better than E1 as storage and strictly worse as
an interchange format.

### E3 — packed move, 16 bits
`from(6) | to(6) | promotion(2) | flag(2)` — 16 bits/ply, **160 bytes/game**.

The natural machine encoding. Decode-free, rules-independent, and it survives
any variant played on a 64-square board. 2.5× better than text for nothing.

### E4 — from/to only, 12 bits + escape
`from(6) | to(6)`, with a 4-bit escape appended only on promotions — about
12.2 bits/ply, **~122 bytes/game**.

Promotions are rare, so the amortised cost is near 12. Castling encodes as
king-from/king-to, en passant is inferable from the position. Now the decoder
needs *some* chess knowledge to tell an en passant capture from a quiet move,
so this sits on the boundary of the group.

**Group A floor is 12 bits/ply.** You cannot beat it without consulting the
rules, because without the rules every one of the 64×64 square pairs is a
candidate.

---

## Group B — schemes that run the rules

The decoder generates the legal moves in the current position and uses the
encoded value to pick one. This is the interesting group.

### E5 — legal-move index, one byte
Enumerate legal moves in a fixed canonical order, store the index — 8 bits/ply,
**80 bytes/game**. The 218-legal-move maximum `[lit]` fits in a byte with room
to spare.

### E6 — legal-move index, `⌈log2 b⌉` bits
Spend only as many bits as the position needs. Self-delimiting, because the
decoder knows `b` before it reads.

### E7 — legal-move index, arithmetic-coded
The same choice priced exactly: `log2 b` bits, with no rounding waste.

| b | `⌈log2 b⌉` | `log2 b` | bytes/game at that b |
|---|---|---|---|
| 5 | 3 | 2.322 | 23.2 |
| 20 | 5 | 4.322 | 43.2 |
| 30 | 5 | 4.907 | 49.1 |
| 40 | 6 | 5.322 | 53.2 |
| 218 | 8 | 7.768 | 77.7 |

The number that matters is `E[log2 b]` over a real corpus, which is strictly
below `log2 E[b]` by Jensen and which we have not measured. **Experiment X1.**
Taking `b ≈ 30` as a working figure gives **≈49 bytes/game**, and a 3.3× win
over E3 for the price of running a move generator.

### E8 — rank the legal moves, then entropy-code the rank
The moves are not equally likely. Order them by a cheap static heuristic —
captures by value, checks, central pawn pushes, castling — and the played move
is usually near the top. Code the *rank* with a fixed Huffman table.

This is the trick used by production chess-database compressors and it reaches
around 4 bits/ply `[lit]`, **≈40 bytes/game**. The heuristic is a hundred lines,
it is deterministic, and it is small enough to write into a specification. It is
the best ratio available without a learned model.

### E9 — engine or policy-network prior, arithmetic-coded
Replace the heuristic with an actual evaluation. Reported results in the
literature reach the region of 1.5–2 bits/ply `[lit]`; take 1.7 as the working
figure, **≈17 bytes/game**.

This is where the ratio axis stops improving much and the permanence axis falls
off a cliff. See `04-permanence.md`. **The model becomes consensus state.**

### E10 — the corpus itself as the prior
Use the empirical distribution of continuations in the database. Popular moves
are cheap; novelties are expensive. In principle the strongest available prior,
because it is fitted to exactly the distribution being encoded.

It is also self-referential — the encoding of game *N*+1 depends on games 1..*N*
— and `03-corpus.md` is entirely about whether that can be made to work and
what it is really worth. Short version: it can, via checkpoints, and it is
**worth less than it looks**, because it is the same saving as E9 counted twice.

---

## Group C — schemes that store something other than moves

### E11 — store positions
80 positions at the ~150-bit floor from `01-position-space.md` — **≈1.45 KiB per
game**, 30× worse than E7. Its one advantage is random access: you can read
position 40 without replaying 39 moves.

Not worth it as storage. It is worth it as an *index*, on a sampled basis: store
a position key every *k* plies so a reader can seek. At *k* = 16 that is 5
checkpoints per game, ~94 bytes — as much as the game. At *k* = 40, 2
checkpoints, ~37 bytes. This is a real knob and it belongs in `05-index.md`.

### E12 — store the position graph, not the games
Store edges of the position DAG with counts, and games become paths through it.
Transpositions collapse. Excellent for study — "how often does this position
arise, from any move order" is a lookup — and it destroys the thing the database
exists for, because you can no longer recover *which* games those were or in
what order the moves came. Derived structure, not storage.

### E13 — bitboard XOR delta
A move changes few bits, so store `position XOR previous_position`. Tempting,
and it is much worse than it looks: the delta must still identify *which* bits
changed, which is the same information as the move plus positional overhead. A
move is already the minimal delta. Recorded here because it looks clever and
should be dismissed once, in writing.

### E14 — commit only
Put a 32-byte hash on chain and the game data somewhere else. **32 bytes/game**,
beating every other scheme, and it is not a chess database — nobody can study
what they cannot read. This is the option that answers a different question, and
it is the D2 fork in `06-decisions.md`.

### E15 — symmetry canonicalisation, as a modifier
Store the canonical representative of the position's orbit plus the group
element. From `01-position-space.md`: worth 0–1 bit per *position*, and since we
store moves, **0 bits**. Rejected as compression; retained as an index key.

---

## The table

Ratios below 4 bits/ply are literature figures `[lit]`, not measurements of
ours; everything else is computed or exact by construction.

| | scheme | bits/ply | bytes/game | decode | permanence | reach |
|---|---|---|---|---|---|---|
| E1 | SAN text | 40 | 400 | rules engine | rules | standard only |
| E2 | UCI text | 40 | 400 | none | none | anything |
| E3 | 16-bit move | 16 | 160 | none | none | any 64-square variant |
| E4 | 12-bit + escape | 12.2 | 122 | partial rules | rules | any 64-square variant |
| E5 | index, one byte | 8 | 80 | full movegen | **rules exactly** | one rule set |
| E6 | index, `⌈log2 b⌉` | ~5.4 | ~54 | full movegen | **rules exactly** | one rule set |
| E7 | index, arithmetic | ~4.9 | ~49 | movegen + coder | **rules exactly** | one rule set |
| E8 | ranked + Huffman | ~4 `[lit]` | ~40 | movegen + table | rules + **table** | one rule set |
| E9 | model prior | ~1.7 `[lit]` | ~17 | movegen + **model** | rules + **model** | one rule set |
| E10 | corpus prior | ≤1.7 | ≤17 | movegen + **corpus** | rules + **history** | one rule set |
| E11 | positions | ~148 | 1,484 | none | none | anything |
| E12 | position DAG | n/a | n/a | — | — | loses identity |
| E13 | XOR delta | ≥16 | ≥160 | rules | rules | — |
| E14 | commit only | — | 32 | **nothing readable** | none | anything |
| E15 | symmetry | −0 | −0 | canonicaliser | rules | — |

## Three readings of that table

**The first 3× is free; the second 3× is not.** Going from E3 to E7 costs one
move generator — code that any chess project has anyway — and returns 160 → 49
bytes. Going from E7 to E9 returns 49 → 17 and costs a learned model that can
never be changed. The first trade is obviously right. The second is the subject
of `04-permanence.md`, and the answer there is no.

**The permanence column has a hard step between E4 and E5.** Everything from E5
down is decodable only under one exact rule set. Not "the rules of chess" — *the
rules as implemented by one specific move generator, including its enumeration
order*. A bug fix in that generator changes the decoding of every game already
stored. That is the deepest coupling in the whole design and it is the reason
`06-decisions.md` D3 exists.

**The reach column quietly rules out variants.** The project wants public
discourse about variations in play structure. Every Group B scheme fixes one
rule set at encode time. This does not forbid variants, but it forces the rule
set to be named in the record and forces every rule set to be a permanent,
versioned consensus object. Better to design for that on day one than to
discover it at variant number two.

## What to measure next

**X1 — the branching histogram.** `E[log2 b]` over a real corpus, split by game
phase. Decides E6 and E7 exactly and is the single most useful number missing.

**X2 — the rank distribution under a stated heuristic.** Turns E8 from a
literature figure into ours, and produces the actual Huffman table.

**X3 — E8 versus E9 on the same corpus.** The premium in the first paragraph of
the previous section is currently 4 → 1.7 bits/ply on other people's numbers.
The decision in `04-permanence.md` rests on that gap, so it should be ours.

Protocols in `measure/EXPERIMENTS.md`.
