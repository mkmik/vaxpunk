//! NOTPIC warnings: a PIC psect holding an address that the loader would
//! have to fix up, or couldn't.

use std::fs;
use std::path::Path;
use std::process::Command;

const SOURCE: [&str; 11] = [
    "        .EXTERNAL EXT",
    "        .PSECT  $CODE$",
    "START:: .ADDRESS START, EXT            ; EXT may be a constant",
    "        .LONG   START - .",
    "        movz    x0, #:abs_g1:START",
    "        movk    x0, #:abs_g0_nc:START  ; the low 16 bits don't move",
    "        .PSECT  $LITERAL$",
    "        .ASCID  /text/",
    "        .PSECT  $DATA$                 ; not PIC",
    "        .ADDRESS START",
    "        .ASCID  /text/",
];

#[test]
fn notpic() {
    let opts = vasm::Options {
        name: "T".into(),
        date: *b"25-SEP-2026 00:00",
        ..Default::default()
    };
    let object = vasm::assemble(&SOURCE.join("\n"), &opts).unwrap_or_else(|d| panic!("{d:?}"));
    let got: Vec<_> = object
        .warnings
        .into_iter()
        .map(|d| (d.line, d.col, d.msg))
        .collect();
    // Line, what the caret points at, and the message.
    let want: Vec<_> = [
        (3, "START,", "address needing a fixup in PIC psect $CODE$"),
        (5, "#:abs_g1", "absolute address bits in PIC psect $CODE$"),
        (
            8,
            "/text/",
            "descriptor pointer needing a fixup in PIC psect $LITERAL$",
        ),
    ]
    .into_iter()
    .map(|(line, at, msg)| {
        let col = SOURCE[line - 1].rfind(at).unwrap() + 1;
        (line, col, format!("%VASM-W-NOTPIC, {msg}"))
    })
    .collect();
    assert_eq!(got, want);
}

#[test]
fn nowarnings() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"));
    let source = dir.join("notpic.mar");
    fs::write(&source, SOURCE.join("\n")).unwrap();
    let vasm = |qualifiers: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_vasm"))
            .args(qualifiers)
            .arg("-o")
            .arg(dir.join("notpic.obj"))
            .arg(&source)
            .output()
            .unwrap()
    };
    let out = vasm(&[]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{stderr}");
    assert_eq!(
        stderr.matches(": warning: %VASM-W-NOTPIC").count(),
        3,
        "{stderr}"
    );
    for quiet in [&["/NOWARNINGS=NOTPIC"][..], &["--nowarnings", "notpic"]] {
        let out = vasm(quiet);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            out.status.success() && stderr.is_empty(),
            "{quiet:?}: {stderr}"
        );
    }
    assert!(!vasm(&["/NOWARNINGS=OTHER"]).status.success());
}
