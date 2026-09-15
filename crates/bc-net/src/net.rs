//! The simulated network itself.
//!
//! Messages are placed in a queue ordered by delivery time, with a
//! monotonic sequence number as tie-break so two messages due at the same
//! millisecond always resolve the same way. Time only advances when a
//! message is delivered, so there is no clock to be flaky about.

use crate::rng::Rng;
use std::collections::BinaryHeap;

pub type NodeId = usize;

/// A message in flight.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Envelope<M> {
    /// Logical milliseconds at which it arrives.
    pub at: u64,
    /// Tie-break, and the reason two runs of the same seed agree.
    pub seq: u64,
    pub from: NodeId,
    pub to: NodeId,
    pub msg: M,
}

// The heap is a max-heap, so the ordering is reversed: earliest `at` first,
// then lowest `seq`.
impl<M: Eq> Ord for Envelope<M> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other
            .at
            .cmp(&self.at)
            .then_with(|| other.seq.cmp(&self.seq))
    }
}
impl<M: Eq> PartialOrd for Envelope<M> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// A split of the node set into groups that cannot talk to each other.
///
/// Nodes in the same group still communicate normally. A node in no group
/// is unreachable from everyone, which is how a crash is modelled.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Partition {
    groups: Vec<Vec<NodeId>>,
}

impl Partition {
    /// No partition: everyone reaches everyone.
    pub fn none() -> Partition {
        Partition { groups: Vec::new() }
    }

    pub fn new(groups: Vec<Vec<NodeId>>) -> Partition {
        Partition { groups }
    }

    pub fn is_split(&self) -> bool {
        !self.groups.is_empty()
    }

    /// Can `from` reach `to`?
    pub fn permits(&self, from: NodeId, to: NodeId) -> bool {
        if self.groups.is_empty() {
            return true;
        }
        self.groups
            .iter()
            .any(|g| g.contains(&from) && g.contains(&to))
    }
}

/// How long messages take.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Latency {
    pub min_ms: u64,
    pub max_ms: u64,
}

impl Default for Latency {
    fn default() -> Latency {
        // Wide enough that messages routinely arrive out of send order,
        // which is the case consensus protocols are actually written for
        // and the case a fixed delay would never produce.
        Latency {
            min_ms: 5,
            max_ms: 60,
        }
    }
}

pub struct Network<M> {
    now: u64,
    seq: u64,
    queue: BinaryHeap<Envelope<M>>,
    partition: Partition,
    latency: Latency,
    rng: Rng,
    nodes: usize,
    pub delivered: u64,
    pub dropped: u64,
}

impl<M: Clone + Eq> Network<M> {
    pub fn new(nodes: usize, seed: u64) -> Network<M> {
        Network {
            now: 0,
            seq: 0,
            queue: BinaryHeap::new(),
            partition: Partition::none(),
            latency: Latency::default(),
            rng: Rng::new(seed),
            nodes,
            delivered: 0,
            dropped: 0,
        }
    }

    pub fn with_latency(mut self, latency: Latency) -> Network<M> {
        self.latency = latency;
        self
    }

    pub fn now(&self) -> u64 {
        self.now
    }

    pub fn nodes(&self) -> usize {
        self.nodes
    }

    pub fn in_flight(&self) -> usize {
        self.queue.len()
    }

    pub fn is_idle(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn partition(&self) -> &Partition {
        &self.partition
    }

    /// Split the network. Messages already in flight across the new split
    /// are **not** recalled — they were sent before it happened, and a
    /// partition that retroactively unsends things is not one.
    pub fn split(&mut self, groups: Vec<Vec<NodeId>>) {
        self.partition = Partition::new(groups);
    }

    pub fn heal(&mut self) {
        self.partition = Partition::none();
    }

    /// Send one message. Dropped silently if the partition forbids it —
    /// which is what a partition looks like from inside a node.
    pub fn send(&mut self, from: NodeId, to: NodeId, msg: M) {
        if !self.partition.permits(from, to) {
            self.dropped += 1;
            return;
        }
        let delay = self.rng.between(self.latency.min_ms, self.latency.max_ms);
        self.seq += 1;
        self.queue.push(Envelope {
            at: self.now + delay,
            seq: self.seq,
            from,
            to,
            msg,
        });
    }

    /// Send to everyone except the sender. Each recipient draws its own
    /// delay, so a broadcast is not an atomic event — which is the single
    /// most important thing about a real one.
    pub fn broadcast(&mut self, from: NodeId, msg: M) {
        for to in 0..self.nodes {
            if to != from {
                self.send(from, to, msg.clone());
            }
        }
    }

    /// Deliver the next message, advancing logical time to its arrival.
    pub fn step(&mut self) -> Option<Envelope<M>> {
        let e = self.queue.pop()?;
        self.now = self.now.max(e.at);
        self.delivered += 1;
        Some(e)
    }

    /// Deliver everything currently in flight, and anything sent as a
    /// result, up to `budget` messages. Returns what was delivered.
    ///
    /// The budget is not a nicety: a protocol that gossips on receipt can
    /// keep the queue non-empty forever, and a test that hangs is worse
    /// than a test that fails.
    pub fn drain(&mut self, budget: usize) -> Vec<Envelope<M>> {
        let mut out = Vec::new();
        while out.len() < budget {
            match self.step() {
                Some(e) => out.push(e),
                None => break,
            }
        }
        out
    }
}
