//! Macro errors and rules that tests/run/macros.mar can't show by running.

use vasm::{Diagnostic, Options};

fn errors(source: &str) -> Vec<Diagnostic> {
    let opts = Options {
        name: "T".into(),
        date: *b"25-SEP-2026 00:00",
        ..Default::default()
    };
    vasm::assemble(source, &opts).err().unwrap_or_default()
}

#[test]
fn errors_show_the_expansion() {
    let source = "
        .MACRO  INNER   REG
        add     REG, REG, #5000
        .ENDM
        .MACRO  OUTER
        INNER   x1
        .ENDM
        OUTER
";
    let e = errors(source);
    assert_eq!(e.len(), 1, "{e:?}");
    let d = &e[0];
    assert_eq!(
        (d.line, d.msg.as_str()),
        (3, "immediate 5000 out of range 0 to 4095")
    );
    assert_eq!(d.text.trim(), "add     x1, x1, #5000", "the expanded line");
    assert_eq!(
        d.context,
        [
            "in macro INNER, at <source>:6",
            "in macro OUTER, at <source>:8"
        ]
    );
}

#[test]
fn block_errors() {
    let e = errors(".MACRO M\nnop\n");
    assert!(e[0].msg.contains("missing .ENDM for macro M"), "{e:?}");
    let e = errors(".IF EQ 0\nnop\n");
    assert!(e[0].msg.contains("missing .ENDC"), "{e:?}");
    let e = errors(".ENDC\n");
    assert!(e[0].msg.contains(".ENDC without .IF"), "{e:?}");
    let e = errors(".MACRO M\n.IF EQ 0\n.ENDM\nM\n");
    assert!(e[0].msg.contains("missing .ENDC before the end"), "{e:?}");
    let e = errors(".MACRO LOOP\nLOOP\n.ENDM\nLOOP\n");
    assert!(e[0].msg.contains("nested more than 100 deep"), "{e:?}");
    let e = errors(".MACRO M A\n.ENDM\nM 1, 2\n");
    assert!(e[0].msg.contains("too many arguments for macro M"), "{e:?}");
    let e = errors(".LIBRARY \"nowhere.mlb\"\n");
    assert!(
        e[0].msg.contains("macro library nowhere.mlb not found"),
        "{e:?}"
    );
}

#[test]
fn redefinition() {
    assert!(
        errors("X = 1\nX = X + 1\n.QUAD X\n").is_empty(),
        "assignments can be redefined"
    );
    let e = errors("L: nop\nL = 1\n");
    assert!(e[0].msg.contains("L is already defined"), "{e:?}");
    let e = errors("X = 1\nX: nop\n");
    assert!(e[0].msg.contains("X is already defined"), "{e:?}");
}

#[test]
fn skipped_code_is_not_assembled() {
    // Bad code inside a false condition, and nested conditionals in it.
    assert!(errors(".IF NE 0\nfrob\n.IF EQ 0\nfrob\n.ENDC\n.ENDC\n").is_empty());
    assert!(errors(".IF DF NOPE\nfrob\n.ELSE\nnop\n.ENDC\n").is_empty());
}
