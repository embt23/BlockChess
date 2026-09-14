//! Sparse Merkle tree — the chain's state commitment.
//!
//! A binary tree of depth 256, where the 256-bit key *is* the path from the
//! root: bit 0 (most significant) picks the child at the top, bit 255 picks the
//! leaf. So a key's position is fixed by the key itself, not by insertion
//! order, and the root is a pure function of the key/value set.
//!
//! # How you store a tree with 2²⁵⁶ leaves
//!
//! You don't. Almost every subtree is entirely empty, and all empty subtrees of
//! the same height have the *same hash*. So precompute one hash per height:
//!
//! ```text
//!   empty[0] = H_empty
//!   empty[h] = H_node( empty[h−1] ‖ empty[h−1] )
//! ```
//!
//! There are only 257 of them. Store the handful of nodes that actually contain
//! data and substitute `empty[h]` everywhere else. A tree holding `n` entries
//! stores O(n · log n) nodes; the other 2²⁵⁶ − n leaves cost nothing because
//! they are all the same nothing.
//!
//! # Why this instead of Ethereum's Merkle-Patricia trie
//!
//! The MPT is radix-16 with four node types, path compression, and RLP. It
//! saves space and it is genuinely unpleasant to implement correctly. This is
//! one node type and about 150 lines — and it gives you something the MPT does
//! not give cheaply:
//!
//! **Proofs of absence are free.** To prove a key is *not* in the tree, hand
//! over the same sibling path and let the verifier recompute the root assuming
//! the leaf is empty. If it matches, the key is absent. Presence and absence
//! use the identical code path; the only difference is what you put at the
//! bottom.
//!
//! That matters here: "this channel has no open dispute" and "this key has
//! never registered a server" are both non-inclusion claims, and light clients
//! need them cheap.
//!
//! # Known limitation: storage is O(n · depth)
//!
//! Measured, not guessed: this implementation stores ~246 nodes per key at
//! depth 256. Random keys diverge within about log₂(n) levels of the root, and
//! below that point each key owns a private chain of single-child nodes running
//! down to its leaf. At ~64 bytes per node that is ~16 MB for a thousand
//! accounts and roughly 15 GB for a million.
//!
//! The fix is path compression: store only nodes where the tree actually
//! branches, and compute the single-child chains on demand — a chain node's
//! hash is fully determined by the leaf beneath it and the empty hashes, so
//! **no root hash and no proof changes**. That makes it a pure internal
//! optimisation, safe to add later without invalidating anything already
//! committed, which is why it is deferred rather than rushed. See
//! `spec/09-open-questions.md`, D15.

use bc_hash::{tagged, tagged_parts, Hash};
use std::collections::HashMap;

/// Depth used by the chain: keys are 256-bit hashes.
pub const DEFAULT_DEPTH: usize = 256;

/// Key type: a 256-bit path.
pub type Key = [u8; 32];

pub fn empty_leaf() -> Hash {
    tagged("BC/smt/empty/v1", &[])
}

pub fn leaf_hash(key: &Key, value: &[u8]) -> Hash {
    // The key is bound into the leaf. Without it, a leaf hash lifted from one
    // position would verify at another.
    tagged_parts("BC/smt/leaf/v1", &[key, value])
}

pub fn node_hash(left: &Hash, right: &Hash) -> Hash {
    tagged_parts("BC/smt/node/v1", &[left, right])
}

/// `empty[h]` for `h` in `0..=depth`: the root of an all-empty subtree of
/// height `h`.
pub fn empty_hashes(depth: usize) -> Vec<Hash> {
    let mut v = Vec::with_capacity(depth + 1);
    v.push(empty_leaf());
    for h in 1..=depth {
        let prev = v[h - 1];
        v.push(node_hash(&prev, &prev));
    }
    v
}

/// Bit `i` of the key, counting from the most significant.
#[inline]
fn bit(key: &Key, i: usize) -> u8 {
    (key[i / 8] >> (7 - i % 8)) & 1
}

/// The key truncated to its top `level` bits, remaining bits zeroed. This is
/// the identity of a node at that level.
fn prefix(key: &Key, level: usize) -> Key {
    let mut p = [0u8; 32];
    let full = level / 8;
    p[..full].copy_from_slice(&key[..full]);
    let rem = level % 8;
    if rem > 0 {
        p[full] = key[full] & (0xFFu8 << (8 - rem));
    }
    p
}

#[inline]
fn flip_bit(key: &mut Key, i: usize) {
    key[i / 8] ^= 1 << (7 - i % 8);
}

/// A sibling path, with empty siblings elided.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Proof {
    /// Non-empty siblings, bottom-up: index 0 is the leaf's sibling.
    pub siblings: Vec<Hash>,
    /// Bit `i` set means the `i`-th sibling (bottom-up) is present in
    /// `siblings`; clear means it is the known empty hash for that height.
    pub bitmap: Vec<u8>,
}

impl Proof {
    #[inline]
    fn has(&self, i: usize) -> bool {
        self.bitmap
            .get(i / 8)
            .is_some_and(|b| b >> (i % 8) & 1 == 1)
    }

    /// Encoded size in bytes.
    pub fn size(&self) -> usize {
        self.bitmap.len() + self.siblings.len() * 32
    }
}

#[derive(Clone, Debug)]
pub struct Smt {
    depth: usize,
    empty: Vec<Hash>,
    /// Non-empty nodes, keyed by (level from root, path prefix).
    nodes: HashMap<(u16, Key), Hash>,
    values: HashMap<Key, Vec<u8>>,
}

impl Default for Smt {
    fn default() -> Self {
        Self::new(DEFAULT_DEPTH)
    }
}

impl Smt {
    pub fn new(depth: usize) -> Smt {
        assert!(depth > 0 && depth <= 256, "depth must be in 1..=256");
        Smt {
            depth,
            empty: empty_hashes(depth),
            nodes: HashMap::new(),
            values: HashMap::new(),
        }
    }

    pub fn depth(&self) -> usize {
        self.depth
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Number of stored internal nodes — what the tree actually costs.
    pub fn stored_nodes(&self) -> usize {
        self.nodes.len()
    }

    pub fn root(&self) -> Hash {
        self.nodes
            .get(&(0, [0u8; 32]))
            .copied()
            .unwrap_or(self.empty[self.depth])
    }

    pub fn get(&self, key: &Key) -> Option<&[u8]> {
        self.values.get(key).map(|v| v.as_slice())
    }

    pub fn insert(&mut self, key: Key, value: Vec<u8>) {
        let leaf = leaf_hash(&key, &value);
        self.values.insert(key, value);
        self.recompute(&key, leaf);
    }

    pub fn remove(&mut self, key: &Key) {
        if self.values.remove(key).is_some() {
            let empty = self.empty[0];
            self.recompute(key, empty);
        }
    }

    /// Rewrite the 257 nodes on the path from this leaf to the root.
    ///
    /// This is the whole cost of a write: O(depth) hashes, independent of how
    /// much is in the tree.
    fn recompute(&mut self, key: &Key, leaf: Hash) {
        let d = self.depth;
        let mut cur = leaf;
        self.put(d as u16, prefix(key, d), cur);

        for level in (0..d).rev() {
            let child_height = d - level - 1;
            let mut sib_prefix = prefix(key, level + 1);
            flip_bit(&mut sib_prefix, level);
            let sib = self
                .nodes
                .get(&((level + 1) as u16, sib_prefix))
                .copied()
                .unwrap_or(self.empty[child_height]);

            cur = if bit(key, level) == 0 {
                node_hash(&cur, &sib)
            } else {
                node_hash(&sib, &cur)
            };
            self.put(level as u16, prefix(key, level), cur);
        }
    }

    /// Store a node, or drop it if it is the empty hash for its height.
    ///
    /// Dropping is what keeps the map sparse, and it also makes "present in the
    /// map" exactly equivalent to "non-empty", which the proof bitmap relies on.
    fn put(&mut self, level: u16, p: Key, h: Hash) {
        let height = self.depth - level as usize;
        if h == self.empty[height] {
            self.nodes.remove(&(level, p));
        } else {
            self.nodes.insert((level, p), h);
        }
    }

    /// A proof for `key` — of inclusion if present, of absence if not. Same
    /// structure either way.
    pub fn prove(&self, key: &Key) -> Proof {
        let d = self.depth;
        let mut siblings = Vec::new();
        let mut bitmap = vec![0u8; d.div_ceil(8)];

        for (i, level) in (0..d).rev().enumerate() {
            let mut sib_prefix = prefix(key, level + 1);
            flip_bit(&mut sib_prefix, level);
            if let Some(h) = self.nodes.get(&((level + 1) as u16, sib_prefix)) {
                bitmap[i / 8] |= 1 << (i % 8);
                siblings.push(*h);
            }
        }
        Proof { siblings, bitmap }
    }
}

/// Verify a proof against a root, with no access to the tree.
///
/// `value` is `Some` to check inclusion, `None` to check absence.
pub fn verify(depth: usize, root: &Hash, key: &Key, value: Option<&[u8]>, proof: &Proof) -> bool {
    if proof.bitmap.len() != depth.div_ceil(8) {
        return false;
    }
    // Padding bits above `depth` must be zero, or one proof would have several
    // encodings.
    let used = depth % 8;
    if used != 0 {
        if let Some(last) = proof.bitmap.last() {
            if last >> used != 0 {
                return false;
            }
        }
    }

    let empty = empty_hashes(depth);
    let mut acc = match value {
        Some(v) => leaf_hash(key, v),
        None => empty[0],
    };

    let mut next = proof.siblings.iter();
    for (i, level) in (0..depth).rev().enumerate() {
        let child_height = depth - level - 1;
        let sib = if proof.has(i) {
            match next.next() {
                Some(h) => *h,
                None => return false, // bitmap claims more siblings than supplied
            }
        } else {
            empty[child_height]
        };
        acc = if bit(key, level) == 0 {
            node_hash(&acc, &sib)
        } else {
            node_hash(&sib, &acc)
        };
    }

    // Reject leftover siblings, so a proof has exactly one encoding.
    next.next().is_none() && acc == *root
}
