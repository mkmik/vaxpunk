//! AMACRO's `EVAX_` built-ins (docs/macro32.md).

use vasm::Dialect;

/// What `mn operands` compiles to in exception code, on vmacro's second
/// pass: code whose stack vmacro doesn't follow.
fn lines(mn: &str, operands: &str) -> Vec<String> {
    let mut m = vmacro::Macro32::default();
    let constant = |e: &str| e.parse().ok();
    for _ in 0..2 {
        m.label("T", false);
        m.statement(".EXCEPTION_ENTRY", "", &constant)
            .unwrap()
            .unwrap();
        let out = m.statement(mn, operands, &constant).unwrap().unwrap();
        if !m.again() {
            return out.iter().map(|l| l.trim().to_string()).collect();
        }
    }
    unreachable!()
}

/// All 64 bits of a register, to and from memory and another register.
#[test]
fn ldq_stq() {
    assert_eq!(lines("EVAX_STQ", "R0, 16(SP)"), ["str x0, [x18, #16]"]);
    assert_eq!(lines("EVAX_LDQ", "R1, 8(R2)"), ["ldr x1, [x19, #8]"]);
    assert_eq!(lines("EVAX_LDQ", "R3, R4"), ["mov x20, x21"]);
}

/// Three-operand built-ins work on all 64 bits, a register's whole x
/// register, and write the third.
#[test]
fn quadword_ops() {
    assert_eq!(
        lines("EVAX_ADDQ", "R2, R3, R4"),
        ["add x14, x19, x20", "mov x21, x14"]
    );
    assert_eq!(
        lines("EVAX_CMPULT", "R2, R3, R4"),
        ["cmp x19, x20", "cset x14, lo", "mov x21, x14"]
    );
    assert_eq!(lines("EVAX_BLT", "R2, 10$"), ["tbnz x19, #63, 10$"]);
    // R2 as it was before (R2)+ moves it, as on the VAX.
    assert_eq!(lines("EVAX_ADDQ", "R2, (R2)+, R3")[0], "mov x14, x19");
    assert_eq!(
        lines("EVAX_CMOVEQ", "R2, R3, R4"),
        ["cmp x19, #0", "csel x21, x20, x21, eq"]
    );
}

/// The PAL built-ins: R0 has the result, as on Alpha.
#[test]
fn pal() {
    assert_eq!(
        lines("EVAX_MTPR_IPL", "R2"),
        ["mov x0, x19", "mov x7, #15", "svc #0"]
    );
    assert_eq!(lines("EVAX_MFPR_PCBB", ""), ["mov x7, #18", "svc #0"]);
}

/// Alpha's built-ins with no ARM64 meaning here are errors that name them.
#[test]
fn unsupported() {
    let mut m = vmacro::Macro32::default();
    let constant = |e: &str| e.parse().ok();
    m.label("T", false);
    m.statement(".EXCEPTION_ENTRY", "", &constant);
    for mn in ["EVAX_EXTBL", "EVAX_TRAPB", "EVAX_RPCC", "EVAX_MTPR_ASTEN"] {
        let e = m
            .statement(mn, "R0, R1, R2", &constant)
            .unwrap()
            .unwrap_err();
        assert!(e.starts_with(&format!("{mn} isn't a built-in here")), "{e}");
    }
}
