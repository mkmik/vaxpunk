//! What each VAX instruction compiles to, checked where the ARM64 carries
//! the VAX meaning: which operand is the source and which the result, the
//! operand size, sign and zero extension, what a register keeps of its old
//! value, and the condition codes a branch after it sees.
//!
//! R0 and R1 are x0 and x1, R2-R11 x19-x28, AP x12, FP x29 and SP x18
//! (DESIGN-0004); vmacro's scratch registers start at x14.

use vasm::Dialect;

/// What the last of `statements` compiles to, in a routine that `entry`
/// declares, on vmacro's second pass; or the error it gives.
fn compile_in(entry: &str, statements: &[(&str, &str)]) -> Result<Vec<String>, String> {
    let mut m = vmacro::Macro32::default();
    let constant = |e: &str| e.parse().ok();
    let (word, rest) = entry.split_once(' ').unwrap_or((entry, ""));
    loop {
        m.label("T", false);
        m.statement(word, rest, &constant).unwrap().unwrap();
        let mut out = Ok(Vec::new());
        for (mn, operands) in statements {
            out = m.statement(mn, operands, &constant).unwrap();
        }
        if !m.again() {
            return out.map(|o| o.iter().map(|l| l.trim().to_string()).collect());
        }
    }
}

/// What `mn operands` compiles to in a JSB routine.
fn lines(mn: &str, operands: &str) -> Vec<String> {
    compile_in(".JSB32_ENTRY", &[(mn, operands)]).unwrap_or_else(|e| panic!("{mn} {operands}: {e}"))
}

/// The error `mn operands` gives in a JSB routine.
fn error(mn: &str, operands: &str) -> String {
    compile_in(".JSB32_ENTRY", &[(mn, operands)])
        .err()
        .unwrap_or_else(|| panic!("{mn} {operands} compiles"))
}

/// What the last of `statements` compiles to in a JSB routine.
fn after(statements: &[(&str, &str)]) -> Vec<String> {
    compile_in(".JSB32_ENTRY", statements).unwrap_or_else(|e| panic!("{statements:?}: {e}"))
}

/// Asserts that `code` has each of `want`, in order, not necessarily
/// next to each other.
#[track_caller]
fn has(code: &[String], want: &[&str]) {
    let mut rest = code.iter();
    for w in want {
        assert!(rest.any(|l| l == w), "{w:?} isn't in order in {code:#?}");
    }
}

/// A register holds its longword sign-extended to 64 bits, so a byte or
/// word written to one keeps the rest of its old longword.
#[test]
fn moves() {
    assert_eq!(lines("MOVL", "#5, R2"), ["movz x19, #5"]);
    // -70000 is ^XFFFEEE90: MOVN for the ones, sign-extended.
    assert_eq!(
        lines("MOVL", "#-70000, R2"),
        ["movn x19, #4463", "movk x19, #65534, lsl #16"]
    );
    assert_eq!(
        lines("MOVB", "R1, R2"),
        ["bfi w19, w1, #0, #8", "sxtw x19, w19"]
    );
    assert_eq!(
        lines("MOVW", "(R3), 4(R4)"),
        ["ldrh w14, [x20]", "strh w14, [x21, #4]"]
    );
    // A literal is cut to the operand's size.
    assert_eq!(lines("MOVB", "#300, R2")[0], "movz w14, #44");
    assert_eq!(lines("MOVW", "#70000, R2")[0], "movz w14, #4464");
    assert_eq!(
        lines("MOVW", "#0, R2"),
        ["bfi w19, wzr, #0, #16", "sxtw x19, w19"]
    );
    // R2:R3 to R4:R5, R2 the low longword.
    assert_eq!(
        lines("MOVQ", "R2, R4"),
        [
            "mov w14, w19",
            "bfi x14, x20, #32, #32",
            "sxtw x21, w14",
            "asr x22, x14, #32"
        ]
    );
    assert_eq!(lines("CLRL", "R2"), ["mov x19, #0"]);
    assert_eq!(lines("CLRQ", "(R3)"), ["str xzr, [x20]"]);
    // A value the linker knows comes from a longword in $LINK$, which the
    // loader fixes up if the image moves.
    let sym = lines("MOVL", "#SYM, R2");
    has(
        &sym,
        &[
            ".PSECT $LINK$",
            "90001$:\t.LONG SYM",
            "adrp x19, 90001$",
            "ldr w19, [x19, #:lo12:90001$]",
            "sxtw x19, w19",
        ],
    );
    has(&lines("MOVQ", "#SYM, R2"), &["90001$:\t.QUAD SYM"]);
}

/// MOVZ zero-extends, CVT sign-extends up and truncates down.
#[test]
fn conversions() {
    assert_eq!(
        lines("MOVZBL", "(R3), R2"),
        ["ldrb w14, [x20]", "sxtw x19, w14"]
    );
    assert_eq!(
        lines("MOVZWL", "R1, R2"),
        ["ubfx w14, w1, #0, #16", "sxtw x19, w14"]
    );
    assert_eq!(
        lines("CVTBL", "R1, R2"),
        ["sbfx w14, w1, #0, #8", "sxtw x19, w14"]
    );
    assert_eq!(
        lines("CVTLB", "R1, R2"),
        ["bfi w19, w1, #0, #8", "sxtw x19, w19"]
    );
    assert_eq!(lines("CVTWL", "R1, R2")[0], "sbfx w14, w1, #0, #16");
}

/// MOVA and PUSHA take the operand's address: a longword, or all 64 bits
/// with .ENABLE QUADWORD.
#[test]
fn addresses() {
    assert_eq!(
        lines("MOVAL", "8(R3), R2"),
        ["add x14, x20, #8", "sxtw x19, w14"]
    );
    assert_eq!(
        lines("MOVAB", "(R3)[R4], R2"),
        ["add x14, x20, w21, sxtw", "sxtw x19, w14"]
    );
    assert_eq!(
        lines("PUSHAL", "8(R3)"),
        ["add x14, x20, #8", "str w14, [x18, #-4]!"]
    );
    assert_eq!(lines("PUSHL", "R2"), ["str w19, [x18, #-4]!"]);
    assert_eq!(
        after(&[(".ENABLE", "QUADWORD"), ("MOVAL", "8(R3), R2")]),
        ["add x14, x20, #8", "mov x19, x14"]
    );
}

/// The two-operand form's second operand is also the result; in the
/// three-operand form the second is the minuend and the divided.
#[test]
fn arithmetic() {
    assert_eq!(
        lines("ADDL3", "#1, R1, R2"),
        ["adds w19, w1, #1", "sxtw x19, w19"]
    );
    assert_eq!(
        lines("ADDL2", "R1, R2"),
        ["adds w19, w19, w1", "sxtw x19, w19"]
    );
    // R3 = R2 - R1, R3 = R2 / R1.
    assert_eq!(
        lines("SUBL3", "R1, R2, R3"),
        ["subs w20, w19, w1", "sxtw x20, w20"]
    );
    assert_eq!(
        lines("DIVL3", "R1, R2, R3"),
        ["sdiv w20, w19, w1", "sxtw x20, w20"]
    );
    assert_eq!(lines("MULL3", "R1, R2, R3")[0], "mul w20, w19, w1");
    assert_eq!(lines("XORL3", "R1, R2, R3")[0], "eor w20, w19, w1");
    assert_eq!(lines("BISL2", "#8, R2")[1], "orr w19, w19, w14");
    // BIC clears the bits the mask has.
    assert_eq!(
        lines("BICL2", "#8, R2"),
        ["movz w14, #8", "bics w19, w19, w14", "sxtw x19, w19"]
    );
    assert_eq!(lines("BICW2", "#8, R2")[1], "bic w14, w19, w14");
    // A byte sum is merged into the register's low byte.
    assert_eq!(
        lines("ADDB2", "R1, R2"),
        [
            "sbfx w14, w1, #0, #8",
            "add w14, w19, w14",
            "bfi w19, w14, #0, #8",
            "sxtw x19, w19"
        ]
    );
    assert_eq!(lines("SUBW2", "#1, R2")[1], "sub w14, w19, w14");
    // A byte division is signed: both operands sign-extended.
    has(
        &lines("DIVB2", "R1, R2"),
        &[
            "sbfx w14, w1, #0, #8",
            "sbfx w15, w19, #0, #8",
            "sdiv w14, w15, w14",
        ],
    );
    assert_eq!(lines("MNEGL", "R1, R2"), ["negs w19, w1", "sxtw x19, w19"]);
    assert_eq!(lines("MNEGB", "R1, R2")[0], "neg w14, w1");
    assert_eq!(lines("MCOML", "R1, R2")[0], "mvn w19, w1");
    assert_eq!(lines("INCL", "R2"), ["adds w19, w19, #1", "sxtw x19, w19"]);
    assert_eq!(lines("INCB", "R2")[0], "add w14, w19, #1");
    assert_eq!(lines("DECL", "R2")[0], "subs w19, w19, #1");
    assert_eq!(
        lines("DECW", "(R3)"),
        ["ldrh w14, [x20]", "sub w14, w14, #1", "strh w14, [x20]"]
    );
}

/// Operands are evaluated in order: an autoincrement in the first moves
/// the register before the second reads it, and a source is read before
/// an autodecrement in the destination moves it.
#[test]
fn operand_order() {
    assert_eq!(
        lines("ADDL2", "(R2)+, R2"),
        [
            "mov x14, x19",
            "add x19, x19, #4",
            "ldr w14, [x14]",
            "mov w15, w19",
            "adds w19, w15, w14",
            "sxtw x19, w19"
        ]
    );
    assert_eq!(
        lines("MOVL", "R2, -(R2)"),
        ["mov w14, w19", "sub x19, x19, #4", "str w14, [x19]"]
    );
}

/// CMP compares the first with the second, signed, the sizes below a
/// longword sign-extended; TST and BIT set N and Z.
#[test]
fn compares() {
    assert_eq!(lines("CMPL", "R1, R2"), ["cmp w1, w19"]);
    assert_eq!(lines("CMPL", "R1, #0"), ["cmp w1, #0"]);
    // CMP's first operand can't be the zero register.
    assert_eq!(lines("CMPL", "#0, R1"), ["mov w14, #0", "cmp w14, w1"]);
    assert_eq!(
        lines("CMPB", "R1, #3"),
        ["sbfx w14, w1, #0, #8", "movz w15, #3", "cmp w14, w15"]
    );
    // #255 as a byte is -1.
    assert_eq!(
        lines("CMPB", "#255, R1"),
        ["movn w14, #0", "sbfx w15, w1, #0, #8", "cmp w14, w15"]
    );
    assert_eq!(lines("TSTL", "R2"), ["tst w19, w19"]);
    assert_eq!(lines("TSTW", "(R3)"), ["ldrsh w14, [x20]", "tst w14, w14"]);
    assert_eq!(lines("BITL", "#4, R2"), ["movz w14, #4", "tst w14, w19"]);
}

/// The branch a VAX conditional branch becomes depends on what set the
/// codes: after a subtraction ARM64's C is VAX's inverted, and after an
/// addition BGTRU and BLEQU need two conditions. A move's codes are set
/// from its value just before the branch.
#[test]
fn condition_codes() {
    let branch = |first: (&str, &str), mn: &str| after(&[first, (mn, "10$")]);
    let tst = ("TSTL", "R1");
    for (mn, b) in [
        ("BNEQ", "b.ne"),
        ("BNEQU", "b.ne"),
        ("BEQL", "b.eq"),
        ("BEQLU", "b.eq"),
        ("BGTR", "b.gt"),
        ("BLEQ", "b.le"),
        ("BGEQ", "b.ge"),
        ("BLSS", "b.lt"),
        ("BVC", "b.vc"),
        ("BVS", "b.vs"),
    ] {
        assert_eq!(branch(tst, mn), [format!("{b} 10$")], "{mn}");
    }
    let cmp = ("CMPL", "R1, R2");
    for (mn, b) in [
        ("BCC", "b.hs"),
        ("BGEQU", "b.hs"),
        ("BCS", "b.lo"),
        ("BLSSU", "b.lo"),
        ("BGTRU", "b.hi"),
        ("BLEQU", "b.ls"),
    ] {
        assert_eq!(branch(cmp, mn), [format!("{b} 10$")], "{mn}");
    }
    let add = ("ADDL2", "R1, R2");
    assert_eq!(branch(add, "BCC"), ["b.cc 10$"]);
    assert_eq!(branch(add, "BLSSU"), ["b.cs 10$"]);
    // Higher: C and Z both clear.
    assert_eq!(branch(add, "BGTRU"), ["b.cs 90001$", "b.ne 10$", "90001$:"]);
    assert_eq!(branch(add, "BLEQU"), ["b.cs 10$", "b.eq 10$"]);
    assert_eq!(
        branch(("MOVB", "R1, R2"), "BNEQ"),
        ["sbfx w16, w1, #0, #8", "tst w16, w16", "b.ne 10$"]
    );
    assert_eq!(
        branch(("MOVL", "R1, R2"), "BLSS"),
        ["tst w1, w1", "b.lt 10$"]
    );
    assert_eq!(
        branch(("MOVQ", "R2, R4"), "BEQL"),
        ["tst x14, x14", "b.eq 10$"]
    );
}

#[test]
fn branches() {
    assert_eq!(lines("BLBS", "R2, 10$"), ["tbnz w19, #0, 10$"]);
    assert_eq!(lines("BLBC", "R2, 10$"), ["tbz w19, #0, 10$"]);
    assert_eq!(lines("BRB", "10$"), ["b 10$"]);
    assert_eq!(lines("BRW", "10$"), ["b 10$"]);
    assert_eq!(lines("JMP", "(R3)"), ["br x20"]);
    assert_eq!(lines("JMP", "@(R3)"), ["ldr w14, [x20]", "br x14"]);
    // ARM64's SP must be 16-aligned at a call.
    assert_eq!(
        lines("JSB", "(R3)"),
        ["and sp, x18, #0xfffffffffffffff0", "blr x20"]
    );
    assert_eq!(lines("RSB", ""), ["ret"]);
    assert_eq!(error("BNEQ", "R1"), "expected a branch destination");
}

/// ACB branches while the index hasn't passed the limit: up to it for a
/// step that isn't negative, down to it for one that is, which for a step
/// in a register is known when it runs.
#[test]
fn loops() {
    assert_eq!(
        lines("ACBL", "#10, #1, R2, 10$"),
        [
            "movz w14, #1",
            "add w19, w19, w14",
            "sxtw x19, w19",
            "cmp w19, #10",
            "b.le 10$"
        ]
    );
    assert_eq!(
        lines("ACBL", "#10, #-1, R2, 10$").last().unwrap(),
        "b.ge 10$"
    );
    has(
        &lines("ACBL", "#10, R3, R2, 10$"),
        &[
            "add w19, w19, w20",
            "tbnz w20, #31, 90001$",
            "b.le 10$",
            "90001$:",
            "b.ge 10$",
        ],
    );
    // A word index is compared as a word.
    has(
        &lines("ACBW", "#10, #1, R2, 10$"),
        &[
            "bfi w19, w16, #0, #16",
            "sbfx w16, w16, #0, #16",
            "cmp w16, w14",
        ],
    );
    assert_eq!(
        lines("AOBLSS", "#10, R2, 10$"),
        [
            "add w19, w19, #1",
            "sxtw x19, w19",
            "cmp w19, #10",
            "b.lt 10$"
        ]
    );
    assert_eq!(lines("AOBLEQ", "#10, R2, 10$").last().unwrap(), "b.le 10$");
    assert_eq!(
        lines("SOBGTR", "R2, 10$"),
        ["subs w19, w19, #1", "sxtw x19, w19", "b.gt 10$"]
    );
    assert_eq!(lines("SOBGEQ", "R2, 10$").last().unwrap(), "b.ge 10$");
}

/// ASH shifts left for a positive count and right, arithmetically, for a
/// negative one; ROTL rotates left.
#[test]
fn shifts() {
    assert_eq!(
        lines("ASHL", "#3, R1, R2"),
        ["lsl w19, w1, #3", "sxtw x19, w19"]
    );
    assert_eq!(lines("ASHL", "#-3, R1, R2")[0], "asr w19, w1, #3");
    // Shifted out entirely: 0 to the left, the sign to the right.
    assert_eq!(lines("ASHL", "#40, R1, R2")[0], "mov w19, #0");
    assert_eq!(lines("ASHL", "#-40, R1, R2")[0], "asr w19, w1, #31");
    has(
        &lines("ASHL", "R3, R1, R2"),
        &[
            "sbfx w14, w20, #0, #8",
            "tbnz w14, #31, 90001$",
            "lsl w19, w1, w14",
            "neg w15, w14",
            "asr w19, w1, w15",
        ],
    );
    assert_eq!(
        lines("ASHQ", "#3, R2, R4"),
        [
            "mov w14, w19",
            "bfi x14, x20, #32, #32",
            "lsl x14, x14, #3",
            "sxtw x21, w14",
            "asr x22, x14, #32"
        ]
    );
    assert_eq!(lines("ROTL", "#8, R1, R2")[0], "ror w19, w1, #24");
    has(
        &lines("ROTL", "R3, R1, R2"),
        &["neg w15, w14", "ror w19, w1, w15"],
    );
}

/// EMUL: R4:R5 = R1 * R2 + R3; EDIV: R4 = R2:R3 / R1, R5 the remainder.
#[test]
fn extended() {
    assert_eq!(
        lines("EMUL", "R1, R2, R3, R4"),
        [
            "smull x14, w1, w19",
            "add x14, x14, w20, sxtw",
            "sxtw x21, w14",
            "asr x22, x14, #32"
        ]
    );
    assert!(
        !lines("EMUL", "R1, R2, #0, R4")
            .iter()
            .any(|l| l.starts_with("add"))
    );
    assert_eq!(
        lines("EDIV", "R1, R2, R4, R5"),
        [
            "mov w14, w19",
            "bfi x14, x20, #32, #32",
            "sxtw x15, w1",
            "sdiv x16, x14, x15",
            "msub x15, x16, x15, x14",
            "sxtw x21, w16",
            "sxtw x22, w15"
        ]
    );
}

/// The character string instructions leave their registers as the VAX
/// does: MOVC3 R1 and R3 past the strings, R0, R2, R4 and R5 0.
#[test]
fn strings() {
    let movc3 = lines("MOVC3", "#10, (R6), (R7)");
    has(
        &movc3,
        &[
            "add x1, x15, x14",
            "add x20, x16, x14",
            // A destination above the source copies backwards.
            "cmp x16, x15",
            "b.hi 90002$",
            "ldrb w22, [x15], #1",
            "90002$:",
            "ldrb w22, [x19, #-1]!",
        ],
    );
    assert_eq!(
        movc3[movc3.len() - 3..],
        ["mov x19, #0", "mov x21, #0", "mov x22, #0"]
    );
    // MOVC5 copies the shorter length, fills the rest, and compares the
    // lengths.
    let movc5 = lines("MOVC5", "#10, (R6), #32, #20, (R7)");
    has(
        &movc5,
        &[
            "csel x19, x14, x17, lo",
            "sub x21, x17, x19",
            "strb w16, [x10], #1",
            "cmp x14, x17",
        ],
    );
    // The bytes compare as both signed and unsigned: at the top of a
    // register. CMPC3's R2 is its R0.
    let cmpc3 = lines("CMPC3", "#10, (R6), (R7)");
    has(
        &cmpc3,
        &["lsl w17, w17, #24", "lsl w10, w10, #24", "cmp w17, w10"],
    );
    assert_eq!(cmpc3.last().unwrap(), "mov x19, x0");
    // CMPC5's shorter string goes on as its fill byte.
    has(
        &lines("CMPC5", "#10, (R6), #32, #20, (R7)"),
        &["csel w11, w11, w16, ne", "csel w8, w8, w16, ne"],
    );
    has(
        &lines("LOCC", "#65, #10, (R6)"),
        &["ldrb w17, [x1]", "cmp w17, w14", "b.eq 90002$"],
    );
    has(&lines("SKPC", "#32, #10, (R6)"), &["b.ne 90002$"]);
}

/// Bit fields: a position known now in a register is an ARM64 bit field;
/// one known when it runs shifts; one in memory reads the quadword it
/// starts in.
#[test]
fn fields() {
    assert_eq!(
        lines("EXTV", "#4, #8, R2, R3"),
        ["sbfx w14, w19, #4, #8", "sxtw x20, w14"]
    );
    assert_eq!(lines("EXTZV", "#4, #8, R2, R3")[0], "ubfx w14, w19, #4, #8");
    assert_eq!(
        lines("EXTZV", "R1, #8, R2, R3"),
        ["lsr w14, w19, w1", "ubfx w14, w14, #0, #8", "sxtw x20, w14"]
    );
    assert_eq!(
        lines("EXTZV", "#4, #8, (R6), R3"),
        ["ldr x14, [x23]", "ubfx x14, x14, #4, #8", "sxtw x20, w14"]
    );
    // A position known when it runs is signed: its byte, then its bit.
    assert_eq!(
        lines("EXTZV", "R1, #8, (R6), R3"),
        [
            "asr w14, w1, #3",
            "add x14, x23, w14, sxtw",
            "and w15, w1, #7",
            "ldr x16, [x14]",
            "lsr x16, x16, x15",
            "ubfx x16, x16, #0, #8",
            "sxtw x20, w16"
        ]
    );
    assert_eq!(lines("EXTZV", "#4, #0, R2, R3"), ["mov x20, #0"]);
    assert_eq!(
        error("EXTZV", "#30, #8, R2, R3"),
        "the field must be in the register"
    );
    assert_eq!(
        error("EXTZV", "#4, #33, R2, R3"),
        "a bit field is 0 to 32 bits"
    );
    assert_eq!(
        lines("INSV", "R1, #4, #8, R2"),
        ["bfi w19, w1, #4, #8", "sxtw x19, w19"]
    );
    // Rotate the field down to bit 0, insert, rotate back.
    assert_eq!(
        lines("INSV", "R1, R3, #8, R2"),
        [
            "ror w19, w19, w20",
            "bfi w19, w1, #0, #8",
            "neg w14, w20",
            "ror w19, w19, w14",
            "sxtw x19, w19"
        ]
    );
    assert_eq!(
        lines("INSV", "R1, #4, #8, (R6)"),
        ["ldr x14, [x23]", "bfi x14, x1, #4, #8", "str x14, [x23]"]
    );
    has(
        &lines("INSV", "R1, R3, #8, (R6)"),
        &[
            "ror x16, x16, x15",
            "bfi x16, x1, #0, #8",
            "neg w15, w15",
            "str x16, [x14]",
        ],
    );
    assert!(lines("INSV", "R1, #4, #0, R2").is_empty());
    assert_eq!(
        error("INSV", "R1, #30, #8, R2"),
        "the field must be in the register"
    );
}

/// BBS and BBC test a bit; BBSS, BBSC, BBCS and BBCC also set or clear it,
/// after reading it. In memory the bit is in the byte at position / 8.
#[test]
fn bit_branches() {
    assert_eq!(lines("BBS", "#3, R2, 10$"), ["tbnz w19, #3, 10$"]);
    assert_eq!(lines("BBC", "#3, R2, 10$"), ["tbz w19, #3, 10$"]);
    assert_eq!(
        lines("BBSS", "#3, R2, 10$"),
        [
            "lsr w14, w19, #3",
            "movz w15, #8",
            "orr w19, w19, w15",
            "sxtw x19, w19",
            "tbnz w14, #0, 10$"
        ]
    );
    assert_eq!(lines("BBCC", "#3, R2, 10$")[2], "bic w19, w19, w15");
    assert_eq!(
        lines("BBSC", "R1, R2, 10$"),
        [
            "lsr w14, w19, w1",
            "mov w15, #1",
            "lsl w15, w15, w1",
            "bic w19, w19, w15",
            "sxtw x19, w19",
            "tbnz w14, #0, 10$"
        ]
    );
    assert_eq!(
        error("BBCS", "#33, R2, 10$"),
        "the bit must be in the register"
    );
    assert_eq!(
        lines("BBS", "#3, (R6), 10$"),
        ["ldrb w14, [x23]", "lsr w15, w14, #3", "tbnz w15, #0, 10$"]
    );
    assert_eq!(
        lines("BBSS", "#11, (R6), 10$"),
        [
            "ldrb w14, [x23, #1]",
            "lsr w15, w14, #3",
            "mov w16, #1",
            "lsl w16, w16, #3",
            "orr w14, w14, w16",
            "strb w14, [x23, #1]",
            "tbnz w15, #0, 10$"
        ]
    );
    has(
        &lines("BBCC", "R1, (R6), 10$"),
        &[
            "asr w14, w1, #3",
            "and w15, w1, #7",
            "bic w16, w16, w10",
            "tbz w17, #0, 10$",
        ],
    );
}

/// CASE: the selector less the base, unsigned, against the limit; in
/// range, a branch by the word displacement in the table after it; out of
/// range, on past the table.
#[test]
fn case() {
    let casel = lines("CASEL", "R1, #0, #2");
    has(
        &casel,
        &[
            "sub w14, w1, #0",
            "cmp w14, #2",
            "adr x15, 90001$",
            "b.ls 90002$",
            "add x15, x15, #((<(2)+1>*2+3)&^C3)",
            "ldrsh x16, [x15, w14, uxtw #1]",
            "add x15, x15, x16",
        ],
    );
    assert_eq!(casel.last().unwrap(), "90001$:");
    has(
        &lines("CASEB", "R1, #1, #3"),
        &["sub w15, w1, w14", "ubfx w15, w15, #0, #8", "cmp w15, #3"],
    );
}

/// The queue instructions keep their links in longwords.
#[test]
fn queues() {
    // The entry R2 goes after R3: its successor is R3's old one.
    assert_eq!(
        lines("INSQUE", "(R2), (R3)"),
        [
            "ldr w14, [x20]",
            "str w14, [x19]",
            "str w20, [x19, #4]",
            "str w19, [x14, #4]",
            "str w19, [x20]",
            // Z: it is the only entry.
            "cmp w14, w20"
        ]
    );
    // V: the queue was empty; Z: it is now.
    assert_eq!(
        lines("REMQUE", "@(R3), R2"),
        [
            "ldr w14, [x20]",
            "ldr w15, [x14]",
            "ldr w16, [x14, #4]",
            "str w15, [x16]",
            "str w16, [x15, #4]",
            "sxtw x19, w14",
            "cmp w14, w15",
            "b.ne 90001$",
            "movz w16, #0x7000, lsl #16",
            "msr nzcv, x16",
            "b 90002$",
            "90001$:",
            "cmp w15, w16",
            "90002$:"
        ]
    );
}

#[test]
fn others() {
    assert_eq!(lines("NOP", ""), ["nop"]);
    assert_eq!(lines("BPT", ""), ["brk #0"]);
    assert!(lines("PROBER", "#0, #4, (R6)").contains(&"mov x7, #143".to_string()));
    for operands in ["#300", "R1"] {
        assert_eq!(
            error("CALL_PAL", operands),
            "CALL_PAL needs a function code from 0 to ^XBF, #n"
        );
    }
    assert_eq!(
        error("MTPR", "R0, #16"),
        "MTPR can't write that processor register"
    );
}

/// CALLS passes the longwords pushed, CALLG those of a list, in x0-x7 and
/// past the eighth on the stack, the count in x9 (DESIGN-0004).
#[test]
fn calls() {
    assert_eq!(
        after(&[("PUSHL", "#1"), ("PUSHL", "#2"), ("CALLS", "#2, G^FOO")]),
        [
            "and sp, x18, #0xfffffffffffffff0",
            "ldpsw x0, x1, [x18, #0]",
            "add x18, x18, #8",
            "mov x9, #2",
            "bl FOO"
        ]
    );
    let mut ten = vec![("PUSHL", "#1"); 10];
    ten.push(("CALLS", "#10, G^FOO"));
    has(
        &after(&ten),
        &[
            "sub sp, x16, #16",
            "add x16, x18, #32",
            "ldrsw x14, [x16], #4",
            "ldpsw x6, x7, [x18, #24]",
            "add x18, x18, #40",
            "mov x9, #10",
        ],
    );
    // A count known when it runs.
    has(
        &lines("CALLS", "R1, G^FOO"),
        &[
            "and x9, x1, #255",
            "mov x8, x18",
            "ldrsw x0, [x8, #0]",
            "add x18, x18, x9, lsl #2",
            "bl FOO",
        ],
    );
    assert_eq!(
        lines("CALLS", "#0, (R3)"),
        [
            "mov x13, x20",
            "and sp, x18, #0xfffffffffffffff0",
            "mov x9, #0",
            "blr x13"
        ]
    );
    // A list at 0 passes no arguments.
    has(
        &lines("CALLG", "(R3), G^FOO"),
        &[
            "mov x8, x20",
            "mov x9, #0",
            "cbz x8, 90001$",
            "ldr w9, [x8], #4",
            "ldrsw x7, [x8, #28]",
            "bl FOO",
        ],
    );
    has(
        &lines("EVAX_CALLG_64", "(R3), G^FOO"),
        &["ldr x9, [x8], #8", "add x16, x8, #64", "ldr x7, [x8, #56]"],
    );
}

/// PUSHR pushes the highest register first, so that the lowest is at the
/// top of the stack; POPR pops it first.
#[test]
fn pushr_popr() {
    assert_eq!(
        lines("PUSHR", "#12"),
        ["str w20, [x18, #-4]!", "str w19, [x18, #-4]!"]
    );
    assert_eq!(
        after(&[("PUSHR", "#12"), ("POPR", "#12")]),
        ["ldrsw x19, [x18], #4", "ldrsw x20, [x18], #4"]
    );
    assert_eq!(error("PUSHR", "R2"), "expected a register mask, #^M<...>");
}

/// Addressing modes, each as the VAX evaluates it.
#[test]
fn operands() {
    assert_eq!(
        lines("MOVL", "-(R3), R2"),
        ["sub x20, x20, #4", "ldr w14, [x20]", "sxtw x19, w14"]
    );
    // (R3)+ moves R3 by the operand's size.
    assert_eq!(lines("MOVB", "(R3)+, R2")[1], "add x20, x20, #1");
    assert_eq!(lines("MOVQ", "(R3)+, R4")[1], "add x20, x20, #8");
    assert_eq!(
        lines("MOVL", "@(R3)+, R2"),
        ["ldr w14, [x20], #4", "ldr w14, [x14]", "sxtw x19, w14"]
    );
    assert_eq!(
        lines("MOVL", "@4(R3), R2"),
        ["ldr w14, [x20, #4]", "ldr w14, [x14]", "sxtw x19, w14"]
    );
    assert_eq!(
        lines("MOVL", "@#4096, R2"),
        ["movz w14, #4096", "ldr w14, [x14]", "sxtw x19, w14"]
    );
    assert_eq!(
        lines("MOVL", "R1, @#4096"),
        ["movz w14, #4096", "str w1, [x14]"]
    );
    // A symbol is PC-relative; @SYM reads the address there.
    assert_eq!(
        lines("MOVL", "R2, SYM"),
        [
            "adrp x14, SYM",
            "add x14, x14, #:lo12:SYM",
            "str w19, [x14]"
        ]
    );
    has(
        &lines("MOVL", "@SYM, R2"),
        &[
            "add x14, x14, #:lo12:SYM",
            "ldr w14, [x14]",
            "ldr w14, [x14]",
        ],
    );
    // The size prefixes change nothing here.
    for op in ["S^#5", "I^#5"] {
        assert_eq!(lines("MOVL", &format!("{op}, R2")), lines("MOVL", "#5, R2"));
    }
    for op in ["B^4(R3)", "W^4(R3)", "L^4(R3)"] {
        assert_eq!(
            lines("MOVL", &format!("{op}, R2")),
            lines("MOVL", "4(R3), R2")
        );
    }
    // Indexed: the index scaled by the operand's size.
    assert_eq!(
        lines("MOVL", "4(R3)[R4], R2"),
        [
            "add x14, x20, #4",
            "add x14, x14, w21, sxtw #2",
            "ldr w14, [x14]",
            "sxtw x19, w14"
        ]
    );
    assert_eq!(
        lines("MOVW", "(R3)[R4], R2")[0],
        "add x14, x20, w21, sxtw #1"
    );
    assert_eq!(
        lines("MOVQ", "(R3)[R4], R2")[0],
        "add x14, x20, w21, sxtw #3"
    );
    assert_eq!(lines("MOVL", "-8(R3), R2")[0], "ldr w14, [x20, #-8]");
    assert_eq!(lines("MOVL", "8000(R3), R2")[0], "ldr w14, [x20, #8000]");
}

/// A displacement a load can't encode is added to the base first.
#[test]
fn displacements() {
    assert_eq!(
        lines("MOVL", "-300(R3), R2"),
        ["sub x15, x20, #300", "ldr w14, [x15]", "sxtw x19, w14"]
    );
    assert_eq!(
        lines("MOVL", "70000(R3), R2"),
        [
            "movz w16, #4464",
            "movk w16, #1, lsl #16",
            "add x15, x20, w16, sxtw",
            "ldr w14, [x15]",
            "sxtw x19, w14"
        ]
    );
    has(
        &lines("MOVL", "70000(R3)[R4], R2"),
        &["add x14, x20, w15, sxtw", "add x14, x14, w21, sxtw #2"],
    );
    // One the linker knows comes from $LINK$, sign-extended from 32 bits.
    has(
        &lines("MOVL", "@SYM(R3), R2"),
        &[
            "90001$:\t.LONG SYM",
            "ldr w14, [x14, #:lo12:90001$]",
            "add x14, x20, w14, sxtw",
            "ldr w14, [x14]",
            "ldr w14, [x14]",
        ],
    );
    has(
        &lines("CMPB", "R1, #SYM"),
        &["ldr w15, [x15, #:lo12:90001$]", "sbfx w15, w15, #0, #8"],
    );
}

#[test]
fn operand_errors() {
    assert_eq!(error("MOVL", "R15, R2"), "PC can't be used as a register");
    assert_eq!(error("MOVL", "R2, #1"), "can't write to an immediate");
    assert_eq!(error("MOVL", "(R3, R2"), "MOVL takes 2 operands");
    assert_eq!(
        error("MOVL", "@-(R3), R2"),
        "autodecrement can't be deferred"
    );
    assert_eq!(
        error("MOVL", "R3[R4], R2"),
        "only a memory operand can be indexed"
    );
    assert_eq!(error("MOVL", "(R3)[X], R2"), "expected an index register");
    // R16 isn't a register: it is a symbol.
    assert_eq!(lines("MOVL", "R16, R2")[0], "adrp x14, R16");
}

/// The `EVAX_` built-ins builtins.rs leaves out: their operands are
/// quadwords, a register all 64 bits of it.
#[test]
fn builtins() {
    assert_eq!(
        lines("EVAX_SEXTB", "R1, R2"),
        ["sxtb x14, w1", "mov x19, x14"]
    );
    has(
        &lines("EVAX_SEXTW", "#-2, R2"),
        &["movn w14, #1", "sxth x14, w14"],
    );
    assert_eq!(lines("EVAX_SEXTL", "(R3), R2")[0], "ldrsw x14, [x20]");
    assert_eq!(lines("EVAX_SEXTB", "R1, (R3)")[1], "strb w14, [x20]");
    assert_eq!(lines("EVAX_LDBU", "R2, (R3)"), ["ldrb w19, [x20]"]);
    assert_eq!(lines("EVAX_LDWU", "R2, 2(R3)"), ["ldrh w19, [x20, #2]"]);
    assert_eq!(lines("EVAX_STB", "R2, (R3)"), ["strb w19, [x20]"]);
    assert_eq!(lines("EVAX_STW", "R2, 2(R3)"), ["strh w19, [x20, #2]"]);
    assert_eq!(
        lines("EVAX_LDAQ", "R2, 8(R3)"),
        ["add x14, x20, #8", "mov x19, x14"]
    );
    // The quadword the address is in, rounded down to 8.
    assert_eq!(
        lines("EVAX_LDQU", "R2, 3(R3)"),
        [
            "add x14, x20, #3",
            "and x14, x14, #0xfffffffffffffff8",
            "ldr x19, [x14]"
        ]
    );
    assert_eq!(lines("EVAX_STQU", "R2, 3(R3)")[2], "str x19, [x14]");
    // Load-locked and store-conditional: the register 1 if it stored.
    assert_eq!(
        lines("EVAX_LDLL", "R2, (R3)"),
        ["ldxr w19, [x20]", "sxtw x19, w19"]
    );
    assert_eq!(lines("EVAX_LDQL", "R2, (R3)"), ["ldxr x19, [x20]"]);
    assert_eq!(
        lines("EVAX_STLC", "R2, (R3)"),
        ["stxr w15, w19, [x20]", "cmp w15, #0", "cset x19, eq"]
    );
    assert_eq!(lines("EVAX_STQC", "R2, (R3)")[0], "stxr w15, x19, [x20]");
    assert_eq!(
        lines("EVAX_SUBQ", "R2, #1, R3"),
        [
            "movz w14, #1",
            "sxtw x14, w14",
            "sub x15, x19, x14",
            "mov x20, x15"
        ]
    );
    assert_eq!(
        lines("EVAX_UMULH", "R2, R3, (R4)"),
        ["umulh x14, x19, x20", "str x14, [x21]"]
    );
    assert_eq!(lines("EVAX_SRA", "R2, #3, R3")[2], "asr x15, x19, x14");
    // R2 as it was before (R2)+ moves it.
    assert_eq!(lines("EVAX_ADDQ", "R2, (R2)+, R3")[0], "mov x14, x19");
    assert_eq!(
        lines("EVAX_CMPLE", "(R3), #0, R4"),
        [
            "ldr x14, [x20]",
            "movz w15, #0",
            "sxtw x15, w15",
            "cmp x14, x15",
            "cset x16, le",
            "mov x21, x16"
        ]
    );
    assert_eq!(lines("EVAX_BEQ", "R2, 10$"), ["cbz x19, 10$"]);
    assert_eq!(
        lines("EVAX_BNE", "(R3), 10$"),
        ["ldr x14, [x20]", "cbnz x14, 10$"]
    );
    assert_eq!(
        lines("EVAX_CMOVLBS", "R2, R3, R4"),
        ["tst x19, #1", "csel x21, x20, x21, ne"]
    );
    has(
        &lines("EVAX_CMOVLBC", "R2, #7, (R4)"),
        &[
            "tst x19, #1",
            "ldr x15, [x21]",
            "csel x15, x14, x15, eq",
            "str x15, [x21]",
        ],
    );
    assert_eq!(lines("EVAX_MB", ""), ["dmb sy"]);
    assert_eq!(lines("EVAX_MTPR_IPL", "R0"), ["mov x7, #15", "svc #0"]);
    assert_eq!(lines("EVAX_MTPR_IPL", "#8")[2], "mov x0, x14");
    assert_eq!(lines("EVAX_MFPR_IPL", ""), ["mov x7, #14", "svc #0"]);
    assert_eq!(lines("EVAX_LDQ", "R1, (R3)"), ["ldr x1, [x20]"]);
    assert_eq!(lines("EVAX_STQ", "R1, 8(R3)"), ["str x1, [x20, #8]"]);
    assert_eq!(lines("EVAX_STQ", "R1, R2"), ["mov x19, x1"]);
    assert_eq!(
        after(&[(".ENABLE", "QUADWORD"), ("EVAX_LDQ", "R2, #5")]),
        ["movz x19, #5"]
    );
}

/// ZAP clears the bytes the mask's low 8 bits say, ZAPNOT keeps them.
#[test]
fn zap() {
    assert_eq!(
        lines("EVAX_ZAP", "R2, #3, R3"),
        ["movz x14, #65535", "bic x15, x19, x14", "mov x20, x15"]
    );
    assert_eq!(
        lines("EVAX_ZAPNOT", "R2, #15, R3"),
        [
            "movz x14, #65535",
            "movk x14, #65535, lsl #16",
            "and x15, x19, x14",
            "mov x20, x15"
        ]
    );
    has(
        &lines("EVAX_ZAPNOT", "R2, R1, R3"),
        &[
            "mov x14, #0",
            "tbz x1, #0, 90001$",
            "orr x14, x14, #0xff",
            "tbz x1, #7, 90008$",
            "orr x14, x14, #0xff00000000000000",
            "and x15, x19, x14",
        ],
    );
}

#[test]
fn builtin_errors() {
    assert_eq!(
        error("EVAX_STW", "(R3), R2"),
        "a load or store built-in's first operand is a register"
    );
    assert_eq!(
        error("EVAX_LDQ", "(R3), R1"),
        "EVAX_LDQ's first operand is a register"
    );
    assert_eq!(
        error("EVAX_ADDQ", "R2, R3, #1"),
        "a literal can't be written"
    );
    assert_eq!(
        error("EVAX_CMOVGT", "R2, R3, #1"),
        "a literal can't be written"
    );
    // MTPR_SISR and MFPR_TXDB aren't PAL calls: SISR is read-only, TXDB
    // write-only.
    for mn in ["EVAX_MTPR_SISR", "EVAX_MFPR_TXDB"] {
        assert!(error(mn, "").starts_with(&format!("{mn} isn't a built-in here")));
    }
}
