//! SAN in both directions, checked against itself and against real games.
//!
//! The property is `parse_san(pos, &to_san(pos, m)) == m` for every legal move
//! of every position reached. That is a strong check because the two
//! directions are written independently: `to_san` decides how much
//! disambiguation is needed, `parse_san` decides what a disambiguator means,
//! and a disagreement between them shows up immediately.
//!
//! It is not a substitute for parsing files somebody else wrote, which is why
//! `corpus/classics.pgn` is also parsed here.

use bc_chess::Position;
use bc_pgn::{parse_san, to_san};

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
}

#[test]
fn san_round_trips_on_every_legal_move() {
    let mut rng = Rng(0xDEAD_BEEF_CAFE_F00D);
    let mut checked = 0usize;

    for _ in 0..200 {
        let mut pos = Position::startpos();
        for _ in 0..160 {
            let legal = pos.generate_legal();
            if legal.is_empty() {
                break;
            }
            // Every legal move here, not just the one we play.
            for &m in &legal {
                let san = to_san(&pos, m);
                let back = parse_san(&pos, &san).unwrap_or_else(|e| {
                    panic!("{san} did not parse back in {}: {e}", pos.to_fen())
                });
                assert_eq!(
                    back,
                    m,
                    "{san} resolved to {} not {} in {}",
                    back.to_uci(),
                    m.to_uci(),
                    pos.to_fen()
                );
                checked += 1;
            }
            let all = legal.as_slice();
            pos = pos.make_move(all[(rng.next() % all.len() as u64) as usize]);
        }
    }
    assert!(checked > 100_000, "only checked {checked} moves");
}

#[test]
fn parses_the_classics() {
    let text = include_str!("../../../corpus/classics.pgn");
    let (games, errors) = bc_pgn::parse_all(text);
    assert!(errors.is_empty(), "{}", errors[0]);
    assert_eq!(games.len(), 4);

    // The Opera Game ends in mate on move 17.
    let opera = &games[0];
    assert_eq!(opera.tag("White"), Some("Morphy, Paul"));
    assert_eq!(opera.moves.len(), 33);
    assert_eq!(opera.result, "1-0");
    let final_pos = *opera.positions().last().unwrap();
    assert!(final_pos.in_check(final_pos.side));
    assert!(
        final_pos.generate_legal().is_empty(),
        "the Opera Game should end in mate"
    );
}

#[test]
fn accepts_the_spellings_that_appear_in_real_files() {
    let pos = Position::from_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").unwrap();
    // Zeros for castling, and a check suffix that carries no information.
    assert_eq!(
        parse_san(&pos, "O-O").unwrap(),
        parse_san(&pos, "0-0").unwrap()
    );
    assert_eq!(
        parse_san(&pos, "O-O-O").unwrap(),
        parse_san(&pos, "0-0-0").unwrap()
    );

    let promo = Position::from_fen("8/P6k/8/8/8/8/7K/8 w - - 0 1").unwrap();
    // `a8=Q`, `a8Q` and a bare `a8` all mean the queen: the last is malformed
    // but common, and the intent is never in doubt.
    let q = parse_san(&promo, "a8=Q").unwrap();
    assert_eq!(parse_san(&promo, "a8Q").unwrap(), q);
    assert_eq!(parse_san(&promo, "a8").unwrap(), q);
    // Underpromotion must not collapse into it.
    assert_ne!(parse_san(&promo, "a8=N").unwrap(), q);
}

#[test]
fn rejects_ambiguity_rather_than_guessing() {
    // Knights on b1 and f1 both reach d2; `Nd2` names neither, and picking
    // one would silently corrupt a game. The parser must refuse.
    let pos = Position::from_fen("4k3/8/8/8/8/8/8/1N3N1K w - - 0 1").unwrap();
    assert!(parse_san(&pos, "Nd2").is_err());
    assert!(parse_san(&pos, "Nbd2").is_ok());
    assert!(parse_san(&pos, "Nfd2").is_ok());
    assert_ne!(
        parse_san(&pos, "Nbd2").unwrap(),
        parse_san(&pos, "Nfd2").unwrap()
    );
}

#[test]
fn writes_the_shortest_sufficient_disambiguator() {
    // SAN's rule: file if it separates, else rank, else both. Getting this
    // wrong produces tokens that round-trip in our own code and nowhere else.
    // Different files: the file alone separates them.
    let pos = Position::from_fen("4k3/8/8/8/8/8/8/1N3N1K w - - 0 1").unwrap();
    let m = parse_san(&pos, "Nbd2").unwrap();
    assert_eq!(to_san(&pos, m), "Nbd2");

    // Same file, different ranks: the file cannot separate them, so the rank
    // must, and the written form has to switch to it.
    let pos = Position::from_fen("4k3/8/8/R7/8/8/8/R3K3 w - - 0 1").unwrap();
    let low = parse_san(&pos, "R1a3").unwrap();
    let high = parse_san(&pos, "R5a3").unwrap();
    assert_ne!(low, high);
    assert_eq!(to_san(&pos, low), "R1a3");
    assert_eq!(to_san(&pos, high), "R5a3");
    assert!(parse_san(&pos, "Ra3").is_err(), "Ra3 is ambiguous here");
}

#[test]
fn strips_comments_variations_and_glyphs() {
    let text = "\
[Event \"?\"]
[Result \"1-0\"]

1. e4 {a comment} e5 $1 2. Nf3 (2. f4 exf4 {a variation nobody played}) Nc6 3. Bb5 1-0
";
    let (games, errors) = bc_pgn::parse_all(text);
    assert!(errors.is_empty(), "{}", errors[0]);
    assert_eq!(games.len(), 1);
    let sans: Vec<String> = {
        let g = &games[0];
        let mut pos = g.start;
        let mut out = Vec::new();
        for &m in &g.moves {
            out.push(to_san(&pos, m));
            pos = pos.make_move(m);
        }
        out
    };
    assert_eq!(sans, ["e4", "e5", "Nf3", "Nc6", "Bb5"]);
}

#[test]
fn a_damaged_game_is_skipped_not_fatal() {
    // Real dumps contain broken games. One must not stop the other three.
    let text = "\
[Event \"good\"]

1. e4 e5 1-0

[Event \"broken\"]

1. e4 Qxh8 1-0

[Event \"also good\"]

1. d4 d5 1-0
";
    let (games, errors) = bc_pgn::parse_all(text);
    assert_eq!(games.len(), 2);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].game_index, 1);
}

// ---------------------------------------------------------------------------
// Pathologies found by running against PGN files we did not write.
//
// The source was python-chess's own test corpus — real files chosen by another
// project because they break parsers. Every case below is reproduced here as a
// minimal fixture rather than by vendoring their files, so the tests stay
// self-contained and say plainly what they are about.
//
// 0.0's build log put it this way: an external oracle is worth more than any
// number of self-written sanity checks. Ten of their twelve files now parse.
// The two that do not are antichess and crazyhouse, which are different games
// under different rules — and refusing them is correct behaviour, not a gap.
// See papers/04-permanence.md §5 on RuleSetId.
// ---------------------------------------------------------------------------

#[test]
fn a_utf8_byte_order_mark_does_not_hide_the_first_tag() {
    // Windows tooling writes these routinely. The BOM sits in front of
    // `[Event`, so the line stops looking like a tag, and the whole file
    // parses as one anonymous game with no moves.
    let text = "\u{feff}[Event \"A\"]\n[Result \"1-0\"]\n\n1. e4 e5 2. Nf3 1-0\n";
    let (games, errors) = bc_pgn::parse_all(text);
    assert!(errors.is_empty(), "{}", errors[0]);
    assert_eq!(games.len(), 1);
    assert_eq!(games[0].moves.len(), 3);
    assert_eq!(
        games[0].tag("Event"),
        Some("A"),
        "the BOM ate the first tag"
    );
}

#[test]
fn a_null_move_truncates_the_game_instead_of_discarding_it() {
    // `Z0` is ChessBase's null-move placeholder; `--` is the other spelling.
    // A null move is not a chess move, so the game cannot be replayed past it
    // — but throwing away the moves before it loses real data.
    for token in ["Z0", "--"] {
        let text = format!("[Event \"?\"]\n\n1. e4 e5 2. Nf3 {token} 3. Bb5 1-0\n");
        let (games, errors) = bc_pgn::parse_all(&text);
        assert!(errors.is_empty(), "{token} should truncate, not fail");
        assert_eq!(games.len(), 1);
        assert_eq!(games[0].moves.len(), 3, "kept the moves before {token}");
        let (ply, tok) = games[0]
            .truncated
            .clone()
            .unwrap_or_else(|| panic!("{token} should have been recorded"));
        assert_eq!(ply, 3);
        assert_eq!(tok, token);
    }
}

#[test]
fn a_whole_game_records_no_truncation() {
    let (games, _) = bc_pgn::parse_all("[Event \"?\"]\n\n1. e4 e5 1-0\n");
    assert!(games[0].truncated.is_none());
}

#[test]
fn movetext_written_in_uci_is_accepted() {
    // CCRL's archives write `g1f3` rather than `Nf3`. It is unambiguous, and
    // refusing it would reject whole archives over a notation choice.
    let text = "[Event \"?\"]\n[Result \"1/2-1/2\"]\n\ne2e4 c7c5 g1f3 d7d6 1/2-1/2\n";
    let (games, errors) = bc_pgn::parse_all(text);
    assert!(errors.is_empty(), "{}", errors[0]);
    assert_eq!(games.len(), 1);
    assert_eq!(games[0].moves.len(), 4);

    // And it must agree with the SAN spelling of the same game.
    let san = "[Event \"?\"]\n\n1. e4 c5 2. Nf3 d6 1/2-1/2\n";
    let (other, _) = bc_pgn::parse_all(san);
    assert_eq!(games[0].moves, other[0].moves);
}

#[test]
fn uci_promotions_survive_the_round_trip() {
    let text =
        "[Event \"?\"]\n[FEN \"4k3/P7/8/8/8/8/8/4K3 w - - 0 1\"]\n[SetUp \"1\"]\n\na7a8q 1-0\n";
    let (games, errors) = bc_pgn::parse_all(text);
    assert!(errors.is_empty(), "{}", errors[0]);
    assert_eq!(games[0].moves.len(), 1);
    let m = games[0].moves[0];
    assert_eq!(m.to_uci(), "a7a8q");
}
