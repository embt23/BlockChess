# data/

## `eco.tsv` — opening names

3,810 named chess openings: ECO code, name, and the move sequence that reaches
it. Used by `blockchess grammar` to answer the question `papers/08-layers.md`
§3 actually cares about — when the compressor invents a symbol for a repeated
move sequence, **is that sequence something chess players already named?**

Without this file the grammar experiment produces a list of integers. With it,
the list has names on it, and the claim becomes checkable.

### Provenance

Concatenated from `a.tsv` … `e.tsv` of
[lichess-org/chess-openings](https://github.com/lichess-org/chess-openings),
which is dedicated to the public domain under **CC0 1.0**. Vendored rather
than fetched at run time so the experiment is reproducible offline and so a
result can be tied to an exact version of the ground truth.

Columns are tab-separated: `eco`, `name`, `pgn`.

It is *reference data, not evidence*. It says what humans have named. It says
nothing about what the compressor will find, which is the entire point of
comparing the two.
