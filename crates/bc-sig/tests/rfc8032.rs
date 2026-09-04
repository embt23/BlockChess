//! RFC 8032 §7.1 test vectors — the external oracle for episode 02, exactly as
//! `perft` is for episode 03. Hand-rolled cryptography is only defensible when
//! it is checked against the specification's own vectors.

use bc_sig::edwards::{Point, BASEPOINT, D};
use bc_sig::field::Fe;
use bc_sig::{Signature, SigningKey, VerifyingKey};

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}
fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
fn arr32(s: &str) -> [u8; 32] {
    unhex(s).try_into().unwrap()
}

/// (secret seed, public key, message, signature)
const VECTORS: &[(&str, &str, &str, &str)] = &[
    (
        "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
        "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a",
        "",
        "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b",
    ),
    (
        "4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb",
        "3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c",
        "72",
        "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00",
    ),
    (
        "c5aa8df43f9f837bedb7442f31dcb7b166d38535076f094b85ce3a2e0b4458f7",
        "fc51cd8e6218a1a38da47ed00230f0580816ed13ba3303ac5deb911548908025",
        "af82",
        "6291d657deec24024827e69c3abe01a30ce548a284743a445e3680d7db5ac3ac18ff9b538d16f290ae67f760984dc6594a7c15e9716ed28dc027beceea1ec40a",
    ),
    (
        // Message is SHA-512("abc") — ties episodes 01 and 02 together.
        "833fe62409237b9d62ec77587520911e9a759cec1d19755b7da901b96dca3d42",
        "ec172b93ad5e563bf4932c70e1245034c35467ef2efd4d64ebf819683467e2bf",
        "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f",
        "dc2a4459e7369633a52b1bf277839a00201009a3efbf3ecb69bea2186c26b58909351fc9ac90b3ecfdfbc7c66431e0303dca179c138ac17ad9bef1177331a704",
    ),
];

#[test]
fn rfc8032_public_keys() {
    for (seed, pk, _, _) in VECTORS {
        let sk = SigningKey::from_seed(&arr32(seed));
        assert_eq!(hex(sk.verifying_key().as_bytes()), *pk, "seed {seed}");
    }
}

#[test]
fn rfc8032_signatures() {
    for (seed, _, msg, sig) in VECTORS {
        let sk = SigningKey::from_seed(&arr32(seed));
        let got = sk.sign(&unhex(msg));
        assert_eq!(hex(&got.0), *sig, "seed {seed}");
    }
}

#[test]
fn rfc8032_verification() {
    for (_, pk, msg, sig) in VECTORS {
        let vk = VerifyingKey(arr32(pk));
        let s = Signature(unhex(sig).try_into().unwrap());
        assert!(vk.verify(&unhex(msg), &s), "pk {pk}");
    }
}

#[test]
fn rejects_tampered_message() {
    let sk = SigningKey::from_seed(&[7u8; 32]);
    let sig = sk.sign(b"e2e4");
    assert!(sk.verifying_key().verify(b"e2e4", &sig));
    assert!(!sk.verifying_key().verify(b"e2e5", &sig));
    assert!(!sk.verifying_key().verify(b"", &sig));
}

#[test]
fn rejects_wrong_key() {
    let a = SigningKey::from_seed(&[1u8; 32]);
    let b = SigningKey::from_seed(&[2u8; 32]);
    let sig = a.sign(b"resign");
    assert!(!b.verifying_key().verify(b"resign", &sig));
}

#[test]
fn rejects_non_canonical_s() {
    // Signature malleability: S and S+ℓ are the same scalar mod ℓ, so both
    // would satisfy the verification equation. Enforcing S < ℓ is what makes
    // the encoding unique.
    let sk = SigningKey::from_seed(&[3u8; 32]);
    let sig = sk.sign(b"a move");
    assert!(sk.verifying_key().verify(b"a move", &sig));

    let s = bc_sig::scalar::from_bytes(&sig.0[32..].try_into().unwrap());
    let mut carry = 0u128;
    let mut s_plus_l = [0u64; 4];
    for i in 0..4 {
        let t = s[i] as u128 + bc_sig::scalar::L[i] as u128 + carry;
        s_plus_l[i] = t as u64;
        carry = t >> 64;
    }

    let mut mauled = sig.0;
    mauled[32..].copy_from_slice(&bc_sig::scalar::to_bytes(&s_plus_l));
    assert_ne!(mauled, sig.0, "the mauled signature must differ in bytes");
    assert!(
        !sk.verifying_key().verify(b"a move", &Signature(mauled)),
        "non-canonical S must be rejected"
    );
}

#[test]
fn basepoint_matches_rfc() {
    // RFC 8032's encoded base point, y = 4/5 with even x.
    assert_eq!(
        hex(&BASEPOINT.compress()),
        "5866666666666666666666666666666666666666666666666666666666666666"
    );
}

#[test]
fn basepoint_has_order_l() {
    // [ℓ]B = identity. This is the definition of ℓ, and it is a strong check on
    // the whole group implementation at once — the point arithmetic, the
    // scalar handling, and the curve constant all have to be right for a
    // 253-bit scalar multiplication to land exactly on (0, 1).
    let l = bc_sig::scalar::L;
    let r = BASEPOINT.mul_scalar(&l);
    assert!(
        r.eq(&bc_sig::edwards::IDENTITY),
        "[l]B should be the identity"
    );
}

#[test]
fn point_arithmetic_is_a_group() {
    let b = *BASEPOINT;
    // P + P == 2P: the complete addition law must agree with the doubling
    // formula, which is the property Weierstrass curves lack.
    assert!(b.add(&b).eq(&b.double()));
    // P + (−P) == identity
    assert!(b.add(&b.neg()).eq(&bc_sig::edwards::IDENTITY));
    // Associativity on a small sample.
    let p2 = b.double();
    let p3 = p2.add(&b);
    assert!(p2.add(&b).eq(&b.add(&p2)));
    assert!(p3.add(&b).eq(&p2.double()));
    // Scalar multiplication is a homomorphism: [5]B == B+B+B+B+B
    let five = b.mul_scalar(&[5, 0, 0, 0]);
    assert!(five.eq(&p3.add(&p2)));
}

#[test]
fn compression_roundtrips() {
    let mut p = *BASEPOINT;
    for _ in 0..20 {
        let enc = p.compress();
        let back = Point::decompress(&enc).expect("round trip");
        assert!(back.eq(&p));
        p = p.add(&BASEPOINT);
    }
}

#[test]
fn rejects_points_not_on_curve() {
    // y = 2 has no corresponding x on this curve.
    let mut enc = Fe::from_u64(2).to_bytes();
    enc[31] &= 0x7F;
    assert!(Point::decompress(&enc).is_none());
}

#[test]
fn curve_constant_is_correct() {
    // d = −121665/121666, i.e. 121666·d + 121665 ≡ 0.
    let lhs = Fe::from_u64(121666).mul(*D).add(Fe::from_u64(121665));
    assert!(lhs.is_zero());
}

#[test]
fn field_arithmetic_sanity() {
    let a = Fe::from_u64(12345);
    assert!(a.mul(a.invert()).eq(bc_sig::field::ONE));
    assert!(a.sub(a).is_zero());
    assert!(a.add(a.neg()).is_zero());
    // (a+b)^2 = a^2 + 2ab + b^2
    let b = Fe::from_u64(987654321);
    let lhs = a.add(b).sq();
    let rhs = a.sq().add(a.mul(b).mul(Fe::from_u64(2))).add(b.sq());
    assert!(lhs.eq(rhs));
}

#[test]
fn field_sub_handles_repeated_borrow() {
    // Regression. Field elements are kept lazily reduced, so a value can be
    // held as any representative in [0, 2^256). Subtracting from a small one
    // borrows past the top limb; folding the 2^256 back in costs 38, which can
    // borrow *again* when the intermediate is itself below 38.
    //
    // The original code corrected only once. Every identity below still looked
    // plausible, curve points stayed on the curve, and the RFC vectors were the
    // only thing that caught it.

    // 2^256 − 37, a non-canonical representative of 1 (since 2^256 ≡ 38).
    let two56_minus_37 = Fe([
        0xFFFF_FFFF_FFFF_FFDB,
        0xFFFF_FFFF_FFFF_FFFF,
        0xFFFF_FFFF_FFFF_FFFF,
        0xFFFF_FFFF_FFFF_FFFF,
    ]);
    assert!(two56_minus_37.eq(bc_sig::field::ONE), "should reduce to 1");

    // 0 − 1 must be p − 1, however the 1 was spelled.
    let want = bc_sig::field::ZERO.sub(bc_sig::field::ONE);
    let got = bc_sig::field::ZERO.sub(two56_minus_37);
    assert!(
        got.eq(want),
        "0 − 1 must not depend on the representative of 1"
    );
    assert!(got.add(bc_sig::field::ONE).is_zero());
}

#[test]
fn field_identities_over_many_values() {
    // A cheap deterministic sweep. The point is to hit representatives all over
    // [0, 2^256), including the ones near the wrap boundaries where lazy
    // reduction goes wrong.
    let mut state = 0x243F_6A88_85A3_08D3u64;
    let mut next = || {
        // xorshift64*
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state.wrapping_mul(0x2545_F491_4F6C_DD1D)
    };

    let mut cases: Vec<Fe> = vec![
        bc_sig::field::ZERO,
        bc_sig::field::ONE,
        Fe([37, 0, 0, 0]),
        Fe([38, 0, 0, 0]),
        Fe([39, 0, 0, 0]),
        Fe([u64::MAX; 4]),                                 // 2^256 − 1
        Fe([u64::MAX - 37, u64::MAX, u64::MAX, u64::MAX]), // 2^256 − 38
        Fe([u64::MAX - 36, u64::MAX, u64::MAX, u64::MAX]), // 2^256 − 37
        Fe(bc_sig::field::P),                              // exactly p
    ];
    for _ in 0..200 {
        cases.push(Fe([next(), next(), next(), next()]));
    }

    for &a in &cases {
        for &b in &cases {
            // (a − b) + b == a
            assert!(a.sub(b).add(b).eq(a), "sub/add roundtrip");
            // a − b == a + (−b)
            assert!(a.sub(b).eq(a.add(b.neg())), "sub vs add-neg");
            // commutativity and distributivity
            assert!(a.mul(b).eq(b.mul(a)), "mul commutes");
            assert!(a.add(b).mul(a).eq(a.sq().add(a.mul(b))), "distributive");
        }
        if !a.is_zero() {
            assert!(a.mul(a.invert()).eq(bc_sig::field::ONE), "inverse");
        }
    }
}

#[test]
fn signatures_over_realistic_game_states() {
    // What the crate is actually for: signing the 109-byte game state from
    // spec/04-channel.md. Two keys, alternating plies, each countersigning.
    let white = SigningKey::from_seed(&[0x11; 32]);
    let black = SigningKey::from_seed(&[0x22; 32]);

    let mut prev = [0u8; 32];
    for ply in 0..40u16 {
        let mut state = Vec::new();
        state.extend_from_slice(&[0xAB; 32]); // channel_id
        state.extend_from_slice(&ply.to_le_bytes());
        state.extend_from_slice(&prev);
        state.extend_from_slice(&[0u8; 2 + 32 + 4 + 4 + 1]);

        let mover = if ply % 2 == 0 { &white } else { &black };
        let other = if ply % 2 == 0 { &black } else { &white };
        let sig = mover.sign(&state);

        assert!(mover.verifying_key().verify(&state, &sig));
        assert!(!other.verifying_key().verify(&state, &sig), "wrong signer");

        prev = bc_hash::tagged("BC/state/v1", &state);
    }
}
