# Working in this repository

## Read this before anything else

**This repository holds several parallel lines of work, and `git branch -a` will
not show you all of them.** A fresh clone fetches only some refs, there is one
open pull request, and the most advanced work has no pull request at all. A
session that trusts the local branch list will conclude the project is a
one-line README plus a spec, and will then cheerfully rebuild things that
already exist. That has happened — `bc-style` contains a PGN parser written in
ignorance of `bc-pgn`, which is better.

So, first command, every time:

```sh
git ls-remote --heads origin
```

Then look at what you find before writing anything.

### The lines, as of 2026-09-09

Six remote branches, **four distinct projects**. They are not versions of one
thing; they are different attacks on the same subject, and three of them do not
know about each other.

| Branch | Project | State |
|---|---|---|
| `main` | — | one line of README. Not the project. |
| `claude/chess-wagering-blockchain-u072d8` | PR #1, the original wagering spec | ancestor of cori, so merging cori subsumes and closes it |
| `claude/stoic-cannon-mgh0yu` | knowledge ledger | already merged into cori |
| `claude/youthful-cori-l69v75` | **The corpus/compression trunk.** Six crates, twelve papers, 121k games, `blockchess` with ten commands | most advanced. Has no PR. |
| `claude/youthful-lamport-snidcm` | **Chess Frontier Atlas.** Eight visual layers over a board — territory, contours, frontier, tension, vision — a whole engine in one self-contained HTML page, a glyph alphabet, a workbench for tagging a photographed journal page | independent of everything else |
| `claude/laughing-pascal-s3wov0` | The MetaPlan and `bc-style` — personality compression, medals, identification, the interaction term | independent of cori |

**The four overlap and nobody has reconciled them.** cori compresses a corpus to
find chess theory; lamport renders what a position *feels* like; laughing-pascal
compresses a player into an identity. All three are compression over chess, and
each was designed as if it were the only one.

Two of them are blocked on the same thing: cori's Stage 0 wants a Lichess dump,
lamport's Phase 1 wants a photographed page tagged. Both are the owner's to
supply, not a session's.

### cori uses versioned folders, not branches

Its layout is `0.0/`, `0.1/`, each a complete Cargo workspace. There is **no
`Cargo.toml` at the repository root** on that branch, so `cargo` run from the
root fails with "could not find Cargo.toml" — `cd 0.1` first. The root
`.gitignore` anchors `/0.0/target` and `/0.1/target`, so a stray root-level
`target/` from another branch shows up as untracked; delete it rather than
committing it.

## The standard

> No layer is built on top of rules that have only been verified by tests we
> wrote ourselves.

Every crate is checked against an oracle *someone else* published — FIPS 180-4,
RFC 8032, published perft counts, RFC 6962 vectors, analytic eigen-spectra.
Where no external oracle exists, construct ground truth instead (`bc-style`'s
synthetic archetypes) and **validate the instrument on a placebo before
believing a positive result**. A method that finds an effect is worthless until
it has been shown able to find none.

`docs/build-log.md` is the record of what went wrong and why it hid. It is the
best-written document here; add to it when something teaches, and match its
voice. Three of its six entries are the same lesson in different clothing:
**a wrong answer that is still well-formed is invisible.** A miscomputed curve
point that is still on the curve, a flipped eigenvector that is still an
eigenvector, a basis fitted to eight games that still has poles and loadings.

## Commands

```sh
# on claude/laughing-pascal-s3wov0 — workspace at the root
cargo test --workspace
cargo run --release -p bc-style --bin style -- demo       # the lab
cargo run --release -p bc-style --bin style -- identify   # held-out attribution
cargo run --release -p bc-style --bin style -- interact   # the interaction term
cargo run --release -p bc-style --bin style -- viz > space.html

# on claude/youthful-cori-l69v75
cd 0.1 && cargo build --release
./target/release/blockchess perft 5
```

## CI

Four jobs. The workflow pins `dtolnay/rust-toolchain@stable`, which **floats** —
a new clippy release can turn a green build red with no commit, and has (the
`chunks_exact_to_as_chunks` lint took cori red for four pushes, and because the
job dies at its clippy step, the nine pipeline steps after it silently skip).

So verify against the toolchain CI actually runs, not the one that happens to be
installed:

```sh
rustup toolchain install <version> --profile minimal --component clippy,rustfmt
cargo +<version> fmt --all -- --check
RUSTFLAGS="-D warnings" cargo +<version> clippy --all-targets --all-features
```

A `rust-toolchain.toml` pin would end this permanently. It is deliberately not
committed: it changes what every contributor's local toolchain resolves to,
which is the repository owner's call.

## Conventions

- **No external dependencies.** Every crate in both workspaces is
  dependency-free, including the linear algebra and the JSON writer. Keep it
  that way unless the owner says otherwise; plumbing is the only reasonable
  exception and has not come up yet.
- **The spec section is written before the code.** These files are the scripts,
  not documentation of the code.
- **Generated artifacts are generated, not committed.** The visualisation page
  is produced by `style viz` from a template plus live data; a checked-in copy
  would drift and then display medals nothing mints.
- **`METAPLAN.md` outranks every other document.** If a spec file disagrees with
  it, the spec file is stale. Specs displaced by the pivot carry a banner saying
  whether they are repurposed, deferred or cut.
- Commit messages here carry the reasoning, not just the change. Match that.
