//! A Merkle tree over an ordered list, following RFC 6962 (Certificate
//! Transparency).
//!
//! Used for `tx_root` and `receipt_root`: commit to the ordered list of
//! transactions in a block with one 32-byte hash, so a light client can be
//! shown that its transaction was included without downloading the block.
//!
//! # Why RFC 6962 rather than "the obvious thing"
//!
//! The obvious thing is: hash the leaves, pair them up, hash the pairs, repeat.
//! Two details in that sentence are wrong, and both have caused real
//! vulnerabilities.
//!
//! ## 1. Leaves and internal nodes must hash differently
//!
//! If `leaf(d) = H(d)` and `node(l, r) = H(l ‖ r)`, then the 64-byte string
//! `l ‖ r` is *both* a valid internal node preimage and a valid leaf. So an
//! attacker can take an internal node from your tree, present it as a leaf, and
//! produce a valid inclusion proof for data that was never in the list. This is
//! the **second-preimage attack** on Merkle trees.
//!
//! RFC 6962 fixes it by prefixing: `0x00` for leaves, `0x01` for nodes. The two
//! preimage spaces can no longer overlap. Cheap, and non-negotiable.
//!
//! ## 2. Odd levels must not duplicate the last node
//!
//! Bitcoin pairs the trailing node with *itself* when a level has an odd count.
//! That makes the lists `[A, B, C]` and `[A, B, C, C]` produce the **same
//! root** — two different blocks with identical headers. That is CVE-2012-2459,
//! and it was a network-splitting denial of service.
//!
//! RFC 6962 splits at the largest power of two below `n` instead, which never
//! duplicates anything, so distinct lists always give distinct roots.
//!
//! Both bugs are demonstrated as tests rather than described.

use bc_hash::{sha256, Hash, Sha256};

/// `MTH({}) = SHA-256()` — the empty tree. RFC 6962 §2.1.
pub fn empty_root() -> Hash {
    sha256(&[])
}

/// `leaf(d) = SHA-256(0x00 ‖ d)`
pub fn leaf_hash(data: &[u8]) -> Hash {
    let mut h = Sha256::new();
    h.update(&[0x00]);
    h.update(data);
    h.finalize()
}

/// `node(l, r) = SHA-256(0x01 ‖ l ‖ r)`
pub fn node_hash(left: &Hash, right: &Hash) -> Hash {
    let mut h = Sha256::new();
    h.update(&[0x01]);
    h.update(left);
    h.update(right);
    h.finalize()
}

/// The largest power of two strictly less than `n`. Requires `n > 1`.
///
/// This is the split point. Note it is *not* `n / 2`: for `n = 5` the split is
/// 4, giving subtrees of 4 and 1, not 2 and 3. Splitting at a power of two is
/// what makes every left subtree complete, which is what lets the tree grow by
/// appending without reshaping anything already committed.
fn split_point(n: usize) -> usize {
    debug_assert!(n > 1);
    1usize << (usize::BITS - 1 - (n - 1).leading_zeros())
}

/// Merkle Tree Hash of an ordered list.
pub fn root(entries: &[&[u8]]) -> Hash {
    match entries.len() {
        0 => empty_root(),
        1 => leaf_hash(entries[0]),
        n => {
            let k = split_point(n);
            node_hash(&root(&entries[..k]), &root(&entries[k..]))
        }
    }
}

/// Convenience wrapper for owned entries.
pub fn root_owned<T: AsRef<[u8]>>(entries: &[T]) -> Hash {
    let refs: Vec<&[u8]> = entries.iter().map(|e| e.as_ref()).collect();
    root(&refs)
}

/// The audit path for entry `index`: the sibling hashes needed to recompute the
/// root, ordered from the leaf upwards.
///
/// Length is ⌈log₂ n⌉, so proving membership of one transaction in a block of a
/// million costs 20 hashes — 640 bytes — instead of the whole block. That
/// logarithmic witness is the entire reason Merkle trees exist.
pub fn audit_path(entries: &[&[u8]], index: usize) -> Option<Vec<Hash>> {
    if index >= entries.len() {
        return None;
    }
    fn go(entries: &[&[u8]], m: usize, out: &mut Vec<Hash>) {
        if entries.len() <= 1 {
            return;
        }
        let k = split_point(entries.len());
        if m < k {
            go(&entries[..k], m, out);
            out.push(root(&entries[k..]));
        } else {
            go(&entries[k..], m - k, out);
            out.push(root(&entries[..k]));
        }
    }
    let mut out = Vec::new();
    go(entries, index, &mut out);
    Some(out)
}

/// Verify an inclusion proof. RFC 6962 §2.1.1.
///
/// The verifier holds only `root`, `index` and `tree_size` — it never sees the
/// list. `fan`/`snn` track where in the tree we are: `fan` is the index within
/// the current subtree, `snn` the index of the last leaf of that subtree. When
/// they are equal we are on the right edge, where the tree is ragged and the
/// sibling sits on the left.
pub fn verify_inclusion(
    root_hash: &Hash,
    index: usize,
    tree_size: usize,
    leaf_data: &[u8],
    path: &[Hash],
) -> bool {
    if index >= tree_size {
        return false;
    }
    let mut fan = index;
    let mut snn = tree_size - 1;
    let mut acc = leaf_hash(leaf_data);

    for sibling in path {
        if snn == 0 {
            // Path is longer than the tree is deep.
            return false;
        }
        if fan & 1 == 1 || fan == snn {
            acc = node_hash(sibling, &acc);
            while fan & 1 == 0 && fan != 0 {
                fan >>= 1;
                snn >>= 1;
            }
        } else {
            acc = node_hash(&acc, sibling);
        }
        fan >>= 1;
        snn >>= 1;
    }

    // `snn == 0` means we consumed exactly enough siblings to reach the root.
    // Without it, a truncated path could verify against a subtree root.
    snn == 0 && acc == *root_hash
}
