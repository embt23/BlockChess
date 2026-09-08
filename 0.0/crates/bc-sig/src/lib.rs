//! Episode 02 — digital signatures.
//!
//! The attack this defeats: *"that wasn't my move."*
//!
//! Ed25519 (RFC 8032) implemented from scratch, so that every step is visible:
//! field arithmetic mod 2^255−19, the twisted Edwards group law, point
//! compression, and the Schnorr-style signing equation.
//!
//! # Why Ed25519 rather than ECDSA
//!
//! **Deterministic nonces.** ECDSA needs a fresh random `k` per signature, and
//! reuse or bias in `k` leaks the private key by simple algebra. That has
//! destroyed real systems. Ed25519 derives its nonce as `SHA-512(prefix ‖ msg)`,
//! so a bad RNG cannot ruin you — there is no RNG in the signing path at all.
//!
//! **Linearity.** The signing equation is
//!
//! ```text
//!   s = r + H(R ‖ A ‖ m)·a        (mod ℓ)
//! ```
//!
//! which is *linear in the secret* `a`. That is what later makes key
//! aggregation (MuSig2) and threshold signatures possible. ECDSA's inversion
//! `s = k⁻¹(z + r·d)` destroys the structure, which is why Bitcoin needed a new
//! signature scheme to get key aggregation.
//!
//! # Security warning
//!
//! `Point::mul_scalar` is **not constant time**, so this crate must not sign
//! with real keys in production. It exists to be read. The node will link
//! `ed25519-dalek`, which is the same mathematics written by cryptographers who
//! have thought hard about side channels.
//!
//! Verification, where every input is public, is fine as written.

#![allow(clippy::should_implement_trait)]

// `Fe` and `Point` expose `add`/`sub`/`mul`/`neg`/`eq` as plain methods rather
// than operator traits, so the code reads like the formulas in the papers it is
// transcribed from. `eq` in particular must *not* be `PartialEq`: field elements
// are kept lazily reduced, so one value has many byte representations and
// structural equality would be silently wrong. Comparing requires canonicalising
// first, which is a real computation, not a derive.

pub mod edwards;
pub mod field;
pub mod scalar;

use bc_hash::sha512;
use edwards::{Point, BASEPOINT};

pub const SECRET_KEY_LEN: usize = 32;
pub const PUBLIC_KEY_LEN: usize = 32;
pub const SIGNATURE_LEN: usize = 64;

/// A private key: 32 uniformly random bytes.
#[derive(Clone)]
pub struct SigningKey {
    seed: [u8; 32],
    /// Clamped scalar derived from the seed.
    a: [u64; 4],
    /// Second half of SHA-512(seed), used as the nonce prefix.
    prefix: [u8; 32],
    public: VerifyingKey,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct VerifyingKey(pub [u8; 32]);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Signature(pub [u8; 64]);

impl SigningKey {
    pub fn from_seed(seed: &[u8; 32]) -> SigningKey {
        let h = sha512(seed);

        // Clamping. Three separate jobs, all of them defensive:
        //  - clearing the low 3 bits makes the scalar a multiple of the
        //    cofactor 8, so it always lands in the prime-order subgroup and
        //    small-subgroup attacks cannot leak key bits;
        //  - setting bit 254 fixes the scalar's length so that a
        //    double-and-add ladder always runs the same number of iterations;
        //  - clearing bit 255 keeps it below 2^255.
        let mut ab = [0u8; 32];
        ab.copy_from_slice(&h[..32]);
        ab[0] &= 248;
        ab[31] &= 127;
        ab[31] |= 64;

        let a = scalar::from_bytes(&ab);
        let mut prefix = [0u8; 32];
        prefix.copy_from_slice(&h[32..]);

        let public = VerifyingKey(BASEPOINT.mul_scalar(&a).compress());
        SigningKey {
            seed: *seed,
            a,
            prefix,
            public,
        }
    }

    pub fn seed(&self) -> &[u8; 32] {
        &self.seed
    }

    pub fn verifying_key(&self) -> VerifyingKey {
        self.public
    }

    pub fn sign(&self, msg: &[u8]) -> Signature {
        // r = H(prefix ‖ m) mod ℓ — deterministic, no randomness anywhere.
        let mut h = bc_hash::Sha512::new();
        h.update(&self.prefix);
        h.update(msg);
        let r = scalar::reduce_bytes(&h.finalize());

        let big_r = BASEPOINT.mul_scalar(&r).compress();

        // k = H(R ‖ A ‖ m) mod ℓ — binding the commitment, the key, and the
        // message together. Leaving A out of this hash is a real historical
        // bug: it enables key-substitution attacks, where one signature
        // verifies under two different public keys.
        let mut h = bc_hash::Sha512::new();
        h.update(&big_r);
        h.update(&self.public.0);
        h.update(msg);
        let k = scalar::reduce_bytes(&h.finalize());

        // s = r + k·a (mod ℓ)
        let s = scalar::mul_add(&k, &self.a, &r);

        let mut sig = [0u8; 64];
        sig[..32].copy_from_slice(&big_r);
        sig[32..].copy_from_slice(&scalar::to_bytes(&s));
        Signature(sig)
    }
}

impl VerifyingKey {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Verify a signature. Returns false rather than erroring — there is
    /// nothing a caller can usefully do with the reason.
    pub fn verify(&self, msg: &[u8], sig: &Signature) -> bool {
        let mut r_bytes = [0u8; 32];
        r_bytes.copy_from_slice(&sig.0[..32]);
        let mut s_bytes = [0u8; 32];
        s_bytes.copy_from_slice(&sig.0[32..]);

        // Reject non-canonical S. See `scalar::is_canonical` — without this,
        // S and S+ℓ both verify and signatures stop being unique.
        if !scalar::is_canonical(&s_bytes) {
            return false;
        }

        let big_r = match Point::decompress(&r_bytes) {
            Some(p) => p,
            None => return false,
        };
        let a_point = match Point::decompress(&self.0) {
            Some(p) => p,
            None => return false,
        };

        let mut h = bc_hash::Sha512::new();
        h.update(&r_bytes);
        h.update(&self.0);
        h.update(msg);
        let k = scalar::reduce_bytes(&h.finalize());

        // Check [S]B = R + [k]A.
        //
        // Where this comes from: the signer set s = r + k·a, so
        //   [s]B = [r]B + [k·a]B = R + [k]([a]B) = R + [k]A.
        // Only someone who knows `a` can produce an `s` satisfying it, because
        // finding `a` from A = [a]B is the discrete log problem.
        let lhs = BASEPOINT.mul_scalar(&scalar::from_bytes(&s_bytes));
        let rhs = big_r.add(&a_point.mul_scalar(&k));
        lhs.eq(&rhs)
    }
}

impl Signature {
    pub fn to_bytes(&self) -> [u8; 64] {
        self.0
    }
    pub fn from_bytes(b: &[u8; 64]) -> Signature {
        Signature(*b)
    }
}
