//! Episode 01 — hashing, domain separation, and hash chains.
//!
//! The attack this defeats: *"I moved first, actually."* Without a way to bind
//! a sequence of events into an order that cannot be rewritten, nothing else
//! in the protocol works.

mod consts;
pub mod sha256;
pub mod sha512;

pub use sha256::{sha256, Sha256};
pub use sha512::{sha512, Sha512};

/// A 256-bit digest.
pub type Hash = [u8; 32];

pub const ZERO_HASH: Hash = [0u8; 32];

/// Domain-separated hash.
///
/// Never hash a raw concatenation. Two different protocol messages whose byte
/// encodings happen to coincide would otherwise produce the same digest, and a
/// signature over one becomes a valid signature over the other.
///
/// The length prefix on the tag matters just as much as the tag: without it,
/// `("ab", "c")` and `("a", "bc")` hash identically, so an attacker can shift
/// bytes across the boundary. Prefixing the length pins the split point.
///
/// See `spec/01-primitives.md` for the registry of tags.
pub fn tagged(tag: &str, data: &[u8]) -> Hash {
    let mut h = Sha256::new();
    h.update(&(tag.len() as u64).to_le_bytes());
    h.update(tag.as_bytes());
    h.update(data);
    h.finalize()
}

/// Domain-separated hash over several pieces, each length-prefixed.
///
/// Use this rather than concatenating yourself; the per-part lengths make the
/// encoding unambiguous, which is what "canonical" means in practice.
pub fn tagged_parts(tag: &str, parts: &[&[u8]]) -> Hash {
    let mut h = Sha256::new();
    h.update(&(tag.len() as u64).to_le_bytes());
    h.update(tag.as_bytes());
    for p in parts {
        h.update(&(p.len() as u64).to_le_bytes());
        h.update(p);
    }
    h.finalize()
}

/// An append-only chain of events, each committing to everything before it.
///
/// This is the structure a game channel uses (`spec/04-channel.md`): signing
/// state *n* is signing the whole game up to ply *n*, because state *n*'s hash
/// contains state *n−1*'s hash, which contains *n−2*'s, and so on down to the
/// opening position. One signature is a statement about all of history.
///
/// It is also, one layer down, exactly what makes a blockchain a chain. The
/// same idea appears at every scale of this system.
#[derive(Debug, Clone, Default)]
pub struct HashChain {
    head: Hash,
    len: u64,
}

impl HashChain {
    /// A chain rooted at the zero hash.
    pub fn new() -> Self {
        HashChain {
            head: ZERO_HASH,
            len: 0,
        }
    }

    /// A chain rooted at a specific genesis value.
    pub fn rooted(genesis: Hash) -> Self {
        HashChain {
            head: genesis,
            len: 0,
        }
    }

    /// Append an event. The new head commits to the old head.
    pub fn append(&mut self, event: &[u8]) -> Hash {
        self.head = tagged_parts("BC/chain/v1", &[&self.head, &self.len.to_le_bytes(), event]);
        self.len += 1;
        self.head
    }

    pub fn head(&self) -> Hash {
        self.head
    }

    pub fn len(&self) -> u64 {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Recompute a head from a genesis value and a sequence of events.
    ///
    /// This is the verification side: given the events someone claims happened,
    /// does the chain they produce match the head you already hold a signature
    /// over? If any event is altered, inserted, removed, or reordered, the head
    /// changes and the check fails.
    pub fn verify(genesis: Hash, events: &[&[u8]], claimed_head: Hash) -> bool {
        let mut c = HashChain::rooted(genesis);
        for e in events {
            c.append(e);
        }
        // Constant-time-ish: compare the whole digest, never early-exit on the
        // first differing byte. Not security-critical here, but the habit is.
        c.head()
            .iter()
            .zip(claimed_head.iter())
            .fold(0u8, |acc, (a, b)| acc | (a ^ b))
            == 0
    }
}

/// Render a digest as lowercase hex.
pub fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}
