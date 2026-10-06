//! AMACRO's `EVAX_` built-ins (docs/macro32.md).

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

/// All 64 bits of a register, to and from memory and another register.
#[test]
fn ldq_stq() {
    assert_eq!(lines("EVAX_STQ", "R0, 16(SP)"), ["str x0, [x18, #16]"]);
    assert_eq!(lines("EVAX_LDQ", "R1, 8(R2)"), ["ldr x1, [x19, #8]"]);
    assert_eq!(lines("EVAX_LDQ", "R3, R4"), ["mov x20, x21"]);
}
