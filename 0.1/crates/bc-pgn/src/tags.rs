//! Tag pairs: `[White "Kasparov, Garry"]`.

/// Split a game chunk into its tag pairs and its movetext.
pub fn split(chunk: &str) -> (Vec<(String, String)>, String) {
    let mut tags = Vec::new();
    let mut movetext = String::new();

    for line in chunk.lines() {
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') {
            if let Some(pair) = parse_tag(t) {
                tags.push(pair);
                continue;
            }
        }
        movetext.push_str(line);
        movetext.push('\n');
    }
    (tags, movetext)
}

fn parse_tag(line: &str) -> Option<(String, String)> {
    let inner = line.strip_prefix('[')?.strip_suffix(']')?;
    let (key, rest) = inner.split_once(char::is_whitespace)?;
    let value = rest.trim();
    let value = value.strip_prefix('"')?.strip_suffix('"')?;
    Some((key.to_string(), unescape(value)))
}

fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(n) = chars.next() {
                out.push(n);
                continue;
            }
        }
        out.push(c);
    }
    out
}
