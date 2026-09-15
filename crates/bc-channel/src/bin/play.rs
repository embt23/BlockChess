//! Milestone D, as an executable sentence: *a full game played off-chain,
//! signed at every ply, and settled cooperatively.*
//!
//! Two players with real Ed25519 keys play Morphy's Opera Game (Paris, 1858)
//! over the channel protocol. Every ply is signed by the mover and
//! countersigned by the opponent. The ledger sees exactly two transactions —
//! `OpenGame` and `CloseGame` — for a game of 33 plies and 66 signatures.
//!
//! ```sh
//! cargo run --release --bin play
//! ```

use bc_channel::game::Channel;
use bc_channel::offer::{GameOffer, GameTerms};
use bc_channel::state::pos_hash;
use bc_channel::{Ledger, Status};
use bc_chess::{Color, Position};
use bc_sig::SigningKey;

/// Morphy — Duke of Brunswick & Count Isouard, Paris 1858. Seventeen moves,
/// and the last one is mate. Shared with the integration test so the two can
/// never drift apart.
fn opera_game() -> Vec<&'static str> {
    include_str!("../../games/opera-1858.txt")
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .flat_map(|l| l.split_whitespace())
        .collect()
}

/// Think time per ply, in milliseconds. Deterministic so the run is
/// reproducible: a little longer in the middlegame, as a human would be.
fn think_ms(ply: usize) -> u32 {
    2_000 + (ply as u32 % 7) * 1_500
}

fn main() {
    let white_sk = SigningKey::from_seed(&[1u8; 32]);
    let black_sk = SigningKey::from_seed(&[2u8; 32]);
    let server_sk = SigningKey::from_seed(&[3u8; 32]);
    let (white_pk, black_pk, server_pk) = (
        white_sk.verifying_key(),
        black_sk.verifying_key(),
        server_sk.verifying_key(),
    );

    let mut ledger = Ledger::new();
    ledger.credit(&white_pk, 1_000);
    ledger.credit(&black_pk, 1_000);
    let supply_before = ledger.total_supply();

    let start = Position::startpos();
    let offer = GameOffer {
        white_pk,
        black_pk,
        stake_white: 100,
        stake_black: 100,
        terms: GameTerms::for_clock(pos_hash(&start), 180_000, 2_000),
        server_pk,
        open_nonce: [7u8; 32],
        rake_bps: 200,
        expiry_block: 100,
    };

    // Both players sign the offer; the server assembles the transaction. It
    // never touches the money — the stakes move under the players' own
    // signatures and nobody else's.
    let accept_w = white_sk.sign(&offer.signing_bytes());
    let accept_b = black_sk.sign(&offer.signing_bytes());
    let channel_id = ledger
        .open_game(&offer, &accept_w, &accept_b)
        .expect("open");

    println!("BlockChess — episode 07, the channel\n");
    println!("  channel  {}", hex(&channel_id));
    println!("  stakes   {} / {}", offer.stake_white, offer.stake_black);
    println!(
        "  time     {}s + {}s, rake {}bps\n",
        offer.terms.base_time_ms / 1000,
        offer.terms.increment_ms / 1000,
        offer.rake_bps
    );

    let mut w = Channel::open(offer, white_sk, Color::White, start);
    let mut b = Channel::open(offer, black_sk, Color::Black, start);

    let opera = opera_game();
    for (i, uci) in opera.iter().enumerate() {
        let (mover, other) = if i % 2 == 0 {
            (&mut w, &mut b)
        } else {
            (&mut b, &mut w)
        };
        let mv = mover
            .position()
            .move_from_uci(uci)
            .unwrap_or_else(|| panic!("ply {}: {uci} is not legal", i + 1));

        let elapsed = think_ms(i);
        let msg = mover.play(mv, elapsed).expect("play");
        // The receiver measures its own elapsed time; here it sees the think
        // time plus a little latency, which is exactly what the grace absorbs.
        other.receive(&msg, elapsed + 120).expect("receive");

        if (i + 1) % 2 == 0 || msg.state.status.is_terminal() {
            println!(
                "  {:>3}. {:<6} ply {:>2}  clocks {:>6}ms / {:<6}ms  {}",
                i / 2 + 1,
                uci,
                msg.state.ply,
                msg.state.clock_w_ms,
                msg.state.clock_b_ms,
                hex(&msg.state.hash())
            );
        }
    }

    // The mating move was asserted by White and countersigned by Black, who
    // checked it and had no legal reply. Two signatures on one state is the
    // whole of the evidence the chain will ever see about this game.
    let final_state = b.countersign().expect("countersign the mate");
    assert_eq!(final_state.state.status, Status::WhiteWins);
    assert!(final_state.is_certified());

    println!("\n  final    {:?}", final_state.state.status);
    println!("  plies    {}", final_state.state.ply);
    println!("  position {}\n", w.position().to_fen());

    let payout = ledger.close_game(&final_state).expect("close");
    println!(
        "  payout   white {} · black {} · rake {}",
        payout.white, payout.black, payout.rake
    );
    println!(
        "  balances white {} · black {} · server {}",
        ledger.balance(&white_pk),
        ledger.balance(&black_pk),
        ledger.balance(&server_pk)
    );

    assert_eq!(
        ledger.total_supply(),
        supply_before,
        "settlement must conserve: nothing is minted for playing (E1)"
    );
    println!("\n  chain saw 2 transactions for {} plies.", opera.len());
    println!("  supply unchanged at {supply_before}.");
    let _ = channel_id;
}

fn hex(h: &[u8; 32]) -> String {
    h[..6].iter().map(|b| format!("{b:02x}")).collect()
}
