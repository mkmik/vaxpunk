//! What vmacro rejects, and where it says so.

fn errors(source: &str) -> Vec<String> {
    let opts = vasm::Options::default();
    match vmacro::compile(source, &opts) {
        Ok(_) => Vec::new(),
        Err(diags) => diags
            .iter()
            .map(|d| format!("{}:{}: {}", d.line, d.col, d.msg))
            .collect(),
    }
}

fn warnings(source: &str) -> Vec<String> {
    let opts = vasm::Options::default();
    let object = vmacro::compile(source, &opts).unwrap_or_else(|d| panic!("{d:?}"));
    object
        .warnings
        .iter()
        .map(|d| format!("{}: {}", d.line, d.msg))
        .collect()
}

#[test]
fn diagnostics() {
    let source = "\
        .PSECT  CODE, EXE
        RET
        .ENTRY  START, ^M<R2>
        MOVL    R1
        MOVL    PC, R0
        CASEL   R0, #0, R1
        EXTZV   #0, R2, R1, R0
        MOVQ    R11, R0
        PUSHR   #^M<R2, SP>
        MOVL    R0, #1
        MTPR    R0, R1
        MTPR    R0, #62
        MFPR    #35, R0
        RET
        .END    START
";
    assert_eq!(
        errors(source),
        [
            "2:9: code outside a routine: declare it with .CALL_ENTRY, .JSB_ENTRY or .EXCEPTION_ENTRY",
            "4:9: MOVL takes 2 operands",
            "5:9: PC can't be used as a register",
            "6:9: the CASE limit must be an immediate, #n",
            "7:9: vmacro needs the field size as a constant, #n",
            "8:9: a quadword needs two registers, up to R10 and R11",
            "9:9: PUSHR and POPR can't save SP or PC",
            "10:9: can't write to an immediate",
            "11:9: vmacro needs the processor register as a constant, #n",
            "12:9: processor register 62 has no PAL call yet",
            "13:9: MFPR can't read that processor register",
        ]
    );
}

#[test]
fn compiles() {
    let source = "\
        .PSECT  CODE, EXE
        .ENTRY  START, ^M<>
        MOVL    #1, R0
        RET
        .END    START
";
    assert_eq!(errors(source), Vec::<String>::new());
}

/// Every routine is declared, and calls go to declared routines.
#[test]
fn declarations() {
    let source = "\
        .PSECT  CODE, EXE
        .ENTRY  START, ^M<>
        JSB     HALF
        CALLS   #0, HALF
        RET
HALF:   RSB
J:      .JSB_ENTRY  OUTPUT=<R2>, SCRATCH=<R3>, PRESERVE=<R4>
        RET
        .JSB_ENTRY
C:      .CALL_ENTRY MAX_ARGS=2, HOME_ARGS=TRUE, QUAD_ARGS=TRUE
D:      .CALL_ENTRY BOGUS=1
E:      .JSB_ENTRY  SCRATCH=<AP>
F:      .CALL_ENTRY
        RSB
        .END    START
";
    assert_eq!(
        errors(source),
        [
            "3:9: HALF isn't a declared routine: declare it with .JSB_ENTRY",
            "4:9: HALF isn't a declared routine: declare it with .CALL_ENTRY or .ENTRY",
            "6:9: RSB in a CALL routine",
            "8:9: RET outside a CALL routine (.CALL_ENTRY or .ENTRY)",
            "9:9: .JSB_ENTRY follows the label that names the routine",
            "10:9: HOME_ARGS and QUAD_ARGS exclude each other",
            "11:9: unknown parameter BOGUS",
            "12:9: AP isn't one of R0-R11",
            "14:9: RSB in a CALL routine",
        ]
    );
}

/// A register written that the entry mask leaves out is a warning, once:
/// vmacro saves it anyway.
#[test]
fn unmasked() {
    let source = "\
        .PSECT  CODE, EXE
        .ENTRY  START, ^M<R2>
        MOVL    #1, R2
        MOVL    #1, R3
        INCL    R3
        RET
        .END    START
";
    assert_eq!(
        warnings(source),
        ["4: R3 is written but isn't in the entry mask"]
    );
}

/// A branch into another routine, or out to another module, ends in that
/// routine's RET or RSB: it must restore what this one saves, and its
/// target be declared. ELSEWHERE's linkage says it keeps every register.
#[test]
fn shared_code() {
    let source = "\
        .EXTRN  ELSEWHERE
        .CALL_LINKAGE ELSEWHERE
        .ENTRY  START, ^M<>
        JSB     A
        JSB     B
        RET
A:      .JSB_ENTRY  SCRATCH=<R2>
        MOVL    #1, R2
        BRB     SHARED
B:      .JSB_ENTRY  SCRATCH=<R2>
        MOVL    #2, R2
SHARED: .GLOBAL_LABEL
        INCL    R2
        JMP     G^ELSEWHERE
C:      .JSB_ENTRY
        MOVL    #1, R3
        BRB     SHARED
        BRB     INSIDE
        JMP     G^ELSEWHERE
D:      .JSB_ENTRY
INSIDE: RSB
        .END    START
";
    assert_eq!(
        errors(source),
        [
            "17:9: a branch to SHARED, in another routine, which doesn't restore the registers this routine saves (R2 R3; it restores none)",
            "18:9: INSIDE is in another routine: declare it with .GLOBAL_LABEL",
            "19:9: a branch to ELSEWHERE, in another module, which doesn't restore the registers this routine saves (R2 R3; it restores none)",
        ]
    );
}

/// Code may go on into the next routine only if neither saves anything.
#[test]
fn falls_into() {
    let source = "\
        .ENTRY  START, ^M<>
        JSB     A
        JSB     C
        RET
A:      .JSB_ENTRY  SCRATCH=<R2>
        MOVL    #1, R2
B:      .JSB_ENTRY  SCRATCH=<R2>
        INCL    R2
C:      .JSB_ENTRY
        MOVL    #1, R3
        RSB
        .END    START
";
    assert_eq!(
        errors(source),
        ["9:2: C comes right after code that goes on into it: end that with a branch"]
    );
}

/// A branch to another JSB routine's entry is a tail call: its prologue
/// saves what it must, so the routine that goes there only needs nothing
/// of its own to restore. .EXCEPTION_ENTRY code has nothing to restore, and
/// a label where code reloads SP is a long jump: both go anywhere.
#[test]
fn tail_calls() {
    let source = "\
        .ENTRY  START, ^M<>
        JSB     A
        JSB     B
        RET
A:      .JSB_ENTRY  SCRATCH=<R2>
        MOVL    #1, R2
        BRB     C
B:      .JSB_ENTRY
        MOVL    #1, R3
        BRB     C
C:      .JSB_ENTRY
        MOVL    #2, R4
        RSB
D:      .EXCEPTION_ENTRY
        BRW     INSIDE
E:      .JSB_ENTRY
        MOVL    #1, R5
        BRB     BACK
        .ENTRY  F, ^M<R2>
INSIDE: .GLOBAL_LABEL
        MOVL    #1, R2
        RET
BACK:   MOVL    SAVED, SP
        RET
SAVED:  .LONG   0
        .END    START
";
    assert_eq!(
        errors(source),
        [
            "10:9: a branch to C, in another routine, which doesn't restore the registers this routine saves (R3; it restores R4)"
        ]
    );
}

/// Modules compiled together know each other's routines: a JSB to one in
/// another module modifies what its declaration says, and a branch into
/// another module is checked as one in the same module.
#[test]
fn modules() {
    let lib = "\
        .PSECT  CODE, EXE
PUT::   .JSB_ENTRY  SCRATCH=<R2>
        MOVL    #1, R2
        MOVL    #1, R3
        RSB
INNER:: MOVL    #3, R3
        RSB
        .END
";
    let user = "\
        .EXTRN  PUT, INNER
        .PSECT  CODE, EXE
TAIL:   .JSB_ENTRY  SCRATCH=<R2>
        BRW     PUT
SHARE:  .JSB_ENTRY
        BRW     INNER
        .END
";
    let opts = vasm::Options::default();
    let results = vmacro::compile_modules(&[(lib, &opts), (user, &opts)]);
    assert!(results[0].is_ok());
    let errors: Vec<String> = match &results[1] {
        Ok(_) => Vec::new(),
        Err(d) => d.iter().map(|d| format!("{}: {}", d.line, d.msg)).collect(),
    };
    assert_eq!(
        errors,
        [
            "6: a branch to INNER, in another module, which doesn't restore the registers this routine saves (none; it restores R3)"
        ]
    );
}
