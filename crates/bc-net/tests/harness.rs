//! The network's own properties. Everything episode 06 concludes rests on
//! these, so they are checked before anything is concluded.

use bc_net::net::Latency;
use bc_net::{Network, Partition};

fn run(seed: u64) -> Vec<(u64, usize, usize, u32)> {
    let mut n: Network<u32> = Network::new(4, seed);
    for round in 0..5u32 {
        for from in 0..4 {
            n.broadcast(from, round * 10 + from as u32);
        }
    }
    n.drain(1_000)
        .into_iter()
        .map(|e| (e.at, e.from, e.to, e.msg))
        .collect()
}

#[test]
fn the_same_seed_delivers_the_same_messages_in_the_same_order() {
    assert_eq!(run(42), run(42));
}

#[test]
fn a_different_seed_gives_a_different_interleaving() {
    assert_ne!(run(42), run(43));
}

/// Messages must arrive out of order, or the harness is testing a network
/// nobody has to write a consensus protocol for.
#[test]
fn messages_between_the_same_pair_can_overtake_each_other() {
    let mut n: Network<u32> = Network::new(2, 7);
    for i in 0..200 {
        n.send(0, 1, i);
    }
    let order: Vec<u32> = n.drain(1_000).into_iter().map(|e| e.msg).collect();
    let sorted: Vec<u32> = (0..200).collect();
    assert_ne!(order, sorted, "nothing ever overtook anything");
    assert_eq!(order.len(), 200, "nothing was lost either");
}

/// Delivery time never goes backwards, whatever the interleaving. A
/// protocol that reasons about timeouts needs this to be true.
#[test]
fn logical_time_is_monotone() {
    let mut n: Network<u32> = Network::new(5, 11);
    for from in 0..5 {
        n.broadcast(from, from as u32);
    }
    let mut last = 0;
    while let Some(e) = n.step() {
        assert!(e.at >= last, "{} < {last}", e.at);
        assert_eq!(n.now(), e.at);
        last = e.at;
    }
}

#[test]
fn a_partition_blocks_exactly_the_pairs_it_names() {
    let p = Partition::new(vec![vec![0, 1], vec![2, 3]]);
    assert!(p.permits(0, 1));
    assert!(p.permits(3, 2));
    assert!(!p.permits(1, 2));
    assert!(!p.permits(0, 3));
    assert!(p.is_split());

    let open = Partition::none();
    assert!(open.permits(0, 3));
    assert!(!open.is_split());
}

#[test]
fn a_split_network_delivers_only_within_groups() {
    let mut n: Network<u32> = Network::new(4, 5);
    n.split(vec![vec![0, 1], vec![2, 3]]);
    n.broadcast(0, 99);
    let got = n.drain(100);
    assert_eq!(got.len(), 1, "only node 1 should hear node 0");
    assert_eq!(got[0].to, 1);
    assert_eq!(n.dropped, 2);
}

/// Healing restores reachability, and messages sent during the split are
/// not retroactively delivered — they were dropped when they were sent.
#[test]
fn healing_restores_delivery_but_does_not_resend_what_was_dropped() {
    let mut n: Network<u32> = Network::new(4, 5);
    n.split(vec![vec![0, 1], vec![2, 3]]);
    n.broadcast(0, 1);
    n.drain(100);
    let lost = n.dropped;
    assert_eq!(lost, 2);

    n.heal();
    n.broadcast(0, 2);
    let got = n.drain(100);
    assert_eq!(got.len(), 3);
    assert_eq!(n.dropped, lost, "healing invented no new deliveries");
}

/// A node in no group hears nothing and is heard by nobody. That is a crash,
/// and it is deliberately the same mechanism as a partition — FLP's point is
/// that from inside the system they are indistinguishable.
#[test]
fn a_node_in_no_group_is_indistinguishable_from_a_crashed_one() {
    let mut n: Network<u32> = Network::new(3, 5);
    n.split(vec![vec![0, 1]]);
    n.broadcast(0, 1);
    n.broadcast(2, 2);
    let got = n.drain(100);
    assert_eq!(got.len(), 1);
    assert_eq!((got[0].from, got[0].to), (0, 1));
}

#[test]
fn a_fixed_latency_preserves_send_order() {
    let mut n: Network<u32> = Network::new(2, 1).with_latency(Latency {
        min_ms: 10,
        max_ms: 10,
    });
    for i in 0..50 {
        n.send(0, 1, i);
    }
    let order: Vec<u32> = n.drain(100).into_iter().map(|e| e.msg).collect();
    assert_eq!(order, (0..50).collect::<Vec<_>>());
}

/// A protocol that gossips on receipt keeps the queue non-empty forever.
/// `drain` must stop rather than hang.
#[test]
fn drain_respects_its_budget() {
    let mut n: Network<u32> = Network::new(3, 2);
    for _ in 0..100 {
        n.broadcast(0, 1);
    }
    assert_eq!(n.drain(10).len(), 10);
    assert!(!n.is_idle());
}
