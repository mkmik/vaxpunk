//! Calls and jumps: `bl` and `b` to a plain address, which reach ±128 MB,
//! and through a register to a G^ one, which may be in another image.

use vasm::Dialect;

/// What `mn operands` compiles to in a JSB routine, on vmacro's second pass.
fn lines(mn: &str, operands: &str) -> Vec<String> {
    let mut m = vmacro::Macro32::default();
    let constant = |e: &str| e.parse().ok();
    for _ in 0..2 {
        m.label("T", false);
        m.statement(".JSB32_ENTRY", "", &constant).unwrap().unwrap();
        let out = m.statement(mn, operands, &constant).unwrap().unwrap();
        if !m.again() {
            return out.iter().map(|l| l.trim().to_string()).collect();
        }
    }
    unreachable!()
}

#[test]
fn near() {
    assert!(lines("CALLS", "#0, START").contains(&"bl START".to_string()));
    assert!(lines("JSB", "PUT").contains(&"b PUT".to_string()));
}

/// Whether `code` takes the address of `sym` with `adrp`, ±4 GB, and then
/// jumps with `op` to a register.
fn far(code: &[String], sym: &str, op: &str) -> bool {
    code.iter()
        .any(|l| l.starts_with("adrp") && l.ends_with(sym))
        && code.iter().any(|l| l.starts_with(&format!("{op} x")))
}

#[test]
fn general() {
    assert!(far(&lines("CALLS", "#1, G^SYS$EXIT"), "SYS$EXIT", "blr"));
    assert!(far(&lines("JSB", "G^EXE$OUTCHAR"), "EXE$OUTCHAR", "br"));
    assert!(far(&lines("JMP", "G^EXE$DELSELF"), "EXE$DELSELF", "br"));
}

/// `.ENTRY` keeps its mask in the frame, at 40, where `$UNWIND` reads which
/// registers the frame saved.
#[test]
fn entry_mask() {
    let mut m = vmacro::Macro32::default();
    let constant = |e: &str| e.parse().ok();
    m.statement(".ENTRY", "START, 12", &constant);
    assert!(m.again());
    let code = m
        .statement(".ENTRY", "START, 12", &constant)
        .unwrap()
        .unwrap();
    let code: Vec<String> = code.iter().map(|l| l.trim().to_string()).collect();
    assert!(code.contains(&"mov x14, #12".to_string()));
    assert!(code.contains(&"stp x18, x14, [sp, #32]".to_string()));
}

/// `PUSHAL (SP)` and `PUSHL SP` push SP through a scratch register: a
/// writeback store of x18 through x18 traps on Apple's cores.
#[test]
fn push_sp() {
    for (mn, op) in [("PUSHAL", "(SP)"), ("PUSHL", "SP")] {
        let code = lines(mn, op);
        assert!(
            !code.iter().any(|l| l.starts_with("str w18")),
            "{mn} {op}: {code:?}"
        );
    }
}
