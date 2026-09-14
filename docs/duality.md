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
| **Where they do not** | Whether the reward is exclusion or credit. |
| **Status** | **DISPUTED.** The analysis pushed; the intuition has not answered. `spec/11` L3 records the attribution design as a *proposal*, not a decision. Do not treat D19 as settled. |

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

### Who writes the code

| | |
|---|---|
| **Analysis** | Asked whether to keep writing the implementation or to hand over scaffolds and failing tests, on the grounds that a series about *learning* this material may be poorly served by code its author did not type. |
| **Intuition** | Has not answered. |
| **Status** | **HALF-SIGNED.** Open. |

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
