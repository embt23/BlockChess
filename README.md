# BlockChess

Chess on a blockchain. Two versions live here, and they are different projects
that happen to share a name.

| | | |
|---|---|---|
| [**0.1**](0.1/) | current | A public, permanent, searchable record of chess games. No money. The mathematics of compressing chess into a permanent log — and a Rust engine and codec you can run today. |
| [**0.0**](0.0/) | archived, complete as far as it went | A from-scratch blockchain for wagered peer-to-peer chess. Full protocol spec, plus SHA-256, Ed25519, a move generator passing `perft(6)`, and Merkle trees, all written from the standards. |

## 0.0 — proof of concept

Archived, not abandoned. Nine specification documents, four Rust crates, every
one checked against an oracle someone else published: FIPS 180-4 vectors,
RFC 8032, RFC 6962, and the published perft counts. `perft(6) = 119,060,324`
exact, first run.

It is left exactly as it was, including `0.0/docs/build-log.md`, which is the
most useful thing in it — three bugs and why each one hid. CI still builds and
tests it; the only change made when archiving was the path.

Its subject was money: escrow, disputes, adjudication, handicap odds, cheat
detection. 0.1 removes all of that on purpose.

## 0.1 — what the project is actually for

The moves should be public. Not the wagers, not the ratings, not the economy —
the games. A growing chess database anyone can study, that nobody owns, and that
does not disappear when a company does.

Which turns the project into one question with a real answer:

> **What is the cheapest permanent encoding of a chess game, and what does each
> saving cost you in something other than bits?**

Permanence is what makes this different from ordinary compression. A compressor
can be upgraded; a consensus decoder cannot, and the price of that is
computable. Four results so far, derived in [`0.1/papers/`](0.1/papers/) and
computed by [`0.1/measure/`](0.1/measure/):

1. **Store moves, not positions** — a position costs ~150 bits, a move ~5.
2. **Chess has almost no symmetry** — a general middlegame position has a
   symmetry group of order 1, so symmetry is worth ~0 bits as compression and
   ~4× as an index key.
3. **The opening trie and the entropy coder are the same saving** — you cannot
   bank it twice, and the trie's real job is search.
4. **Below ~50 bytes a game you are storing signatures, not chess** — batching
   the attestations beats every move-encoding decision combined.

### Run it

```sh
cd 0.1 && cargo build --release
./target/release/blockchess perft 5                      # the rules are right
./target/release/blockchess play                         # a board in the terminal
./target/release/blockchess measure corpus/classics.pgn  # the real numbers
./target/release/blockchess pack corpus/classics.pgn out.bcg
```

Start at [`0.1/START-HERE.md`](0.1/START-HERE.md) if you want the walkthrough,
or [`0.1/README.md`](0.1/README.md) for the summary.

## Why folders and not branches

Versions here are directories, and old versions are also git tags. They are not
branches, and that is deliberate.

**A branch is for a line of work that continues.** 0.0 is finished — it will
never take another commit. Putting a frozen thing on a branch says the opposite
of what is true about it.

**0.1 reads 0.0 constantly.** That is most of why 0.0 was kept: the build log,
the knowledge ledger, the atlas. Side by side you can grep both, open both, and
link between them. On separate branches you can hold exactly one at a time, and
every cross-reference in the papers becomes a dead link. `0.1/crates/bc-chess`
came straight out of `0.0/` by copying a directory; across branches that is a
cherry-pick.

**One CI run covers both.** 0.0's tests still execute on every push, so the
archive is provably intact rather than merely present. On separate branches
nobody would notice the day it stopped building.

**The archive is the tag, not the folder.** `v0.0` points at the last commit
where 0.0 stood at the repository root:

```sh
git show v0.0            # what it was
git checkout v0.0        # stand in it; git switch - to come back
```

Which gives the graceful exit from the thing folders are genuinely bad at —
accumulating. `0.0/`, `0.1/`, `0.2/`, `0.3/` would be a mess. So the rule is:

> **A version stays as a folder while a living version still reads it. Once
> nothing refers to it, tag it and delete the folder.** The tag keeps it
> forever, and `git show <tag>` brings it back.

By that rule 0.0 stays for now, because 0.1 is still mining it, and 0.1 moves
to the repository root once it is unambiguously *the* project.

## Licence

TBD — intended to be permissive and open source.
