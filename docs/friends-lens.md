# The Friends Lens — runbook

Everything below runs today. It is the first large deliverable: your friends
send in games they have already played, the lens is discovered from those games
and nothing else, and each person gets a link that shows them where they stand.

## Build

```sh
git checkout claude/laughing-pascal-s3wov0
cargo build --release
alias style=./target/release/style
```

## Try it with nobody

Before involving anyone, run the whole flow on constructed players so you know
what a working answer looks like.

```sh
style write 6 > /tmp/demo.pgn      # synthetic games, real PGN
style arena demo add /tmp/demo.pgn
style arena demo build
style arena demo serve 8080        # then open http://localhost:8080
```

## With your friends

Each person needs a Lichess account with public games. One command each:

```sh
style arena club fetch alice
style arena club fetch bob
style arena club fetch carol
# …one per person

style arena club build
style arena club serve 8080
```

Then send each person **their own link**: `http://your-box:8080/#alice`. Same
page, same corpus, different view — their card, their medal, their position,
and who they play most like.

Adding more games later is the same two commands, `fetch` then `build`. Nothing
already landed is rewritten.

## How to tell it worked

`style arena club build` says one of two things.

- **"the corpus is large enough to support the lens"** — the answer means
  something. Roughly 180 game-sides, which is ten people at ten games each.
- **"NOT YET MEANINGFUL"** — it will still draw the page, and the page will say
  so at the top. Get more games before believing any of it.

Then read the deck at the top of the page. It reports one of two outcomes, and
**both are real results**:

- *"The games separated anyway"* — the eighteen measurements tell your friends
  apart.
- *"The games did not separate"* — they do not. If you all play at a similar
  level in a similar way, this is the honest answer and the page will say it
  rather than inventing differences.

## Checking nobody has fiddled with it

```sh
style arena club status   # is the evidence intact?
style arena club audit    # are the published medals the ones it produces?
```

`status` reports the roster, the corpus root, and whether anything has changed
since it landed. Editing a stored game, doctoring a manifest line, deleting a
file or reordering submissions are each detected, because every entry is hashed
and chained to the one before it.

`audit` is the stronger claim and the one the whole idea rests on: **anyone can
recompute your identity.** It ignores the page and the medal file, replays the
games, refits the lens from scratch, mints every medal again, and diffs the
result against what was published. A third party needs nothing but this
directory — and the point is that they need not trust whoever ran `build`.

It exits non-zero when it fails, so it works in a cron job:

```
PASS — every published medal is the medal these games produce.
```

If you add games and forget to rebuild, it says so rather than passing:
the medals were minted under a different corpus.

## The deeper questions

```sh
style arena club build            # then, on the same corpus:
style identify club/games/*.pgn   # how many games until someone is identified
style interact club/games/*.pgn   # whether opponents change how you play
```

`identify` also answers the question a sceptic asks first: **is this basis just
rating in disguise?** Lichess exports carry ratings, so on a real corpus it
correlates each discovered axis against them and tells you. If the correlations
are high, the medal is an Elo rating with extra steps.

## What has not been tested

`fetch` has never successfully reached Lichess — this was built without network
access. The guard against bad usernames and the failure path were checked; a
response that actually contains games was not. If the first fetch behaves oddly,
compare the game count it reports against what Lichess shows for that account,
and fall back to a manual export:

```sh
curl -H "Accept: application/x-chess-pgn" \
  "https://lichess.org/api/games/user/alice?max=200&clocks=true" > alice.pgn
style arena club add alice.pgn
```
