//! Points on the twisted Edwards curve
//!
//! ```text
//!   −x² + y²  =  1 + d·x²·y²        d = −121665/121666  (mod p)
//! ```
//!
//! Two things make this curve pleasant. First, the addition law is *complete*:
//! one formula works for every pair of points, including doubling and the
//! identity. Weierstrass curves need special cases for those, and special cases
//! are branches, and branches leak timing. Second, the identity is the ordinary
//! point (0, 1) rather than a fictional "point at infinity", so there is no
//! exceptional value to represent.
//!
//! We work in *extended* coordinates (X : Y : Z : T) with x = X/Z, y = Y/Z and
//! x·y = T/Z. Carrying the redundant T lets addition avoid an inversion; the
//! single inversion is deferred to the moment we encode a point.

use crate::field::{Fe, ONE, ZERO};
use std::sync::LazyLock;

/// d = −121665 / 121666.
pub static D: LazyLock<Fe> = LazyLock::new(|| {
    Fe::from_u64(121665)
        .neg()
        .mul(Fe::from_u64(121666).invert())
});

/// A square root of −1, which exists because p ≡ 1 (mod 4). Needed when
/// decompressing a point, to pick between the two candidate roots.
pub static SQRT_M1: LazyLock<Fe> = LazyLock::new(|| {
    // 2^((p−1)/4)
    const EXP: [u64; 4] = [
        0xFFFF_FFFF_FFFF_FFFB,
        0xFFFF_FFFF_FFFF_FFFF,
        0xFFFF_FFFF_FFFF_FFFF,
        0x1FFF_FFFF_FFFF_FFFF,
    ];
    Fe::from_u64(2).pow(&EXP)
});

/// The standard base point: the point with y = 4/5 and even x.
pub static BASEPOINT: LazyLock<Point> = LazyLock::new(|| {
    let y = Fe::from_u64(4).mul(Fe::from_u64(5).invert());
    let mut enc = y.to_bytes();
    enc[31] &= 0x7F; // sign bit 0 => even x
    Point::decompress(&enc).expect("base point must decompress")
});

#[derive(Clone, Copy, Debug)]
pub struct Point {
    pub x: Fe,
    pub y: Fe,
    pub z: Fe,
    pub t: Fe,
}

pub const IDENTITY: Point = Point {
    x: ZERO,
    y: ONE,
    z: ONE,
    t: ZERO,
};

impl Point {
    /// add-2008-hwcd-3, for a = −1. Complete: no special cases.
    pub fn add(&self, o: &Point) -> Point {
        let a = self.y.sub(self.x).mul(o.y.sub(o.x));
        let b = self.y.add(self.x).mul(o.y.add(o.x));
        let c = self.t.mul(*D).mul(o.t).mul(Fe::from_u64(2));
        let d = self.z.mul(o.z).mul(Fe::from_u64(2));
        let e = b.sub(a);
        let f = d.sub(c);
        let g = d.add(c);
        let h = b.add(a);
        Point {
            x: e.mul(f),
            y: g.mul(h),
            t: e.mul(h),
            z: f.mul(g),
        }
    }

    /// dbl-2008-hwcd, for a = −1. Cheaper than `add` because it skips the
    /// multiplication by d.
    pub fn double(&self) -> Point {
        let a = self.x.sq();
        let b = self.y.sq();
        let c = self.z.sq().mul(Fe::from_u64(2));
        let d = a.neg();
        let e = self.x.add(self.y).sq().sub(a).sub(b);
        let g = d.add(b);
        let f = g.sub(c);
        let h = d.sub(b);
        Point {
            x: e.mul(f),
            y: g.mul(h),
            t: e.mul(h),
            z: f.mul(g),
        }
    }

    pub fn neg(&self) -> Point {
        Point {
            x: self.x.neg(),
            y: self.y,
            z: self.z,
            t: self.t.neg(),
        }
    }

    /// Scalar multiplication, double-and-add over the bits of `s`.
    ///
    /// **This is not constant time.** The `if bit == 1` below runs a different
    /// amount of work depending on a bit of the scalar, so an attacker who can
    /// measure timing can recover a private key. That is fine for learning and
    /// for verification (where the scalar is public), and unacceptable for
    /// production signing. See the crate docs.
    pub fn mul_scalar(&self, s: &[u64; 4]) -> Point {
        let mut result = IDENTITY;
        for i in (0..256).rev() {
            result = result.double();
            if (s[i / 64] >> (i % 64)) & 1 == 1 {
                result = result.add(self);
            }
        }
        result
    }

    /// Compress to 32 bytes: y in the low 255 bits, the parity of x in bit 255.
    ///
    /// Only one bit is needed for x because the curve equation determines x up
    /// to sign given y, and the two roots are x and p − x, which differ in
    /// parity (p is odd).
    pub fn compress(&self) -> [u8; 32] {
        let zi = self.z.invert();
        let x = self.x.mul(zi);
        let y = self.y.mul(zi);
        let mut out = y.to_bytes();
        out[31] |= x.parity() << 7;
        out
    }

    /// Recover a point from its 32-byte encoding, or `None` if the bytes do not
    /// encode a curve point.
    pub fn decompress(b: &[u8; 32]) -> Option<Point> {
        // A non-canonical y (>= p) must be rejected: otherwise a public key has
        // two valid encodings.
        if !Fe::bytes_are_canonical(b) {
            return None;
        }
        let sign = b[31] >> 7;
        let y = Fe::from_bytes(b);

        // From −x² + y² = 1 + d x² y²:   x² = (y² − 1) / (d y² + 1)
        let yy = y.sq();
        let u = yy.sub(ONE);
        let v = D.mul(yy).add(ONE);

        // Candidate root for p ≡ 5 (mod 8):
        //   x = u·v³·(u·v⁷)^((p−5)/8)
        const EXP: [u64; 4] = [
            0xFFFF_FFFF_FFFF_FFFD,
            0xFFFF_FFFF_FFFF_FFFF,
            0xFFFF_FFFF_FFFF_FFFF,
            0x0FFF_FFFF_FFFF_FFFF,
        ];
        let v3 = v.sq().mul(v);
        let v7 = v3.sq().mul(v);
        let mut x = u.mul(v3).mul(u.mul(v7).pow(&EXP));

        // Check v·x² = u; if instead v·x² = −u, multiply by sqrt(−1).
        let check = v.mul(x.sq());
        if !check.eq(u) {
            if check.eq(u.neg()) {
                x = x.mul(*SQRT_M1);
            } else {
                return None; // y is not on the curve
            }
        }

        // x = 0 with the sign bit set is the one encoding with no valid point.
        if x.is_zero() && sign == 1 {
            return None;
        }
        if x.parity() != sign {
            x = x.neg();
        }

        Some(Point {
            x,
            y,
            z: ONE,
            t: x.mul(y),
        })
    }

    pub fn eq(&self, o: &Point) -> bool {
        // Projective equality: compare cross-products rather than normalising.
        self.x.mul(o.z).eq(o.x.mul(self.z)) && self.y.mul(o.z).eq(o.y.mul(self.z))
    }
}
