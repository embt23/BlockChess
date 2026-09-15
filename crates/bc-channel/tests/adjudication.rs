//! Episode 08 — what happens when someone lies or vanishes.
//!
//! The first test in this file is the project's own definition of done:
//! *you can win against an opponent who disconnects.* Everything before it in
//! the repository is a prerequisite for that sentence; everything after is
//! expansion.

mod common;

use bc_channel::clock::{budget_blocks, FLOOR_BLOCKS, MIN_MOVE_BLOCKS};
use bc_channel::dispute::{DisputeError, MoveOutcome, Refutation};
use bc_channel::ledger::adjudicate::{Evidence, RepetitionProof};
use bc_channel::ledger::LedgerError;
use bc_channel::TimeControl;
use bc_channel::{ClaimKind, Status};
use bc_chess::{Color, Move, Position};
use common::{table, table_capped, STAKE, START_BALANCE};
use std::collections::BTreeMap;

/// Enough blocks for any window in these tests to have closed.
const LATER: u64 = 100_000;

#[test]
fn you_can_win_against_an_opponent_who_disconnects() {
    // Milestone E, as one test.
    let mut t = table(Position::startpos());
    let before = t.ledger.total_supply();
    t.play(&["e2e4", "e7e5", "g1f3"]);

    // Black stops answering. White holds a state Black signed — not because
    // White asked for one, but because Black's countersignature rode along
    // with Black's own last move. The player who is waiting always has it.
    let (evidence, packed) = t.evidence(Color::White);
    // Ply 2, not 3: White's own ply-3 state is signed by White alone, and a
    // state you signed yourself proves nothing. The ply-2 state is the
    // highest one Black put their name to.
    assert_eq!(evidence.state.ply, 2);
    assert!(
        evidence.sig(Color::Black).is_some(),
        "signed by the opponent"
    );

    t.ledger
        .dispute_open(Color::White, &evidence, &packed, 1_000)
        .expect("open the dispute");
    assert!(t.ledger.in_dispute(&t.white.channel_id));

    // The evidence is a state Black signed, so on-chain it is *White's* turn
    // — which is the situation exactly: Black went quiet while White was
    // waiting to be answered. White resumes the game by replaying the move
    // Black never acknowledged. "You post ply 37 and the game resumes at 38."
    let id = t.white.channel_id;
    let d = t.ledger.dispute(&id).unwrap();
    assert_eq!(d.side_to_move(), Color::White);
    let mv = d.pos.move_from_uci("g1f3").unwrap();
    assert_eq!(
        t.ledger
            .dispute_move(&id, Color::White, mv, 1_001, None)
            .unwrap(),
        None,
        "the game continues; now it is Black's move"
    );
    assert_eq!(t.ledger.dispute(&id).unwrap().side_to_move(), Color::Black);

    // Black's deadline runs. Black does not move.
    assert_eq!(
        t.ledger.dispute_finalize(&id, 1_002),
        Err(LedgerError::Dispute(DisputeError::NotYetDecided)),
        "the deadline has not passed yet"
    );

    let payout = t.ledger.dispute_finalize(&id, LATER).expect("finalize");

    assert_eq!(payout.white, 2 * STAKE);
    assert_eq!(payout.black, 0);
    assert_eq!(
        t.ledger.balance(&t.offer.white_pk),
        START_BALANCE + STAKE,
        "White is paid without Black's cooperation"
    );
    assert_eq!(t.ledger.total_supply(), before);
}

#[test]
fn a_state_you_signed_yourself_proves_nothing() {
    let mut t = table(Position::startpos());
    t.play(&["e2e4"]);
    // White's head is signed by White alone. Posting it as evidence against
    // Black is posting your own assertion.
    let own = *t.white.head();
    let packed = t.white.position().pack().as_slice().to_vec();
    assert!(own.sig(Color::Black).is_none());
    assert_eq!(
        t.ledger.dispute_open(Color::White, &own, &packed, 10),
        Err(LedgerError::BadSignature)
    );
}

#[test]
fn the_position_must_be_the_one_the_state_names() {
    let mut t = table(Position::startpos());
    t.play(&["e2e4", "e7e5"]);
    let (ev, _) = t.evidence(Color::White);

    // A different position, however legal, is not this game.
    let lie = Position::startpos().pack().as_slice().to_vec();
    assert_eq!(
        t.ledger.dispute_open(Color::White, &ev, &lie, 10),
        Err(LedgerError::BadPosition)
    );
    // Nor is a corrupt encoding.
    assert_eq!(
        t.ledger.dispute_open(Color::White, &ev, &[0u8; 4], 10),
        Err(LedgerError::BadPosition)
    );
}

#[test]
fn higher_ply_wins_and_stale_states_are_overridden() {
    let mut t = table(Position::startpos());
    t.play(&["e2e4", "e7e5"]);
    let (early, early_pos) = t.evidence(Color::White);
    assert_eq!(early.state.ply, 2);

    t.play(&["g1f3", "b8c6"]);
    let (late, late_pos) = t.evidence(Color::White);
    assert_eq!(late.state.ply, 4);

    // Black opens on the stale state — perhaps hoping the later moves are
    // forgotten. White overrides with the higher ply, which is the whole of
    // `P2` on-chain.
    t.ledger
        .dispute_open(Color::White, &early, &early_pos, 10)
        .unwrap();
    assert_eq!(t.ledger.dispute(&t.white.channel_id).unwrap().ply, 2);

    t.ledger
        .dispute_open(Color::White, &late, &late_pos, 20)
        .unwrap();
    assert_eq!(t.ledger.dispute(&t.white.channel_id).unwrap().ply, 4);

    // And the stale one cannot be posted again to wind the game back.
    assert_eq!(
        t.ledger.dispute_open(Color::White, &early, &early_pos, 30),
        Err(LedgerError::Dispute(DisputeError::StaleState))
    );
    assert_eq!(
        t.ledger.dispute_open(Color::White, &late, &late_pos, 30),
        Err(LedgerError::Dispute(DisputeError::StaleState)),
        "equal ply is not higher"
    );
}

#[test]
fn the_game_continues_on_chain_under_the_same_rules() {
    // Not arbitration — chess, slowly. White forces the game on-chain and
    // Black, who was merely slow rather than gone, plays on and mates.
    let start = Position::from_fen("rnbqkbnr/pppp1ppp/8/4p3/6P1/5P2/PPPPP2P/RNBQKBNR b KQkq - 0 2")
        .unwrap();
    let mut t = table(start);
    let (ev, packed) = t.evidence(Color::White);
    t.ledger
        .dispute_open(Color::White, &ev, &packed, 100)
        .unwrap();

    let id = t.white.channel_id;
    let d = t.ledger.dispute(&id).unwrap();
    assert_eq!(d.side_to_move(), Color::Black);
    let mate = d.pos.move_from_uci("d8h4").unwrap();

    // Fool's mate, played through the adjudicator. The on-chain engine runs
    // here and nowhere else — and it runs `is_move_legal`, not `outcome()`.
    //
    // `P3`: the chain does **not** look for mate. Playing the mating move
    // without saying so leaves the game running, because noticing would cost
    // the ∀ over ~218 moves that the invariant exists to avoid.
    assert_eq!(
        t.ledger
            .dispute_move(&id, Color::Black, mate, 110, None)
            .expect("legal on-chain move"),
        None,
        "the chain does not notice mate by itself"
    );
    assert!(t.ledger.in_dispute(&id), "still running");
    assert!(
        t.ledger.dispute(&id).unwrap().pos.is_checkmate(),
        "…even though it is, in fact, mate"
    );
}

#[test]
fn the_mating_player_says_so_and_nobody_can_refute_it() {
    // The same position, played the way a client actually would: the claim
    // rides along with the move, as `spec/05`'s `DisputeMove { …, [status] }`
    // always allowed. One transaction, and no quantifier.
    let start = Position::from_fen("rnbqkbnr/pppp1ppp/8/4p3/6P1/5P2/PPPPP2P/RNBQKBNR b KQkq - 0 2")
        .unwrap();
    let mut t = table(start);
    let (ev, packed) = t.evidence(Color::White);
    t.ledger
        .dispute_open(Color::White, &ev, &packed, 100)
        .unwrap();
    let id = t.white.channel_id;

    let mate = t
        .ledger
        .dispute(&id)
        .unwrap()
        .pos
        .move_from_uci("d8h4")
        .unwrap();
    assert_eq!(
        t.ledger
            .dispute_move(&id, Color::Black, mate, 110, Some(ClaimKind::Checkmate))
            .unwrap(),
        None,
        "optimistic: a window opens rather than settling"
    );

    // White is genuinely mated, so there is no refuting move to post.
    for from in 0u8..64 {
        for to in 0u8..64 {
            assert!(t
                .ledger
                .dispute_refute(&id, Color::White, Move::normal(from, to), 120)
                .is_err());
        }
    }

    let payout = t.ledger.dispute_finalize(&id, LATER).unwrap();
    assert_eq!(payout.black, 2 * STAKE);
}

#[test]
fn claiming_a_mate_that_is_not_one_is_refuted_by_a_single_move() {
    // And the cost asymmetry, stated as a test: the claimant asserted a ∀
    // that nobody computed, and one legal move settles it.
    let mut t = table(Position::startpos());
    t.play(&["e2e4"]);
    let (ev, packed) = t.evidence(Color::Black);
    t.ledger
        .dispute_open(Color::Black, &ev, &packed, 100)
        .unwrap();
    let id = t.white.channel_id;

    let mv = t
        .ledger
        .dispute(&id)
        .unwrap()
        .pos
        .move_from_uci("e7e5")
        .unwrap();
    t.ledger
        .dispute_move(&id, Color::Black, mv, 110, Some(ClaimKind::Checkmate))
        .unwrap();

    let escape = t
        .ledger
        .dispute(&id)
        .unwrap()
        .pos
        .move_from_uci("g1f3")
        .unwrap();
    assert_eq!(
        t.ledger
            .dispute_refute(&id, Color::White, escape, 120)
            .unwrap(),
        Refutation::ResumedAtMove
    );
}

#[test]
fn an_illegal_move_on_chain_costs_gas_and_time_and_nothing_else() {
    let mut t = table(Position::startpos());
    t.play(&["e2e4"]);
    let (ev, packed) = t.evidence(Color::Black);
    t.ledger
        .dispute_open(Color::Black, &ev, &packed, 100)
        .unwrap();

    let id = t.white.channel_id;
    let before = t.ledger.dispute(&id).unwrap().pos;
    assert_eq!(
        t.ledger
            .dispute_move(&id, Color::Black, Move(0xFFFF), 110, None),
        Err(LedgerError::Dispute(DisputeError::IllegalMove))
    );
    // The transaction reverted: the board did not move, and neither did the
    // deadline. Submitting garbage buys nothing.
    assert_eq!(t.ledger.dispute(&id).unwrap().pos, before);

    // Nor can the wrong player move.
    let legal = before.move_from_uci("e7e5").unwrap();
    assert_eq!(
        t.ledger.dispute_move(&id, Color::White, legal, 110, None),
        Err(LedgerError::Dispute(DisputeError::NotYourTurn))
    );
    // And not after the deadline.
    assert_eq!(
        t.ledger.dispute_move(&id, Color::Black, legal, LATER, None),
        Err(LedgerError::Dispute(DisputeError::DeadlinePassed))
    );
}

// ---------------------------------------------------------------------------
// Optimistic claims
// ---------------------------------------------------------------------------

/// Set up a dispute in a position where Black is genuinely mated.
fn mated_dispute() -> (common::Table, bc_hash::Hash) {
    let start = Position::from_fen("rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3")
        .unwrap();
    let mut t = table(start);
    let (ev, packed) = t.evidence(Color::Black);
    t.ledger
        .dispute_open(Color::Black, &ev, &packed, 100)
        .unwrap();
    let id = t.white.channel_id;
    (t, id)
}

#[test]
fn an_unrefuted_mate_claim_stands() {
    let (mut t, id) = mated_dispute();
    // White is mated here, so Black claims it. Nobody computes the ∀.
    assert_eq!(
        t.ledger
            .dispute_claim(&id, Color::Black, ClaimKind::Checkmate, Evidence::None, 200)
            .unwrap(),
        None,
        "optimistic: a window opens rather than settling"
    );
    let payout = t.ledger.dispute_finalize(&id, LATER).unwrap();
    assert_eq!(payout.black, 2 * STAKE);
}

#[test]
fn a_false_mate_claim_is_refuted_by_one_move_and_punished() {
    let mut t = table(Position::startpos());
    t.play(&["e2e4"]);
    let (ev, packed) = t.evidence(Color::Black);
    t.ledger
        .dispute_open(Color::Black, &ev, &packed, 100)
        .unwrap();
    let id = t.white.channel_id;

    // White claims Black is mated after 1.e4, which is nonsense.
    t.ledger
        .dispute_claim(&id, Color::White, ClaimKind::Checkmate, Evidence::None, 200)
        .unwrap();
    let budget_before = t.ledger.dispute(&id).unwrap().budget_of(Color::White);

    // Black answers with one legal move. That is the entire refutation.
    let escape = t
        .ledger
        .dispute(&id)
        .unwrap()
        .pos
        .move_from_uci("e7e5")
        .unwrap();
    assert_eq!(
        t.ledger
            .dispute_refute(&id, Color::Black, escape, 210)
            .unwrap(),
        Refutation::ResumedAtMove
    );

    let d = t.ledger.dispute(&id).unwrap();
    assert_eq!(d.ply, 2, "the game resumed at the refuting move");
    assert_eq!(d.side_to_move(), Color::White);
    assert_eq!(
        d.budget_of(Color::White),
        budget_before / 2,
        "a false claim costs half the liar's budget"
    );
}

#[test]
fn only_the_player_the_claim_is_against_may_refute_it() {
    let (mut t, id) = mated_dispute();
    t.ledger
        .dispute_claim(&id, Color::Black, ClaimKind::Checkmate, Evidence::None, 200)
        .unwrap();
    let any = Move::normal(12, 28);
    assert_eq!(
        t.ledger.dispute_refute(&id, Color::Black, any, 210),
        Err(LedgerError::Dispute(DisputeError::NotYourTurn)),
        "the claimant cannot refute themselves"
    );
    // And a genuinely mated player has nothing to post.
    assert_eq!(
        t.ledger.dispute_refute(&id, Color::White, any, 210),
        Err(LedgerError::Dispute(DisputeError::NotRefutable))
    );
}

#[test]
fn a_refutation_after_the_window_is_too_late() {
    let mut t = table(Position::startpos());
    t.play(&["e2e4"]);
    let (ev, packed) = t.evidence(Color::Black);
    t.ledger
        .dispute_open(Color::Black, &ev, &packed, 100)
        .unwrap();
    let id = t.white.channel_id;
    t.ledger
        .dispute_claim(&id, Color::White, ClaimKind::Checkmate, Evidence::None, 200)
        .unwrap();
    let escape = t
        .ledger
        .dispute(&id)
        .unwrap()
        .pos
        .move_from_uci("e7e5")
        .unwrap();
    assert_eq!(
        t.ledger.dispute_refute(&id, Color::Black, escape, LATER),
        Err(LedgerError::Dispute(DisputeError::RefutationWindowClosed))
    );
}

#[test]
fn optimism_adds_no_new_liveness_requirement() {
    // The safety argument, made executable. A player who cannot refute within
    // the window equally cannot move within it, because the two windows are
    // the same Δ — so a false mate claim can never take a game that the
    // timeout rule would not already have taken.
    let mut t = table(Position::startpos());
    t.play(&["e2e4"]);
    let (ev, packed) = t.evidence(Color::Black);
    t.ledger
        .dispute_open(Color::Black, &ev, &packed, 100)
        .unwrap();
    let id = t.white.channel_id;

    let move_deadline = t.ledger.dispute(&id).unwrap().deadline_block;
    t.ledger
        .dispute_claim(&id, Color::White, ClaimKind::Checkmate, Evidence::None, 100)
        .unwrap();
    let refute_deadline = t
        .ledger
        .dispute(&id)
        .unwrap()
        .claim
        .unwrap()
        .refutable_until;

    assert_eq!(
        refute_deadline, move_deadline,
        "claimed at the same height, the two windows must coincide"
    );
}

#[test]
fn a_stalemate_refutation_does_not_let_the_opponent_pick_your_move() {
    // `spec/05` says a refutation resumes the game "at the refuting move".
    // That is right for mate, where the move belongs to the refuter. It is
    // wrong for stalemate: there the claimant is the side to move, and the
    // refuter is the opponent, who is merely exhibiting that a move exists.
    // Playing it for them would let an opponent choose their move.
    let mut t = table(Position::startpos());
    t.play(&["e2e4"]);
    let (ev, packed) = t.evidence(Color::Black);
    t.ledger
        .dispute_open(Color::Black, &ev, &packed, 100)
        .unwrap();
    let id = t.white.channel_id;

    // Black, to move, falsely claims stalemate.
    t.ledger
        .dispute_claim(&id, Color::Black, ClaimKind::Stalemate, Evidence::None, 200)
        .unwrap();
    let ply_before = t.ledger.dispute(&id).unwrap().ply;

    let exhibit = t
        .ledger
        .dispute(&id)
        .unwrap()
        .pos
        .move_from_uci("e7e5")
        .unwrap();
    assert_eq!(
        t.ledger
            .dispute_refute(&id, Color::White, exhibit, 210)
            .unwrap(),
        Refutation::ClaimStruck
    );

    let d = t.ledger.dispute(&id).unwrap();
    assert_eq!(d.ply, ply_before, "the exhibited move was not played");
    assert_eq!(d.side_to_move(), Color::Black, "Black must still move");
    assert!(d.claim.is_none());
}

// ---------------------------------------------------------------------------
// The O(1) endings
// ---------------------------------------------------------------------------

#[test]
fn resignation_and_draw_agreement_settle_on_the_spot() {
    let mut t = table(Position::startpos());
    t.play(&["e2e4"]);
    let (ev, packed) = t.evidence(Color::Black);
    t.ledger
        .dispute_open(Color::Black, &ev, &packed, 100)
        .unwrap();
    let id = t.white.channel_id;
    let ply = t.ledger.dispute(&id).unwrap().ply;

    let sig =
        bc_sig::SigningKey::from_seed(&[2u8; 32]).sign(&bc_channel::msg::resign_bytes(&id, ply));
    let payout = t
        .ledger
        .dispute_claim(
            &id,
            Color::Black,
            ClaimKind::Resign,
            Evidence::Resign(sig),
            200,
        )
        .unwrap()
        .expect("immediate");
    assert_eq!(payout.white, 2 * STAKE, "Black resigned");
}

#[test]
fn a_forged_resignation_is_refused() {
    let mut t = table(Position::startpos());
    t.play(&["e2e4"]);
    let (ev, packed) = t.evidence(Color::Black);
    t.ledger
        .dispute_open(Color::Black, &ev, &packed, 100)
        .unwrap();
    let id = t.white.channel_id;
    let ply = t.ledger.dispute(&id).unwrap().ply;

    // White signs "Black resigns" and submits it.
    let forged =
        bc_sig::SigningKey::from_seed(&[1u8; 32]).sign(&bc_channel::msg::resign_bytes(&id, ply));
    assert_eq!(
        t.ledger.dispute_claim(
            &id,
            Color::Black,
            ClaimKind::Resign,
            Evidence::Resign(forged),
            200
        ),
        Err(LedgerError::BadSignature)
    );
}

#[test]
fn the_board_checkable_draws_need_no_evidence_but_must_be_true() {
    // Insufficient material: two bare kings. Claimed at ply 0, because a
    // channel that *moves* into a dead position notices before any dispute
    // can start — `claimable_status` marks it drawn on the spot.
    let mut t = table(Position::from_fen("8/8/4k3/8/8/4K3/8/8 w - - 0 1").unwrap());
    let (ev, packed) = t.evidence(Color::White);
    t.ledger
        .dispute_open(Color::White, &ev, &packed, 100)
        .unwrap();
    let id = t.white.channel_id;
    let payout = t
        .ledger
        .dispute_claim(
            &id,
            Color::Black,
            ClaimKind::InsufficientMaterial,
            Evidence::None,
            200,
        )
        .unwrap()
        .expect("immediate");
    assert_eq!((payout.white, payout.black), (STAKE, STAKE));

    // And the same claim in a position full of pieces is simply false.
    let mut t = table(Position::startpos());
    t.play(&["e2e4"]);
    let (ev, packed) = t.evidence(Color::Black);
    t.ledger
        .dispute_open(Color::Black, &ev, &packed, 100)
        .unwrap();
    let id = t.white.channel_id;
    assert_eq!(
        t.ledger.dispute_claim(
            &id,
            Color::Black,
            ClaimKind::InsufficientMaterial,
            Evidence::None,
            200
        ),
        Err(LedgerError::Dispute(DisputeError::UnsupportedEvidence))
    );
    assert_eq!(
        t.ledger
            .dispute_claim(&id, Color::Black, ClaimKind::FiftyMove, Evidence::None, 200),
        Err(LedgerError::Dispute(DisputeError::UnsupportedEvidence))
    );
}

#[test]
fn threefold_is_three_signatures_and_no_history() {
    // A repetition claim replays nothing. It is three signatures the opponent
    // already gave, which is the general principle in its cheapest form:
    // structure the protocol so that what you will need to prove later is
    // something your adversary had to sign earlier.
    //
    // Note what the shuffle demonstrates on the way: the channel itself spots
    // the third occurrence and claims the draw. The halfmove clock differs at
    // every occurrence, so this only works because repetition compares
    // `rep_hash` and not `pos_hash` (`docs/build-log.md` §05).
    let start = Position::from_fen("7k/8/8/8/8/8/8/R6K w - - 0 1").unwrap();
    let mut t = table(start);

    // A setup move first, so the repeated position is not the opening one —
    // ply 0 carries no signatures and so can never be part of a proof.
    let mut certified: BTreeMap<u16, (bc_channel::Signed, Position)> = BTreeMap::new();
    let mut snapshot = |t: &common::Table| {
        for ch in [&t.white, &t.black] {
            let c = *ch.certified();
            if c.is_certified() {
                certified.insert(c.state.ply, (c, *ch.certified_position()));
            }
        }
    };

    for mv in [
        "h1g1", "h8g8", "a1a2", "g8h8", "a2a1", "h8g8", "a1a2", "g8h8", "a2a1",
    ] {
        t.ply(mv);
        snapshot(&t);
    }
    // The game ended on the board; certifying the last state is the ordinary
    // cooperative close, and it also completes the third occurrence.
    assert_eq!(t.white.head().state.status, Status::Draw);
    let last = t.black.countersign().expect("countersign the drawn state");
    certified.insert(last.state.ply, (last, *t.black.position()));

    // Three occurrences of one position, at distinct plies.
    let mut groups: BTreeMap<bc_hash::Hash, Vec<u16>> = BTreeMap::new();
    for (ply, (_, pos)) in &certified {
        groups
            .entry(bc_channel::rep_hash(pos))
            .or_default()
            .push(*ply);
    }
    let plies = groups
        .values()
        .find(|v| v.len() >= 3)
        .expect("the shuffle repeats a position three times")
        .clone();
    assert_eq!(&plies[..3], &[1, 5, 9], "certified occurrences");

    let packed: Vec<Vec<u8>> = plies[..3]
        .iter()
        .map(|p| certified[p].1.pack().as_slice().to_vec())
        .collect();

    let (ev, ev_packed) = t.evidence(Color::White);
    t.ledger
        .dispute_open(Color::White, &ev, &ev_packed, 100)
        .unwrap();
    let id = t.white.channel_id;

    let proof = RepetitionProof {
        occurrences: [
            (certified[&plies[0]].0, packed[0].as_slice()),
            (certified[&plies[1]].0, packed[1].as_slice()),
            (certified[&plies[2]].0, packed[2].as_slice()),
        ],
    };
    let payout = t
        .ledger
        .dispute_claim(
            &id,
            Color::White,
            ClaimKind::Threefold,
            Evidence::Threefold(&proof),
            200,
        )
        .unwrap()
        .expect("immediate: three signature checks and three hashes");
    assert_eq!((payout.white, payout.black), (STAKE, STAKE));
}

#[test]
fn the_same_occurrence_three_times_is_not_a_repetition() {
    let start = Position::from_fen("7k/8/8/8/8/8/8/R6K w - - 0 1").unwrap();
    let mut t = table(start);
    t.play(&["a1a2", "h8g8", "a2a1", "g8h8", "a1a2"]);

    let one = *t.white.certified();
    assert!(one.is_certified());
    let packed = t.white.certified_position().pack().as_slice().to_vec();

    let (ev, ev_packed) = t.evidence(Color::White);
    t.ledger
        .dispute_open(Color::White, &ev, &ev_packed, 100)
        .unwrap();
    let id = t.white.channel_id;

    // The same certified state submitted three times is one occurrence with
    // a photocopier, not three.
    let proof = RepetitionProof {
        occurrences: [
            (one, packed.as_slice()),
            (one, packed.as_slice()),
            (one, packed.as_slice()),
        ],
    };
    assert_eq!(
        t.ledger.dispute_claim(
            &id,
            Color::White,
            ClaimKind::Threefold,
            Evidence::Threefold(&proof),
            200
        ),
        Err(LedgerError::BadPosition),
        "distinct plies, or one state counts three times"
    );
}

// ---------------------------------------------------------------------------
// Time
// ---------------------------------------------------------------------------

#[test]
fn a_player_who_stalled_arrives_with_a_smaller_budget() {
    // The griefing brake: forcing a dispute while low on time is forcing it
    // into a losing position. Dilation carries the disadvantage across.
    let mut t = table(Position::startpos());
    let mv = t.white.position().move_from_uci("e2e4").unwrap();
    let msg = t.white.play(mv, 170_000).unwrap(); // White burns nearly all of it
    t.black.receive(&msg, 170_100).unwrap();

    let (ev, packed) = t.evidence(Color::Black);
    t.ledger
        .dispute_open(Color::Black, &ev, &packed, 100)
        .unwrap();
    let d = t.ledger.dispute(&t.white.channel_id).unwrap();

    // τ now comes from the time-control class rather than a constant (`D22`),
    // so the test reads it from the terms it is actually playing under.
    let tau = t.offer.terms.budget_tau_ms();
    assert_eq!(
        t.offer.terms.time_control,
        TimeControl::Blitz,
        "3+2 is blitz"
    );
    assert_eq!(d.budget_of(Color::White), budget_blocks(12_000, tau));
    assert_eq!(d.budget_of(Color::Black), budget_blocks(180_000, tau));
    assert!(d.budget_of(Color::Black) > d.budget_of(Color::White) * 10);
    // But White can still physically move, which is the point of the floor.
    assert!(d.budget_of(Color::White) > FLOOR_BLOCKS);
}

#[test]
fn a_budget_runs_out_and_the_game_is_lost_on_time() {
    // A near-flagged player gets the floor: enough to move, not enough to
    // outlast anybody.
    let start = Position::startpos();
    let mut t = table(start);
    let mv = t.white.position().move_from_uci("e2e4").unwrap();
    let msg = t.white.play(mv, 179_999).unwrap();
    t.black.receive(&msg, 180_000).unwrap();
    t.ply("e7e5");

    let (ev, packed) = t.evidence(Color::White);
    t.ledger
        .dispute_open(Color::White, &ev, &packed, 0)
        .unwrap();
    let id = t.white.channel_id;

    // White has the floor plus the increment's worth, and must keep moving.
    let d = t.ledger.dispute(&id).unwrap();
    assert_eq!(d.side_to_move(), Color::White);
    let budget = d.budget_of(Color::White) as u64;

    // Each move costs at least MIN_MOVE_BLOCKS, so the budget is exhausted in
    // a bounded number of moves however fast White replies.
    assert!(
        budget / MIN_MOVE_BLOCKS < 20,
        "budget {budget} is not small"
    );

    let mut height = 1;
    let mut moves = 0;
    while t.ledger.in_dispute(&id) && moves < 40 {
        let d = t.ledger.dispute(&id).unwrap();
        if height > d.deadline_block {
            break;
        }
        let side = d.side_to_move();
        let mv = *d.pos.generate_legal().as_slice().first().unwrap();
        if t.ledger.dispute_move(&id, side, mv, height, None).is_err() {
            break;
        }
        height += 1;
        moves += 1;
    }
    // Whoever ran out, somebody did, and the chain can say so without either
    // player's cooperation.
    let payout = t.ledger.dispute_finalize(&id, LATER).expect("finalize");
    assert_eq!(payout.white + payout.black, 2 * STAKE);
}

#[test]
fn the_ply_cap_forces_a_draw_and_bounds_the_worst_case() {
    // This is the number a validator has to be able to afford.
    let start = Position::from_fen("7k/8/8/8/8/8/8/R6K w - - 0 1").unwrap();
    let mut t = table_capped(start, 4);
    t.play(&["a1a2", "h8g8"]);

    let (ev, packed) = t.evidence(Color::White);
    t.ledger
        .dispute_open(Color::White, &ev, &packed, 100)
        .unwrap();
    let id = t.white.channel_id;

    let d = t.ledger.dispute(&id).unwrap();
    let mv = d.pos.move_from_uci("a2a3").unwrap();
    assert_eq!(
        t.ledger
            .dispute_move(&id, Color::White, mv, 101, None)
            .unwrap(),
        None,
        "ply 3 of 4"
    );
    let d = t.ledger.dispute(&id).unwrap();
    let mv = d.pos.move_from_uci("g8h8").unwrap();
    let payout = t
        .ledger
        .dispute_move(&id, Color::Black, mv, 102, None)
        .unwrap()
        .expect("the cap ends it");
    assert_eq!((payout.white, payout.black), (STAKE, STAKE), "a draw");
}

#[test]
fn a_settled_channel_cannot_be_disputed() {
    let mut t = table(Position::startpos());
    t.play(&["f2f3", "e7e5", "g2g4", "d8h4"]);
    let certified = t.white.countersign().unwrap();
    t.ledger.close_game(&certified).unwrap();

    let (ev, packed) = t.evidence(Color::White);
    assert_eq!(
        t.ledger.dispute_open(Color::White, &ev, &packed, 100),
        Err(LedgerError::AlreadySettled)
    );
}

#[test]
fn disputes_conserve_the_supply_like_everything_else() {
    for ending in 0..3 {
        let mut t = table(Position::startpos());
        let before = t.ledger.total_supply();
        t.play(&["e2e4", "e7e5"]);
        let (ev, packed) = t.evidence(Color::White);
        t.ledger
            .dispute_open(Color::White, &ev, &packed, 100)
            .unwrap();
        let id = t.white.channel_id;

        let _ = match ending {
            0 => t.ledger.dispute_finalize(&id, LATER).unwrap(),
            1 => {
                let ply = t.ledger.dispute(&id).unwrap().ply;
                let sig = bc_sig::SigningKey::from_seed(&[2u8; 32])
                    .sign(&bc_channel::msg::resign_bytes(&id, ply));
                t.ledger
                    .dispute_claim(
                        &id,
                        Color::Black,
                        ClaimKind::Resign,
                        Evidence::Resign(sig),
                        200,
                    )
                    .unwrap()
                    .unwrap()
            }
            _ => {
                let d = t.ledger.dispute(&id).unwrap();
                let side = d.side_to_move();
                let mv = *d.pos.generate_legal().as_slice().first().unwrap();
                t.ledger.dispute_move(&id, side, mv, 110, None).unwrap();
                t.ledger.dispute_finalize(&id, LATER).unwrap()
            }
        };
        assert_eq!(t.ledger.total_supply(), before, "ending {ending}");
        assert!(!t.ledger.in_dispute(&id));
    }
}

#[test]
fn a_dispute_over_a_terminal_state_is_refused() {
    // Disputes are for games still in progress. A finished game is settled
    // with two signatures and no adjudicator.
    let mut t = table(Position::startpos());
    t.play(&["f2f3", "e7e5", "g2g4", "d8h4"]);
    let (ev, packed) = t.evidence(Color::White);
    assert_eq!(ev.state.status, Status::BlackWins);
    assert_eq!(
        t.ledger.dispute_open(Color::White, &ev, &packed, 100),
        Err(LedgerError::Dispute(DisputeError::NotOngoing))
    );
}

#[test]
fn a_move_cannot_sidestep_a_pending_claim() {
    let (mut t, id) = mated_dispute();
    t.ledger
        .dispute_claim(&id, Color::Black, ClaimKind::Checkmate, Evidence::None, 200)
        .unwrap();
    assert_eq!(
        t.ledger
            .dispute_move(&id, Color::White, Move::normal(0, 1), 210, None),
        Err(LedgerError::Dispute(DisputeError::ClaimPending))
    );
    // And a second claim cannot be stacked on the first.
    assert_eq!(
        t.ledger
            .dispute_claim(&id, Color::Black, ClaimKind::Stalemate, Evidence::None, 210),
        Err(LedgerError::Dispute(DisputeError::ClaimPending))
    );
    let _ = MoveOutcome::Continues;
}
