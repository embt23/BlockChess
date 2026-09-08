//! The opening trie: games keyed by move order.
//!
//! Answers Q1 — "show me games in this opening" — by walking down from the
//! start. Each node is one exact sequence of moves, each edge a move, and the
//! count on a node is how many games passed through it.
//!
//! `papers/03-corpus.md` established that this is the *wrong* structure for
//! compression — prefix sharing and predictive coding are one saving, not two.
//! It is the right structure for search, which is what it is doing here, and
//! `papers/08-layers.md` §7 notes it is also the first layer of the grammar:
//! every node visited more than *k* times is a pattern with a name waiting
//! for it.

use bc_chess::Move;

use crate::Score;

pub type NodeId = u32;

#[derive(Debug, Clone)]
pub struct Node {
    /// Edges out, kept sorted by move so lookup is a binary search and the
    /// iteration order is deterministic — the same corpus must always produce
    /// the same trie, or "rebuild it yourself and check" is not a real offer.
    pub children: Vec<(Move, NodeId)>,
    pub score: Score,
    pub depth: u16,
}

#[derive(Debug, Clone)]
pub struct Trie {
    pub nodes: Vec<Node>,
}

impl Default for Trie {
    fn default() -> Self {
        Trie::new()
    }
}

impl Trie {
    pub fn new() -> Trie {
        Trie {
            nodes: vec![Node {
                children: Vec::new(),
                score: Score::default(),
                depth: 0,
            }],
        }
    }

    pub const ROOT: NodeId = 0;

    pub fn insert(&mut self, moves: &[Move], result: &str, max_depth: usize) {
        let mut cur = Self::ROOT;
        self.nodes[cur as usize].score.record(result);
        for (d, &m) in moves.iter().enumerate() {
            if d >= max_depth {
                break;
            }
            cur = self.child_or_insert(cur, m, d as u16 + 1);
            self.nodes[cur as usize].score.record(result);
        }
    }

    fn child_or_insert(&mut self, node: NodeId, m: Move, depth: u16) -> NodeId {
        let children = &self.nodes[node as usize].children;
        match children.binary_search_by_key(&m.0, |(mv, _)| mv.0) {
            Ok(i) => children[i].1,
            Err(i) => {
                let id = self.nodes.len() as NodeId;
                self.nodes.push(Node {
                    children: Vec::new(),
                    score: Score::default(),
                    depth,
                });
                self.nodes[node as usize].children.insert(i, (m, id));
                id
            }
        }
    }

    pub fn child(&self, node: NodeId, m: Move) -> Option<NodeId> {
        let children = &self.nodes[node as usize].children;
        children
            .binary_search_by_key(&m.0, |(mv, _)| mv.0)
            .ok()
            .map(|i| children[i].1)
    }

    /// Follow a line from the root. `None` if it leaves the trie — which is
    /// itself the answer to "has anyone played this?", and is what makes a
    /// novelty detectable.
    pub fn walk(&self, moves: &[Move]) -> Option<NodeId> {
        let mut cur = Self::ROOT;
        for &m in moves {
            cur = self.child(cur, m)?;
        }
        Some(cur)
    }

    pub fn node(&self, id: NodeId) -> &Node {
        &self.nodes[id as usize]
    }

    /// The continuations from a node, most played first. Ties broken by move
    /// so the output is stable.
    pub fn continuations(&self, id: NodeId) -> Vec<(Move, NodeId, Score)> {
        let mut out: Vec<(Move, NodeId, Score)> = self.nodes[id as usize]
            .children
            .iter()
            .map(|&(m, c)| (m, c, self.nodes[c as usize].score))
            .collect();
        out.sort_by(|a, b| b.2.total().cmp(&a.2.total()).then(a.0 .0.cmp(&b.0 .0)));
        out
    }

    /// Every node reached by at least `min_games`, with the line that reaches
    /// it. This is the opening book, and it is also the candidate list for
    /// `papers/08-layers.md`'s discovered vocabulary.
    pub fn frequent_lines(&self, min_games: u32) -> Vec<(Vec<Move>, Score)> {
        let mut out = Vec::new();
        let mut stack = vec![(Self::ROOT, Vec::new())];
        while let Some((id, line)) = stack.pop() {
            let node = self.node(id);
            if !line.is_empty() {
                out.push((line.clone(), node.score));
            }
            for &(m, child) in node.children.iter().rev() {
                if self.nodes[child as usize].score.total() >= min_games {
                    let mut next = line.clone();
                    next.push(m);
                    stack.push((child, next));
                }
            }
        }
        out.sort_by(|a, b| {
            b.1.total()
                .cmp(&a.1.total())
                .then(a.0.len().cmp(&b.0.len()))
        });
        out
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.len() <= 1
    }
}
