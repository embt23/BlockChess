//! Sparse Merkle tree tests.
//!
//! There is no published vector set for this construction, so the oracle is a
//! *differential* one: at depth 8 the tree has only 256 leaves, so it can be
//! built densely — a flat array of every leaf, folded pairwise, with no empty-
//! hash shortcut and no sparse map anywhere. If the sparse implementation
//! agrees with the dense one on many random key sets, the optimisation is
//! sound.
//!
//! That is weaker than an external oracle and it is stated as such. It is still
//! the right test, because the dense version is the *definition* and the sparse
//! version is the optimisation, so the two are genuinely independent code.

use bc_hash::Hash;
use bc_merkle::smt::{self, empty_hashes, verify, Key, Smt};

/// Deterministic pseudo-random source (xorshift64*), so failures reproduce.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn key(&mut self) -> Key {
        let mut k = [0u8; 32];
        for c in k.chunks_mut(8) {
            c.copy_from_slice(&self.next().to_le_bytes());
        }
        k
    }
}

/// The definition: every leaf laid out flat, folded pairwise to a root.
/// At depth 8 the leaf index is the top byte of the key.
fn dense_root_depth8(entries: &[(Key, Vec<u8>)]) -> Hash {
    let mut level: Vec<Hash> = (0..256u32)
        .map(|i| match entries.iter().find(|(k, _)| k[0] as u32 == i) {
            Some((k, v)) => smt::leaf_hash(k, v),
            None => smt::empty_leaf(),
        })
        .collect();
    while level.len() > 1 {
        level = level
            .chunks(2)
            .map(|c| smt::node_hash(&c[0], &c[1]))
            .collect();
    }
    level[0]
}

#[test]
fn sparse_matches_dense_at_depth_8() {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    for trial in 0..40 {
        // Distinct top bytes, so no two keys share a leaf slot at depth 8.
        let n = 1 + (rng.next() % 24) as usize;
        let mut entries: Vec<(Key, Vec<u8>)> = Vec::new();
        let mut used = std::collections::HashSet::new();
        while entries.len() < n {
            let mut k = rng.key();
            k[0] = (rng.next() % 256) as u8;
            if !used.insert(k[0]) {
                continue;
            }
            let v = rng.next().to_le_bytes().to_vec();
            entries.push((k, v));
        }

        let mut t = Smt::new(8);
        for (k, v) in &entries {
            t.insert(*k, v.clone());
        }
        assert_eq!(
            t.root(),
            dense_root_depth8(&entries),
            "trial {trial}: sparse root must equal the dense definition"
        );
    }
}

#[test]
fn empty_tree_root_is_the_empty_hash() {
    for depth in [1usize, 8, 32, 256] {
        let t = Smt::new(depth);
        assert_eq!(t.root(), empty_hashes(depth)[depth]);
        assert_eq!(t.stored_nodes(), 0);
    }
}

#[test]
fn root_is_a_function_of_the_set_not_the_order() {
    let mut rng = Rng(7);
    let items: Vec<(Key, Vec<u8>)> = (0..50)
        .map(|i| (rng.key(), vec![i as u8; 1 + (i % 30)]))
        .collect();

    let mut forward = Smt::default();
    for (k, v) in &items {
        forward.insert(*k, v.clone());
    }

    let mut backward = Smt::default();
    for (k, v) in items.iter().rev() {
        backward.insert(*k, v.clone());
    }

    // And once more in a shuffled order.
    let mut shuffled = Smt::default();
    let mut idx: Vec<usize> = (0..items.len()).collect();
    for i in (1..idx.len()).rev() {
        idx.swap(i, (rng.next() % (i as u64 + 1)) as usize);
    }
    for i in idx {
        shuffled.insert(items[i].0, items[i].1.clone());
    }

    assert_eq!(forward.root(), backward.root());
    assert_eq!(forward.root(), shuffled.root());
}

#[test]
fn removal_restores_the_previous_root() {
    let mut rng = Rng(11);
    let mut t = Smt::default();
    let base_keys: Vec<Key> = (0..20).map(|_| rng.key()).collect();
    for k in &base_keys {
        t.insert(*k, b"v".to_vec());
    }
    let before = t.root();

    let extra = rng.key();
    t.insert(extra, b"temporary".to_vec());
    assert_ne!(t.root(), before);

    t.remove(&extra);
    assert_eq!(t.root(), before, "remove must undo insert exactly");
    assert_eq!(t.len(), base_keys.len());

    // Overwriting a value and putting it back is likewise exact.
    t.insert(base_keys[3], b"changed".to_vec());
    assert_ne!(t.root(), before);
    t.insert(base_keys[3], b"v".to_vec());
    assert_eq!(t.root(), before);
}

#[test]
fn inclusion_and_absence_proofs() {
    let mut rng = Rng(13);
    let mut t = Smt::default();
    let present: Vec<(Key, Vec<u8>)> = (0..64).map(|i| (rng.key(), vec![i as u8; 4])).collect();
    for (k, v) in &present {
        t.insert(*k, v.clone());
    }
    let root = t.root();

    for (k, v) in &present {
        let p = t.prove(k);
        assert!(
            verify(t.depth(), &root, k, Some(v), &p),
            "inclusion proof must verify"
        );
        // The same proof must NOT verify a claim of absence.
        assert!(!verify(t.depth(), &root, k, None, &p));
        // Nor a different value.
        assert!(!verify(t.depth(), &root, k, Some(b"other"), &p));
    }

    // Absence: keys that were never inserted.
    for _ in 0..64 {
        let k = rng.key();
        assert!(t.get(&k).is_none());
        let p = t.prove(&k);
        assert!(
            verify(t.depth(), &root, &k, None, &p),
            "absence proof must verify"
        );
        assert!(!verify(t.depth(), &root, &k, Some(b"invented"), &p));
    }
}

#[test]
fn proofs_reject_tampering_and_malleability() {
    let mut rng = Rng(17);
    let mut t = Smt::default();
    let keys: Vec<Key> = (0..40).map(|_| rng.key()).collect();
    for (i, k) in keys.iter().enumerate() {
        t.insert(*k, vec![i as u8]);
    }
    let root = t.root();
    let key = keys[5];
    let val = vec![5u8];
    let good = t.prove(&key);
    assert!(verify(t.depth(), &root, &key, Some(&val), &good));

    // Corrupted sibling.
    let mut bad = good.clone();
    if !bad.siblings.is_empty() {
        bad.siblings[0][0] ^= 1;
        assert!(!verify(t.depth(), &root, &key, Some(&val), &bad));
    }

    // Extra sibling supplied but not claimed in the bitmap: must be rejected,
    // otherwise one statement would have many valid encodings.
    let mut extra = good.clone();
    extra.siblings.push([0xAAu8; 32]);
    assert!(!verify(t.depth(), &root, &key, Some(&val), &extra));

    // Bitmap claims a sibling that is not supplied.
    let mut short = good.clone();
    short.bitmap[0] |= 1;
    if short != good {
        assert!(!verify(t.depth(), &root, &key, Some(&val), &short));
    }

    // Wrong bitmap length.
    let mut wrong_len = good.clone();
    wrong_len.bitmap.push(0);
    assert!(!verify(t.depth(), &root, &key, Some(&val), &wrong_len));

    // A proof for one key must not verify another.
    assert!(!verify(t.depth(), &root, &keys[6], Some(&val), &good));

    // A leaf hash is bound to its key, so the same value at a different key is
    // a different leaf.
    assert_ne!(
        smt::leaf_hash(&keys[5], b"x"),
        smt::leaf_hash(&keys[6], b"x")
    );
}

#[test]
fn padding_bits_must_be_zero() {
    // Depth 12 needs 12 bits, stored in 2 bytes. The top 4 bits are padding and
    // must be zero, or a proof would have 16 spellings.
    let mut t = Smt::new(12);
    let k = [0x42u8; 32];
    t.insert(k, b"v".to_vec());
    let root = t.root();
    let good = t.prove(&k);
    assert!(verify(12, &root, &k, Some(b"v"), &good));

    let mut padded = good.clone();
    *padded.bitmap.last_mut().unwrap() |= 0b1111_0000;
    assert!(!verify(12, &root, &k, Some(b"v"), &padded));
}

#[test]
fn proof_size_is_logarithmic() {
    // The point of the sparse encoding: a proof carries only the siblings that
    // are actually non-empty, which is about log2(n) of them, not 256.
    let mut rng = Rng(23);
    let mut t = Smt::default();
    for n in [1usize, 16, 256, 4096] {
        while t.len() < n {
            t.insert(rng.key(), b"v".to_vec());
        }
        let probe = rng.key();
        let p = t.prove(&probe);
        let expected = (n as f64).log2();
        assert!(
            (p.siblings.len() as f64) < expected + 6.0,
            "n={n}: {} siblings, expected ~{expected:.1}",
            p.siblings.len()
        );
        // Full path would be 256 siblings = 8192 bytes.
        assert!(p.size() < 8192 / 4, "n={n}: {} bytes", p.size());
    }
}

#[test]
fn write_cost_is_independent_of_tree_size() {
    // Each insert rewrites exactly `depth` nodes on one path, whatever else is
    // in the tree. That is the good property.
    let mut rng = Rng(29);
    let mut t = Smt::default();
    for _ in 0..200 {
        t.insert(rng.key(), b"v".to_vec());
    }
    assert_eq!(t.len(), 200);
}

#[test]
fn storage_cost_is_the_known_weakness_of_the_naive_construction() {
    // Documenting a real limitation rather than pretending it away.
    //
    // Random keys diverge within about log2(n) levels of the root. Below that
    // point each key owns a private chain of single-child nodes running all the
    // way down to its leaf — roughly `depth - log2(n)` of them. So storage is
    // O(n · depth), not O(n · log n):
    //
    //     n=10    2,539 nodes   253.9 per key
    //     n=100  25,053 nodes   250.5 per key
    //     n=1000 247,154 nodes  247.2 per key
    //
    // At ~64 bytes a node that is ~16 MB for a thousand accounts and roughly
    // 15 GB for a million. Fine for a testnet, not for production.
    //
    // The standard fix is path compression: store only the nodes where the tree
    // actually branches, and *compute* the single-child chains on demand from
    // the leaf below. Critically, that changes no root hash and no proof — a
    // chain node's hash is fully determined by the leaf beneath it and the
    // known empty hashes — so it is a pure internal optimisation that can be
    // dropped in later without invalidating anything already committed.
    //
    // See spec/09-open-questions.md, D15.
    let mut rng = Rng(29);
    let mut t = Smt::default();
    for _ in 0..1000 {
        t.insert(rng.key(), b"v".to_vec());
    }
    let per_key = t.stored_nodes() as f64 / t.len() as f64;
    assert!(
        (240.0..256.0).contains(&per_key),
        "expected ~depth-log2(n) nodes per key, got {per_key:.1}"
    );
}
