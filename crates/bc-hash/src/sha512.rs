//! SHA-512, from FIPS 180-4.
//!
//! Structurally identical to SHA-256 with everything widened: 64-bit words,
//! 1024-bit blocks, 80 rounds, a 128-bit length field, and different rotation
//! amounts. Reading the two side by side is the clearest way to see that a
//! hash family is one design instantiated at several widths.
//!
//! We need this because Ed25519 (episode 02) specifies SHA-512 internally.

use crate::consts::{H0_512, K512};

const BLOCK: usize = 128;

#[derive(Clone)]
pub struct Sha512 {
    state: [u64; 8],
    buf: [u8; BLOCK],
    buflen: usize,
    total: u128,
}

impl Default for Sha512 {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha512 {
    pub fn new() -> Self {
        Sha512 {
            state: H0_512,
            buf: [0u8; BLOCK],
            buflen: 0,
            total: 0,
        }
    }

    pub fn update(&mut self, mut data: &[u8]) {
        self.total = self.total.wrapping_add(data.len() as u128);

        if self.buflen > 0 {
            let take = core::cmp::min(BLOCK - self.buflen, data.len());
            self.buf[self.buflen..self.buflen + take].copy_from_slice(&data[..take]);
            self.buflen += take;
            data = &data[take..];
            if self.buflen < BLOCK {
                // Still a partial block, and `data` is now empty. Return here:
                // falling through would reach the tail below, which sets
                // `buflen = data.len()` — i.e. zero — and silently discard
                // everything we just buffered.
                return;
            }
            let block = self.buf;
            self.compress(&block);
            self.buflen = 0;
        }

        while data.len() >= BLOCK {
            let (block, rest) = data.split_at(BLOCK);
            self.compress(block.try_into().unwrap());
            data = rest;
        }

        self.buf[..data.len()].copy_from_slice(data);
        self.buflen = data.len();
    }

    pub fn finalize(mut self) -> [u8; 64] {
        let bitlen = self.total.wrapping_mul(8);

        self.update_raw(&[0x80]);
        while self.buflen != BLOCK - 16 {
            self.update_raw(&[0x00]);
        }
        self.update_raw(&bitlen.to_be_bytes());
        debug_assert_eq!(self.buflen, 0);

        let mut out = [0u8; 64];
        for (i, w) in self.state.iter().enumerate() {
            out[i * 8..i * 8 + 8].copy_from_slice(&w.to_be_bytes());
        }
        out
    }

    fn update_raw(&mut self, data: &[u8]) {
        for &b in data {
            self.buf[self.buflen] = b;
            self.buflen += 1;
            if self.buflen == BLOCK {
                let block = self.buf;
                self.compress(&block);
                self.buflen = 0;
            }
        }
    }

    fn compress(&mut self, block: &[u8; BLOCK]) {
        let mut w = [0u64; 80];
        for i in 0..16 {
            w[i] = u64::from_be_bytes(block[i * 8..i * 8 + 8].try_into().unwrap());
        }
        for i in 16..80 {
            let s0 = w[i - 15].rotate_right(1) ^ w[i - 15].rotate_right(8) ^ (w[i - 15] >> 7);
            let s1 = w[i - 2].rotate_right(19) ^ w[i - 2].rotate_right(61) ^ (w[i - 2] >> 6);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;

        for i in 0..80 {
            let s1 = e.rotate_right(14) ^ e.rotate_right(18) ^ e.rotate_right(41);
            let ch = (e & f) ^ (!e & g);
            let t1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K512[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(28) ^ a.rotate_right(34) ^ a.rotate_right(39);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }

        for (s, v) in self.state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *s = s.wrapping_add(v);
        }
    }
}

pub fn sha512(data: &[u8]) -> [u8; 64] {
    let mut h = Sha512::new();
    h.update(data);
    h.finalize()
}
