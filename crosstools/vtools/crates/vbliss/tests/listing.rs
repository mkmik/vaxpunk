//! vbliss's listing of each program in tests/bliss, and of the listing's
//! own probes in tests/bliss-listings, against BLISSA64's, NAME.lis from
//! the oracle: the source part with its macro expansions, %PRINT output
//! and diagnostics, line for line, page headers and the summary at the
//! end dropped from both.

use std::fs;
use std::path::Path;

/// A listing without its page headers (two lines, the blank after them
/// and the blank before a page break) and without the summary after the
/// source: library statistics and the command line.
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
        if lines[i] == "COMMAND LINE:" || lines[i].contains("Library Statistics") {
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
    let tests = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests");
    let dir = tests.join("bliss");
    let mut checked = 0;
    let mut failed = Vec::new();
    let entries = fs::read_dir(&dir)
        .unwrap()
        .chain(fs::read_dir(tests.join("bliss-listings")).unwrap());
    for entry in entries {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|e| e != "lis") {
            continue;
        }
        let source = ["b64", "b32"]
            .map(|e| path.with_extension(e))
            .into_iter()
            .find(|p| p.exists())
            .unwrap();
        let text = fs::read_to_string(&source).unwrap();
        // The oracle's qualifiers, then the source's.
        let mut oracle = vbliss::Options::default();
        oracle
            .qualifier("/SOURCE_LIST=(EXPAND_MACROS,REQUIRE)")
            .unwrap();
        let opts = vbliss::Options {
            include: vec![dir.clone(), dir.join("../../lib")],
            ..oracle.with_source(&text)
        };
        let out = vbliss::translate(&source, &text, &opts);
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

/// The pages: each with its header, the module's name from the second
/// on, %TITLE's and %SBTTL's from the next page, /SOURCE_LIST=PAGE_SIZE
/// lines a page, the machine code on pages of its own and the command
/// line at the end (oracle LSTPAGE, LSTMC, LSTTTL).
#[test]
fn pages() {
    let source = "MODULE P (MAIN = M) =\nBEGIN\n%TITLE 'Tables'\n%SBTTL 'The first'\n\
                  OWN X;\n!\n!\n!\n!\n!\n!\n!\n!\n!\n!\n!\n!\n!\n!\n!\n!\n\
                  ROUTINE M = 1;\nEND\nELUDOM\n";
    let mut opts = vbliss::Options {
        date: Some("27-OCT-2026 12:00:00".into()),
        command: "vbliss /LIST P.B64".into(),
        ..Default::default()
    };
    for q in ["/SOURCE_LIST=PAGE_SIZE=20", "/MACHINE_CODE"] {
        assert!(opts.qualifier(q).unwrap());
    }
    let out = vbliss::translate(Path::new("P.B64"), source, &opts);
    let l = out.listing;
    let headers: Vec<&str> = l
        .lines()
        .filter(|s| s.contains("12:00:00"))
        .map(str::trim_end)
        .collect();
    let want = [
        "                                Source Listing                  27-OCT-2026 12:00:00  vbliss 0.1.0                      Page 1",
        "                                                                27-OCT-2026 12:00:00  P.B64",
        "P                               Tables                          27-OCT-2026 12:00:00  vbliss 0.1.0                      Page 2",
        "                                The first                       27-OCT-2026 12:00:00  P.B64",
        "P                               Machine Code Listing            27-OCT-2026 12:00:00  vbliss 0.1.0                      Page 3",
    ];
    assert_eq!(&headers[..5], want, "{l}");
    assert!(l.contains("    0\t      1 MODULE P"), "{l}");
    // 17 lines a page under a header of three.
    assert!(l.contains("    1\t     17 !\n\nP  "), "{l}");
    assert!(l.contains("\nM:\n"), "{l}");
    assert!(l.ends_with("COMMAND LINE:\n\nvbliss /LIST P.B64\n"), "{l}");
}
