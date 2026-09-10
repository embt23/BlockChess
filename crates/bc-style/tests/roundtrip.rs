//! The reader's oracle.
//!
//! The parser previously met four games somebody typed by hand. That is not
//! evidence about a file downloaded from a chess site. Here it meets thousands
//! of generated games instead: render a game whose moves are already known,
//! parse it back, and demand the same moves.
//!
//! Round-tripping is a real oracle rather than a self-consistency check,
//! because the two directions are written independently — the writer computes
//! disambiguation from the legal move list, the reader resolves it by filtering
//! that list, and a bug in either shows up as a game that will not replay.

use bc_style::pgn;
use bc_style::synth::{self, Rng};
use bc_style::{Corpus, GameRecord};

fn games(n: usize, seed: u64) -> Vec<GameRecord> {
    let mut rng = Rng::new(seed);
    let mut out = Vec::new();
    while out.len() < n {
        for (i, w) in synth::ARCHETYPES.iter().enumerate() {
            for (j, b) in synth::ARCHETYPES.iter().enumerate() {
                if i != j && out.len() < n {
                    out.push(synth::play(w, b, &mut rng, 160));
                }
            }
        }
    }
    out
}

/// Wrap movetext in the tag set a real export carries.
fn as_export(g: &GameRecord, white: &str, black: &str) -> String {
    format!(
        "[Event \"Rated Blitz game\"]\n[Site \"https://lichess.org/aBcD1234\"]\n\
         [Date \"2026.01.15\"]\n[White \"{white}\"]\n[Black \"{black}\"]\n\
         [Result \"1-0\"]\n[WhiteElo \"1523\"]\n[BlackElo \"1498\"]\n\
         [Variant \"Standard\"]\n[TimeControl \"300+3\"]\n[ECO \"B01\"]\n\
         [Termination \"Normal\"]\n\n{} 1-0\n\n",
        pgn::write_movetext(g)
    )
}

#[test]
fn every_generated_game_survives_a_round_trip() {
    let gs = games(120, 0x5EED_0001);
    let mut text = String::new();
    for (i, g) in gs.iter().enumerate() {
        text.push_str(&as_export(g, &format!("w{i}"), &format!("b{i}")));
    }

    let (back, skipped) = pgn::parse(&text);
    assert_eq!(skipped, 0, "{skipped} generated games failed to replay");
    assert_eq!(back.len(), gs.len());
    for (a, b) in gs.iter().zip(&back) {
        assert_eq!(a.moves, b.moves, "moves changed through the round trip");
    }
}

/// Every SAN token the writer can emit must be one the reader accepts, and the
/// interesting ones are rare — promotions, en passant, both-coordinate
/// disambiguation. Over many games they all appear.
#[test]
fn the_hard_san_forms_appear_and_survive() {
    let gs = games(400, 0xDEADBEEF);
    let mut seen_promo = false;
    let mut seen_castle = false;
    let mut seen_disambig = false;
    let mut seen_mate = false;

    for g in &gs {
        let text = pgn::write_movetext(g);
        seen_promo |= text.contains('=');
        seen_castle |= text.contains("O-O");
        seen_mate |= text.contains('#');
        // A disambiguated piece move: letter, then a coordinate, then the square.
        for tok in text.split_whitespace() {
            let t = tok.trim_end_matches(['+', '#']);
            if t.len() >= 4 && t.starts_with(['N', 'B', 'R', 'Q', 'K']) && !t.contains("O-O") {
                let body = t.trim_start_matches(['N', 'B', 'R', 'Q', 'K']);
                if body.len() > 2 && !body.starts_with('x') {
                    seen_disambig = true;
                }
            }
        }
    }
    assert!(seen_castle, "no castling in 400 games");
    assert!(seen_mate, "no mate in 400 games");
    assert!(seen_disambig, "no disambiguated move in 400 games");
    assert!(seen_promo, "no promotion in 400 games");
}

/// The messy shapes a real export actually contains. Each is a separate
/// assertion so a failure names the thing that broke.
#[test]
fn real_world_formatting_is_tolerated() {
    let g = &games(1, 0x1234)[0];
    let moves = pgn::write_movetext(g);

    let cases: Vec<(&str, String)> = vec![
        ("clock comments", {
            let mut s = String::new();
            for tok in moves.split_whitespace() {
                s.push_str(tok);
                s.push_str(" { [%clk 0:02:31] } ");
            }
            s
        }),
        ("eval and clock together", {
            let mut s = String::new();
            for tok in moves.split_whitespace() {
                s.push_str(tok);
                s.push_str(" { [%eval 0.24] [%clk 0:01:07] } ");
            }
            s
        }),
        ("numeric annotation glyphs", moves.replace(' ', " $1 ")),
        (
            "black move numbers with three dots",
            moves.replace(". ", "... "),
        ),
        ("no space after the move number", moves.replace(". ", ".")),
        ("windows line endings", moves.replace(' ', "\r\n")),
        (
            "semicolon comment at the end",
            format!("{moves} ; a trailing remark"),
        ),
    ];

    for (name, body) in cases {
        let text = format!("[White \"a\"]\n[Black \"b\"]\n\n{body} 1-0\n");
        let (back, skipped) = pgn::parse(&text);
        assert_eq!(skipped, 0, "{name}: game was rejected");
        assert_eq!(back.len(), 1, "{name}: wrong game count");
        assert_eq!(back[0].moves, g.moves, "{name}: moves changed");
    }
}

/// A file holding many games must yield exactly those games, with the right
/// names attached to the right moves.
#[test]
fn many_games_in_one_file_keep_their_identities() {
    let gs = games(40, 0xFEED);
    let mut text = String::new();
    for (i, g) in gs.iter().enumerate() {
        text.push_str(&as_export(g, &format!("alice{i}"), &format!("bob{i}")));
    }
    let (back, skipped) = pgn::parse(&text);
    assert_eq!(skipped, 0);
    assert_eq!(back.len(), 40);
    for (i, g) in back.iter().enumerate() {
        assert_eq!(g.white, format!("alice{i}"));
        assert_eq!(g.black, format!("bob{i}"));
        assert_eq!(g.moves, gs[i].moves);
    }
}

/// A corpus assembled from a round trip must commit to the same root, or
/// re-ingesting your own export would silently invalidate every medal.
#[test]
fn a_round_tripped_corpus_has_the_same_root() {
    let gs = games(30, 0xC0DE);
    let mut original = Corpus::new();
    let mut text = String::new();
    for (i, g) in gs.iter().enumerate() {
        let named = GameRecord {
            white: format!("w{i}"),
            black: format!("b{i}"),
            start: g.start,
            moves: g.moves.clone(),
            white_elo: Some(1500),
            black_elo: Some(1500),
        };
        text.push_str(&as_export(&named, &named.white, &named.black));
        original.push(named);
    }

    let (back, _) = pgn::parse(&text);
    let mut rebuilt = Corpus::new();
    for g in back {
        rebuilt.push(g);
    }
    assert_eq!(
        original.root(),
        rebuilt.root(),
        "a corpus does not survive being written out and read back"
    );
}

/// Regression for `docs/build-log.md` 07, and the most important test in this
/// file. `[WhiteElo "1523"]` starts with `White`, so a prefix match names every
/// player after their rating — on a real export, silently, with every medal
/// minted for a number instead of a person.
#[test]
fn a_rating_tag_does_not_become_the_player() {
    let g = &games(1, 0x99)[0];
    let text = format!(
        "[White \"realname\"]\n[WhiteElo \"1523\"]\n[Black \"otherperson\"]\n\
         [BlackElo \"1498\"]\n\n{} 1-0\n",
        pgn::write_movetext(g)
    );
    let (back, skipped) = pgn::parse(&text);
    assert_eq!(skipped, 0);
    assert_eq!(back[0].white, "realname", "the rating became the player");
    assert_eq!(back[0].black, "otherperson");
    assert_eq!(back[0].white_elo, Some(1523));
    assert_eq!(back[0].black_elo, Some(1498));
}

/// Tag order must not matter — some exporters put the ratings first.
#[test]
fn tag_order_does_not_matter() {
    let g = &games(1, 0x98)[0];
    let text = format!(
        "[BlackElo \"2100\"]\n[WhiteElo \"2200\"]\n[Black \"bee\"]\n[White \"eff\"]\n\n{} *\n",
        pgn::write_movetext(g)
    );
    let (back, _) = pgn::parse(&text);
    assert_eq!(back[0].white, "eff");
    assert_eq!(back[0].white_elo, Some(2200));
}

/// Chess960 has different castling. Replaying it with these rules would produce
/// a *plausible* wrong game rather than an error, which is the worst outcome.
#[test]
fn unsupported_variants_are_refused() {
    let g = &games(1, 0x97)[0];
    let moves = pgn::write_movetext(g);

    for bad in ["Chess960", "Atomic", "Horde", "Racing Kings"] {
        let text = format!("[White \"a\"]\n[Black \"b\"]\n[Variant \"{bad}\"]\n\n{moves} *\n");
        let (back, skipped) = pgn::parse(&text);
        assert!(back.is_empty(), "{bad} was accepted");
        assert_eq!(skipped, 1, "{bad} was not counted as rejected");
    }

    // "From Position" is ordinary chess from a supplied FEN.
    for ok in ["Standard", "standard", "From Position"] {
        let text = format!("[White \"a\"]\n[Black \"b\"]\n[Variant \"{ok}\"]\n\n{moves} *\n");
        let (back, skipped) = pgn::parse(&text);
        assert_eq!(skipped, 0, "{ok} was rejected");
        assert_eq!(back.len(), 1);
    }
}

/// A byte-order mark on the first line would otherwise stop `[Event ...]`
/// looking like a tag, and the file would parse as one nameless game.
#[test]
fn a_byte_order_mark_does_not_hide_the_first_tag() {
    let g = &games(1, 0x96)[0];
    let text = format!(
        "\u{feff}[White \"withbom\"]\n[Black \"b\"]\n\n{} *\n",
        pgn::write_movetext(g)
    );
    let (back, skipped) = pgn::parse(&text);
    assert_eq!(skipped, 0);
    assert_eq!(back[0].white, "withbom");
}

/// Ratings change after the fact and are somebody else's opinion, so the corpus
/// must not commit to them — otherwise a re-export with updated ratings would
/// move the root and invalidate every medal minted under it.
#[test]
fn ratings_are_not_part_of_the_commitment() {
    let g = &games(1, 0x95)[0];
    let mut a = Corpus::new();
    let mut b = Corpus::new();
    a.push(GameRecord {
        white_elo: Some(1200),
        black_elo: Some(1200),
        ..g.clone()
    });
    b.push(GameRecord {
        white_elo: Some(2400),
        black_elo: None,
        ..g.clone()
    });
    assert_eq!(a.root(), b.root(), "the corpus committed to a rating");
}
