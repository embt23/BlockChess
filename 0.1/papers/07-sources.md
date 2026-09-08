# 07 — Sources

Every value marked `[lit]` in the papers, with its origin, so that each can be
checked or replaced by a measurement of our own. Nothing else in `papers/` is
taken on trust: the rest is either computed by `measure/` or is arithmetic.

---

## Position counts

| Value | Claim | Source |
|---|---|---|
| ~1e43 | positions | Shannon, "Programming a Computer for Playing Chess", *Philosophical Magazine* 41 (1950). An estimate made in passing; quoted for historical contrast only. |
| 1.7986e46 | rigorous upper bound on legal positions | S. Chinchalkar, "An Upper Bound for the Number of Reachable Positions", *ICCA Journal* 19(3), 1996. |
| 4.82e44 | estimate of legal positions | J. Tromp, "Chess Position Ranking" / "Number of legal chess positions" (2021). Monte Carlo estimate with a stated confidence interval, **not** a bound. Cited as an estimate throughout. |

`measure/counting.py` derives 6.19e51 independently. The gap to Chinchalkar is
attributed in `01-position-space.md` §3 to constraints a product formula cannot
express, and that attribution is an argument, not a proof.

## Chess facts

| Value | Claim | Source |
|---|---|---|
| 218 | maximum legal moves in any position | **No longer taken on trust.** Our own generator counts exactly 218 in the usual witness position `R6R/3Q4/1Q4Q1/4Q3/2Q4Q/Q4Q2/pp1Q4/kBNN1KB1 w - -`; reproduce with `blockchess show "<that FEN>"`. It remains a literature claim that no position exceeds it — that is a search result, not something one position can confirm. |
| perft counts | move-generator correctness oracle | Chess Programming Wiki, "Perft Results". `blockchess perft <n>` checks against them and prints MATCH or MISMATCH. The generator in `crates/bc-chess` passes all of them. |

## Compression figures

These are the weakest citations in the papers and the ones most in need of
replacement. Both underpin D3 and D4.

| Value | Claim | Status |
|---|---|---|
| ~4 bits/ply | static heuristic ranking + Huffman | Reported for production chess-database compressors of this design; the widely cited public example is the Lichess game encoder. **Not independently verified here.** Experiment X2 produces our own figure and our own table. |
| ~1.5–2 bits/ply | engine or policy-network prior + arithmetic coding | The range reported across published lossless chess compressors using an engine prior. Taken as 1.7 as a working figure. **Not independently verified here.** Experiment X3. |

`04-permanence.md` §2 computes the "price of permanence" as the difference
between these two rows. **That headline number is therefore a difference of two
literature values and should be treated as provisional until X2 and X3 are
done.** It is flagged here rather than buried because the whole of D4 rests on
it, and because the conclusion happens to be robust: at per-game attestation the
gap is worth 13% of the record, and it would take a much larger gap than any
reported figure to change the recommendation.

## Cryptographic sizes

Ed25519 public key 32 bytes, signature 64 bytes: RFC 8032. BLS aggregate
signature 48 bytes on BLS12-381: draft-irtf-cfrg-bls-signature. Both are
standard and used only for arithmetic in `measure/envelope.py`.

## Corpus scale

6e9 games as the working figure for "a large public database", used only to
give the storage tables a scale. Order-of-magnitude, from the size of the
largest public game archives. Nothing depends on it beyond one column.
