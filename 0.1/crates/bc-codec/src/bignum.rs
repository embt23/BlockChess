//! Just enough arbitrary-precision arithmetic for mixed-radix coding.
//!
//! Four operations, all of them a schoolbook loop over base-2^32 limbs:
//! multiply by a small number, add a small number, and the two halves of
//! dividing by a small number. Nothing here needs to be fast — a game is
//! eighty plies — and everything here needs to be obviously correct, so it is
//! written the obvious way.
//!
//! Limbs are little-endian: `limbs[0]` is least significant, and the
//! representation is canonical because trailing zero limbs are always
//! trimmed. That matters: `papers/02-encodings.md` requires the encoding of a
//! game to be unique, and two spellings of the same number would break it.

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Big {
    limbs: Vec<u32>,
}

impl Big {
    pub fn zero() -> Big {
        Big { limbs: Vec::new() }
    }

    pub fn is_zero(&self) -> bool {
        self.limbs.is_empty()
    }

    /// `self = self * m + a`, with `m >= 1`.
    pub fn mul_add(&mut self, m: u32, a: u32) {
        let mut carry = a as u64;
        for limb in self.limbs.iter_mut() {
            let v = (*limb as u64) * (m as u64) + carry;
            *limb = v as u32;
            carry = v >> 32;
        }
        while carry > 0 {
            self.limbs.push(carry as u32);
            carry >>= 32;
        }
        self.trim();
    }

    /// `self /= d`, returning the remainder. `d` must be non-zero.
    pub fn div_rem(&mut self, d: u32) -> u32 {
        debug_assert!(d != 0, "division by zero");
        let mut rem = 0u64;
        for limb in self.limbs.iter_mut().rev() {
            let cur = (rem << 32) | (*limb as u64);
            *limb = (cur / d as u64) as u32;
            rem = cur % d as u64;
        }
        self.trim();
        rem as u32
    }

    /// Little-endian bytes, canonical: no trailing zero byte.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.limbs.len() * 4);
        for limb in &self.limbs {
            out.extend_from_slice(&limb.to_le_bytes());
        }
        while out.last() == Some(&0) {
            out.pop();
        }
        out
    }

    pub fn from_bytes(bytes: &[u8]) -> Big {
        let mut limbs = Vec::with_capacity(bytes.len() / 4 + 1);
        for chunk in bytes.chunks(4) {
            let mut buf = [0u8; 4];
            buf[..chunk.len()].copy_from_slice(chunk);
            limbs.push(u32::from_le_bytes(buf));
        }
        let mut b = Big { limbs };
        b.trim();
        b
    }

    fn trim(&mut self) {
        while self.limbs.last() == Some(&0) {
            self.limbs.pop();
        }
    }
}
