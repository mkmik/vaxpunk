//! Privileged instructions become PAL calls (docs/design/0001-pal-interface.md
//! at the repository root): `svc #0` with the function code in x7.

use vasm::Dialect;

fn lines(mn: &str, operands: &str) -> Vec<String> {
    let mut m = vmacro::Macro32::default();
    let constant = |e: &str| e.parse().ok();
    let out = m.statement(mn, operands, &constant).unwrap().unwrap();
    out.iter().map(|l| l.trim().to_string()).collect()
}

/// `MTPR #8, #PR$_IPL`, the example in DESIGN-0001.
#[test]
fn mtpr() {
    assert_eq!(
        lines("MTPR", "#8, #18"),
        [
            "movz w14, #8",
            "mov x15, x0",
            "mov x16, x7",
            "mov w0, w14",
            "mov x7, #15",
            "svc #0",
            "mov x17, x0",
            "mov x7, x16",
            "mov x0, x15",
        ]
    );
}

/// `MFPR #PR$_IPL, R7`: the result lands in R7 after R7 is back.
#[test]
fn mfpr() {
    assert_eq!(
        lines("MFPR", "#18, R7"),
        [
            "mov x14, x0",
            "mov x15, x7",
            "mov x7, #14",
            "svc #0",
            "mov x16, x0",
            "mov x7, x15",
            "mov x0, x14",
            "mov w7, w16",
        ]
    );
}

#[test]
fn halt() {
    assert!(lines("HALT", "").contains(&"mov x7, #0".to_string()));
}
