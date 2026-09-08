//! Official test vectors. These are the ground truth for episode 01 — the same
//! role `perft` plays for the chess engine. If these pass, the implementation
//! is right; there is no need to reason about whether it "looks correct".

use bc_hash::{hex, sha256, sha512, tagged, tagged_parts, HashChain};

#[test]
fn sha256_fips_vectors() {
    assert_eq!(
        hex(&sha256(b"")),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        hex(&sha256(b"abc")),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        hex(&sha256(
            b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
        )),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
}

#[test]
fn sha256_million_a() {
    // The classic long-message vector: one million 'a'. Exercises the block
    // loop and the 64-bit length field.
    let mut h = bc_hash::Sha256::new();
    for _ in 0..1000 {
        h.update(&[b'a'; 1000]);
    }
    assert_eq!(
        hex(&h.finalize()),
        "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
    );
}

#[test]
fn sha256_streaming_matches_oneshot() {
    // Feeding the same bytes in arbitrarily-sized chunks must give the same
    // digest. This is what catches buffer-management bugs, which is where
    // hand-written hash implementations actually go wrong.
    let data: Vec<u8> = (0..1000u32).map(|i| (i % 251) as u8).collect();
    let oneshot = sha256(&data);
    for chunk in [1usize, 7, 63, 64, 65, 127, 128, 333] {
        let mut h = bc_hash::Sha256::new();
        for part in data.chunks(chunk) {
            h.update(part);
        }
        assert_eq!(h.finalize(), oneshot, "chunk size {chunk}");
    }
}

#[test]
fn sha512_fips_vectors() {
    assert_eq!(
        hex(&sha512(b"")),
        "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce\
47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e"
    );
    assert_eq!(
        hex(&sha512(b"abc")),
        "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a\
2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
    );
}

#[test]
fn sha512_streaming_matches_oneshot() {
    let data: Vec<u8> = (0..2000u32).map(|i| (i % 251) as u8).collect();
    let oneshot = sha512(&data);
    for chunk in [1usize, 127, 128, 129, 255, 256, 999] {
        let mut h = bc_hash::Sha512::new();
        for part in data.chunks(chunk) {
            h.update(part);
        }
        assert_eq!(h.finalize(), oneshot, "chunk size {chunk}");
    }
}

#[test]
fn domain_separation_actually_separates() {
    assert_ne!(tagged("BC/state/v1", b"x"), tagged("BC/pos/v1", b"x"));
}

#[test]
fn length_prefix_stops_boundary_shifting() {
    // Without per-part length prefixes these two would be identical, and an
    // attacker could move bytes across the boundary between fields.
    assert_ne!(
        tagged_parts("t", &[b"ab", b"c"]),
        tagged_parts("t", &[b"a", b"bc"])
    );
}

#[test]
fn chain_detects_every_kind_of_tampering() {
    let events: Vec<&[u8]> = vec![b"e2e4", b"e7e5", b"g1f3", b"b8c6"];
    let mut c = HashChain::new();
    for e in &events {
        c.append(e);
    }
    let head = c.head();
    assert!(HashChain::verify([0u8; 32], &events, head));

    let altered: Vec<&[u8]> = vec![b"e2e4", b"e7e6", b"g1f3", b"b8c6"];
    assert!(!HashChain::verify([0u8; 32], &altered, head));

    // Reordered — the position index inside each link catches this.
    let swapped: Vec<&[u8]> = vec![b"e7e5", b"e2e4", b"g1f3", b"b8c6"];
    assert!(!HashChain::verify([0u8; 32], &swapped, head));

    assert!(!HashChain::verify([0u8; 32], &events[..3], head));

    let mut longer = events.clone();
    longer.push(b"f1b5");
    assert!(!HashChain::verify([0u8; 32], &longer, head));
}

#[test]
fn constants_derive_from_prime_roots() {
    // The SHA constants are not magic numbers: K[i] is the fractional part of
    // the cube root of the i-th prime, scaled by 2^32. Re-derive a few here so
    // the file cannot silently drift from its own documentation.
    fn iroot(x: u128, n: u32) -> u128 {
        if x == 0 {
            return 0;
        }
        let mut hi: u128 = 1;
        while hi.pow(n) <= x {
            hi *= 2;
        }
        let (mut lo, mut hi) = (hi / 2, hi);
        while lo < hi {
            let mid = (lo + hi).div_ceil(2);
            if mid.pow(n) <= x {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }
        lo
    }
    let frac = |p: u128, n: u32| -> u32 { (iroot(p << (n * 32), n) - (iroot(p, n) << 32)) as u32 };

    let h0: Vec<u32> = [2u128, 3, 5, 7, 11, 13, 17, 19]
        .iter()
        .map(|&p| frac(p, 2))
        .collect();
    assert_eq!(h0[0], 0x6a09e667);
    assert_eq!(h0[7], 0x5be0cd19);

    let k: Vec<u32> = [2u128, 3, 5, 7].iter().map(|&p| frac(p, 3)).collect();
    assert_eq!(k, vec![0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5]);
}
