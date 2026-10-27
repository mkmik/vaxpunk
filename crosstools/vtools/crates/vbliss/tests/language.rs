//! Constructs of the language that the test programs in tests/bliss don't
//! show: what vbliss accepts, what it rejects and with which message, and
//! what it makes of them in the IR (t8y:/host/vbliss).

fn translate(source: &str) -> vbliss::Output {
    vbliss::translate(
        std::path::Path::new("language.b64"),
        source,
        &vbliss::Options::default(),
    )
}

/// A module with `decls` and a routine M whose body is `body`.
fn module(decls: &str, body: &str) -> String {
    format!(
        "MODULE T (MAIN = M) =\nBEGIN\n{decls}\nROUTINE M =\n    BEGIN\n{body}\n    1\n    END;\nEND\nELUDOM\n"
    )
}

/// The IR of a module that must compile.
fn ir(source: &str) -> String {
    let out = translate(source);
    assert!(out.error().is_none(), "{:?}", out.error());
    out.ir.unwrap()
}

/// The first error's message of a module that must not compile.
fn error(source: &str) -> String {
    translate(source)
        .error()
        .map(|d| d.msg.clone())
        .expect("compiled")
}

#[test]
fn up_level_local() {
    let src = module(
        "ROUTINE OUTER =\n    BEGIN\n    LOCAL L;\n    ROUTINE INNER = .L;\n    INNER()\n    END;",
        "OUTER();",
    );
    assert_eq!(error(&src), "Illegal up-level reference to L");
    // A formal is the routine's own, an OWN anyone's.
    ir(&module(
        "ROUTINE OUTER (F) =\n    BEGIN\n    OWN O;\n    ROUTINE INNER = .O;\n    .F + INNER()\n    END;",
        "OUTER(1);",
    ));
}

#[test]
fn dec_mcs_source() {
    // read_source reads a byte a character: an e acute is 233 in a string.
    let path = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("mcs.b64");
    let mut bytes = module("OWN X;", "X = %C'?';").into_bytes();
    let q = bytes.iter().position(|&b| b == b'?').unwrap();
    bytes[q] = 0xE9;
    std::fs::write(&path, &bytes).unwrap();
    let text = vbliss::read_source(&path).unwrap();
    assert!(ir(&text).contains("233"));
    assert_eq!(vbliss::latin1(&text), bytes);
}
