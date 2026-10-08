//! Calls and jumps: `bl` and `b` to an address, `G^` or not, which the
//! linker sends through a veneer when the target is out of their ±128 MB,
//! as in another image, and through a register to any other operand.

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
    assert!(lines("JSB", "PUT").contains(&"bl PUT".to_string()));
}

#[test]
fn general() {
    assert!(lines("CALLS", "#0, G^SYS$EXIT").contains(&"bl SYS$EXIT".to_string()));
    assert!(lines("JSB", "G^EXE$OUTCHAR").contains(&"bl EXE$OUTCHAR".to_string()));
    assert!(lines("JMP", "G^EXE$DELSELF").contains(&"b EXE$DELSELF".to_string()));
    // A constant address is reached through a register.
    assert!(
        lines("JSB", "G^4096")
            .iter()
            .any(|l| l.starts_with("blr x"))
    );
}

/// `.ENTRY` builds DESIGN-0004's frame and its descriptor, which says what
/// it saves: x18, and here R2 and R3 (x19 and x20), at 32 in 64 bytes.
#[test]
fn entry_frame() {
    let mut m = vmacro::Macro32::default();
    let constant = |e: &str| e.parse().ok();
    m.statement(".ENTRY", "START, 12", &constant);
    assert!(m.again());
    let code = m
        .statement(".ENTRY", "START, 12", &constant)
        .unwrap()
        .unwrap();
    let code: Vec<String> = code.iter().map(|l| l.trim().to_string()).collect();
    for line in [
        "FDSC$$0:\t.LONG 0, 7, 32, 64",
        "stp x29, x30, [sp, #-64]!",
        "stp xzr, x16, [sp, #16]",
        "mov x29, sp",
        "stp x18, x19, [x29, #32]",
        "str x20, [x29, #48]",
    ] {
        assert!(code.contains(&line.to_string()), "{line}: {code:?}");
    }
}

/// `(FP)` is the condition handler, at 16 in the frame; the rest of the
/// frame isn't the source's.
#[test]
fn handler() {
    let mut m = vmacro::Macro32::default();
    let constant = |e: &str| e.parse().ok();
    for _ in 0..2 {
        m.statement(".ENTRY", "START, 0", &constant);
        let code = m.statement("MOVL", "R1, (FP)", &constant).unwrap().unwrap();
        let bad = m.statement("MOVL", "R1, 8(FP)", &constant).unwrap();
        let local = m
            .statement("MOVL", "R1, -4(FP)", &constant)
            .unwrap()
            .unwrap();
        if !m.again() {
            assert!(
                code.iter().any(|l| l.trim() == "str w1, [x29, #16]"),
                "{code:?}"
            );
            assert!(bad.is_err());
            assert!(
                local.iter().any(|l| l.trim() == "str w1, [x29, #-4]"),
                "{local:?}"
            );
            return;
        }
    }
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

/// A CALL routine with nothing to keep is frameless (DESIGN-0004): no
/// prologue, and `RET` is `ret`. One that uses FP, as a handler's
/// establisher does, has a frame.
#[test]
fn frameless() {
    let constant = |e: &str| e.parse().ok();
    let compile = |lines: &[(&str, &str)]| {
        let mut m = vmacro::Macro32::default();
        loop {
            let mut code = Vec::new();
            for (mn, operands) in lines {
                code.extend(m.statement(mn, operands, &constant).unwrap().unwrap());
            }
            if !m.again() {
                return code
                    .iter()
                    .map(|l| l.trim().to_string())
                    .collect::<Vec<_>>();
            }
        }
    };
    let leaf = compile(&[(".ENTRY", "LEAF, 0"), ("MOVL", "#1, R0"), ("RET", "")]);
    assert_eq!(leaf, ["LEAF::", "movz x0, #1", "tst w0, w0", "ret"]);
    let framed = compile(&[(".ENTRY", "OWN, 0"), ("CLRL", "(FP)"), ("RET", "")]);
    assert!(
        framed.iter().any(|l| l.starts_with("stp x29, x30")),
        "{framed:?}"
    );
}
