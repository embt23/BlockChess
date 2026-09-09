//! Re-Pair, implemented so it finishes on a real corpus.
//!
//! The naive version — count every pair, take the best, rewrite the whole
//! sequence, repeat — is O(n) per rule and so O(n·k) overall. At n = 10⁷
//! symbols and k = 10⁴ rules that is 10¹¹ operations, which does not finish.
//!
//! So the counts are maintained incrementally. Three structures:
//!
//! - a doubly linked list over the symbol array, so a replacement is O(1) and
//!   the neighbours of a hole are still reachable after it is made;
//! - an occurrence list per pair, so the positions to rewrite are known
//!   without scanning;
//! - a max-heap over (count, pair) with **lazy invalidation** — entries are
//!   pushed on every count change and verified against the live count when
//!   popped, which is much simpler than a decrease-key structure and, since
//!   each count change pushes at most one entry, no slower asymptotically.
//!
//! The subtlety worth naming: occurrences of a pair can overlap. In `aaa` the
//! pair `(a,a)` occurs at positions 0 and 1, and replacing both would consume
//! the middle `a` twice. Every occurrence is therefore re-validated at the
//! moment it is used, and a position whose symbols have already changed is
//! skipped rather than trusted.
//!
//! ## Measured, not asserted
//!
//! "Finishes on a real corpus" is a claim, so it is checked. `examples/scale.rs`
//! builds a synthetic corpus shaped like chess — many sequences sharing
//! prefixes from a skewed popularity distribution, then diverging — and runs
//! the inducer over it:
//!
//! ```text
//!  10,000 sequences     801,929 symbols     0.46s    1,504 rules
//!  50,000 sequences   4,012,053 symbols     4.66s    2,352 rules
//! 200,000 sequences  16,053,909 symbols    12.80s    4,022 rules
//! ```
//!
//! A month of Lichess is on the order of 10⁷ plies, so it lands in that last
//! row: about ten seconds. The naive rewrite-everything loop would not have
//! finished at all.
//!
//! The example says nothing about chess and is not evidence for anything in
//! `papers/08-layers.md` — its tails are random, so its compression ratio is
//! meaningless. It measures one thing: that this terminates at scale.

use std::collections::{BinaryHeap, HashMap};

use crate::Limits;

/// A symbol. Terminals are whatever the caller passes in; invented symbols are
/// allocated above the largest terminal.
pub type Sym = u32;

/// One invented symbol: `lhs -> (a, b)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rule {
    pub lhs: Sym,
    pub a: Sym,
    pub b: Sym,
    /// How many times this symbol was substituted when it was created. Not the
    /// same as how often it survives in the final output — a later rule may
    /// absorb it — but it is the honest measure of how much work it did.
    pub occurrences: u32,
}

/// The result of induction: the invented symbols, and the corpus rewritten
/// in terms of them.
#[derive(Debug, Clone)]
pub struct Grammar {
    pub rules: Vec<Rule>,
    /// The lowest symbol id that is a rule rather than a terminal.
    pub first_rule_sym: Sym,
    /// Each input sequence, after substitution.
    pub sequences: Vec<Vec<Sym>>,
}

impl Grammar {
    pub fn rule(&self, s: Sym) -> Option<&Rule> {
        if s < self.first_rule_sym {
            return None;
        }
        self.rules.get((s - self.first_rule_sym) as usize)
    }

    /// Expand a symbol all the way back to terminals.
    ///
    /// Iterative rather than recursive: a deep grammar over a large corpus can
    /// nest thousands of levels, and a recursive expansion overflows the stack
    /// on exactly the inputs that are most interesting.
    pub fn expand(&self, s: Sym) -> Vec<Sym> {
        let mut out = Vec::new();
        let mut stack = vec![s];
        while let Some(top) = stack.pop() {
            match self.rule(top) {
                None => out.push(top),
                Some(r) => {
                    stack.push(r.b);
                    stack.push(r.a);
                }
            }
        }
        out
    }

    /// How many terminals a symbol stands for.
    pub fn expanded_len(&self, s: Sym) -> usize {
        self.expand(s).len()
    }

    /// Total symbols in the rewritten corpus, plus two per rule for the
    /// grammar itself. This is the quantity Re-Pair is minimising, and the
    /// honest one to report: a grammar that shrinks the sequences by growing
    /// itself has achieved nothing.
    pub fn total_size(&self) -> usize {
        self.sequences.iter().map(|s| s.len()).sum::<usize>() + 2 * self.rules.len()
    }
}

/// A sentinel that may never be part of a pair. Sits between sequences so that
/// the end of one game and the start of the next are never treated as a
/// pattern — a mistake that would otherwise invent a symbol for "someone
/// resigned and someone else opened 1.e4".
const BOUNDARY: Sym = Sym::MAX;

pub fn induce(sequences: &[Vec<Sym>], limits: Limits) -> Grammar {
    let max_terminal = sequences
        .iter()
        .flat_map(|s| s.iter())
        .copied()
        .max()
        .unwrap_or(0);
    let mut next_sym = max_terminal + 1;
    assert!(next_sym < BOUNDARY, "terminal alphabet too large");

    // Flatten with boundaries between sequences.
    let mut sym: Vec<Sym> = Vec::new();
    let mut starts: Vec<usize> = Vec::with_capacity(sequences.len());
    for s in sequences {
        starts.push(sym.len());
        sym.extend_from_slice(s);
        sym.push(BOUNDARY);
    }
    let n = sym.len();

    // Doubly linked list over live positions. usize::MAX is "none".
    const NONE: usize = usize::MAX;
    let mut prev: Vec<usize> = (0..n).map(|i| if i == 0 { NONE } else { i - 1 }).collect();
    let mut next: Vec<usize> = (0..n)
        .map(|i| if i + 1 == n { NONE } else { i + 1 })
        .collect();

    let mut counts: HashMap<(Sym, Sym), u32> = HashMap::new();
    let mut occ: HashMap<(Sym, Sym), Vec<usize>> = HashMap::new();
    let mut heap: BinaryHeap<(u32, Sym, Sym)> = BinaryHeap::new();

    for i in 0..n {
        if let Some(p) = pair_at(&sym, &next, i) {
            *counts.entry(p).or_insert(0) += 1;
            occ.entry(p).or_default().push(i);
        }
    }
    for (&p, &c) in counts.iter() {
        if c >= limits.min_occurrences {
            heap.push((c, p.0, p.1));
        }
    }

    let mut rules: Vec<Rule> = Vec::new();

    while rules.len() < limits.max_rules {
        // Pop until an entry agrees with the live count. Stale entries are
        // the price of not having decrease-key, and they are cheap.
        let (count, a, b) = loop {
            let Some((c, a, b)) = heap.pop() else {
                break (0, 0, 0);
            };
            if counts.get(&(a, b)).copied().unwrap_or(0) == c {
                break (c, a, b);
            }
        };
        if count < limits.min_occurrences {
            break;
        }

        let lhs = next_sym;
        next_sym += 1;
        let positions = occ.remove(&(a, b)).unwrap_or_default();
        counts.remove(&(a, b));

        let mut replaced = 0u32;
        for i in positions {
            // Re-validate: the position may have been consumed by an earlier
            // replacement in this same round (the `aaa` case), or eaten as the
            // right half of a neighbouring pair.
            if sym[i] != a {
                continue;
            }
            let j = next[i];
            if j == NONE || sym[j] != b {
                continue;
            }

            // The two pairs straddling this one lose an occurrence, and the
            // two that will straddle the new symbol gain one. Doing this
            // before the edit keeps the reads consistent.
            let l = prev[i];
            let r = next[j];
            if l != NONE && sym[l] != BOUNDARY {
                bump(&mut counts, &mut heap, (sym[l], a), -1);
            }
            if r != NONE && sym[r] != BOUNDARY {
                bump(&mut counts, &mut heap, (b, sym[r]), -1);
            }

            // Splice j out and turn i into the new symbol.
            sym[i] = lhs;
            sym[j] = BOUNDARY;
            next[i] = r;
            if r != NONE {
                prev[r] = i;
            }
            replaced += 1;

            if l != NONE && sym[l] != BOUNDARY {
                let p = (sym[l], lhs);
                bump(&mut counts, &mut heap, p, 1);
                occ.entry(p).or_default().push(l);
            }
            if r != NONE && sym[r] != BOUNDARY {
                let p = (lhs, sym[r]);
                bump(&mut counts, &mut heap, p, 1);
                occ.entry(p).or_default().push(i);
            }
        }

        if replaced == 0 {
            next_sym -= 1;
            continue;
        }
        rules.push(Rule {
            lhs,
            a,
            b,
            occurrences: replaced,
        });
    }

    // Walk the linked list back out into per-sequence vectors.
    let first_rule_sym = max_terminal + 1;
    let mut out: Vec<Vec<Sym>> = Vec::with_capacity(sequences.len());
    for &start in &starts {
        let mut seq = Vec::new();
        let mut i = start;
        while i != NONE && sym[i] != BOUNDARY {
            seq.push(sym[i]);
            i = next[i];
        }
        out.push(seq);
    }

    Grammar {
        rules,
        first_rule_sym,
        sequences: out,
    }
}

fn pair_at(sym: &[Sym], next: &[usize], i: usize) -> Option<(Sym, Sym)> {
    if sym[i] == BOUNDARY {
        return None;
    }
    let j = *next.get(i)?;
    if j == usize::MAX || sym[j] == BOUNDARY {
        return None;
    }
    Some((sym[i], sym[j]))
}

fn bump(
    counts: &mut HashMap<(Sym, Sym), u32>,
    heap: &mut BinaryHeap<(u32, Sym, Sym)>,
    p: (Sym, Sym),
    delta: i32,
) {
    let e = counts.entry(p).or_insert(0);
    let new = (*e as i64 + delta as i64).max(0) as u32;
    *e = new;
    if delta > 0 {
        heap.push((new, p.0, p.1));
    }
}
