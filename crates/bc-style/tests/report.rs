//! The page is generated, not checked in, so these are the checks that a
//! checked-in file would otherwise get from being looked at.

use bc_style::report::{self, TEMPLATE};
use bc_style::{synth, Corpus, Lab};

fn lab() -> Lab {
    let mut c = Corpus::new();
    for g in synth::round_robin(3, 0x5EED_1234, 120) {
        c.push(g);
    }
    Lab::fit(c, 4).expect("corpus should fit")
}

#[test]
fn the_template_has_exactly_one_substitution_point() {
    assert_eq!(TEMPLATE.matches("__DATA__").count(), 1);
    assert!(TEMPLATE.contains("<title>Personality Space</title>"));
}

#[test]
fn the_rendered_page_substitutes_everything() {
    let page = report::render_page(&lab());
    assert!(
        !page.contains("__DATA__"),
        "placeholder survived into the output"
    );
    assert!(page.contains("<title>Personality Space</title>"));
    assert!(page.len() > TEMPLATE.len(), "no data was inserted");
}

/// The page must show what this run actually computed, not a stale snapshot.
#[test]
fn the_page_carries_this_runs_measurements() {
    let lab = lab();
    let page = report::render_page(&lab);
    let root = bc_hash::hex(&lab.basis.corpus_root);
    assert!(page.contains(&root), "corpus root missing from page");

    for name in lab.corpus.players() {
        let medal = bc_hash::hex(&lab.profile(&name).unwrap().medal(&lab.basis));
        assert!(page.contains(&medal), "{name}'s medal is not on the page");
    }
}

#[test]
fn the_json_carries_every_section_the_page_reads() {
    let json = report::build_json(&lab());
    for key in [
        "corpus_root",
        "axes",
        "points",
        "players",
        "separation",
        "loadings",
        "links",
        "dispersion",
        "path_length",
    ] {
        assert!(json.contains(&format!("\"{key}\"")), "missing key {key}");
    }
    assert_eq!(
        json.matches('{').count(),
        json.matches('}').count(),
        "unbalanced braces in emitted JSON"
    );
    assert_eq!(
        json.matches('[').count(),
        json.matches(']').count(),
        "unbalanced brackets in emitted JSON"
    );
}

/// Player names come from PGN headers, so they are arbitrary text. A raw quote
/// or backslash would break out of the JSON string and corrupt the whole
/// document — the same injection shape as an unescaped SQL literal.
#[test]
fn player_names_are_escaped() {
    assert_eq!(report::esc("a\"b"), "a\\\"b");
    assert_eq!(report::esc("a\\b"), "a\\\\b");
    assert_eq!(report::esc("a\nb"), "a\\nb");
    assert_eq!(report::esc("plain"), "plain");
    // A raw control byte must become an escape, not travel through as itself.
    assert_eq!(report::esc("a\u{1}b"), "a\\u0001b");
}

/// A PGN with a hostile player name must still produce a page whose JSON is
/// structurally intact.
#[test]
fn a_hostile_player_name_does_not_break_the_page() {
    let text = "[White \"a\\\"</script>b\"]\n[Black \"B\"]\n\n\
                1. e4 e5 2. Bc4 Nc6 3. Qh5 Nf6 4. Qxf7# 1-0\n";
    let (games, skipped) = bc_style::pgn::parse(text);
    assert_eq!(skipped, 0, "the fixture game should replay");
    let mut c = Corpus::new();
    for g in games {
        c.push(g);
    }
    let lab = Lab::fit(c, 1).expect("should fit");
    let json = report::build_json(&lab);
    assert_eq!(
        json.matches('{').count(),
        json.matches('}').count(),
        "a player name broke out of its JSON string"
    );
}
