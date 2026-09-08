# measure/

Every number in `papers/` that is not an arithmetic identity or an explicitly
attributed literature value is produced here.

```sh
python measure/run_all.py           # regenerate RESULTS.md
python measure/run_all.py --check   # fail if RESULTS.md is stale (CI runs this)
```

No dependencies beyond the standard library. `symmetry.py` takes about half a
minute; everything else is instant.

| File | What it computes |
|---|---|
| `board.py` | Board geometry: king adjacency, the eight elements of D4, pawn-legal squares |
| `counting.py` | The ladder of upper bounds on the number of positions, and the bit cost of each constraint |
| `symmetry.py` | Which symmetry group survives the rules, and the orbit counts that say what it is worth |
| `plyrate.py` | Bits per ply for every encoding whose cost depends only on the branching factor |
| `envelope.py` | How much of a stored game is chess and how much is signatures |
| `run_all.py` | Runs all of the above into `RESULTS.md` |
| `RESULTS.md` | Generated. Do not edit. |
| `EXPERIMENTS.md` | The numbers we do not have, what each one decides, and how to get it |

## The standard

Carried over from 0.0, and the only thing carried over:

> An external oracle is worth more than any number of self-written sanity
> checks. A reference you wrote five minutes ago is not an oracle, it is a
> second implementation with its own bugs.

So: nothing here needs a chess engine, and that is deliberate. Every figure in
this directory is combinatorics that can be checked by hand on a small case, or
arithmetic on sizes fixed by standards documents. The moment a number needs a
move generator, it belongs in `EXPERIMENTS.md` instead, and it does not get
quoted until the generator has passed perft.
