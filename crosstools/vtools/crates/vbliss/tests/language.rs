//! Constructs of the language that the test programs in tests/bliss don't
//! show: what vbliss accepts, what it rejects and with which message, and
//! what it makes of them in the IR (t8y:/host/vbliss).

fn translate(source: &str) -> vbliss::Output {
    vbliss::translate(
        std::path::Path::new("language.b64"),
        source,
        &vbliss::Options::default(),
    )
}

/// A module with `decls` and a routine M whose body is `body`.
fn module(decls: &str, body: &str) -> String {
    format!(
        "MODULE T (MAIN = M) =\nBEGIN\n{decls}\nROUTINE M =\n    BEGIN\n{body}\n    1\n    END;\nEND\nELUDOM\n"
    )
}

/// The IR of a module that must compile.
fn ir(source: &str) -> String {
    let out = translate(source);
    assert!(out.error().is_none(), "{:?}", out.error());
    out.ir.unwrap()
}

/// The first error's message of a module that must not compile.
fn error(source: &str) -> String {
    translate(source)
        .error()
        .map(|d| d.msg.clone())
        .expect("compiled")
}

#[test]
fn up_level_local() {
    let src = module(
        "ROUTINE OUTER =\n    BEGIN\n    LOCAL L;\n    ROUTINE INNER = .L;\n    INNER()\n    END;",
        "OUTER();",
    );
    assert_eq!(error(&src), "Illegal up-level reference to L");
    // A formal is the routine's own, an OWN anyone's.
    ir(&module(
        "ROUTINE OUTER (F) =\n    BEGIN\n    OWN O;\n    ROUTINE INNER = .O;\n    .F + INNER()\n    END;",
        "OUTER(1);",
    ));
}

#[test]
fn dec_mcs_source() {
    // read_source reads a byte a character: an e acute is 233 in a string.
    let path = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("mcs.b64");
    let mut bytes = module("OWN X;", "X = %C'?';").into_bytes();
    let q = bytes.iter().position(|&b| b == b'?').unwrap();
    bytes[q] = 0xE9;
    std::fs::write(&path, &bytes).unwrap();
    let text = vbliss::read_source(&path).unwrap();
    assert!(ir(&text).contains("233"));
    assert_eq!(vbliss::latin1(&text), bytes);
}

#[test]
fn general_routine_calls() {
    // LINKAGE(address, actuals) calls by the linkage named (LRM 12.2).
    let ir = ir(&module(
        "LINKAGE L = JSB (REGISTER = 2);\nOWN P, X;",
        "X = L(.P, 5);\nX = BLISS(.P, 6);",
    ));
    assert!(ir.contains("jsb %"), "{ir}");
    assert!(ir.contains("(6)"), "{ir}");
    assert_eq!(
        error(&module("LINKAGE L = CALL;", "L();")),
        "a general routine call needs the routine's address"
    );
}

#[test]
fn empty_actuals() {
    // An actual left out passes 0 (LRM 12.1.1).
    let ir = ir(&module("EXTERNAL ROUTINE R;", "R(1, , 3);\nR(1, );"));
    assert!(ir.contains("call $R(1, 0, 3)"), "{ir}");
    assert!(ir.contains("call $R(1, 0)"), "{ir}");
}

#[test]
fn predeclared_linkages() {
    // BLISS and the FORTRAN linkages are the calling standard, and like any
    // predeclared name a module may declare them again (LRM 13.5, 19.3).
    ir(&module(
        "EXTERNAL ROUTINE R : BLISS, F : FORTRAN, FS : FORTRAN_SUB, FF : FORTRAN_FUNC;",
        "R(); F(); FS(); FF();",
    ));
    ir(&module(
        "OWN BLISS;\nSTRUCTURE VECTOR[I; N] = [N] (VECTOR + I)<0, 8>;",
        "BLISS = 1;",
    ));
}

#[test]
fn linkage_switch() {
    // The LINKAGE switch gives routines declared after it, and calls
    // through an address, its linkage (LRM 12.1.3, 18.2).
    let ir = ir(&module(
        "LINKAGE MYJ = JSB (REGISTER = 2);\nSWITCHES LINKAGE(MYJ);\nEXTERNAL ROUTINE R;\nOWN P;",
        "R(5);\n(.P)(6);",
    ));
    assert!(ir.contains("jsb $R(5 r2)"), "{ir}");
    assert!(ir.contains("(6 r2)"), "{ir}");
}

#[test]
fn interrupt_linkages() {
    // Entered by the PAL: formals from the stack, the PC and PSL last; the
    // parameters popped, then REI (LRM 13.3.4.2, RN 2.8.4).
    let src = "MODULE T =\nBEGIN\nLINKAGE INT = INTERRUPT, EXC = EXCEPTION;\n\
        ROUTINE H (CODE, PC, PSL) : INT = (OWN X; X = .CODE);\n\
        ROUTINE E : EXC = 0;\nEND\nELUDOM\n";
    let out = translate(src);
    assert!(out.error().is_none(), "{:?}", out.error());
    let (ir, asm) = (out.ir.unwrap(), out.asm.unwrap());
    assert!(
        ir.contains("stackarg 0") && ir.contains("rei 1") && ir.contains("rei 0"),
        "{ir}"
    );
    assert!(
        asm.contains("add     sp, sp, #8\n        mov     x7, #146\n        svc     #0"),
        "{asm}"
    );
    assert_eq!(
        error(&module(
            "LINKAGE INT = INTERRUPT;\nROUTINE H : INT = 0;",
            "H();"
        )),
        "a routine with an INTERRUPT or EXCEPTION linkage can't be called"
    );
}

#[test]
fn nocount() {
    // A NOCOUNT linkage, or the module's NOCOUNT, passes no count in x9,
    // and its routines can't read their argument lists (RN 2.8.5).
    let src = module(
        "LINKAGE NC = CALL : NOCOUNT, WC = CALL : COUNT;\nEXTERNAL ROUTINE R : NC, W : WC;",
        "R(1, 2);\nW(3);",
    );
    let out = translate(&src);
    let (ir, asm) = (out.ir.unwrap(), out.asm.unwrap());
    assert!(ir.contains("call nocount $R(1, 2"), "{ir}");
    assert!(ir.contains("call $W(3)"), "{ir}");
    assert_eq!(asm.matches("mov     x9, #").count(), 1, "{asm}");
    let src = src
        .replace("(MAIN = M)", "(MAIN = M, NOCOUNT)")
        .replace("WC = CALL : COUNT", "WC = CALL");
    assert!(self::ir(&src).contains("call nocount $W(3"));
    assert_eq!(
        error(&module(
            "LINKAGE NC = CALL : NOCOUNT;\nBUILTIN ACTUALCOUNT;\nROUTINE R : NC = ACTUALCOUNT();",
            "R();"
        )),
        "ACTUALCOUNT can't be used in a routine whose linkage is NOCOUNT"
    );
}

#[test]
fn forward_data() {
    // FORWARD names OWN or GLOBAL data before it is declared, and an OWN
    // is its own address in its INITIAL (LRM 10.3).
    let ir = ir(&module(
        "FORWARD Y;\nOWN X : INITIAL(Y), Y : INITIAL(X), Q : VECTOR[2] INITIAL(Q, Q);",
        "X = .Y;",
    ));
    assert!(ir.contains("data $X in $OWN$ align 8 { 8 $Y }"), "{ir}");
    assert!(ir.contains("{ 8 $Q, 8 $Q }"), "{ir}");
    assert_eq!(
        error(&module("FORWARD Y;", "")),
        "Name declared FORWARD is not defined: Y"
    );
    assert_eq!(
        error(&module("OWN X;\nOWN X;", "")),
        "Name already declared in this block: X"
    );
}

#[test]
fn data_attributes() {
    // GRANULARITY, and ALIGN or VOLATILE between REF and the structure,
    // are accepted (RN 2.16.1, 2.25), as are field attributes (RN 2.28).
    ir(&module(
        "FIELD F = SET F1 = [0, 0, 32, 0] : VOLATILE, F2 = [5, 0, 16, 0] : NOCHECK_ALIGNMENT TES;\n\
         OWN G : BYTE GRANULARITY(0), B : BLOCK[8, BYTE] FIELD(F);",
        "BEGIN\nLOCAL PV : REF VOLATILE BLOCK[, BYTE], PA : REF ALIGN(0) BLOCK[, BYTE];\n\
         PV = B; PA = B; B[F1] = .PV[F2] + .PA[F1];\nEND;",
    ));
    // Literals take a range and EXTERNAL_NAME (LRM 9.12, RN 2.14).
    let ir = ir(&module(
        "LITERAL LS = -3 : SIGNED(4);\nGLOBAL LITERAL GL = 12 : UNSIGNED(4) EXTERNAL_NAME('glit');\n\
         EXTERNAL LITERAL EL : UNSIGNED(6) EXTERNAL_NAME('elit');\nOWN X;",
        "X = LS + GL + EL;",
    ));
    assert!(
        ir.contains("global $GLIT = 12") && ir.contains("$ELIT"),
        "{ir}"
    );
    assert_eq!(
        error(&module("LITERAL L = 99 : UNSIGNED(4);", "")),
        "Literal value cannot be represented in the declared number of bits"
    );
}
