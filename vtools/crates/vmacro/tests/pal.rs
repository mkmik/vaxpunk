//! Privileged instructions become PAL calls (docs/design/0001-pal-interface.md
//! at the repository root): `svc #0` with the function code in x7.

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

/// `MTPR #8, #PR$_IPL`, the example in DESIGN-0001.
#[test]
fn mtpr() {
    assert_eq!(
        lines("MTPR", "#8, #18"),
        [
            "movz w14, #8",
            "mov x15, x0",
            "mov w0, w14",
            "mov x7, #15",
            "svc #0",
            "mov x16, x0",
            "mov x0, x15",
        ]
    );
}

/// `MFPR #PR$_IPL, R7`: the result lands in R7, x24, sign-extended.
#[test]
fn mfpr() {
    assert_eq!(
        lines("MFPR", "#18, R7"),
        [
            "mov x14, x0",
            "mov x7, #14",
            "svc #0",
            "mov x15, x0",
            "mov x0, x14",
            "sxtw x24, w15",
        ]
    );
}

#[test]
fn halt() {
    assert!(lines("HALT", "").contains(&"mov x7, #0".to_string()));
}

/// `REI` doesn't come back to the next instruction: the PAL restores every
/// register from the frame.
#[test]
fn rei() {
    assert_eq!(lines("REI", ""), ["mov x7, #146", "svc #0"]);
}

/// `CHMK #3`: the code in R0, which brings the service's status back.
#[test]
fn chmk() {
    assert_eq!(
        lines("CHMK", "#3"),
        ["movz w14, #3", "mov w0, w14", "mov x7, #131", "svc #0",]
    );
}

/// The other modes' CHMx are the same, with their own code.
#[test]
fn chmx() {
    assert!(lines("CHME", "#0").contains(&"mov x7, #130".to_string()));
    assert!(lines("CHMS", "#0").contains(&"mov x7, #132".to_string()));
    assert!(lines("CHMU", "#0").contains(&"mov x7, #133".to_string()));
}

/// `PROBEW #3, #8, (R1)`: base, length and mode in x0-x2; R0 and R1 come
/// back once the result is out of x0. A branch tests the result, x14: Z is
/// set if the mode may not write.
#[test]
fn probew() {
    assert_eq!(
        lines("PROBEW", "#3, #8, (R1)"),
        [
            "movz w14, #3",
            "movz w15, #8",
            "mov x16, x1",
            "mov x17, x0",
            "mov x10, x1",
            "mov x0, x16",
            "mov x1, x15",
            "mov x2, x14",
            "mov x7, #144",
            "svc #0",
            "mov x14, x0",
            "mov x1, x10",
            "mov x0, x17",
        ]
    );
}

/// `CALL_PAL #5`, SWPCTX: arguments in R0-R5, which the PAL takes in x0-x5.
#[test]
fn call_pal() {
    assert_eq!(
        lines("CALL_PAL", "#5"),
        [
            "mov x2, x19",
            "mov x3, x20",
            "mov x4, x21",
            "mov x5, x22",
            "mov x7, #5",
            "svc #0",
        ]
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
