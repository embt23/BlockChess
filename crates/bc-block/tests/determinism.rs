//! Block and transaction encoding is consensus code, so it gets the same
//! grep `bc-adjudicator` gets (`D24`).
//!
//! Floating point and hash-map iteration order are the two classic ways for
//! two validators to compute different bytes from the same input while both
//! passing their own tests. Neither appears here, and the source is checked
//! rather than the intention.

use std::path::Path;

#[test]
fn nothing_in_the_block_layer_uses_floats_or_hash_maps() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut offenders = Vec::new();
    let mut stack = vec![src];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap();
            let mut in_tests = false;
            for (n, line) in text.lines().enumerate() {
                if line.trim_start().starts_with("mod tests") {
                    in_tests = true;
                }
                if in_tests {
                    continue;
                }
                let code = line.split("//").next().unwrap_or("");
                for bad in ["f32", "f64", "HashMap", "HashSet"] {
                    if code.contains(bad) {
                        offenders.push(format!("{}:{} {bad}", path.display(), n + 1));
                    }
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "non-deterministic constructs: {offenders:#?}"
    );
}
