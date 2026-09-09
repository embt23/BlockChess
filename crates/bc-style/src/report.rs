//! Reporting — the lab's measurements as JSON, and as a page.
//!
//! Kept in the library rather than the binary so both are testable. The page is
//! generated on demand rather than checked in: a committed HTML file would
//! drift the moment the feature set or the basis changed, and would then be
//! quietly displaying medals that nothing mints.

use crate::{identify, interaction, Lab};
use bc_hash::hex;

/// The page template. A real HTML file rather than a string literal, so it can
/// be edited and highlighted as HTML; `__DATA__` is the one substitution point.
pub const TEMPLATE: &str = include_str!("../assets/personality-space.html");

/// Escape a string for JSON. Player names come from PGN headers, so they are
/// arbitrary text and cannot be interpolated raw.
pub fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn nums(v: &[f64]) -> String {
    v.iter()
        .map(|x| format!("{x:.4}"))
        .collect::<Vec<_>>()
        .join(",")
}

/// The template with this lab's measurements substituted in.
///
/// The page therefore always shows what the crate actually computed. A
/// checked-in HTML file would drift the moment the feature set or the basis
/// changed, and would then be quietly displaying medals nothing mints.
pub fn render_page(lab: &Lab) -> String {
    assert_eq!(
        TEMPLATE.matches("__DATA__").count(),
        1,
        "template must have exactly one substitution point"
    );
    TEMPLATE.replace("__DATA__", &build_json(lab))
}

/// Everything the visualiser needs, and nothing it does not.
pub fn build_json(lab: &Lab) -> String {
    let b = &lab.basis;
    let mut out = String::from("{\n");
    out.push_str(&format!(
        "  \"corpus_root\": \"{}\",\n",
        hex(&b.corpus_root)
    ));
    out.push_str(&format!("  \"games\": {},\n", lab.corpus.len()));
    out.push_str(&format!("  \"samples\": {},\n", b.samples));
    out.push_str(&format!("  \"k\": {},\n", b.k));

    // Whether any of the rest means anything. First, so a consumer that reads
    // only the head of the document still gets the caveat.
    let a = b.adequacy();
    out.push_str(&format!(
        "  \"adequacy\": {{\"samples\":{},\"rank\":{},\"requested\":{},\"wanted_samples\":{},\"trustworthy\":{},\"warning\":{}}},\n",
        a.samples,
        a.rank,
        a.requested,
        a.wanted_samples,
        a.trustworthy(),
        match a.warning() {
            Some(w) => format!("\"{}\"", esc(&w)),
            None => "null".to_string(),
        }
    ));

    out.push_str("  \"axes\": [\n");
    for i in 0..b.k {
        let (pos, neg) = b.poles(i);
        let loadings: Vec<String> = b
            .loadings(i, 5)
            .into_iter()
            .map(|(n, w)| format!("{{\"feature\":\"{n}\",\"weight\":{w:.4}}}"))
            .collect();
        out.push_str(&format!(
            "    {{\"axis\":{i},\"explained\":{:.4},\"pos\":\"{pos}\",\"neg\":\"{neg}\",\"loadings\":[{}]}}{}\n",
            b.explained(i),
            loadings.join(","),
            if i + 1 < b.k { "," } else { "" }
        ));
    }
    out.push_str("  ],\n");

    out.push_str("  \"points\": [\n");
    for (i, p) in lab.points.iter().enumerate() {
        out.push_str(&format!(
            "    {{\"player\":\"{}\",\"game\":{},\"coords\":[{}]}}{}\n",
            esc(&p.player),
            p.game,
            nums(&p.coords),
            if i + 1 < lab.points.len() { "," } else { "" }
        ));
    }
    out.push_str("  ],\n");

    let players = lab.corpus.players();
    out.push_str("  \"players\": [\n");
    for (i, name) in players.iter().enumerate() {
        let Some(p) = lab.profile(name) else { continue };
        let chain = lab.chain_for(name);
        let links: Vec<String> = chain
            .links
            .iter()
            .map(|l| {
                format!(
                    "{{\"index\":{},\"coords\":[{}],\"games\":{},\"moved\":{:.4},\"medal\":\"{}\"}}",
                    l.index,
                    nums(&l.coords),
                    l.games,
                    l.moved,
                    hex(&l.medal)
                )
            })
            .collect();
        out.push_str(&format!(
            "    {{\"name\":\"{}\",\"coords\":[{}],\"dispersion\":{:.4},\"games\":{},\"medal\":\"{}\",\"head\":\"{}\",\"path_length\":{:.4},\"links\":[{}]}}{}\n",
            esc(name),
            nums(&p.coords),
            p.dispersion,
            p.games,
            hex(&p.medal(b)),
            hex(&chain.head()),
            chain.path_length(),
            links.join(","),
            if i + 1 < players.len() { "," } else { "" }
        ));
    }
    out.push_str("  ],\n");

    let (within, between) = lab.separation();
    out.push_str(&format!(
        "  \"separation\": {{\"within\":{within:.4},\"between\":{between:.4},\"accuracy\":{:.4},\"chance\":{:.4}}},\n",
        lab.nearest_neighbour_accuracy(),
        1.0 / players.len().max(1) as f64
    ));

    // Episode 12, on a held-out split — the honest counterpart to the
    // in-sample `separation.accuracy` above.
    out.push_str("  \"identification\": ");
    match identify::split(&lab.corpus, b.k) {
        Some(sp) => {
            let m = identify::confusion(&sp);
            let n = sp.centroids.len();
            let correct: usize = (0..n).map(|i| m[i][i]).sum();
            let total: usize = m.iter().flatten().sum::<usize>().max(1);
            let curve: Vec<String> = identify::convergence(&sp, 12, 40, 0xC0FFEE)
                .iter()
                .map(|c| format!("{{\"games\":{},\"accuracy\":{:.4}}}", c.games, c.accuracy))
                .collect();
            let margin = identify::closest_pair(&sp.centroids)
                .map(|(x, y, d, cells)| {
                    format!(
                        "{{\"a\":\"{}\",\"b\":\"{}\",\"distance\":{d:.4},\"cells\":{cells:.1}}}",
                        esc(&x),
                        esc(&y)
                    )
                })
                .unwrap_or_else(|| "null".into());
            out.push_str(&format!(
                "{{\"held_out_accuracy\":{:.4},\"curve\":[{}],\"margin\":{margin}}},\n",
                correct as f64 / total as f64,
                curve.join(",")
            ));
        }
        None => out.push_str("null,\n"),
    }

    // Episode 18 — does the opponent move you?
    let rows: Vec<String> = interaction::analyse(lab, 200, 0x1_9AC7_2E51)
        .iter()
        .map(|r| {
            format!(
                "{{\"player\":\"{}\",\"ratio\":{:.4},\"p\":{:.4}}}",
                esc(&r.player),
                r.ratio,
                r.p_value
            )
        })
        .collect();
    let pulls: Vec<String> = interaction::pulls(lab)
        .iter()
        .map(|p| {
            format!(
                "{{\"opponent\":\"{}\",\"magnitude\":{:.4}}}",
                esc(&p.opponent),
                p.magnitude
            )
        })
        .collect();
    out.push_str(&format!(
        "  \"interaction\": {{\"players\":[{}],\"pulls\":[{}]}}\n}}",
        rows.join(","),
        pulls.join(",")
    ));
    out
}
