//! SHA-256, implemented from FIPS 180-4.
//!
//! Episode 01. The whole point is the Merkle–Damgård construction: you have a
//! function that squashes a fixed 512 bits into 256, and you need to hash
//! messages of any length. So you pad the message to a multiple of 512 bits,
//! chop it into blocks, and chain the output of each block in as the input to
//! the next. The final chaining value is the digest.
//!
//! The padding is `0x80`, then zeros, then the *bit* length as a big-endian
//! u64. Encoding the length is not decoration — without it, `H("a")` and
//! `H("a\x80\x00...")` would collide, because they would produce the same
//! padded block.

use crate::consts::{H0_256, K256};

const BLOCK: usize = 64;

#[derive(Clone)]
pub struct Sha256 {
    state: [u32; 8],
    buf: [u8; BLOCK],
    buflen: usize,
    /// Total message length in *bytes*; multiplied by 8 at finalisation.
    total: u64,
}

impl Default for Sha256 {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha256 {
    pub fn new() -> Self {
        Sha256 {
            state: H0_256,
            buf: [0u8; BLOCK],
            buflen: 0,
            total: 0,
        }
    }

    pub fn update(&mut self, mut data: &[u8]) {
        self.total = self.total.wrapping_add(data.len() as u64);

        // Top up a partial buffer first.
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

        // Then whole blocks straight out of the input.
        while data.len() >= BLOCK {
            let (block, rest) = data.split_at(BLOCK);
            self.compress(block.try_into().unwrap());
            data = rest;
        }

        // Keep the remainder for next time.
        self.buf[..data.len()].copy_from_slice(data);
        self.buflen = data.len();
    }

    pub fn finalize(mut self) -> [u8; 32] {
        let bitlen = self.total.wrapping_mul(8);

        self.update_raw(&[0x80]);
        // Pad with zeros until there are exactly 8 bytes left in the block.
        while self.buflen != BLOCK - 8 {
            self.update_raw(&[0x00]);
        }
        self.update_raw(&bitlen.to_be_bytes());
        debug_assert_eq!(self.buflen, 0);

        let mut out = [0u8; 32];
        for (i, w) in self.state.iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&w.to_be_bytes());
        }
        out
    }

    /// `update` without touching the length counter — padding must not extend
    /// the length it is encoding.
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
        // Message schedule: the 16 words of the block, expanded to 64.
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes(block[i * 4..i * 4 + 4].try_into().unwrap());
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K256[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
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

        // The feed-forward. This addition is what makes the compression
        // function one-way: without it, the round function is an invertible
        // permutation and you could run the whole thing backwards.
        for (s, v) in self.state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *s = s.wrapping_add(v);
        }
    }
}

pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(data);
    h.finalize()
}
