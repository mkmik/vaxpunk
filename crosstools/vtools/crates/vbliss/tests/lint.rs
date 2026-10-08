//! The dot lint (PRD-0004 *The dot lint*): what each rule finds, that the
//! test programs have nothing for it to find, and how many of the dots
//! deleted from them it notices, which must stay above a floor.

use std::fs;
use std::path::{Path, PathBuf};

fn lints(path: &Path, text: &str) -> Option<Vec<vbliss::Diag>> {
    let opts = vbliss::Options {
        include: vec![Path::new(env!("CARGO_MANIFEST_DIR")).join("../../lib")],
        ..vbliss::Options::from_source(text)
    };
    let out = vbliss::translate(path, text, &opts);
    out.error().is_none().then_some(out.lints)
}

#[test]
fn rules() {
    let text = "MODULE T =
BEGIN
LIBRARY 'SYS$LIBRARY:STARLET';
LITERAL L = 5;
OWN X, Y : VECTOR[2];
EXTERNAL ROUTINE SYS$QIOW, SYS$ASSIGN;
ROUTINE R (A) =
    BEGIN
    IF NOT X THEN 1;
    WHILE X AND .A DO A = 0;
    A = X + 1;
    A = .Y + 1;
    A = .(X + 8);
    A = .L;
    A = .R;
    A = X GTR 5;
    SYS$QIOW(0, X, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0);
    SYS$ASSIGN(0, 4, 0, 0);
    SYS$QIOW(0, .X, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0);
    IF X THEN 1;  ! LINT: ADDRESS
    .A
    END;
END
ELUDOM
";
    let found: Vec<(u32, String)> = lints(Path::new("t.b64"), text)
        .expect("it compiles")
        .into_iter()
        .map(|d| (d.line, d.msg))
        .collect();
    let want = [
        (9, "tests the address of X"),
        (10, "tests the address of X"),
        (11, "does arithmetic on the address of X"),
        (14, "fetches from LITERAL L"),
        (15, "fetches from routine R"),
        (16, "compares the address of X"),
        (17, "passes the address of X as argument 2 of SYS$QIOW"),
        (18, "passes 4 as argument 2 of SYS$ASSIGN"),
    ];
    assert_eq!(found.len(), want.len(), "{found:?}");
    for ((line, msg), (wl, wm)) in found.iter().zip(want) {
        assert!(
            *line == wl && msg.starts_with(wm),
            "{line}: {msg}, expected {wl}: {wm}"
        );
    }
}

/// The BLISS test programs, with the modules of the program directories.
fn programs() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/bliss");
    let mut out = Vec::new();
    for e in fs::read_dir(&dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            out.extend(fs::read_dir(&p).unwrap().map(|e| e.unwrap().path()));
        } else {
            out.push(p);
        }
    }
    out.retain(|p| p.extension().is_some_and(|e| e == "b64" || e == "b32"));
    out.sort();
    out
}

#[test]
fn test_programs_are_clean() {
    for p in programs() {
        let text = fs::read_to_string(&p).unwrap();
        let found = lints(&p, &text).expect("it compiles");
        assert!(found.is_empty(), "{}: {:?}", p.display(), found);
    }
}

/// Where the fetches are in a source: each `.` that isn't in a comment or
/// a string and comes before a name or a parenthesis.
fn dots(text: &str) -> Vec<usize> {
    let b = text.as_bytes();
    let (mut out, mut i) = (Vec::new(), 0);
    while i < b.len() {
        match b[i] {
            b'!' => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'\'' => {
                i += 1;
                while i < b.len() && b[i] != b'\'' {
                    i += 1;
                }
            }
            b'.' if b
                .get(i + 1)
                .is_some_and(|c| c.is_ascii_alphabetic() || b"(.$_".contains(c)) =>
            {
                out.push(i)
            }
            _ => {}
        }
        i += 1;
    }
    out
}

/// Deletes each dot of each test program in turn and counts how many the
/// lint notices: its recall. Deletions that don't compile don't count.
#[test]
fn recall() {
    let (mut caught, mut total) = (0, 0);
    for p in programs() {
        let text = fs::read_to_string(&p).unwrap();
        for at in dots(&text) {
            let mutant = format!("{}{}", &text[..at], &text[at + 1..]);
            let Some(found) = lints(&p, &mutant) else {
                continue;
            };
            total += 1;
            if !found.is_empty() {
                caught += 1;
            }
        }
    }
    let recall = 100 * caught / total.max(1);
    println!("dot lint recall: {caught} of {total} deleted dots, {recall}%");
    // The floor: raise it as the lint improves (rules 5 and 6 to come).
    assert!(
        recall >= RECALL_FLOOR,
        "recall {recall}% fell below {RECALL_FLOOR}%"
    );
}

const RECALL_FLOOR: usize = 35;
