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

/// `REI` doesn't come back to the next instruction: the PAL restores R7.
#[test]
fn rei() {
    assert_eq!(lines("REI", ""), ["mov x7, #146", "svc #0"]);
}

/// `CHMK #3`: the code in R0, which brings the service's status back.
#[test]
fn chmk() {
    assert_eq!(
        lines("CHMK", "#3"),
        [
            "movz w14, #3",
            "mov x15, x7",
            "mov w0, w14",
            "mov x7, #131",
            "svc #0",
            "mov x7, x15",
        ]
    );
}

/// The other modes' CHMx are the same, with their own code.
#[test]
fn chmx() {
    assert!(lines("CHME", "#0").contains(&"mov x7, #130".to_string()));
    assert!(lines("CHMS", "#0").contains(&"mov x7, #132".to_string()));
    assert!(lines("CHMU", "#0").contains(&"mov x7, #133".to_string()));
}

/// `PROBEW #3, #8, (R1)`: base, length and mode in R0-R2, which come back
/// with R7 once the result is out of R0. A branch tests the result, x14: Z
/// is set if the mode may not write.
#[test]
fn probew() {
    assert_eq!(
        lines("PROBEW", "#3, #8, (R1)"),
        [
            "movz w14, #3",
            "movz w15, #8",
            "mov x16, x1",
            "mov x17, x0",
            "mov x18, x7",
            "mov x0, x16",
            "mov x16, x1",
            "mov x1, x15",
            "mov x15, x2",
            "mov x2, x14",
            "mov x7, #144",
            "svc #0",
            "mov x14, x0",
            "mov x2, x15",
            "mov x1, x16",
            "mov x7, x18",
            "mov x0, x17",
        ]
    );
}

/// `CALL_PAL #5`, SWPCTX: arguments already in R0-R5, R7 kept.
#[test]
fn call_pal() {
    assert_eq!(
        lines("CALL_PAL", "#5"),
        ["mov x14, x7", "mov x7, #5", "svc #0", "mov x7, x14"]
    );
}

/// The registers the scheduler and the software interrupts use.
#[test]
fn registers() {
    assert!(lines("MFPR", "#16, R1").contains(&"mov x7, #18".to_string()));
    assert!(lines("MTPR", "R2, #17").contains(&"mov x7, #23".to_string()));
    assert!(lines("MTPR", "#3, #20").contains(&"mov x7, #24".to_string()));
    assert!(lines("MFPR", "#21, R0").contains(&"mov x7, #25".to_string()));
}
