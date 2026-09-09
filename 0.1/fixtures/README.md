# fixtures/

**Synthetic. Never evidence about chess.** These exist to check that an
instrument responds correctly, which is a different job from measuring
anything.

An experiment that only ever runs on real data cannot tell a null result from
a broken tool. So `blockchess think` — experiment X11, "do bits track
seconds?" — is checked against two corpora whose answer is known in advance
because the answer was constructed.

| file | think times are | expected | measured |
|---|---|---|---|
| `clocks-null.pgn` | random, unrelated to the position | ≈ 0 | **+0.009** |
| `clocks-positive.pgn` | long exactly where players leave the main line | strongly positive | **+0.617** |

```sh
blockchess think fixtures/clocks-null.pgn     --corpus fixtures/clocks-null.pgn
blockchess think fixtures/clocks-positive.pgn --corpus fixtures/clocks-positive.pgn
```

`clocks-positive.pgn` is 200 games in which 90% follow one line and 10%
deviate at ply 4 — and the deviators are given 20–40 seconds on the deviation
against 1–5 seconds elsewhere. That is the hypothesis of
`papers/10-players.md` §1 made true by construction, so recovering it proves
the pipeline can see it. It proves nothing whatever about human beings.

Both carry `[TimeControl "300+3"]`, so they also exercise the increment term —
the trap that would otherwise bias every think time in the project by three
seconds a move.
