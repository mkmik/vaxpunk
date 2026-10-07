//! vbliss's listing of each program in tests/bliss against BLISSA64's,
//! NAME.lis from the oracle: the source part with its macro expansions,
//! %PRINT output and diagnostics, line for line, page headers and the
//! summary at the end dropped from both.

use std::fs;
use std::path::Path;

/// A listing without its page headers (two lines, the blank after them
/// and the blank before a page break) and without what follows the source.
fn body(listing: &str) -> String {
    let lines: Vec<&str> = listing.lines().collect();
    let mut out: Vec<&str> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if lines[i].contains("Source Listing") {
            if out.last() == Some(&"") {
                out.pop();
            }
            i += 3;
            continue;
        }
        if lines[i] == "COMMAND LINE:" {
            break;
        }
        out.push(lines[i]);
        i += 1;
    }
    while out.last() == Some(&"") {
        out.pop();
    }
    out.join("\n") + "\n"
}

#[test]
fn listings() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/bliss");
    let mut checked = 0;
    let mut failed = Vec::new();
    for entry in fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|e| e != "lis") {
            continue;
        }
        let source = path.with_extension("b64");
        let text = fs::read_to_string(&source).unwrap();
        let out = vbliss::translate(&source, &text, &vbliss::Options::default());
        let (ours, theirs) = (
            body(&out.listing),
            body(&fs::read_to_string(&path).unwrap()),
        );
        if ours != theirs {
            let line = ours
                .lines()
                .zip(theirs.lines())
                .position(|(a, b)| a != b)
                .unwrap_or(ours.lines().count().min(theirs.lines().count()));
            failed.push(format!(
                "{}: line {}:\n  vbliss:   {:?}\n  BLISSA64: {:?}",
                path.display(),
                line + 1,
                ours.lines().nth(line),
                theirs.lines().nth(line)
            ));
        }
        checked += 1;
    }
    assert!(failed.is_empty(), "{}", failed.join("\n"));
    assert!(checked > 0);
}
