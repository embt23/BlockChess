//! Milestone E, as an executable sentence: *you can win against an opponent
//! who disconnects.*
//!
//! This is the whole point of the project. Everything before it is a
//! prerequisite; everything after is expansion. Up to episode 07 the system
//! was merely convenient — it settled games that both players agreed about.
//! Here it settles one that only one player is still present for.
//!
//! ```sh
//! cargo run --release --bin dispute
//! ```

use bc_channel::clock::{budget_blocks, TAU_MS};
use bc_channel::game::Channel;
use bc_channel::offer::{GameOffer, GameTerms};
use bc_channel::state::pos_hash;
use bc_channel::{Ledger, Status};
use bc_chess::{Color, Position};
use bc_sig::SigningKey;

/// Two-second blocks, as `spec/05` assumes throughout.
const BLOCK_SECONDS: u64 = 2;

fn main() {
    let white_sk = SigningKey::from_seed(&[1u8; 32]);
    let black_sk = SigningKey::from_seed(&[2u8; 32]);
    let (white_pk, black_pk) = (white_sk.verifying_key(), black_sk.verifying_key());

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
        terms: GameTerms {
            start_pos_hash: pos_hash(&start),
            base_time_ms: 180_000,
            increment_ms: 2_000,
            max_plies: 600,
            delta_blocks: 256,
            budget_tau_ms: TAU_MS,
            rules_mask: 0xFF,
            adjudicator_ver: 1,
        },
        server_pk: bc_sig::VerifyingKey([0u8; 32]),
        open_nonce: [11u8; 32],
        rake_bps: 0,
        expiry_block: 100,
    };
    let id = ledger
        .open_game(
            &offer,
            &white_sk.sign(&offer.signing_bytes()),
            &black_sk.sign(&offer.signing_bytes()),
        )
        .expect("open");

    let mut w = Channel::open(offer, white_sk, Color::White, start);
    let mut b = Channel::open(offer, black_sk, Color::Black, start);

    println!("BlockChess — episode 08, the adjudicator\n");
    println!(
        "  stakes   {} each, Δ = {} blocks\n",
        offer.stake_white, offer.terms.delta_blocks
    );

    // --- a normal game, off-chain -----------------------------------------
    println!("  off-chain, both players present:");
    for (i, uci) in ["e2e4", "e7e5", "g1f3"].iter().enumerate() {
        let (mover, other) = if i % 2 == 0 {
            (&mut w, &mut b)
        } else {
            (&mut b, &mut w)
        };
        let mv = mover.position().move_from_uci(uci).unwrap();
        let msg = mover.play(mv, 4_000).expect("play");
        other.receive(&msg, 4_100).expect("receive");
        println!("    ply {}  {uci}", msg.state.ply);
    }

    // --- and then one of them stops ---------------------------------------
    println!("\n  Black stops answering. White waits, then goes to chain.\n");

    // White's evidence is the highest state *Black* signed. White never had
    // to ask for it: Black's countersignature rode along with Black's own
    // last move, so the player who is waiting always holds one.
    let (evidence, pos) = w.evidence();
    let packed = pos.pack();
    println!(
        "    evidence      state at ply {}, signed by Black",
        evidence.state.ply
    );
    println!("    position      {}", pos.to_fen());

    let mut height = 1_000;
    ledger
        .dispute_open(Color::White, &evidence, packed.as_slice(), height)
        .expect("open the dispute");

    let d = ledger.dispute(&id).unwrap();
    println!(
        "    budgets       white {} blocks · black {} blocks",
        d.budget_of(Color::White),
        d.budget_of(Color::Black)
    );
    println!(
        "                  ({} ms and {} ms of game clock, at τ = {} ms/block)",
        evidence.state.clock_w_ms, evidence.state.clock_b_ms, TAU_MS
    );
    debug_assert_eq!(
        d.budget_of(Color::White),
        budget_blocks(evidence.state.clock_w_ms, TAU_MS)
    );

    // The game resumes on-chain at the next ply — under the same rules, just
    // slowly. White replays the move Black never acknowledged.
    height += 1;
    let mv = ledger
        .dispute(&id)
        .unwrap()
        .pos
        .move_from_uci("g1f3")
        .unwrap();
    ledger
        .dispute_move(&id, Color::White, mv, height)
        .expect("White moves on-chain");
    let d = ledger.dispute(&id).unwrap();
    println!("\n    ply {}          White plays g1f3 on-chain", d.ply);
    println!(
        "    deadline      block {} — Black must move by then",
        d.deadline_block
    );
    println!(
        "                  that is {} blocks, about {} minutes",
        d.deadline_block - height,
        (d.deadline_block - height) * BLOCK_SECONDS / 60
    );

    // --- Black never comes back -------------------------------------------
    let deadline = d.deadline_block;
    assert!(
        ledger.dispute_finalize(&id, deadline).is_err(),
        "not while the window is still open"
    );
    println!("\n    …Black does not move.\n");

    let payout = ledger
        .dispute_finalize(&id, deadline + 1)
        .expect("the deadline passed");

    println!("    verdict       {:?}", Status::WhiteWins);
    println!(
        "    payout        white {} · black {}",
        payout.white, payout.black
    );
    println!(
        "    balances      white {} · black {}",
        ledger.balance(&white_pk),
        ledger.balance(&black_pk)
    );

    assert_eq!(payout.white, offer.pot());
    assert_eq!(ledger.total_supply(), supply_before, "E1: nothing minted");
    assert!(!ledger.in_dispute(&id));

    println!("\n  White was paid without Black's cooperation, without a");
    println!("  trusted third party, and without the chain being asked who");
    println!("  was right — only whether anybody moved.");
}
