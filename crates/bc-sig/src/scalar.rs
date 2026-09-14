//! Arithmetic modulo the group order ℓ.
//!
//! ```text
//!   ℓ = 2^252 + 27742317777372353535851937790883648493   (prime)
//! ```
//!
//! The curve has 8ℓ points; ℓ is the order of the prime-order subgroup we
//! actually work in, and the factor 8 is the cofactor. Scalars — private keys,
//! nonces, challenges — live in Z/ℓZ, *not* mod p. Confusing the two is the
//! classic beginner error: p is about coordinates, ℓ is about how many times
//! you can add a point to itself before returning to the identity.

/// ℓ, as little-endian 64-bit limbs.
pub const L: [u64; 4] = [
    0x5812_631a_5cf5_d3ed,
    0x14de_f9de_a2f7_9cd6,
    0x0000_0000_0000_0000,
    0x1000_0000_0000_0000,
];

#[inline]
fn geq(a: &[u64; 4], b: &[u64; 4]) -> bool {
    for i in (0..4).rev() {
        if a[i] != b[i] {
            return a[i] > b[i];
        }
    }
    true
}

#[inline]
fn sub_assign(a: &mut [u64; 4], b: &[u64; 4]) {
    let mut borrow = 0i128;
    for i in 0..4 {
        let t = a[i] as i128 - b[i] as i128 - borrow;
        a[i] = t as u64;
        borrow = if t < 0 { 1 } else { 0 };
    }
}

/// Reduce an arbitrary-length little-endian integer modulo ℓ.
///
/// Plain binary long division: walk the bits from the top, shifting the
/// remainder left and conditionally subtracting ℓ. Barrett reduction would be
/// faster, but this is called at most three times per signature and it is
/// obviously correct, which matters more here.
pub fn reduce_bytes(bytes_le: &[u8]) -> [u64; 4] {
    let mut r = [0u64; 4];
    for i in (0..bytes_le.len() * 8).rev() {
        // r <<= 1
        let mut carry = 0u64;
        for limb in r.iter_mut() {
            let next = *limb >> 63;
            *limb = (*limb << 1) | carry;
            carry = next;
        }
        // r remains < 2^253 after each conditional subtraction, so the shift
        // above can never carry out of the top limb.
        debug_assert_eq!(carry, 0);

        r[0] |= ((bytes_le[i / 8] >> (i % 8)) & 1) as u64;
        if geq(&r, &L) {
            sub_assign(&mut r, &L);
        }
    }
    r
}

pub fn reduce_limbs(limbs: &[u64]) -> [u64; 4] {
    let mut bytes = Vec::with_capacity(limbs.len() * 8);
    for l in limbs {
        bytes.extend_from_slice(&l.to_le_bytes());
    }
    reduce_bytes(&bytes)
}

/// (a * b + c) mod ℓ.
pub fn mul_add(a: &[u64; 4], b: &[u64; 4], c: &[u64; 4]) -> [u64; 4] {
    // Full 256x256 -> 512 product, then reduce.
    let mut w = [0u64; 8];
    for i in 0..4 {
        let mut carry = 0u64;
        for j in 0..4 {
            let t = (a[i] as u128) * (b[j] as u128) + w[i + j] as u128 + carry as u128;
            w[i + j] = t as u64;
            carry = (t >> 64) as u64;
        }
        w[i + 4] = carry;
    }
    let mut r = reduce_limbs(&w);

    // Add c, then one conditional subtraction: both operands are < ℓ < 2^253,
    // so the sum cannot overflow the four limbs.
    let mut carry = 0u128;
    for i in 0..4 {
        let t = r[i] as u128 + c[i] as u128 + carry;
        r[i] = t as u64;
        carry = t >> 64;
    }
    debug_assert_eq!(carry, 0);
    if geq(&r, &L) {
        sub_assign(&mut r, &L);
    }
    r
}

pub fn to_bytes(s: &[u64; 4]) -> [u8; 32] {
    let mut out = [0u8; 32];
    for i in 0..4 {
        out[i * 8..i * 8 + 8].copy_from_slice(&s[i].to_le_bytes());
    }
    out
}

pub fn from_bytes(b: &[u8; 32]) -> [u64; 4] {
    let mut limbs = [0u64; 4];
    for i in 0..4 {
        limbs[i] = u64::from_le_bytes(b[i * 8..i * 8 + 8].try_into().unwrap());
    }
    limbs
}

/// Is this 32-byte scalar already reduced, i.e. strictly less than ℓ?
///
/// Verification MUST enforce this on the `S` half of a signature. Without the
/// check, `S` and `S + ℓ` are both accepted, so anyone can take a valid
/// signature and produce a second, different, equally valid one — signature
/// malleability. It does not let an attacker forge a signature on a *new*
/// message, but it does mean signature bytes are not a unique identifier for
/// what was signed. Never key a map on a signature.
pub fn is_canonical(b: &[u8; 32]) -> bool {
    let s = from_bytes(b);
    !geq(&s, &L)
}
