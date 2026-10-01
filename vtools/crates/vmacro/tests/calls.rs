//! Calls and jumps: `bl` and `b` to a plain address, which reach ±128 MB,
//! and through a register to a G^ one, which may be in another image.

use vasm::Dialect;

fn lines(mn: &str, operands: &str) -> Vec<String> {
    let mut m = vmacro::Macro32::default();
    let constant = |e: &str| e.parse().ok();
    let out = m.statement(mn, operands, &constant).unwrap().unwrap();
    out.iter().map(|l| l.trim().to_string()).collect()
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
