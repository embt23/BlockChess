//! `style` — the lab, on the command line.
//!
//!   style demo [rounds]      fit a lens to synthetic players with known styles
//!   style pgn <file> [k]     fit a lens to a PGN export
//!   style export [rounds]    the same, as JSON on stdout, for the visualiser

use bc_hash::hex;
use bc_style::{synth, Corpus, Lab};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("demo");

    let json = cmd == "export";
    let corpus = match cmd {
        "demo" | "export" => {
            let rounds: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(10);
            let games = synth::round_robin(rounds, 0x5EED_1234, 120);
            if !json {
                println!(
                    "synthetic corpus: {} games between {} constructed personalities\n",
                    games.len(),
                    synth::ARCHETYPES.len()
                );
            }
            let mut c = Corpus::new();
            for g in games {
                c.push(g);
            }
            c
        }
        "pgn" => {
            let Some(path) = args.get(2) else {
                eprintln!("usage: style pgn <file> [k]");
                std::process::exit(2);
            };
            let text = match std::fs::read_to_string(path) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("cannot read {path}: {e}");
                    std::process::exit(1);
                }
            };
            let (games, skipped) = bc_style::pgn::parse(&text);
            println!("ingested {} games ({skipped} rejected)\n", games.len());
            let mut c = Corpus::new();
            for g in games {
                c.push(g);
            }
            c
        }
        _ => {
            eprintln!("usage: style [demo [rounds] | pgn <file> [k] | export [rounds]]");
            std::process::exit(2);
        }
    };

    if corpus.is_empty() {
        eprintln!("empty corpus — nothing to fit");
        std::process::exit(1);
    }

    let k: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(4);
    let Some(lab) = Lab::fit(corpus, k) else {
        eprintln!("no usable games in corpus");
        std::process::exit(1);
    };

    if json {
        emit_json(&lab);
        return;
    }

    println!("corpus root  {}", hex(&lab.basis.corpus_root));
    println!("samples      {} game-sides", lab.basis.samples);

    println!("\nthe axes, as this corpus expresses them");
    println!("(discovered, then named by the features loading hardest at each pole)");
    for i in 0..lab.basis.k {
        let (pos, neg) = lab.basis.poles(i);
        println!(
            "\n  axis {i}   {:>5.1}% of variance      {neg}  ←→  {pos}",
            lab.basis.explained(i) * 100.0
        );
        for (name, w) in lab.basis.loadings(i, 4) {
            println!("            {w:+.3}  {name}");
        }
    }

    println!("\nplayers");
    for name in lab.corpus.players() {
        let Some(p) = lab.profile(&name) else {
            continue;
        };
        let chain = lab.chain_for(&name);
        let coords: Vec<String> = p.coords.iter().map(|c| format!("{c:+.2}")).collect();
        println!("\n  {name}");
        println!("    position     [{}]", coords.join("  "));
        println!(
            "    dispersion   {:.2}  over {} games",
            p.dispersion, p.games
        );
        println!("    medal        {}", hex(&p.medal(&lab.basis)));
        println!(
            "    chain        {} links, path length {:.2}",
            chain.links.len(),
            chain.path_length()
        );
        println!("    head         {}", hex(&chain.head()));
    }

    let (within, between) = lab.separation();
    let acc = lab.nearest_neighbour_accuracy();
    let n = lab.corpus.players().len().max(1);
    println!("\nseparation");
    println!("  mean distance, same player       {within:.3}");
    println!("  mean distance, different players {between:.3}");
    println!(
        "  ratio                            {:.2}×",
        if within > 0.0 { between / within } else { 0.0 }
    );
    println!(
        "  nearest-neighbour accuracy       {:.1}%   (chance {:.1}%)",
        acc * 100.0,
        100.0 / n as f64
    );
    println!(
        "\n{}",
        if between > within * 1.2 {
            "Games cluster by who played them. The compression carries identity."
        } else {
            "Games do NOT separate by player. The points carry no identity — \
             any medal minted from this lens is noise."
        }
    );
}

/// Escape a string for JSON. Player names come from PGN headers, so they are
/// arbitrary text and cannot be interpolated raw.
fn esc(s: &str) -> String {
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

/// Everything the visualiser needs, and nothing it does not.
fn emit_json(lab: &Lab) {
    let b = &lab.basis;
    let mut out = String::from("{\n");
    out.push_str(&format!(
        "  \"corpus_root\": \"{}\",\n",
        hex(&b.corpus_root)
    ));
    out.push_str(&format!("  \"games\": {},\n", lab.corpus.len()));
    out.push_str(&format!("  \"samples\": {},\n", b.samples));
    out.push_str(&format!("  \"k\": {},\n", b.k));

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
        "  \"separation\": {{\"within\":{within:.4},\"between\":{between:.4},\"accuracy\":{:.4},\"chance\":{:.4}}}\n}}",
        lab.nearest_neighbour_accuracy(),
        1.0 / players.len().max(1) as f64
    ));
    println!("{out}");
}
