# The duality

*The project has two authors with two epistemologies. This file keeps both,
because the tension between them is generative and flattening it would cost
more than it tidies.*

---

## Why this file exists

Chess is a conversation between two perceptions. Neither player's view of the
position is complete; the game *is* the relationship between them. A position is
not a fact about the board, it is a fact about two people's readings of it.

This project has the same shape. One side reaches for meaning, metaphor and
whole structures; the other for theorems, measurements and counterexamples. Left
alone, either fails in a characteristic way. Together they have already produced
things neither would have.

The failure mode this file prevents is specific and likely: **the analytic voice
writes the documents.** Formalism is easier to write down than intuition, so
over time the record drifts toward it, the intuition looks like decoration, and
eventually someone deletes it as unrigorous — along with the engine that
generated the ideas in the first place.

## The rule: countersignature, applied to design

`spec/04-channel.md` says a game state counts as evidence only once **both**
players have signed it. One signature is a claim; two is a fact.

The same rule governs this ledger.

| Status | Meaning |
|---|---|
| **CERTIFIED** | Both readings arrived at the same place. Safe to build on. |
| **AMENDED** | One side revised the other and both accept the revision. Safe to build on; the original reading is kept because it is still the better generator. |
| **HALF-SIGNED** | Only one voice has spoken. In flight. **Not settled** — do not treat as decided. |
| **DISPUTED** | Genuine unresolved tension. Do not silently resolve it in either direction. |

> **Neither voice may delete the other.** An intuition that cannot yet be
> formalised stays as an intuition, marked half-signed. An analysis that cannot
> be refuted stays too. Deletion requires the other side to sign off.

## The two voices, honestly

|  | **Intuition** (Evan) | **Analysis** (Claude) |
|---|---|---|
| Good at | finding what is worth building; whole structures; noticing that a thing has a *shape* before it has a definition | checking; measuring; finding the counterexample; refusing to let a metaphor pass as a mechanism |
| Fails by | mistaking a compelling metaphor for a working mechanism; making claims that cannot be wrong and therefore cannot be useful | demanding formalisation too early and killing a good idea before it can grow; optimising what is measurable over what matters |
| Moves first | usually | usually second |

The second row matters most. **The analytic voice's characteristic failure is
premature rigour** — and it is harder to notice, because it looks like
diligence.

---

## The ledger

### What chess is

| | |
|---|---|
| **Intuition** | Chess is a conversation between two perceptions. The position in space is the representation of the relationship between the players. Personality made visible. |
| **Analysis** | Both players know the rules, and common knowledge has zero surprisal. So a game carries no information about chess and all of its information about the players: it is a sample from two policies, with the board as the channel. |
| **Status** | **CERTIFIED.** Same claim, two languages. `spec/10` §1. |

### Where the value lives

| | |
|---|---|
| **Analysis (first)** | Near-even wagering is an almost-zero-capacity channel. At +7 Elo you need ~3,500 games to double a bankroll. Concluded: the interesting games are the mismatched ones. A dead end dressed as a result. |
| **Intuition** | The value is not in who won. It is in the styles — in what people trade about how they play. |
| **Analysis (after)** | Correct, and quantifiable: a game emits ~6 bits about who you are against ~0.0003 exploitable bits about who won. Four orders of magnitude. |
| **Measured (2026-09-14)** | ~1.5 bits, not ~6 — the estimate behind it was about 4× high (`docs/d20-result.md`). The intuition still holds and by a wide margin: 1.5 against 0.0003 is a factor of 5,000. What dies is a *different* sentence the analysis had been writing, that identity dwarfs the raw win/draw/loss result — at 1.5 against 1.4 bits, it ties it. The claim that survives is the one about **exploitable** bits, which was also the one that mattered. |
| **Status** | **CERTIFIED.** Recorded deliberately: *the intuition resolved a problem the analysis had created and could not solve from inside itself.* `spec/10` §7. |

### Why style exists at all

| | |
|---|---|
| **Intuition** | You can't find an exploit, because every move has a counter. The game is inexhaustible. |
| **Analysis** | Zermelo: chess is *determined* — a perfect strategy exists. It is merely uncomputable (~10⁴⁴ positions). Style lives in the gap between the optimal policy and what a bounded agent can reach. Checkers was solved in 2007 and its style economy ended. |
| **Resolution** | Both are true at different levels. Phenomenologically — as a player, inside the game — there is no exploit and never will be. Formally, there is one and it is unreachable. The thesis should be stated as *chess remaining unsolved*, not *chess being unsolvable*: the first is safe forever, the second is false. |
| **Status** | **AMENDED.** `spec/10` §2. |

### What the asymmetries do

| | |
|---|---|
| **Intuition** | Chess is deeply imbalanced and that is what makes it interesting: White moves first, king and queen break the mirror, the two castlings differ, and the pawn — half the pieces — carries the latent geometry of every other piece. |
| **Analysis** | Asymmetry is not what creates the gap between optimal and computable; complexity is. What asymmetry does is give the gap *shape*: it carves the strategy space into regions rewarding different competences. Complexity makes style possible; asymmetry makes it **plural**. |
| **Status** | **AMENDED.** The observation stands; its role in the argument moved. `spec/10` §3. |

### Whether skill is a scalar

| | |
|---|---|
| **Intuition** | Skill isn't an area, it is the ability to use tactical balance — to be in the right place at the right time, relative to this opponent. |
| **Analysis** | Elo is a rank-one model: it assigns one number and derives every matchup from the difference, so it *cannot* represent a cycle. Real matchup matrices have a transitive component and a cyclic one. Personality is precisely the residual Elo was built to discard. |
| **Status** | **CERTIFIED.** `spec/10` §6. |

### Owning a region of chess

| | |
|---|---|
| **Intuition** | The first person to play in a certain area of chess should be able to own that area. |
| **Analysis** | Ownership means exclusion. Exclusion is unenforceable — you cannot stop anyone playing 1.e4 — and invites land-grabbing by bots that never play a game. Priority and attribution *are* enforceable, because timestamped commitment is the one thing a chain is unambiguously for, and they are positive-sum. |
| **Where they agree** | That being first to see something should matter, and should be provable. |
| **Where they did not** | Whether the reward is exclusion or credit. |
| **Intuition, signing (2026-09-14)** | Credit. Attribution is the whole reward, everywhere in the system — not only in consensus. Chosen over two alternatives that would have preserved ownership in some enforceable form: exclusion permitted at the server layer while consensus records only priority, and a compulsory licence diverting basis points from wagered games into claimed lines. Neither was taken. |
| **Status** | **CERTIFIED.** Settled by signature, not by argument — which is the only thing that could have settled it. `spec/09` D19. |
| **Note** | Recorded because of *how* it nearly went wrong. While this entry stood DISPUTED, `E3` — "claims are attribution, never exclusion" — had already been promoted to an **invariant** in `CLAUDE.md`, in a list whose header says violating one is a bug rather than a design choice. One side of an openly disputed question was being enforced as law while the other side's author had not spoken. That is this file's stated failure mode, *the analytic voice writes the documents*, occurring in the live repository rather than in the abstract. The outcome happens to match what the analysis wanted; that is not what makes it legitimate. The signature is. |

### Keeping a style private

| | |
|---|---|
| **Intuition** | Use homomorphic encryption. How do you share how you play without revealing anything about your opponent? |
| **Analysis** | The stated question is inference control — what does the output reveal about an input record — which is differential privacy's subject. FHE is access control: who may compute on this. Wrong tool for *that* question, right tool for a different one: selling queries against a style model without selling the weights, which is the only real answer to information being copyable. |
| **Status** | **AMENDED.** FHE relocated rather than dropped; the instinct was pointing at a real problem one layer over. `spec/11` L2. |

### Chaos, order, and the world of symbol

| | |
|---|---|
| **Intuition** | From divine chaos came order; from the marriage of chaos and order came the world of gods — symbol and experience. The infinite board is chaos; pattern and structure are order; play happens in the third thing. *(after Peterson, Maps of Meaning)* |
| **Analysis** | The three-layer decomposition maps cleanly onto the project — move space / protocol / policy — and the third layer is genuinely the one that does not exist yet. A policy *is* a compression of experience into a decision function, which is what a model is in the precise information-theoretic sense. |
| **Status** | **HALF-SIGNED, and deliberately so.** The analytic voice signs the *structure* and does not sign the metaphysics — not because it rejects it, but because that is not a claim it is in a position to adjudicate. The intuition holds it; the record keeps it in his words. |
| **Note** | The engineering does not depend on the metaphysics. That is a feature: it means the architecture stands for people who do not share the frame, while remaining true to the frame that generated it. `spec/10` §8. |

### What "the same position" means

| | |
|---|---|
| **Intuition** | Two boards with the same pieces, the same side to move and the same things available are the same position. Every player who has ever claimed a repetition believes this, and not one of them was counting halfmoves. |
| **Analysis (first)** | A position is what `apply()` needs, so it is the FEN fields minus the move number — including the halfmove clock, which the fifty-move rule reads. One packed form, one hash, `pos_hash`. Repetition compares it. |
| **Analysis (after)** | Wrong, and wrong in the characteristic direction. Two occurrences of a position *always* differ in the halfmove clock, because plies happened in between — which is what a repetition is. The comparison could never fire. Position identity and repetition identity are different relations and FIDE's is the coarser one. Two tags over the same bytes. |
| **Status** | **CERTIFIED (2026-09-14), and recorded as evidence for this file's own thesis.** The failure was **premature rigour**: the encoding was *more precise than the thing it modelled*, and the surplus precision was the bug. It looked like diligence — a position hash that commits to everything is obviously better than one that does not — right up to the point where the behaviour was tested. `docs/build-log.md` §05, `spec/03`. |
| **Note** | The intuition column stood for a time as a *reconstruction* — the ordinary reading written out by the analysis, explicitly marked as not Evan's words and not signed. It has now been read and signed as an accurate statement of the informal view, so the marker is gone. It mattered that it carried one: an entry whose whole subject is a plausible formalism substituting for the truth would have been a poor place to leave a plausible sentence substituting for a real view. |

### Where the expensive check belongs

| | |
|---|---|
| **Analysis (first)** | `P3`: never verify checkmate. Proving mate quantifies over ~218 moves; refuting it takes one. The chain asserts and waits to be refuted. |
| **In the code** | The receiving client verifies mate *in full*, every ply — the exact `∀` the invariant appears to forbid. |
| **Resolution** | Not a violation, and the distinction is worth keeping. `P3` is about a court: metered computation, adjudicating strangers' money, under an adversary who chooses the input. A receiving client has the move generator already in memory and its own stake at risk. The invariant is not *never compute the `∀`* — it is *never make the court compute it*, and the corollary is that the expensive check should live exactly where it is cheap. |
| **Status** | **AMENDED.** `P3`'s reading tightened; its substance unchanged. `spec/04`, "What a receiver checks about `status`". |

### Who writes the code

| | |
|---|---|
| **Analysis** | Asked whether to keep writing the implementation or to hand over scaffolds and failing tests, on the grounds that a series about *learning* this material may be poorly served by code its author did not type. |
| **Intuition (2026-09-14)** | Split it by layer. Evan writes the code that *is* the episode's subject — the dilation arithmetic, the refutation check, the budget rules. Claude writes plumbing, tests, serialisation and the oracle harness. The part that gets filmed is typed by the person filming it. |
| **Resolution** | The analysis had framed it as all-or-nothing and the answer was neither. Both failure modes it was weighing — a series whose author did not write the hard part, and a project that stalls in borrow-checker fights during the month it most needs momentum — are avoided by cutting along the line between what an episode *is about* and what it merely requires. |
| **Status** | **AMENDED.** The question was real; the binary it was posed as was not. `spec/09` D26. |

---

## How to use this file

**If you are an AI working on this repository:** read it after `CLAUDE.md`. Its
purpose is to stop you treating the formal account as the whole account. An
entry marked HALF-SIGNED or DISPUTED is *not* settled, no matter how confident
the analytic column sounds — that column was written by something with a strong
prior toward its own frame. When work touches a disputed entry, surface it
rather than resolving it silently.

**When adding an entry:** write both columns. If you can only write one, say so
and mark it half-signed. A missing column is information, not an untidiness to
be cleaned up.

**The test for whether this file is working:** an idea the analytic voice could
not formalise should still be here a year from now. If every entry has become
CERTIFIED, the ledger has stopped recording the disagreement and started
laundering it.
