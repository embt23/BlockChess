//! Certificate Transparency test vectors — the external oracle for the list
//! tree. Two of them can be checked without trusting anything else:
//! `MTH([])` is SHA-256 of the empty string, and `MTH([""])` is SHA-256 of a
//! single zero byte.

use bc_merkle::tree::{audit_path, leaf_hash, node_hash, root, verify_inclusion};

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}
fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// The eight standard CT test entries.
const ENTRIES: &[&str] = &[
    "",
    "00",
    "10",
    "2021",
    "3031",
    "40414243",
    "5051525354555657",
    "606162636465666768696a6b6c6d6e6f",
];

/// Published Merkle Tree Hash for the first n entries, n = 0..=8.
const ROOTS: &[&str] = &[
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "6e340b9cffb37a989ca544e6bb780a2c78901d3fb33738768511a30617afa01d",
    "fac54203e7cc696cf0dfcb42c92a1d9dbaf70ad9e621f4bd8d98662f00e3c125",
    "aeb6bcfe274b70a14fb067a5e5578264db0fa9b51af5e0ba159158f329e06e77",
    "d37ee418976dd95753c1c73862b9398fa2a2cf9b4ff0fdfe8b30cd95209614b7",
    "4e3bbb1f7b478dcfe71fb631631519a3bca12c9aefca1612bfce4c13a86264d4",
    "76e67dadbcdf1e10e1b74ddc608abd2f98dfb16fbce75277b5232a127f2087ef",
    "ddb89be403809e325750d3d263cd78929c2942b7942a34b77e122c9594a74c8c",
    "5dc9da79a70659a9ad559cb701ded9a2ab9d823aad2f4960cfe370eff4604328",
];

fn entries(n: usize) -> Vec<Vec<u8>> {
    ENTRIES[..n].iter().map(|s| unhex(s)).collect()
}

#[test]
fn ct_roots() {
    for (n, want) in ROOTS.iter().enumerate() {
        let owned = entries(n);
        let refs: Vec<&[u8]> = owned.iter().map(|v| v.as_slice()).collect();
        assert_eq!(hex(&root(&refs)), *want, "MTH(D[{n}])");
    }
}

#[test]
fn ct_anchors_are_independently_checkable() {
    // These two do not depend on the tree structure at all.
    assert_eq!(hex(&root(&[])), hex(&bc_hash::sha256(&[])));
    assert_eq!(hex(&root(&[&[][..]])), hex(&bc_hash::sha256(&[0x00])));
}

#[test]
fn audit_paths_verify_for_every_index() {
    for n in 1..=8usize {
        let owned = entries(n);
        let refs: Vec<&[u8]> = owned.iter().map(|v| v.as_slice()).collect();
        let r = root(&refs);
        for i in 0..n {
            let path = audit_path(&refs, i).expect("index in range");
            assert!(
                verify_inclusion(&r, i, n, refs[i], &path),
                "n={n} i={i} should verify"
            );
            // Path length is logarithmic in the tree size.
            assert!(path.len() <= n.next_power_of_two().trailing_zeros() as usize);
        }
        assert!(audit_path(&refs, n).is_none(), "index out of range");
    }
}

#[test]
fn audit_paths_reject_tampering() {
    let owned = entries(8);
    let refs: Vec<&[u8]> = owned.iter().map(|v| v.as_slice()).collect();
    let r = root(&refs);
    let path = audit_path(&refs, 3).unwrap();

    assert!(verify_inclusion(&r, 3, 8, refs[3], &path));
    // Wrong data.
    assert!(!verify_inclusion(&r, 3, 8, b"forged", &path));
    // Right data, wrong position.
    assert!(!verify_inclusion(&r, 4, 8, refs[3], &path));
    // Truncated path must not verify against a subtree root.
    assert!(!verify_inclusion(
        &r,
        3,
        8,
        refs[3],
        &path[..path.len() - 1]
    ));
    // Corrupted sibling.
    let mut bad = path.clone();
    bad[0][0] ^= 1;
    assert!(!verify_inclusion(&r, 3, 8, refs[3], &bad));
}

#[test]
fn second_preimage_attack_is_blocked() {
    // Without leaf/node domain separation, the 64-byte concatenation of two
    // child hashes is simultaneously a valid internal-node preimage and a valid
    // leaf. An attacker could then present an internal node as a leaf and prove
    // "inclusion" of data never in the list.
    //
    // RFC 6962 prefixes leaves with 0x00 and nodes with 0x01, so the two
    // preimage spaces are disjoint by construction.
    let a = leaf_hash(b"tx-a");
    let b = leaf_hash(b"tx-b");
    let internal = node_hash(&a, &b);

    // The bytes an attacker would submit as a "leaf".
    let mut forged_leaf = Vec::new();
    forged_leaf.extend_from_slice(&a);
    forged_leaf.extend_from_slice(&b);

    assert_ne!(
        leaf_hash(&forged_leaf),
        internal,
        "an internal node must not be reachable as a leaf hash"
    );

    // Concretely: in a two-entry tree, the root IS that internal node, and the
    // forged 64-byte "leaf" must not verify as a member.
    let refs: Vec<&[u8]> = vec![b"tx-a", b"tx-b"];
    let r = root(&refs);
    assert_eq!(r, internal);
    assert!(!verify_inclusion(&r, 0, 1, &forged_leaf, &[]));
}

#[test]
fn cve_2012_2459_does_not_apply() {
    // Bitcoin pairs a trailing odd node with itself. That makes [A,B,C] and
    // [A,B,C,C] hash to the same root — two distinct blocks with identical
    // headers, which was a network-splitting denial of service.
    fn bitcoin_style_root(entries: &[&[u8]]) -> bc_hash::Hash {
        let mut level: Vec<bc_hash::Hash> = entries.iter().map(|e| leaf_hash(e)).collect();
        while level.len() > 1 {
            if level.len() % 2 == 1 {
                let last = *level.last().unwrap();
                level.push(last); // <- the bug
            }
            level = level.chunks(2).map(|c| node_hash(&c[0], &c[1])).collect();
        }
        level[0]
    }

    let three: Vec<&[u8]> = vec![b"A", b"B", b"C"];
    let four: Vec<&[u8]> = vec![b"A", b"B", b"C", b"C"];

    assert_eq!(
        bitcoin_style_root(&three),
        bitcoin_style_root(&four),
        "the duplicating scheme really does collide (this is the CVE)"
    );
    assert_ne!(
        root(&three),
        root(&four),
        "splitting at a power of two must keep distinct lists distinct"
    );
}

#[test]
fn distinct_lists_give_distinct_roots() {
    // A broader sweep of the same property: no two different lists of up to
    // four short entries may share a root.
    let alphabet: [&[u8]; 3] = [b"a", b"b", b"c"];
    let mut seen = std::collections::HashMap::new();

    // Every list of length 0..=4 over the alphabet, each generated once.
    let mut all: Vec<Vec<&[u8]>> = vec![vec![]];
    let mut frontier: Vec<Vec<&[u8]>> = vec![vec![]];
    for _ in 0..4 {
        let mut next = Vec::new();
        for l in &frontier {
            for a in alphabet {
                let mut m = l.clone();
                m.push(a);
                next.push(m);
            }
        }
        all.extend(next.iter().cloned());
        frontier = next;
    }
    assert_eq!(all.len(), 1 + 3 + 9 + 27 + 81);

    for l in &all {
        let r = root(l);
        if let Some(prev) = seen.insert(r, l.clone()) {
            panic!("root collision between {prev:?} and {l:?}");
        }
    }
}
