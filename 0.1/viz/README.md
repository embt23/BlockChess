# viz/

The graphics layer. **These pages do not implement chess.**

`papers/04-permanence.md` §5 is about a move generator being consensus-critical
— two implementations that disagree corrupt data silently. The same argument
applies here for a smaller reason: a board drawn by a second, JavaScript
implementation of the rules would eventually disagree with the Rust one, and
nobody would notice.

So the engine emits positions and the page draws them:

```sh
blockchess export game.pgn --notes game.notes.txt > game.json
```

One record per ply — FEN, legal move count, bits, the move played, and any
notes anchored there. The page decodes FEN into squares and renders. That is
the only chess knowledge it has, and FEN decoding cannot disagree about
legality because it never asks.

| file | what it is |
|---|---|
| `perception.html` | Step a game; watch bits per ply, human notes, and which discovered patterns the game is currently inside. |

Published at
<https://claude.ai/code/artifact/b053074b-2f6d-458f-8e7c-16f4a9d6c306>.

## Making one from your own game

```sh
blockchess play --save mygame.pgn      # 'note <text>' while playing, then quit
blockchess annotate mygame.pgn mygame.notes.txt
blockchess export mygame.pgn --notes mygame.notes.txt > mygame.json
```

Then swap the payload in the page. The patterns and the lift table come from
`blockchess grammar` over a real corpus and are static in the page — they
describe the corpus, not your game.
