//! The two transactions a game costs the chain, and the arithmetic between
//! them.
//!
//! The property under test throughout is conservation: the ledger's total is
//! the same after every settlement as before it. `E1` says the supply never
//! grows for playing, and the reason is economic rather than monetary — the
//! moment playing pays, two bots farm each other forever.

mod common;

use bc_channel::ledger::LedgerError;
use bc_channel::msg::Signed;
use bc_channel::state::Status;
use bc_channel::{GameOffer, Ledger};
use bc_chess::Position;
use bc_sig::SigningKey;
use common::{keys, offer_for, table, table_with_rake, STAKE, START_BALANCE};

const FOOLS: [&str; 4] = ["f2f3", "e7e5", "g2g4", "d8h4"];

#[test]
fn a_game_costs_the_chain_two_transactions() {
    let mut t = table(Position::startpos());
    let before = t.ledger.total_supply();

    // The stakes are already escrowed: that was transaction one.
    assert_eq!(t.ledger.balance(&t.offer.white_pk), START_BALANCE - STAKE);
    assert_eq!(t.ledger.balance(&t.offer.black_pk), START_BALANCE - STAKE);

    t.play(&FOOLS);
    let certified = t.white.countersign().unwrap();
    let payout = t.ledger.close_game(&certified).expect("close");

    // Transaction two. Thirty-three signatures were exchanged in the Opera
    // Game; here it is four. Either way the chain sees two.
    assert_eq!(payout.black, 2 * STAKE);
    assert_eq!(payout.white, 0);
    assert_eq!(t.ledger.balance(&t.offer.black_pk), START_BALANCE + STAKE);
    assert_eq!(t.ledger.total_supply(), before);
}

#[test]
fn the_chain_verifies_no_chess() {
    // Two people who agree about the result are allowed to be wrong about
    // it. This is a perfectly ordinary position after 1.e4, signed by both
    // as a White win, and the ledger pays out — because it is their money.
    let mut t = table(Position::startpos());
    t.play(&["e2e4"]);
    let mut s = *t.white.head();
    s.state.status = Status::WhiteWins;

    let (wsk, bsk, _) = keys();
    s.white_sig = Some(wsk.sign(&s.state.hash()));
    s.black_sig = Some(bsk.sign(&s.state.hash()));

    let payout = t.ledger.close_game(&s).expect("agreement settles");
    assert_eq!(payout.white, 2 * STAKE);
}

#[test]
fn an_old_state_cannot_move_money() {
    // `P2` in one assertion. Every non-terminal state pays nobody, so posting
    // one accomplishes nothing — which is why there are no revocation
    // secrets, no penalty transactions, and no watchtower holding a secret
    // on your behalf.
    let mut t = table(Position::startpos());
    t.play(&FOOLS[..2]);
    let mut old = *t.white.head();
    let (wsk, bsk, _) = keys();
    old.white_sig = Some(wsk.sign(&old.state.hash()));
    old.black_sig = Some(bsk.sign(&old.state.hash()));
    assert!(old.is_certified());

    assert_eq!(
        t.ledger.close_game(&old).unwrap_err(),
        LedgerError::NotTerminal
    );
}

#[test]
fn one_signature_is_a_claim_and_two_is_a_certificate() {
    let mut t = table(Position::startpos());
    t.play(&FOOLS);

    // Black asserted the mate. Black alone cannot settle it.
    let half = *t.black.head();
    assert!(!half.is_certified());
    assert_eq!(
        t.ledger.close_game(&half).unwrap_err(),
        LedgerError::BadSignature
    );

    // White countersigns, and the same state becomes payable.
    let certified = t.white.countersign().unwrap();
    assert!(t.ledger.close_game(&certified).is_ok());
}

#[test]
fn a_settled_channel_settles_once() {
    let mut t = table(Position::startpos());
    t.play(&FOOLS);
    let certified = t.white.countersign().unwrap();
    assert!(t.ledger.close_game(&certified).is_ok());
    assert_eq!(
        t.ledger.close_game(&certified).unwrap_err(),
        LedgerError::AlreadySettled
    );
}

#[test]
fn resignation_costs_the_chain_one_signature() {
    let mut t = table(Position::startpos());
    t.play(&["e2e4", "e7e5"]);
    let before = t.ledger.total_supply();

    let sig = t.white.resign();
    let payout = t
        .ledger
        .close_by_resignation(&t.white.channel_id, t.white.ply(), &t.offer.white_pk, &sig)
        .expect("resign");

    assert_eq!(payout.black, 2 * STAKE);
    assert_eq!(t.ledger.total_supply(), before);
}

#[test]
fn a_resignation_signed_by_the_wrong_person_is_not_a_resignation() {
    let mut t = table(Position::startpos());
    t.play(&["e2e4"]);
    let impostor = SigningKey::from_seed(&[9u8; 32]);
    let sig = impostor.sign(&bc_channel::msg::resign_bytes(
        &t.white.channel_id,
        t.white.ply(),
    ));
    assert_eq!(
        t.ledger
            .close_by_resignation(&t.white.channel_id, t.white.ply(), &t.offer.white_pk, &sig)
            .unwrap_err(),
        LedgerError::BadSignature
    );
}

#[test]
fn a_draw_needs_both_signatures_and_returns_both_stakes() {
    let mut t = table(Position::startpos());
    t.play(&["e2e4", "e7e5"]);
    let before = t.ledger.total_supply();

    let w = t.white.sign_draw();
    let b = t.black.sign_draw();
    assert_eq!(t.white.ply(), t.black.ply(), "both sign the same ply");

    // One signature is an offer, not an agreement.
    assert_eq!(
        t.ledger
            .close_by_draw(&t.white.channel_id, t.white.ply(), &w, &w)
            .unwrap_err(),
        LedgerError::BadSignature
    );

    let payout = t
        .ledger
        .close_by_draw(&t.white.channel_id, t.white.ply(), &w, &b)
        .expect("draw");
    assert_eq!((payout.white, payout.black), (STAKE, STAKE));
    assert_eq!(t.ledger.balance(&t.offer.white_pk), START_BALANCE);
    assert_eq!(t.ledger.total_supply(), before);
}

#[test]
fn rake_comes_out_of_the_pot_and_nowhere_else() {
    let mut t = table_with_rake(Position::startpos(), 500); // the 5% maximum
    let before = t.ledger.total_supply();
    t.play(&FOOLS);
    let certified = t.white.countersign().unwrap();
    let payout = t.ledger.close_game(&certified).unwrap();

    assert_eq!(payout.rake, 10, "5% of a pot of 200");
    assert_eq!(payout.black, 190);
    assert_eq!(payout.white + payout.black + payout.rake, 2 * STAKE);
    assert_eq!(t.ledger.total_supply(), before);
}

#[test]
fn an_odd_pot_loses_nothing_to_rounding() {
    // Unequal stakes and a rake that does not divide evenly: the two shares
    // plus the rake must still be exactly the pot. Dust that vanishes here
    // is a burn, and a burn is a mint with the sign flipped.
    let (wsk, bsk, ssk) = keys();
    let start = Position::startpos();
    let mut offer = offer_for(&start, 333, &ssk);
    offer.stake_white = 101;
    offer.stake_black = 67;

    let mut ledger = Ledger::new();
    ledger.credit(&offer.white_pk, 1_000);
    ledger.credit(&offer.black_pk, 1_000);
    let before = ledger.total_supply();
    let id = ledger
        .open_game(
            &offer,
            &wsk.sign(&offer.signing_bytes()),
            &bsk.sign(&offer.signing_bytes()),
        )
        .unwrap();

    let w = wsk.sign(&bc_channel::msg::draw_bytes(&id, 0));
    let b = bsk.sign(&bc_channel::msg::draw_bytes(&id, 0));
    let payout = ledger.close_by_draw(&id, 0, &w, &b).unwrap();
    assert_eq!(payout.white + payout.black + payout.rake, 168);
    assert_eq!(ledger.total_supply(), before);
}

// ---------------------------------------------------------------------------
// Opening
// ---------------------------------------------------------------------------

fn fresh() -> (Ledger, GameOffer, SigningKey, SigningKey) {
    let (wsk, bsk, ssk) = keys();
    let offer = offer_for(&Position::startpos(), 0, &ssk);
    let mut ledger = Ledger::new();
    ledger.credit(&offer.white_pk, START_BALANCE);
    ledger.credit(&offer.black_pk, START_BALANCE);
    (ledger, offer, wsk, bsk)
}

#[test]
fn a_server_relays_signatures_and_cannot_touch_the_money() {
    // The server assembles and submits; the stakes move under the players'
    // own signatures. Nothing here is authorised by the submitter, so
    // whoever calls `open_game` is irrelevant to the outcome (`P6`).
    let (mut ledger, offer, wsk, bsk) = fresh();
    let before = ledger.total_supply();
    ledger
        .open_game(
            &offer,
            &wsk.sign(&offer.signing_bytes()),
            &bsk.sign(&offer.signing_bytes()),
        )
        .expect("anyone may submit");
    assert_eq!(ledger.total_supply(), before, "escrow is not a payment");
    assert_eq!(ledger.balance(&offer.white_pk), START_BALANCE - STAKE);
}

#[test]
fn an_offer_one_player_did_not_sign_is_not_an_offer() {
    let (mut ledger, offer, wsk, _) = fresh();
    let impostor = SigningKey::from_seed(&[9u8; 32]);
    assert_eq!(
        ledger
            .open_game(
                &offer,
                &wsk.sign(&offer.signing_bytes()),
                &impostor.sign(&offer.signing_bytes()),
            )
            .unwrap_err(),
        LedgerError::BadSignature
    );
    assert_eq!(ledger.balance(&offer.white_pk), START_BALANCE);
}

#[test]
fn a_stale_offer_cannot_be_submitted_late() {
    // `expiry_block` is what stops a server sitting on a signed offer until
    // the odds move.
    let (mut ledger, offer, wsk, bsk) = fresh();
    ledger.height = offer.expiry_block + 1;
    assert_eq!(
        ledger
            .open_game(
                &offer,
                &wsk.sign(&offer.signing_bytes()),
                &bsk.sign(&offer.signing_bytes()),
            )
            .unwrap_err(),
        LedgerError::Expired
    );
}

#[test]
fn the_same_offer_cannot_open_two_channels() {
    let (mut ledger, offer, wsk, bsk) = fresh();
    let (w, b) = (
        wsk.sign(&offer.signing_bytes()),
        bsk.sign(&offer.signing_bytes()),
    );
    ledger.open_game(&offer, &w, &b).unwrap();
    assert_eq!(
        ledger.open_game(&offer, &w, &b).unwrap_err(),
        LedgerError::DuplicateChannel
    );

    // A different nonce is a different channel, which is what the nonce is
    // for: states from one game must never be evidence in another.
    let mut again = offer;
    again.open_nonce = [8u8; 32];
    assert_ne!(again.channel_id(), offer.channel_id());
    ledger
        .open_game(
            &again,
            &wsk.sign(&again.signing_bytes()),
            &bsk.sign(&again.signing_bytes()),
        )
        .expect("a second game between the same two people");
}

#[test]
fn you_cannot_stake_what_you_do_not_have() {
    let (_, offer, wsk, bsk) = fresh();
    let mut ledger = Ledger::new();
    ledger.credit(&offer.white_pk, STAKE - 1);
    ledger.credit(&offer.black_pk, START_BALANCE);
    assert_eq!(
        ledger
            .open_game(
                &offer,
                &wsk.sign(&offer.signing_bytes()),
                &bsk.sign(&offer.signing_bytes()),
            )
            .unwrap_err(),
        LedgerError::InsufficientFunds
    );
}

#[test]
fn a_rake_with_no_server_to_pay_is_refused() {
    // Otherwise it is a burn, and a burn is a mint with the sign flipped.
    let (mut ledger, mut offer, wsk, bsk) = fresh();
    offer.server_pk = bc_sig::VerifyingKey([0u8; 32]);
    offer.rake_bps = 100;
    assert!(!offer.valid());
    assert_eq!(
        ledger
            .open_game(
                &offer,
                &wsk.sign(&offer.signing_bytes()),
                &bsk.sign(&offer.signing_bytes()),
            )
            .unwrap_err(),
        LedgerError::BadOffer
    );
}

#[test]
fn a_rake_over_five_percent_is_refused() {
    let (mut ledger, mut offer, wsk, bsk) = fresh();
    offer.rake_bps = 501;
    assert_eq!(
        ledger
            .open_game(
                &offer,
                &wsk.sign(&offer.signing_bytes()),
                &bsk.sign(&offer.signing_bytes()),
            )
            .unwrap_err(),
        LedgerError::BadOffer
    );
}

#[test]
fn settling_a_channel_the_ledger_never_opened_is_refused() {
    let (mut ledger, ..) = fresh();
    let s = Signed::new(bc_channel::GameState::genesis([9u8; 32], [0u8; 32], 1));
    assert_eq!(
        ledger.close_game(&s).unwrap_err(),
        LedgerError::UnknownChannel
    );
}
