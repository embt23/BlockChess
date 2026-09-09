# Five minutes, in front of someone

Run from `0.1/`. Every command is fast; none of them needs a network.

```sh
cd ~/Desktop/claude/BlockChess/0.1
cargo build --release          # once
```

---

## 1. "This thing knows the rules, and I can prove it"

```sh
./target/release/blockchess perft 6
```

```
  perft(6) = 119,060,324
  1.94s, 61.5 Mnps
  published    119,060,324   MATCH
```

**What to say:** it just walked every legal chess game to six moves deep — a
hundred and nineteen million positions, in two seconds — and got the same count
somebody else published decades ago. If one rule were wrong, that number would
be wrong. It is the only place in the project where correctness is checkable
exactly.

---

## 2. "Here is the most crowded position in chess"

```sh
./target/release/blockchess show "R6R/3Q4/1Q4Q1/4Q3/2Q4Q/Q4Q2/pp1Q4/kBNN1KB1 w - - 0 1"
```

A board full of queens, then:

```
  legal      218 moves
  E7 cost    7.768 bits to name one of them
```

**What to say:** 218 is the most legal moves any chess position is known to
allow. We used to take that number on trust from a textbook. Now it is counted.

---

## 3. "A move is a number, and that is the whole trick"

```sh
./target/release/blockchess show
```

Twenty legal first moves, listed with their index. **1.e4 is number 13.**

**What to say:** so you do not write down "pawn from e2 to e4" — sixteen bits.
You write down *13*. log₂(20) is 4.3 bits. The catch is that to read the file
back you need the full rules of chess, and the *order* of that list is now part
of the file format forever.

---

## 4. Hand them the keyboard

```sh
./target/release/blockchess play
```

They type `e4`, `Nf3`, whatever. Then have them type **`bits`**.

**What to say:** watch the cost per move move around. Sharp position, more
options, more bits. Quiet endgame, fewer. That number averaged over every game
ever played is what the whole storage design rests on.

---

## 5. "Watch it compress, and watch gzip fail"

```sh
./target/release/blockchess pack corpus/classics.pgn out.bcg
```

```
  PGN in       1237 bytes
  .bcg out      145 bytes
  ratio         8.5x
  every game was decoded back and compared before writing.
```

Then:

```sh
gzip -9 -c corpus/classics.pgn | wc -c   # 718 — shrinks the PGN by 1.7x
gzip -9 -c out.bcg | wc -c               # 166 — BIGGER than the 145 it started at
```

**What to say:** this is the good bit. gzip squeezes the original file almost in
half. Handed ours, it cannot find a single thing to exploit — it makes the file
*bigger*, because all it manages to add is its own header. That is what it looks
like when there is no pattern left. A perfect compressor's output is
indistinguishable from random noise, which is exactly the property a hash
function is designed to have. They get there from opposite directions: the hash
destroys structure, the compressor spends it.

---

## 6. The one that is actually interesting

```sh
./target/release/blockchess grammar corpus/classics.pgn --min 2 --show 5
```

```
       2x   2ply  1. e4 e5
               = C20 King's Pawn Game
```

**What to say:** that is a general-purpose compressor that has never been told
what chess is. It sees anonymous numbers. Its only instruction is "find the
thing that repeats and give it a name". With three games there is exactly one
repetition, it found it, and then we looked the sequence up in a database of
3,810 opening names that it never had access to. It had a name. It has had one
since the sixteenth century.

Scale that to a million games and the question becomes: **how much of chess
theory does a compressor rediscover from nothing but repetition?** That is the
experiment this is all built for, and it is one download away from running.

---

## The page

[From a Square to a Theory](https://claude.ai/code/artifact/07ce0527-6180-4b82-a711-a8056c2c7ef0)
— the six layers, board geometry up to theory, every figure tied to the command
that produced it.

**It is private until you share it.** Open it and use the share menu.

---

## If they ask "so is it a blockchain yet"

No, and that is on purpose. Everything above is a working chess database with
no chain in it. `PLAN.md` puts the chain at stage 7, after the parts that are
useful on their own — because the previous version of this project put the
chain first and had nothing runnable for ten months.
