//! D25 — what an `adjudicator_ver` actually is.
//!
//! `D11` commits to keeping every adjudicator version forever, and never said
//! what a version *is*. Both obvious answers are wrong on their own:
//!
//! - **A bare integer** is a promise with nothing behind it. Two builds can
//!   both claim version 3 and disagree about en passant, and the channel that
//!   trusted the number cannot tell.
//! - **A bare content hash** is self-identifying but illegible on the wire,
//!   unwritable in a spec before the build exists, and makes reproducible
//!   builds load-bearing for consensus.
//!
//! So: both. Channels negotiate a small integer; consensus holds a registry
//! mapping each integer to the hash of the ruleset it denotes. Readable where
//! humans read it, pinned where money depends on it.
//!
//! ## Why this makes the governance question smaller
//!
//! The registry is the **only** object `D11`'s upgrade authority controls, so
//! "who controls upgrades" reduces to "who may add a row to one table". And
//! because old rows are never removed, an upgrade is purely *additive*: a
//! channel pinned to version 3 is adjudicated by version 3 no matter what is
//! registered afterwards. Whoever holds the key cannot reach into an open
//! channel, cannot change who won a finished game, and cannot strand money.
//! The realistic abuse is refusal — declining to register someone else's fix —
//! which is a much smaller thing to guard against.

use alloc::collections::BTreeMap;
use bc_hash::{tagged, Hash};

/// The hash of a ruleset: what the integer on the wire actually denotes.
///
/// Domain-separated like everything else, so a ruleset hash can never be
/// mistaken for a state hash or a position hash (`spec/01`).
pub fn ruleset_hash(descriptor: &[u8]) -> Hash {
    tagged("BC/ruleset/v1", descriptor)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryError {
    /// The version is not registered, so nothing can be adjudicated under it.
    Unknown(u16),
    /// A registered version was given a different hash. Versions are
    /// immutable; this is the whole point of the registry.
    Immutable(u16),
    /// Versions are added in order, so that "highest known" is meaningful.
    OutOfOrder(u16),
}

impl core::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}
#[cfg(feature = "std")]
impl std::error::Error for RegistryError {}

/// Consensus state: version → the ruleset it denotes.
///
/// Append-only by construction. There is no `remove`, and there is no way to
/// overwrite a row, because a channel signed under version 3 has to still
/// mean version 3 in a decade.
#[derive(Debug, Clone, Default)]
pub struct RulesetRegistry {
    rows: BTreeMap<u16, Hash>,
}

impl RulesetRegistry {
    pub fn new() -> RulesetRegistry {
        RulesetRegistry::default()
    }

    /// Register a version. The only write the upgrade authority may make.
    pub fn register(&mut self, ver: u16, hash: Hash) -> Result<(), RegistryError> {
        match self.rows.get(&ver) {
            Some(existing) if *existing == hash => return Ok(()), // idempotent
            Some(_) => return Err(RegistryError::Immutable(ver)),
            None => {}
        }
        if self.rows.keys().next_back().is_some_and(|&hi| ver <= hi) {
            return Err(RegistryError::OutOfOrder(ver));
        }
        self.rows.insert(ver, hash);
        Ok(())
    }

    pub fn get(&self, ver: u16) -> Result<Hash, RegistryError> {
        self.rows
            .get(&ver)
            .copied()
            .ok_or(RegistryError::Unknown(ver))
    }

    /// Does this version denote the ruleset this node is running?
    ///
    /// A node that answers `false` must refuse to adjudicate the channel
    /// rather than guess — disagreeing about the rules with money on it is
    /// exactly the consensus split `P5` exists to prevent.
    pub fn agrees(&self, ver: u16, local: &Hash) -> bool {
        self.get(ver).is_ok_and(|h| h == *local)
    }

    pub fn highest(&self) -> Option<u16> {
        self.rows.keys().next_back().copied()
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(tag: &str) -> Hash {
        ruleset_hash(tag.as_bytes())
    }

    #[test]
    fn a_version_means_one_ruleset_forever() {
        let mut r = RulesetRegistry::new();
        r.register(1, h("v1")).unwrap();
        // Re-registering the same thing is fine; changing it is not.
        r.register(1, h("v1")).unwrap();
        assert_eq!(
            r.register(1, h("v1-but-different")),
            Err(RegistryError::Immutable(1))
        );
        assert_eq!(r.get(1).unwrap(), h("v1"));
    }

    #[test]
    fn upgrades_are_additive_so_an_open_channel_cannot_be_reached_into() {
        let mut r = RulesetRegistry::new();
        r.register(1, h("v1")).unwrap();
        let pinned = r.get(1).unwrap();

        // Whatever the authority does next…
        r.register(2, h("v2")).unwrap();
        r.register(3, h("v3")).unwrap();

        // …a channel pinned to 1 still resolves to exactly what it signed.
        assert_eq!(r.get(1).unwrap(), pinned);
        assert_eq!(r.highest(), Some(3));
    }

    #[test]
    fn an_unregistered_version_is_refused_rather_than_guessed() {
        let r = RulesetRegistry::new();
        assert_eq!(r.get(7), Err(RegistryError::Unknown(7)));
        assert!(!r.agrees(7, &h("whatever")));
    }

    #[test]
    fn a_node_running_different_rules_does_not_agree() {
        let mut r = RulesetRegistry::new();
        r.register(1, h("en-passant-as-written")).unwrap();
        assert!(r.agrees(1, &h("en-passant-as-written")));
        // Same integer, different rules. This is the case a bare version
        // number cannot detect, and the reason the hash exists.
        assert!(!r.agrees(1, &h("en-passant-but-subtly-wrong")));
    }

    #[test]
    fn versions_are_added_in_order() {
        let mut r = RulesetRegistry::new();
        r.register(2, h("v2")).unwrap();
        assert_eq!(r.register(1, h("v1")), Err(RegistryError::OutOfOrder(1)));
    }

    #[test]
    fn a_ruleset_hash_is_domain_separated() {
        // Must not collide with any other hash in the protocol over the same
        // bytes — `spec/01`'s whole argument.
        assert_ne!(ruleset_hash(b"x"), bc_hash::tagged("BC/state/v1", b"x"));
        assert_ne!(ruleset_hash(b"x"), bc_hash::tagged("BC/pos/v1", b"x"));
    }
}
