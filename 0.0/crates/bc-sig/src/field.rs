//! Arithmetic in F_p, p = 2^255 − 19.
//!
//! Elements are four 64-bit limbs, little-endian. Intermediate values are
//! allowed to sit anywhere in [0, 2^256); we only canonicalise when encoding or
//! comparing.
//!
//! The whole reduction strategy rests on one line of arithmetic:
//!
//! ```text
//!   2^256  =  2·2^255  =  2·(p + 19)  ≡  38   (mod p)
//! ```
//!
//! So any bits that overflow past 256 can be folded back in by multiplying them
//! by 38 and adding. That is why this prime was chosen: the reduction is a
//! multiply by a small constant rather than a division.

pub const P: [u64; 4] = [
    0xFFFF_FFFF_FFFF_FFED,
    0xFFFF_FFFF_FFFF_FFFF,
    0xFFFF_FFFF_FFFF_FFFF,
    0x7FFF_FFFF_FFFF_FFFF,
];

#[derive(Clone, Copy, Debug)]
#[allow(clippy::needless_range_loop)] // limb index is meaningful
pub struct Fe(pub [u64; 4]);

pub const ZERO: Fe = Fe([0, 0, 0, 0]);
pub const ONE: Fe = Fe([1, 0, 0, 0]);

/// Add `38 * top` into `x`, propagating carries. `top` is the number of times
/// 2^256 overflowed, which is always tiny.
#[inline]
fn fold38(x: &mut [u64; 4], top: u128) {
    let mut carry = 38u128 * top;
    for limb in x.iter_mut() {
        let t = *limb as u128 + carry;
        *limb = t as u64;
        carry = t >> 64;
    }
    // At most one more round: carry is 0 or 1 here.
    if carry != 0 {
        let mut c = 38u128;
        for limb in x.iter_mut() {
            let t = *limb as u128 + c;
            *limb = t as u64;
            c = t >> 64;
            if c == 0 {
                break;
            }
        }
    }
}

// Several loops below index two operands and the output by limb position.
// Clippy suggests zipping; the explicit index is what makes the schoolbook
// arithmetic legible, so keep it.
#[allow(clippy::needless_range_loop)]
impl Fe {
    pub fn add(self, o: Fe) -> Fe {
        let mut r = [0u64; 4];
        let mut carry = 0u128;
        for i in 0..4 {
            let t = self.0[i] as u128 + o.0[i] as u128 + carry;
            r[i] = t as u64;
            carry = t >> 64;
        }
        fold38(&mut r, carry);
        Fe(r)
    }

    pub fn sub(self, o: Fe) -> Fe {
        let mut r = [0u64; 4];
        let mut borrow = 0i128;
        for i in 0..4 {
            let t = self.0[i] as i128 - o.0[i] as i128 - borrow;
            r[i] = t as u64;
            borrow = if t < 0 { 1 } else { 0 };
        }
        // A borrow out of the top limb means we implicitly added 2^256, which
        // is 38 mod p, so take 38 back off.
        //
        // Subtracting 38 can itself borrow, when the raw value sitting in `r`
        // is below 38 — and then the result is wrong by another 2^256 and needs
        // a second pass. Stopping after one pass is a real bug that survives
        // almost every test you would think to write: it fires only for the
        // handful of representatives below 38, still returns a value that
        // *looks* fine, and even keeps elliptic-curve points on the curve.
        // See `field_sub_handles_repeated_borrow`.
        while borrow != 0 {
            let mut take = 38i128;
            for limb in r.iter_mut() {
                let t = *limb as i128 - take;
                *limb = t as u64;
                take = if t < 0 { 1 } else { 0 };
            }
            borrow = take;
        }
        Fe(r)
    }

    pub fn neg(self) -> Fe {
        ZERO.sub(self)
    }

    pub fn mul(self, o: Fe) -> Fe {
        // Schoolbook 4x4 -> 8 limbs.
        let mut w = [0u64; 8];
        for i in 0..4 {
            let mut carry = 0u64;
            for j in 0..4 {
                let t = (self.0[i] as u128) * (o.0[j] as u128) + w[i + j] as u128 + carry as u128;
                w[i + j] = t as u64;
                carry = (t >> 64) as u64;
            }
            // w[i+4] has not been written yet by this or any earlier round.
            w[i + 4] = carry;
        }

        // Fold the high half back in: result = lo + 38 * hi.
        let mut acc = [0u64; 5];
        let mut carry = 0u64;
        for i in 0..4 {
            let t = 38u128 * (w[4 + i] as u128) + carry as u128;
            acc[i] = t as u64;
            carry = (t >> 64) as u64;
        }
        acc[4] = carry;

        let mut r = [0u64; 4];
        let mut c = 0u128;
        for i in 0..4 {
            let t = w[i] as u128 + acc[i] as u128 + c;
            r[i] = t as u64;
            c = t >> 64;
        }
        fold38(&mut r, c + acc[4] as u128);
        Fe(r)
    }

    pub fn sq(self) -> Fe {
        self.mul(self)
    }

    /// Square-and-multiply over the bits of `e` (little-endian limbs).
    pub fn pow(self, e: &[u64; 4]) -> Fe {
        let mut result = ONE;
        let mut base = self;
        for limb in e.iter() {
            let mut bits = *limb;
            for _ in 0..64 {
                if bits & 1 == 1 {
                    result = result.mul(base);
                }
                base = base.sq();
                bits >>= 1;
            }
        }
        result
    }

    /// Multiplicative inverse via Fermat: a^(p−2) ≡ a^(−1) (mod p).
    ///
    /// Slower than the extended Euclidean algorithm but branch-free on the
    /// input, which is what you want when the input is secret.
    pub fn invert(self) -> Fe {
        // p − 2 = 2^255 − 21
        const P_MINUS_2: [u64; 4] = [
            0xFFFF_FFFF_FFFF_FFEB,
            0xFFFF_FFFF_FFFF_FFFF,
            0xFFFF_FFFF_FFFF_FFFF,
            0x7FFF_FFFF_FFFF_FFFF,
        ];
        self.pow(&P_MINUS_2)
    }

    /// Fully reduce into [0, p).
    pub fn canonical(self) -> [u64; 4] {
        let mut t = self.0;

        // Fold anything at or above bit 255 back in: 2^255 ≡ 19.
        for _ in 0..2 {
            let over = (t[3] >> 63) as u128;
            t[3] &= 0x7FFF_FFFF_FFFF_FFFF;
            let mut carry = 19u128 * over;
            for limb in t.iter_mut() {
                let x = *limb as u128 + carry;
                *limb = x as u64;
                carry = x >> 64;
            }
        }

        // Now t < 2^255. It may still be in [p, 2^255): test by adding 19 and
        // seeing whether bit 255 appears.
        let mut s = t;
        let mut c = 19u128;
        for limb in s.iter_mut() {
            let x = *limb as u128 + c;
            *limb = x as u64;
            c = x >> 64;
        }
        if s[3] >> 63 == 1 {
            s[3] &= 0x7FFF_FFFF_FFFF_FFFF;
            t = s;
        }
        t
    }

    pub fn is_zero(self) -> bool {
        self.canonical() == [0, 0, 0, 0]
    }

    pub fn eq(self, o: Fe) -> bool {
        self.canonical() == o.canonical()
    }

    /// Least significant bit of the canonical representative — the "sign" used
    /// in point compression.
    pub fn parity(self) -> u8 {
        (self.canonical()[0] & 1) as u8
    }

    pub fn to_bytes(self) -> [u8; 32] {
        let c = self.canonical();
        let mut out = [0u8; 32];
        for i in 0..4 {
            out[i * 8..i * 8 + 8].copy_from_slice(&c[i].to_le_bytes());
        }
        out
    }

    /// Read 32 little-endian bytes. The top bit is ignored (it carries the sign
    /// in point encoding, not field data).
    pub fn from_bytes(b: &[u8; 32]) -> Fe {
        let mut limbs = [0u64; 4];
        for i in 0..4 {
            limbs[i] = u64::from_le_bytes(b[i * 8..i * 8 + 8].try_into().unwrap());
        }
        limbs[3] &= 0x7FFF_FFFF_FFFF_FFFF;
        Fe(limbs)
    }

    pub fn from_u64(v: u64) -> Fe {
        Fe([v, 0, 0, 0])
    }

    /// Is the 32-byte encoding already canonical, i.e. strictly less than p?
    pub fn bytes_are_canonical(b: &[u8; 32]) -> bool {
        let mut limbs = [0u64; 4];
        for i in 0..4 {
            limbs[i] = u64::from_le_bytes(b[i * 8..i * 8 + 8].try_into().unwrap());
        }
        limbs[3] &= 0x7FFF_FFFF_FFFF_FFFF;
        for i in (0..4).rev() {
            if limbs[i] < P[i] {
                return true;
            }
            if limbs[i] > P[i] {
                return false;
            }
        }
        false // exactly p
    }
}
