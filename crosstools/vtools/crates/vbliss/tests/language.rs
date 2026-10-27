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
    let out = translate(&module("EXTERNAL ROUTINE R;", "R(1, , 3);\nR(1, );"));
    // BLISSA64 says so, at the first actual (oracle MACITR).
    assert_eq!(
        out.diags
            .iter()
            .filter(|d| d.msg == "Null expression appears in value-required context")
            .count(),
        2
    );
    let ir = out.ir.unwrap();
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
        "Name already declared in this block:  X"
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
    // A value out of the range is BLISSA64's warning (oracle RANGE).
    let out = translate(&module("LITERAL L = 99 : UNSIGNED(4);", ""));
    assert!(out.error().is_none());
    assert!(out.diags.iter().any(|d| d.sev == 'W'
        && d.msg == "Literal value cannot be represented in the declared number of bits"));
}

#[test]
fn psect_classes() {
    // Nothing goes in a linkage section or $INITIAL$ on ARM64 (RN 2.22).
    ir(&module(
        "PSECT LINK = MYLINK (READ, NOWRITE), INITIAL = MYINIT (NOWRITE);",
        "BEGIN\nLOCAL L : VECTOR[3] INITIAL(1, 2, 3);\n.L[2]\nEND;",
    ));
}

#[test]
fn codecomment() {
    // CODECOMMENT's texts are comments in the code; its value is the
    // block's (LRM 4.10).
    let out = translate(&module(
        "OWN X;",
        "X = CODECOMMENT 'one', 'two' : BEGIN 5 END;",
    ));
    let (ir, asm) = (out.ir.unwrap(), out.asm.unwrap());
    assert!(
        ir.contains("comment \"one\"") && ir.contains("store8 5, $X"),
        "{ir}"
    );
    assert!(asm.contains("; one\n        ; two"), "{asm}");
}

#[test]
fn host_and_target() {
    // vaxpunk is ARM64 and VMS; an unknown keyword is 0 (RN 2.12).
    let out = translate(&module(
        "%PRINT(%TARGET(ARM64), %TARGET(VMS), %TARGET(ARM64, VMS), %TARGET(ALPHA), %HOST(VMS), \
         %HOST(FOO), %BLISS(BLISS32V), '[', %BLISS32V(1), ']')",
        "",
    ));
    assert!(
        out.listing.contains("%PRINT:\t1110100[]"),
        "{}",
        out.listing
    );
}

#[test]
fn switches() {
    // %SWITCHES sees the module head's and SWITCHES' settings, block by
    // block (LRM 18.2); NOCODE checks the module and makes no code.
    let out = translate(
        "MODULE T (NOSAFE, ZIP) =\nBEGIN\n%PRINT(%SWITCHES(SAFE), %SWITCHES(ZIP), %SWITCHES(ERRS))\n\
         SWITCHES SAFE, NOCHECK_ALIGNMENT;\n%PRINT(%SWITCHES(SAFE), %SWITCHES(CHECK_ALIGNMENT))\n\
         ROUTINE R = BEGIN SWITCHES NOSAFE; %PRINT(%SWITCHES(SAFE)) 0 END;\n\
         %PRINT(%SWITCHES(SAFE), %SWITCHES(OVERFLOW), %SWITCHES(QUAD_LITERALS))\nEND\nELUDOM\n",
    );
    for want in [
        "%PRINT:\t011",
        "%PRINT:\t10",
        "%PRINT:\t0\n",
        "%PRINT:\t101",
    ] {
        assert!(out.listing.contains(want), "{want}: {}", out.listing);
    }
    let out = translate("MODULE T (NOCODE) =\nBEGIN\nROUTINE R = 1;\nEND\nELUDOM\n");
    assert!(out.error().is_none() && out.nocode && out.asm.is_none());
}

#[test]
fn libraries() {
    // A library's source is read in a context of its own: it doesn't see
    // the module's names, gives the module only the names it declares
    // itself, and holds declarations only (LRM 16.6).
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("libraries");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("la.r64"), "LITERAL A1 = 1;\nMACRO MA = 2 %;\n").unwrap();
    std::fs::write(
        dir.join("lb.r64"),
        "LIBRARY 'LA';\nKEYWORDMACRO K(IDENT = 0) = IDENT + A1 %;\nLITERAL B1 = A1 + 1;\n",
    )
    .unwrap();
    std::fs::write(dir.join("lc.r64"), "OWN X;\n").unwrap();
    let path = dir.join("t.b64");
    let src = module(
        "MACRO IDENT = 'V1' %;\nLIBRARY 'LB';\nLIBRARY 'LB';\n\
         %PRINT(%DECLARED(A1), %DECLARED(B1), %DECLARED(%QUOTE MA), %DECLARED(%QUOTE K))",
        "",
    );
    let out = vbliss::translate(&path, &src, &vbliss::Options::default());
    assert!(out.error().is_none(), "{:?}", out.error());
    assert!(out.listing.contains("%PRINT:\t0101"), "{}", out.listing);
    let out = vbliss::translate(
        &path,
        &module("LIBRARY 'LC';", ""),
        &vbliss::Options::default(),
    );
    assert_eq!(
        out.error().map(|d| d.msg.as_str()),
        Some("Illegal declaration type in library source module")
    );
}

#[test]
fn dec_forms() {
    // What DEC's own libraries lean on, which BLISSA64 takes: a
    // KEYWORDMACRO without formals, BUILTIN again in a block, prototypes,
    // routine value attributes and ADDRESSING_MODE.
    ir(&module(
        "KEYWORDMACRO KZ = 5 %;\nBUILTIN ROT;\nBUILTIN ROT, BARRIER;\n\
         EXTERNAL ROUTINE P (A, B : REF VECTOR) : NOVALUE ADDRESSING_MODE(GENERAL),\n\
         Q : BLISS ADDRESSING_MODE(GENERAL) LONG SIGNED;\n\
         FORWARD ROUTINE F (A) : NOVALUE;\nROUTINE F (A) : NOVALUE = 0;\nOWN X;",
        "X = KZ + ROT(1, 1) + Q();\nP(1, X);\nF(2);",
    ));
}

#[test]
fn floating_calls() {
    // %xFLOAT actuals by value (RN 2.10.4, DESIGN-0004): S and T in
    // v0-v7 with their class in x9, VAX formats general; results from v0
    // or x0.
    let out = translate(&module(
        "EXTERNAL ROUTINE R, G;\nOWN T, SS, X, Y;",
        "R(1, %TFLOAT(T), %SFLOAT(SS), 2);\nG(%GFLOAT(X); %GFLOAT(Y));\nR(%TFLOAT(T); %TFLOAT(Y));",
    ));
    let (ir, asm) = (out.ir.unwrap(), out.asm.unwrap());
    assert!(ir.contains("fcall $R(1, t:$T, s:$SS, 2)"), "{ir}");
    assert!(ir.contains("fcall $G(g:$X) -> g:$Y"), "{ir}");
    // Four arguments, the second of class 2 (T), the third of class 1 (S).
    assert!(
        asm.contains(&format!("mov     x9, #{}", 4 | 2 << 10 | 1 << 12)),
        "{asm}"
    );
    assert!(
        asm.contains("ldr     d0, [x16]") && asm.contains("ldr     s1, [x16]"),
        "{asm}"
    );
    assert!(asm.contains("str     d0, [x16]"), "{asm}");
}

#[test]
fn floating_literals_and_nofp() {
    // %H is the VAX's (oracle FLTERR); ENVIRONMENT(NOFP) warns of the
    // floating built-ins (oracle NOFP).
    let out = translate(&module("OWN X;", "X = %H'1.0';"));
    assert!(
        out.diags
            .iter()
            .any(|d| d.msg == "Language feature not present in BLISS-64E:  %H")
    );
    let out = translate(
        "MODULE T (ENVIRONMENT(NOFP)) =\nBEGIN\nBUILTIN ADDT;\nOWN A, B, C;\n\
         ROUTINE M = (ADDT(A, B, C); 1);\nEND\nELUDOM\n",
    );
    assert!(out.diags.iter().any(|d| d.sev == 'W'
        && d.msg == "Illegal use of floating point built-in function with ENVIRONMENT(NOFP)"));
}

#[test]
fn string_encodings() {
    // %RAD50_11 and %P, which BLISSA64 has (oracle RAD64): RAD50 words,
    // VAX packed decimal.
    let ir = ir(&module(
        "BIND R = UPLIT(%RAD50_11 'ABC123') : VECTOR[, WORD], P = UPLIT BYTE(%P'-123', %P'4567') : VECTOR[, BYTE];\nOWN X;",
        "X = .R[0] + .P[0];",
    ));
    // ABC is 1683, 123 50913, words in memory order; -123 is 12 3D and
    // 4567 04 56 7C.
    assert!(ir.contains(r#""\x93\x06\xe1\xc6"#), "{ir}");
    assert!(ir.contains(r#"{ "\x12=", "\x04V|""#), "{ir}");
}

#[test]
fn macro_actuals() {
    // Empty actuals at the end don't count (oracle MACCNT); %STRING takes
    // a reserved word (oracle STRKEY); a module's declarations may have a
    // stray semicolon (oracle STRAY).
    let out = translate(&module(
        "MACRO L(A) = %LENGTH %, C(A)[] = 1 %;\n\
         %PRINT(L(), L(1), L(1,), L(1,,), L(,1), 0 C(,), C(1))\n\
         %PRINT(%STRING(GLOBAL, ROUTINE));;",
        "",
    ));
    assert!(out.error().is_none(), "{:?}", out.error());
    assert!(out.listing.contains("%PRINT:\t0111201"), "{}", out.listing);
    assert!(
        out.listing.contains("%PRINT:\tGLOBALROUTINE"),
        "{}",
        out.listing
    );
}

#[test]
fn overflow_switch() {
    // OVERFLOW checks a routine's +, - and *, signaling SS$_INTOVF and
    // going on with the wrapped value (RN 2.26); compile-time arithmetic
    // and NOOVERFLOW, the default, don't.
    let src = "MODULE T (OVERFLOW) =\nBEGIN\nOWN X, V : VECTOR[2 * 3];\n\
               ROUTINE R = X = .X + 1 - .X * 2;\nEND\nELUDOM\n";
    let ir = ir(src);
    for want in ["addv", "subv", "mulv", "call $LIB$SIGNAL(1148)", "z 48"] {
        assert!(ir.contains(want), "{want}: {ir}");
    }
    assert!(!self::ir(&src.replace("(OVERFLOW)", "")).contains("addv"));
}
