//! Routines as a whole: CALL frames and their argument lists, JSB
//! routines' saves and tail calls, the VAX stack vmacro follows, AMACRO's
//! 64-bit macros and the directives vmacro handles itself.
//!
//! The sources here go to vmacro's dialect statement by statement, without
//! vasm's expression evaluation: entry masks are numbers, 4 for ^M<R2>.

use vasm::Dialect;

/// Compiles `source`, a statement a line, with `NAME:` or `NAME::` before
/// one for a label: all vmacro makes of it on its second pass, else the
/// statements it rejects, each with its error. Directives vmacro leaves to
/// vasm come out in `<>`.
fn code(source: &str) -> Result<Vec<String>, Vec<String>> {
    let mut m = vmacro::Macro32::default();
    let constant = |e: &str| e.parse().ok();
    loop {
        let (mut out, mut errors) = (Vec::new(), Vec::new());
        for line in source.lines().map(str::trim).filter(|l| !l.is_empty()) {
            let line = match line.split_once(':') {
                Some((label, rest)) if !label.contains(' ') => {
                    let (global, rest) = match rest.strip_prefix(':') {
                        Some(rest) => (true, rest),
                        None => (false, rest),
                    };
                    m.label(label, global);
                    out.push(format!("{label}:"));
                    rest.trim()
                }
                _ => line,
            };
            if line.is_empty() {
                continue;
            }
            let (mn, operands) = line.split_once(' ').unwrap_or((line, ""));
            let operands = operands.trim();
            match m.statement(mn, operands, &constant) {
                Some(Ok(lines)) => out.extend(lines.iter().map(|l| l.trim().to_string())),
                Some(Err(e)) => {
                    errors.push(format!("{}: {e}", format!("{mn} {operands}").trim_end()))
                }
                None => out.push(format!("<{line}>")),
            }
        }
        if !m.again() {
            return if errors.is_empty() {
                Ok(out)
            } else {
                Err(errors)
            };
        }
    }
}

#[track_caller]
fn compiles(source: &str) -> Vec<String> {
    code(source).unwrap_or_else(|e| panic!("{e:#?}"))
}

#[track_caller]
fn errors(source: &str) -> Vec<String> {
    code(source)
        .err()
        .unwrap_or_else(|| panic!("compiles:\n{source}"))
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

/// HOME_ARGS: the prologue copies the count, at most MAX_ARGS, and the
/// arguments to the frame at 32, where AP points: the first eight from
/// x0-x7, the rest from the caller's stack. The descriptor says where the
/// registers are saved and how big the frame is.
#[test]
fn home_args() {
    let code = compiles(
        "
A:      .CALL_ENTRY MAX_ARGS=10, HOME_ARGS=TRUE
        MOVL    4(AP), R0
        RET
",
    );
    has(
        &code,
        &[
            "FDSC$$0:\t.LONG 0, 1, 80, 96",
            "FDSC$$0N:\t.ASCIC \"A\"",
            "stp x29, x30, [sp, #-96]!",
            "mov x17, #10",
            "csel x16, x16, x17, ls",
            "str w16, [x29, #32]",
            "stp w0, w1, [x29, #36]",
            "stp w6, w7, [x29, #60]",
            "subs x16, x16, #8",
            "add x13, x29, #96",
            "add x17, x29, #68",
            "str x18, [x29, #80]",
            "add x12, x29, #32",
            "ldr w14, [x12, #4]",
            "ldp x29, x30, [sp], #96",
        ],
    );
}

/// A frame too big for `stp`'s reach is made with a `sub`, and its saves
/// go through x17.
#[test]
fn big_frame() {
    let code = compiles(
        "
A:      .CALL_ENTRY MAX_ARGS=255, HOME_ARGS=TRUE, PRESERVE=<R2,R3,R4,R5,R6,R7,R8,R9,R10,R11>
        MOVL    4(AP), R2
        RET
",
    );
    has(
        &code,
        &[
            "FDSC$$0:\t.LONG 0, 2047, 1056, 1152",
            "sub sp, sp, #1152",
            "stp x29, x30, [sp]",
            "add x17, x29, #1056",
            "stp x18, x19, [x17, #0]",
            "str x28, [x17, #80]",
            "add x17, x29, #1056",
            "ldr x28, [x17, #80]",
            "ldp x29, x30, [sp]",
            "add sp, sp, #1152",
            "ret",
        ],
    );
}

/// QUAD_ARGS: the list is of quadwords, so argument n is at 8 * n, and
/// AP is indexed by them.
#[test]
fn quad_args() {
    let routine = |body: &str| {
        code(&format!(
            "A: .CALL_ENTRY MAX_ARGS=3, QUAD_ARGS=TRUE\n{body}\nRET"
        ))
    };
    let code = routine(
        "
        MOVL    8(AP), R0
        EVAX_LDQ R0, 8(AP)
        MOVL    (AP)[R1], R0
        MOVL    @4(AP), R0
        CALLG   (AP), G^FOO
",
    )
    .unwrap();
    has(
        &code,
        &[
            "str x16, [x29, #32]",
            "stp x0, x1, [x29, #40]",
            "str x2, [x29, #56]",
            "ldr w14, [x12, #16]",
            "ldr x0, [x12, #16]",
            "add x14, x12, w1, sxtw #3",
            "ldr x14, [x12, #8]",
            // Its own list passed on is of quadwords.
            "ldr x9, [x8], #8",
            "ldr x7, [x8, #56]",
        ],
    );
    assert_eq!(
        routine("MOVB (AP)[R1], R0").unwrap_err(),
        ["MOVB (AP)[R1], R0: in a QUAD_ARGS routine, AP is indexed by longwords or quadwords"]
    );
}

/// AP is x12 where the routine reads it: set before the first read, and
/// again after a call, which changes x12.
#[test]
fn ap_after_call() {
    let code = compiles(
        "
        .ENTRY  A, 4
        MOVL    4(AP), R2
        CALLS   #0, G^FOO
        MOVL    8(AP), R0
        RET
",
    );
    has(
        &code,
        &[
            "add x12, x29, #32",
            "ldr w14, [x12, #4]",
            "bl FOO",
            "add x12, x29, #32",
            "ldr w14, [x12, #8]",
        ],
    );
}

/// After code loads FP, RET returns from the frame FP is at, restoring the
/// registers its descriptor says.
#[test]
fn ret_from_another_frame() {
    let code = compiles(
        "
        .ENTRY  A, 4
        MOVL    R2, FP
        RET
",
    );
    has(
        &code,
        &[
            "sxtw x29, w19",
            "ldr x16, [x29, #24]",
            "ldr w15, [x16, #4]",
            "ldr w17, [x16, #8]",
            "tbz w15, #0, 90003$",
            "ldr x18, [x17], #8",
            "tbz w15, #10, 90013$",
            "ldr x28, [x17], #8",
            "ldr w17, [x16, #12]",
            "add sp, sp, x17",
            "ret",
        ],
    );
}

/// A JSB routine saves what it writes below both stacks and restores it at
/// RSB. A conditional branch to another JSB routine is a tail call: it
/// restores first, on the branch's path only.
#[test]
fn jsb_tail_call() {
    let code = compiles(
        "
A:      .JSB_ENTRY
        MOVL    #1, R2
        BEQL    B
        RSB
B:      .JSB_ENTRY
        RSB
",
    );
    assert_eq!(
        code,
        [
            "A:",
            "mov x16, sp",
            "cmp x16, x18",
            "csel x17, x16, x18, lo",
            "and x17, x17, #0xfffffffffffffff0",
            "sub x17, x17, #32",
            "mov sp, x17",
            "stp x16, x18, [x17]",
            "str x19, [x17, #16]",
            "mov x18, x17",
            "BODY$$0:",
            "movz x19, #1",
            "tst w19, w19",
            "b.eq 90001$",
            "b 90002$",
            "90001$:",
            "ldr x19, [x18, #16]",
            "ldp x16, x17, [x18]",
            "mov sp, x16",
            "mov x18, x17",
            "b B",
            "90002$:",
            "ldr x19, [x18, #16]",
            "ldp x16, x17, [x18]",
            "mov sp, x16",
            "mov x18, x17",
            "ret",
            "B:",
            "BODY$$1:",
            "ret",
        ]
    );
}

/// A JSB routine that branches to its own entry goes on past its prologue.
#[test]
fn jsb_own_entry() {
    let code = compiles(
        "
A:      .JSB_ENTRY
        MOVL    #1, R2
        DECL    R3
        BNEQ    A
        RSB
",
    );
    has(
        &code,
        &["stp x19, x20, [x17, #16]", "BODY$$0:", "b.ne BODY$$0"],
    );
}

/// A JSB routine reading AP reads its caller's list: a warning, unless its
/// declaration says it takes AP.
#[test]
fn jsb_reads_ap() {
    let warning = ".WARN a JSB routine that reads AP reads its caller's argument list, which its callers must home (HOME_ARGS=TRUE): say so with INPUT=<AP>";
    let routine = |input: &str| compiles(&format!("A: .JSB_ENTRY {input}\nMOVL 4(AP), R0\nRSB"));
    assert!(routine("").contains(&warning.to_string()));
    let quiet = routine("INPUT=<AP>");
    assert!(!quiet.iter().any(|l| l.starts_with(".WARN")), "{quiet:#?}");
    has(&quiet, &["add x12, x29, #32", "ldr w14, [x12, #4]"]);
}

/// vmacro follows what a routine pushes, through branches, by the size of
/// each push and pop, and stops at code that changes SP by an amount it
/// can't know.
#[test]
fn stack() {
    assert_eq!(
        errors(
            "
        .ENTRY  A, 0
        MOVZWL  -(SP), R0
        CVTBL   -(SP), R0
        MOVL    (SP)+, R0
        RET
"
        ),
        [
            "MOVL (SP)+, R0: (SP)+ reaches past what this routine pushed (it pushed 3 bytes): the VAX stack has no return address or frame (DESIGN-0004)"
        ]
    );
    assert_eq!(
        errors("A: .JSB_ENTRY\nMOVL 4(SP), R0\nRSB"),
        [
            "MOVL 4(SP), R0: 4(SP) reaches past what this routine pushed (it pushed nothing): the VAX stack has no return address or frame (DESIGN-0004)"
        ]
    );
    for (body, what) in [
        ("PUSHL #1\nADDL2 #8, SP", "ADD to SP"),
        ("MOVAL 8(SP), SP", "MOVA to SP"),
        ("POPR #4", "POPR"),
    ] {
        let e = errors(&format!("A: .JSB_ENTRY\n{body}\nRSB"));
        assert!(e[0].contains(&format!("{what} reaches past")), "{e:?}");
    }
    // Popped on each path as pushed.
    compiles(
        "
A:      .JSB_ENTRY
        PUSHL   R0
        BEQL    10$
        MOVL    (SP)+, R0
        RSB
10$:    TSTL    (SP)+
        RSB
",
    );
    // An amount known when it runs: anything goes after.
    compiles(
        "
        .ENTRY  A, 0
        SUBL2   #8, SP
        MOVL    R0, 4(SP)
        ADDL2   R1, SP
        MOVL    R0, 4(SP)
        RET
",
    );
    compiles("A: .JSB_ENTRY\nPUSHL R0\nMOVAL 4(SP), SP\nRSB");
    compiles("A: .JSB_ENTRY\nPUSHL R0\nMOVL @(SP)+, R0\nRSB");
}

/// $PUSH64 and $POP64 move a register's 64 bits; an exception routine's
/// stack isn't its own to follow.
#[test]
fn push64_pop64() {
    let code = compiles(".ENTRY A, 0\n$PUSH64 R2\n$POP64 R2\nRET");
    has(&code, &["str x19, [x18, #-8]!", "ldr x19, [x18], #8"]);
    compiles("A: .EXCEPTION_ENTRY\n$POP64 R2\nMOVL (SP)+, R2\nRSB");
    assert_eq!(
        errors("A: .JSB_ENTRY\n$POP64 R2\nRSB"),
        [
            "$POP64 R2: $POP64 reaches past what this routine pushed (0 bytes): the VAX stack has no return address or frame (DESIGN-0004)"
        ]
    );
    assert_eq!(
        errors(
            "
A:      .JSB_ENTRY
        $PUSH64 SP
        $PUSH64 #1
        $POP64
        RSB
"
        ),
        [
            "$PUSH64 SP: $PUSH64 takes one of R0-R11",
            "$PUSH64 #1: $PUSH64 takes a register",
            "$POP64: $POP64 takes a register",
        ]
    );
}

/// $SETUP_CALL64, $PUSH_ARG64 and $CALL64: the arguments are pushed below
/// SP, last first, then loaded into x0-x7, those past the eighth left on
/// the stack for the callee.
#[test]
fn call64() {
    let code = compiles(
        "
        .ENTRY  A, 0
        $SETUP_CALL64 3
        $PUSH_ARG64 #1
        $PUSH_ARG64 R2
        $PUSH_ARG64 (R3)
        $CALL64 (R4)
        RET
",
    );
    has(
        &code,
        &[
            "mov x13, x21",
            "ldp x0, x1, [sp, #0]",
            "ldr x2, [sp, #16]",
            "add sp, sp, #32",
            "mov x9, #3",
            "blr x13",
        ],
    );
    let mut ten = String::from(".ENTRY A, 0\n$SETUP_CALL64 10\n");
    for n in 1..=10 {
        ten += &format!("$PUSH_ARG64 #{n}\n");
    }
    ten += "$CALL64 G^FOO\nRET";
    has(
        &compiles(&ten),
        &[
            "ldp x6, x7, [sp, #48]",
            "add sp, sp, #64",
            "mov x9, #10",
            "bl FOO",
            "add sp, sp, #16",
        ],
    );
    assert_eq!(
        errors(
            "
        .ENTRY  A, 0
        $SETUP_CALL64 1
        $PUSH64 R2
        $SETUP_CALL64 1
        $PUSH_ARG64 #1
        $PUSH_ARG64 #2
        $CALL64
        RET
"
        ),
        [
            "$PUSH64 R2: between $SETUP_CALL64 and $CALL64 nothing may push, pop, call or return: the arguments wait below VAX SP",
            "$SETUP_CALL64 1: $SETUP_CALL64 before the last one's $CALL64",
            "$PUSH_ARG64 #2: $PUSH_ARG64 pushes one of the 1 arguments $SETUP_CALL64 said",
            "$CALL64: $CALL64 takes the routine to call",
        ]
    );
    assert_eq!(
        errors(".ENTRY A, 0\n$CALL64 G^FOO\nRET"),
        ["$CALL64 G^FOO: $CALL64 without $SETUP_CALL64"]
    );
}

/// $IS_32BITS branches on whether a quadword is its low longword
/// sign-extended; $IS_DESC64 on whether a descriptor is a 64-bit one:
/// MBO 1 and MBMO -1.
#[test]
fn is_32bits_is_desc64() {
    let one = |macro_: &str| {
        let code = compiles(&format!("A: .JSB_ENTRY\n{macro_}\n10$: RSB\n20$: RSB"));
        code[2..code.iter().position(|l| l == "10$:").unwrap()].to_vec()
    };
    assert_eq!(one("$IS_32BITS R2"), ["cmp x19, w19, sxtw"]);
    assert_eq!(
        one("$IS_32BITS R2, 10$, 20$"),
        ["cmp x19, w19, sxtw", "b.eq 10$", "b.ne 20$"]
    );
    assert_eq!(
        one("$IS_32BITS R2, , 20$"),
        ["cmp x19, w19, sxtw", "b.ne 20$"]
    );
    let desc = [
        "ldrh w14, [x19]",
        "cmp w14, #1",
        "b.ne 90001$",
        "ldr w14, [x19, #4]",
        "cmn w14, #1",
        "b.eq 10$",
        "90001$:",
    ];
    assert_eq!(one("$IS_DESC64 R2, 10$, SIZE=LONG"), desc);
    assert_eq!(one("$IS_DESC64 R2, 10$, SIZE=QUAD"), desc);
    assert_eq!(one("$IS_DESC64 (R2), 10$")[0], "ldr w14, [x19]");
    assert_eq!(
        errors("A: .JSB_ENTRY\n$IS_DESC64 R2, 10$, SIZE=WORD\n$IS_DESC64 R2\n10$: RSB"),
        [
            "$IS_DESC64 R2, 10$, SIZE=WORD: SIZE is LONG or QUAD, not WORD",
            "$IS_DESC64 R2: $IS_DESC64 takes a descriptor's address and a label",
        ]
    );
    // A tail call is a branch, after which the routine restores.
    assert_eq!(
        errors(
            "
A:      .JSB_ENTRY
        MOVL    #1, R3
        $IS_32BITS R2, B
        RSB
B:      .JSB_ENTRY
        RSB
"
        ),
        ["$IS_32BITS R2, B: $IS_32BITS to another JSB routine: a tail call is a BRB or BRW"]
    );
}

/// What a JSB to another module's routine modifies, from a linkage.
#[test]
fn linkages() {
    let code = compiles(
        "
        .DEFINE_LINKAGE LK, OUTPUT=<R2>, SCRATCH=<R3>
        .CALL_LINKAGE FOO, LINKAGE_NAME=LK
A:      .JSB_ENTRY
        JSB     G^FOO
        RSB
",
    );
    // R2 and R3 change, and the JSB, x30: all three saved.
    has(
        &code,
        &["stp x19, x20, [x17, #16]", "str x30, [x17, #32]", "bl FOO"],
    );
    let used = compiles(
        "
        .DEFINE_LINKAGE LK, SCRATCH=<R4>
A:      .JSB_ENTRY
        .USE_LINKAGE LINKAGE_NAME=LK
        JSB     G^FOO
        RSB
",
    );
    has(&used, &["stp x21, x30, [x17, #16]", "bl FOO"]);
    assert_eq!(
        errors(
            "
        .CALL_LINKAGE BAR, LINKAGE_NAME=NONE
        .DEFINE_LINKAGE
        .CALL_LINKAGE A, B
        .USE_LINKAGE BOGUS=1
"
        ),
        [
            ".CALL_LINKAGE BAR, LINKAGE_NAME=NONE: no linkage NONE: define it with .DEFINE_LINKAGE",
            ".DEFINE_LINKAGE: .DEFINE_LINKAGE takes the linkage's name first",
            ".CALL_LINKAGE A, B: .CALL_LINKAGE takes the routine's name first",
            ".USE_LINKAGE BOGUS=1: unknown parameter BOGUS",
        ]
    );
}

#[test]
fn declaration_errors() {
    assert_eq!(
        errors(
            "
A:      .CALL_ENTRY MAX_ARGS=256
B:      .CALL_ENTRY 3
C:      .JSB_ENTRY OUTPUT=<X>
        .ENTRY
        .ENTRY  X, Y, Z
        .GLOBAL_LABEL
"
        ),
        [
            ".CALL_ENTRY MAX_ARGS=256: MAX_ARGS is 0 to 255",
            ".CALL_ENTRY 3: .CALL_ENTRY takes only parameters, NAME=value",
            ".JSB_ENTRY OUTPUT=<X>: X isn't one of R0-R11 or AP",
            ".ENTRY: .ENTRY takes a name and a register mask",
            ".ENTRY X, Y, Z: .ENTRY takes a name and a register mask",
            ".GLOBAL_LABEL: .GLOBAL_LABEL follows the label it declares",
        ]
    );
}

/// The directives vmacro turns into vasm's, and those it drops.
#[test]
fn directives() {
    assert_eq!(
        compiles(
            "
        .ADDRESS FOO, BAR
        .BLKA   2
        .SIGNED_BYTE -1
        .SIGNED_WORD -1
        .EXTRN  FOO
        .SBTTL  Nothing
        .PAGE
        .ENABLE LOCAL_BLOCK
        .DSABL  FLAGGING
        .PSECT  CODE1, EXE
"
        ),
        [
            ".LONG FOO, BAR",
            ".BLKL 2",
            ".BYTE -1",
            ".WORD -1",
            ".EXTERNAL FOO",
            ".PSECT CODE1, EXE, NOWRT",
        ]
    );
    assert_eq!(
        errors(".ERROR Not yet\n"),
        [".ERROR Not yet: %MACRO-E-GENERR, Not yet"]
    );
}

/// A CALL routine's descriptor goes next to its code: .SAVE_PSECT and
/// .RESTORE_PSECT, which vasm does, bring back the code's psect.
#[test]
fn descriptor_psect() {
    let code = compiles(
        "
        .PSECT  CODE1, EXE
        .SAVE_PSECT
        .PSECT  DATA, NOEXE
        .RESTORE_PSECT
        .ENTRY  A, 4
        MOVL    #1, R2
        RET
",
    );
    has(
        &code,
        &[
            "<.SAVE_PSECT>",
            "<.RESTORE_PSECT>",
            ".PSECT CODE1_FDSC, PIC, SHR, EXE, NOWRT, QUAD",
        ],
    );
}

/// /ENABLE=QUADWORD on the command line holds from the start of each pass.
#[test]
fn enable_quadword() {
    let constant = |e: &str| e.parse().ok();
    let mut m = vmacro::Macro32::default();
    assert!(m.enable("quadword"));
    assert!(!m.enable("FLAGGING"));
    loop {
        m.label("T", false);
        m.statement(".JSB32_ENTRY", "", &constant);
        let out = m
            .statement("MOVAL", "8(R3), R2", &constant)
            .unwrap()
            .unwrap();
        if !m.again() {
            assert_eq!(out[1].trim(), "mov x19, x14");
            return;
        }
    }
}
