//! A network you can argue with.
//!
//! Episode 06 is about what consensus does when the network misbehaves —
//! FLP, partial synchrony, the ⅓ bound. None of that can be *demonstrated*
//! over real sockets, because you cannot ask a real network to partition
//! validators 2-and-2 for exactly four hundred milliseconds and then heal.
//! You can ask this one, and it will do the same thing every time.
//!
//! ## Determinism is the whole feature
//!
//! Every choice — how long a message takes, which of two simultaneous
//! deliveries happens first — comes from a seeded generator and a total
//! order on envelopes. Two runs with the same seed deliver the same
//! messages in the same order at the same logical times. A consensus bug
//! that only shows up under one interleaving in ten thousand is then a bug
//! with a seed number attached, which is the difference between a bug you
//! can fix and a bug you can only complain about.
//!
//! There is no thread, no clock and no I/O anywhere in this crate. Time is
//! an integer that only moves when a message is delivered.
//!
//! ## What it deliberately does not do
//!
//! No sockets, no framing, no encryption, no Noise handshake. Those are the
//! `spec/04` P2P layer and a different episode. This crate answers exactly
//! one question — *who heard what, when* — because that is the only
//! question a consensus safety argument depends on.

pub mod net;
pub mod rng;

pub use net::{Envelope, Network, NodeId, Partition};
pub use rng::Rng;
