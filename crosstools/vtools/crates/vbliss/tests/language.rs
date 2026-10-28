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
fn main_routine() {
    // MAIN names the module's routine, not one an inner block declared
    // EXTERNAL under the same name.
    let out = translate(&module(
        "ROUTINE A = BEGIN EXTERNAL ROUTINE M : EXTERNAL_NAME('INNER'); 0 END;",
        "",
    ));
    let asm = out.asm.unwrap();
    assert!(asm.contains(".END    M\n"), "{asm}");
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
    // A literal's column counts them a byte each too.
    let text = module("OWN X : INITIAL('\u{e9}\u{e9}\u{e9}'+1);", "");
    assert!(
        ir(&text).contains(&(0xE9E9E9 + 1).to_string()),
        "{}",
        ir(&text)
    );
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
    // Even a routine's name, declared with another (oracle GCL).
    let ir = translate(&module(
        "LINKAGE L = JSB (REGISTER = 2);\nEXTERNAL ROUTINE EXT;\nOWN X;",
        "X = L(EXT, 7);",
    ))
    .ir
    .unwrap();
    assert!(ir.contains("jsb $EXT"), "{ir}");
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
    // One parameter, padded to 16 bytes: the PC and PSL after it, and
    // REI's frame at x18 past it.
    assert!(
        asm.contains(
            "add     sp, sp, #16\n        mov     x18, sp\n        mov     x7, #146\n        svc     #0"
        ),
        "{asm}"
    );
    let size: u32 = asm
        .split_once("stp     x29, x30, [sp, #-")
        .and_then(|(_, r)| r.split_once(']'))
        .and_then(|(n, _)| n.parse().ok())
        .unwrap();
    for off in [size, size + 16, size + 24] {
        assert!(asm.contains(&format!("[x29, #{off}]\n")), "{off}: {asm}");
    }
    assert_eq!(
        error(&module(
            "LINKAGE INT = INTERRUPT;\nROUTINE H : INT = 0;",
            "H();"
        )),
        "a routine with an INTERRUPT or EXCEPTION linkage can't be called"
    );
}

#[test]
fn switches_count() {
    // %SWITCHES(COUNT) and %SWITCHES(NOCOUNT) are both 1 in BLISSA64,
    // under /NOCOUNT or the module's NOCOUNT too (oracle SWC, SW3).
    let src = module(
        "OWN X : INITIAL(%SWITCHES(COUNT) + 2 * %SWITCHES(NOCOUNT));",
        "",
    );
    let mut opts = vbliss::Options::default();
    assert_eq!(opts.qualifier("/NOCOUNT"), Ok(true));
    for (src, opts) in [
        (src.clone(), vbliss::Options::default()),
        (src.clone(), opts),
        (
            src.replace("(MAIN = M)", "(MAIN = M, NOCOUNT)"),
            vbliss::Options::default(),
        ),
    ] {
        let out = vbliss::translate(std::path::Path::new("language.b64"), &src, &opts);
        let ir = out.ir.unwrap();
        assert!(ir.contains("data $X in $OWN$ align 8 { 8 3 }"), "{ir}");
    }
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
    // Only the module's: SWITCHES NOCOUNT is a warning, and changes no
    // call (oracle NCT).
    let out = translate(&module(
        "EXTERNAL ROUTINE W;\nROUTINE X = BEGIN SWITCHES NOCOUNT; 0 END;",
        "W(3);",
    ));
    assert!(
        out.diags
            .iter()
            .any(|d| d.sev == 'W' && d.msg == "Missing or invalid switch specified")
    );
    assert!(!out.ir.unwrap().contains("nocount"));
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
        "Name declared FORWARD is not defined:  Y"
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
    // The widest ranges short of 64 bits too.
    let out = translate(&module(
        "LITERAL U = %X'7FFFFFFFFFFFFFFF' : UNSIGNED(63), S = -1 : SIGNED(63);",
        "",
    ));
    assert!(out.error().is_none() && out.diags.is_empty());
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
        "X = CODECOMMENT 'one', %STRING('two', %CHAR(10), 'nop') : BEGIN 5 END;",
    ));
    let (ir, asm) = (out.ir.unwrap(), out.asm.unwrap());
    assert!(
        ir.contains("comment \"one\"") && ir.contains("store8 5, $X"),
        "{ir}"
    );
    // A text of two lines is two comment lines, not a NOP.
    assert!(
        asm.contains("; one\n        ; two\n        ; nop\n"),
        "{asm}"
    );
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
    std::fs::write(
        dir.join("ld.r64"),
        "SWITCHES ZIP;\nLITERAL D1 = 1, D2 = %X'FFFFFFFF';\n",
    )
    .unwrap();
    std::fs::write(dir.join("le.r64"), "COMPILETIME N = 0;\n").unwrap();
    let path = dir.join("t.b64");
    let src = module(
        "MACRO IDENT = 'V1' %;\nLIBRARY 'LB';\nLIBRARY 'LB';\n\
         %PRINT(%DECLARED(A1), %DECLARED(B1), %DECLARED(%QUOTE MA), %DECLARED(%QUOTE K))",
        "",
    );
    let out = vbliss::translate(&path, &src, &vbliss::Options::default());
    assert!(out.error().is_none(), "{:?}", out.error());
    assert!(out.listing.contains("%PRINT:\t0101"), "{}", out.listing);
    // A library's switches are its own.
    let out = vbliss::translate(
        &path,
        &module("LIBRARY 'LD';\n%PRINT(%SWITCHES(ZIP))", ""),
        &vbliss::Options::default(),
    );
    assert!(out.listing.contains("%PRINT:\t0\n"), "{}", out.listing);
    // And the module's aren't the library's: NOQUAD_LITERALS in the
    // module head leaves the library's %X'FFFFFFFF' 4294967295.
    let out = vbliss::translate(
        &path,
        &module("LIBRARY 'LD';\n%PRINT(%NUMBER(D2))", "").replace(
            "MODULE T (MAIN = M)",
            "MODULE T (MAIN = M, NOQUAD_LITERALS)",
        ),
        &vbliss::Options::default(),
    );
    assert!(
        out.listing.contains("%PRINT:\t4294967295\n"),
        "{}",
        out.listing
    );
    // Each LIBRARY declares its own COMPILETIME names, with the library's
    // values, which %ASSIGN in one block doesn't change in another (oracle
    // LCTT).
    let out = vbliss::translate(
        &path,
        "MODULE T =\nBEGIN\nLIBRARY 'LE';\n%ASSIGN(N, 3)\n\
         ROUTINE A = BEGIN LIBRARY 'LE'; %PRINT(%NUMBER(N)) %ASSIGN(N, 5) 1 END;\n\
         ROUTINE B = BEGIN LIBRARY 'LE'; %PRINT(%NUMBER(N)) 2 END;\n\
         %PRINT(%NUMBER(N))\nEND\nELUDOM\n",
        &vbliss::Options::default(),
    );
    let printed: Vec<&str> = out
        .listing
        .lines()
        .filter_map(|l| l.split("%PRINT:\t").nth(1))
        .collect();
    assert_eq!(printed, ["0", "0", "3"], "{}", out.listing);
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
    assert!(ir.contains("fcall $G(g:$X) -> g:%"), "{ir}");
    // Four arguments, the second of class 2 (T), the third of class 1 (S).
    assert!(
        asm.contains(&format!("mov     x9, #{}", 4 | 2 << 10 | 1 << 12)),
        "{asm}"
    );
    assert!(
        asm.contains("ldr     d0, [x16]") && asm.contains("ldr     s1, [x16]"),
        "{asm}"
    );
    assert!(asm.contains("fmov    x") && asm.contains(", d0\n"), "{asm}");
    // A result's destination is found after the call, which may change it.
    let ir = translate(&module("EXTERNAL ROUTINE R;\nOWN P;", "R(; %TFLOAT(.P));"))
        .ir
        .unwrap();
    let (call, dest) = (ir.find("fcall $R() -> t:%"), ir.find("load8u $P"));
    assert!(call.is_some() && dest > call, "{ir}");
    // Not in a linkage's register.
    assert_eq!(
        error(&module(
            "LINKAGE J = JSB (REGISTER = 2);\nEXTERNAL ROUTINE R : J;\nOWN T;",
            "R(%TFLOAT(T));"
        )),
        "%xFLOAT actuals need the calling standard's linkage"
    );
    assert_eq!(
        error(&module(
            "LINKAGE J = JSB (REGISTER = 2);\nEXTERNAL ROUTINE R : J;\nOWN T;",
            "R(; %TFLOAT(T));"
        )),
        "%xFLOAT actuals need the calling standard's linkage"
    );
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
    // Nor link-time constants, which the linker works out (oracle OVFL).
    let ir = self::ir(&src.replace(
        "ROUTINE R = ",
        "ROUTINE R = BEGIN OWN P : INITIAL(X + 8); BIND Q = UPLIT(X + 16, X - 8); .P + .Q END;\nROUTINE S = ",
    ));
    assert!(ir.contains("$X+8") && ir.contains("$X+16"), "{ir}");
}

#[test]
fn dec_macro_idioms() {
    // What DEC's STARLET and TPAMAC lean on: a block may hold null
    // expressions, where a macro expanded to nothing ($FAB_INIT); %EXPLODE
    // of an empty string gives no actuals ($XABPRO()); ALIGN(0) packs
    // static data (TPAMAC's tables), which otherwise goes at its natural
    // boundary (LRM 9.5.3).
    let src = module(
        "MACRO N(X) = %LENGTH %;\n%PRINT(N(%EXPLODE('')), N(%EXPLODE('AB')))\n\
         OWN B : BYTE, W : WORD, L : LONG ALIGN(0), V : VECTOR[1, BYTE];",
        "(B = 1;; W = 2;);",
    );
    let out = translate(&src);
    assert!(out.error().is_none(), "{:?}", out.error());
    assert!(out.listing.contains("%PRINT:\t02"), "{}", out.listing);
    let ir = out.ir.unwrap();
    for want in [
        "$B in $OWN$ align 1 ",
        "$W in $OWN$ align 2 ",
        "$L in $OWN$ align 1 ",
        "$V in $OWN$ align 8 ",
    ] {
        assert!(ir.contains(want), "{want}: {ir}");
    }
}

#[test]
fn register_builtins() {
    // BUILTIN R0 and R1 are x0 and x1, fetched or assigned where the
    // source says; R2 to R11 are the routine's register data, as
    // REGISTER R2 = 2 would be. Declared outside the routine, R2 is an
    // up-level reference (BLISSA64 warns, oracle REGBI); R12 is no
    // register built-in.
    let ir = ir(&module(
        "ROUTINE S = 7;",
        "BUILTIN R0, R2; S(); R2 = .R0; R0 = .R2;",
    ));
    assert!(ir.contains("regarg r0") && ir.contains("setreg r0"), "{ir}");
    let up = module("BUILTIN R2;", "R2 = 1;");
    assert_eq!(error(&up), "Illegal up-level reference to R2");
    assert_eq!(
        error(&module("", "BUILTIN R12;")),
        "R12 is not a built-in vbliss knows"
    );
    let bare = error(&module("", "BUILTIN R0; LOCAL A; A = R0;"));
    assert!(
        bare.contains("R0 is a register: it has no address"),
        "{bare}"
    );
}

#[test]
fn qualifiers() {
    // The BLISS verb's qualifiers, shortened to four characters as DCL
    // allows; the switches they set the defaults of.
    let mut o = vbliss::Options::default();
    for q in [
        "/MACH",
        "/CHECK=(ALL,NOFIELD)",
        "/OPTIMIZE=LEVEL=4",
        "/TERM=NOERRORS",
    ] {
        assert!(o.qualifier(q).unwrap(), "{q}");
    }
    assert!(o.machine_code);
    for s in [
        ("CHECK_LONGWORD", true),
        ("CHECK_FIELD", false),
        ("ERRS", false),
    ] {
        assert!(o.switches.contains(&(s.0.into(), s.1)), "{s:?}");
    }
    assert!(o.qualifier("/SYNTAX_LEVEL=4").is_err());
    assert!(o.qualifier("/SOURCE_LIST=HEADER").is_err());
    assert!(!o.qualifier("/FOO").unwrap());
    let with = |q: &str, src: &str| {
        let mut o = vbliss::Options::default();
        assert!(o.qualifier(q).unwrap());
        vbliss::translate(std::path::Path::new("q.b64"), src, &o)
    };
    // Syntax level 2, the default, reserves QUAD; level 1 doesn't (oracle
    // SYNLV, SYNLV1).
    let quad = module("OWN QUAD;", "");
    assert_eq!(
        error(&quad),
        "Name to be declared is a reserved word:  QUAD"
    );
    assert!(with("/SYNTAX_LEVEL=1", &quad).error().is_none());
    let long = format!("OWN {};", "A".repeat(65));
    assert!(
        translate(&module(&long, "")).diags[0]
            .msg
            .contains("longer than 64 characters")
    );
    // vasm has every name in upper case: /NAMES can't make another.
    assert!(o.qualifier("/NAMES=UPPERCASE").is_ok());
    assert!(o.qualifier("/NAMES=LOWERCASE").is_err());
    // /NOCODE and /LIBRARY make no code; a library's source has only
    // declarations.
    assert!(with("/NOCODE", &module("", "")).nocode);
    let lib = with("/LIBRARY", "LITERAL A = 1;\nMACRO M = 2 %;\n");
    assert!(lib.nocode && lib.error().is_none());
    let bad = with("/LIBRARY", "ROUTINE R = 1;\n");
    assert!(bad.error().is_some());
    let xref = with("/CROSS_REFERENCE", &module("", ""));
    assert_eq!(
        xref.diags[0].msg,
        "CROSS_REFERENCE not implemented in BLISS-64E"
    );
}

#[test]
fn checks() {
    // What /CHECK finds that the tests/bliss-listings probes don't show,
    // in BLISSA64's words (oracle CHKAT, CHKRED; CHKSHR crashed GEM).
    let msgs = |q: &str, src: &str| -> Vec<String> {
        let mut o = vbliss::Options::default();
        assert!(o.qualifier(q).unwrap());
        let out = vbliss::translate(std::path::Path::new("c.b64"), src, &o);
        assert!(out.error().is_none(), "{:?}", out.error());
        out.diags.iter().map(|d| d.msg.clone()).collect()
    };
    let taken = msgs(
        "/CHECK=ADDRESS_TAKEN",
        &module(
            "OWN O, V : VECTOR[2];",
            "LOCAL A, B, C : ALIAS, D : VOLATILE, P;\nER(A); A = B; A = (IF .B THEN B ELSE A);\n\
             IF A THEN P = 1; P = C; P = D; P = A + 4; P = O; P = V; P = V[1];",
        )
        .replace("ROUTINE M", "EXTERNAL ROUTINE ER : NOVALUE;\nROUTINE M"),
    );
    let want: Vec<String> = ["B", "A", "O", "V"]
        .iter()
        .map(|n| format!("Address of variable {n} in value context; assuming ALIAS attribute"))
        .collect();
    assert_eq!(taken, want);
    let redeclared = msgs(
        "/CHECK=REDECLARE",
        "MODULE T =\nBEGIN\nOWN X;\nLINKAGE GL = CALL : GLOBAL(G = 5);\n\
         ROUTINE R : GL = BEGIN GLOBAL REGISTER G = 5; G = 1; 0 END;\n\
         ROUTINE M = BEGIN LOCAL X; X = 1; BEGIN LOCAL X; X = 2 END; .X END;\nEND\nELUDOM\n",
    );
    assert_eq!(
        redeclared,
        [
            "LINKAGE GL's GLOBAL REGISTER G is being redeclared locally",
            "Symbol X is declared OWN/GLOBAL in an outer block",
            "Symbol X is declared LOCAL in an outer block"
        ]
    );
    // An address in a shareable psect needs fixing up where the image
    // is loaded, which makes its page the process's own.
    let share = msgs(
        "/CHECK=SHARE",
        &module(
            "PSECT OWN = SHRD (SHARE, NOWRITE);\nOWN X, P : INITIAL(X), K : INITIAL(1);",
            "",
        ),
    );
    assert_eq!(share, ["Non-shareable value in SHARE PSECT SHRD"]);
}

#[test]
fn diagnostics() {
    // BLISSA64's messages, and going on after an error to report the
    // module's others (oracle DIAG1, DIAG2, MSGS; tests/bliss-listings).
    let out = translate(
        "MODULE D =\nBEGIN\nOWN QUAD;\nROUTINE N = (IF 1 THEN 2 ELSE);\nROUTINE P = (.X = ; 1);\n\
         ROUTINE Q = BEGIN 1 );\nEND\nELUDOM\n",
    );
    let msgs: Vec<(char, &str)> = out.diags.iter().map(|d| (d.sev, d.msg.as_str())).collect();
    assert_eq!(
        msgs,
        [
            ('E', "Name to be declared is a reserved word:  QUAD"),
            ('E', "expected an expression, found )"),
            ('W', "Undeclared name:  X"),
            (
                'W',
                "Missing operand following \"=\".  A literal zero has been inserted"
            ),
            ('E', "BEGIN paired with right parenthesis"),
        ]
    );
    // A NOVALUE routine's value: a warning, unless its routine has an
    // error, as BLISSA64 looks at values only then.
    let nv = "ROUTINE NV : NOVALUE = 0;";
    let warned = translate(&module(nv, "LOCAL Z; Z = NV();"));
    assert_eq!(warned.diags[0].msg, "ROUTINE NV has no value");
    let not = translate(&module(nv, "LOCAL Z; Z = NV(); Z = (;"));
    assert!(not.diags.iter().all(|d| d.sev == 'E'), "{:?}", not.diags);
}

/// A module compiled against vaxpunk's own require files, `lib/`.
fn with_lib(source: &str) -> vbliss::Output {
    let opts = vbliss::Options {
        include: vec![std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../lib")],
        ..vbliss::Options::default()
    };
    vbliss::translate(std::path::Path::new("lib.b64"), source, &opts)
}

#[test]
fn service_macros() {
    // vdefs's keyword macros for the system services (UM 8.3): the
    // keywords in any order, the left-out ones' defaults, $HIBER bare.
    let out = with_lib(&module(
        "LIBRARY 'SYS$LIBRARY:STARLET';\nOWN CHAN : WORD, IOSB : VECTOR[4, WORD];",
        "$ASSIGN(CHAN = CHAN, DEVNAM = %ASCID 'TT');\n\
         $QIOW(CHAN = .CHAN, FUNC = IO$_WRITEVBLK, IOSB = IOSB, P2 = 10);\n$HIBER;",
    ));
    assert!(out.error().is_none(), "{:?}", out.error());
    let ir = out.ir.unwrap();
    for call in [
        "call $SYS$ASSIGN(",
        "call $SYS$QIOW(0, ",
        "call $SYS$HIBER()",
    ] {
        assert!(ir.contains(call), "{call}: {ir}");
    }
    let missing = with_lib(&module(
        "LIBRARY 'SYS$LIBRARY:STARLET';",
        "$QIOW(FUNC = 1);",
    ));
    assert_eq!(missing.error().unwrap().msg, "%ERROR:\t$QIOW needs CHAN");
}

#[test]
fn rms_and_helper_macros() {
    // vdefs's RMS control block macros (UM 8.4), static and at run time,
    // the RMS services, and STARLET's helper macros.
    let out = with_lib(&module(
        "LIBRARY 'SYS$LIBRARY:STARLET';\nOWN BUF : $BBLOCK[132],\n\
         INFAB : $FAB(FNM = 'MYFILE.SRC', FAC = <GET, PUT>, SHR = <GET, MSE>, RFM = FIX),\n\
         INRAB : $RAB(UBF = BUF, USZ = 132, ROP = <LOC, RAH>, FAB = INFAB),\n\
         KEY : $XABKEY(POS = <0, 10>, SIZ = <4, 6>, FLG = DUP), SUM : $XABSUM();\n\
         LITERAL $EQULST(XX$_, LOCAL, 4, 2, (A), (B), (C, 10));\n\
         LITERAL SHR$_BEGIN = 1000, SHR$_ENDED = 1000;\n$SHR_MSGDEF(COPY, 103, LOCAL, (BEGIN, INFO), (ENDED, 2))\n\
         %PRINT(%NUMBER(COPY$_BEGIN), ' ', %NUMBER(COPY$_ENDED))",
        "LOCAL F : $FAB_DECL;\n$FAB_INIT(FAB = F, FNM = 'Y.DAT');\n$OPEN(FAB = INFAB);\n\
         $GET(RAB = INRAB, ERR = 0);\n\
         %PRINT(%NUMBER(XX$_A), ' ', %NUMBER(XX$_B), ' ', %NUMBER(XX$_C), ' ', $FIELDWIDTH(FAB$B_FAC), ' ', $BYTEOFFSET(FAB$B_FAC))",
    ));
    assert!(out.error().is_none(), "{:?}", out.error());
    assert!(
        out.listing.contains("%PRINT:\t4 6 10 8 22"),
        "{}",
        out.listing
    );
    // SHR$_BEGIN + 103 * 65536 + STS$K_INFO, and + 2.
    assert!(
        out.listing.contains("%PRINT:\t6751211 6751210"),
        "{}",
        out.listing
    );
    let ir = out.ir.unwrap();
    // The FAB: BID 3, BLN 80 ('P'); FAC GET|PUT and SHR GET|MSE at 22;
    // FNA at 44, then FNS 10.
    let fab = ir.lines().find(|l| l.starts_with("data $INFAB")).unwrap();
    for want in [
        "{ \"\\x03P",
        "\\x03\\x12",
        "4 $P.1, \"\\x00\\x00\\x00\\x00\\x0a",
    ] {
        assert!(fab.contains(want), "{want}: {fab}");
    }
    for call in ["call $SYS$OPEN(", "call $SYS$GET(", "call $LIB$SIGNAL"] {
        if call.contains("SIGNAL") {
            assert!(!ir.contains(call), "{ir}");
        } else {
            assert!(ir.contains(call), "{call}: {ir}");
        }
    }
}

#[test]
fn other_requires() {
    // vaxpunk's TPAMAC, ARCH_DEFS and CLIMAC: a LIB$TPARSE state table's
    // transitions packed in _LIB$STATE$, each a word of type and flags
    // and what the flags say follows; the keywords' text in _LIB$KEY1$.
    let out = with_lib(
        "MODULE T (MAIN = M) =\nBEGIN\nLIBRARY 'SYS$LIBRARY:STARLET';\n\
         REQUIRE 'SYS$LIBRARY:TPAMAC';\nREQUIRE 'SYS$LIBRARY:ARCH_DEFS';\n\
         REQUIRE 'SYS$LIBRARY:CLIMAC';\nOWN OPTS;\nFORWARD ROUTINE ACT;\n\
         $INIT_STATE(STB, KTB);\n\
         $STATE(, ('HELLO',,, 1, OPTS), ('BYE', NEXT, ACT,,, 7), ((SUB), TPA$_EXIT));\n\
         $STATE(NEXT, ('X'), (TPA$_EOS, TPA$_EXIT));\n$STATE(SUB, (TPA$_DECIMAL, TPA$_EXIT));\n\
         LITERAL CLI$C_REQDESC = 28, CLI$K_GETQUAL = 1;\n\
         OWN DESC : $CLIREQDESC(RQTYPE = GETQUAL);\n\
         ROUTINE ACT = 1;\nROUTINE M = %IF ALPHA AND ARM64 AND NOT VAX %THEN 1 %ELSE 0 %FI;\n\
         END\nELUDOM\n",
    );
    assert!(out.diags.is_empty(), "{:?}", out.diags);
    let asm = out.asm.unwrap();
    for want in [
        // HELLO: keyword 0 with an address and a mask.
        "WORD_STB::\n$$TPA_WORD:\n        .WORD  24832\n$$TPA_ITEM:\n        .LONG  OPTS\n",
        // BYE: keyword 1, a parameter, an action and a target.
        "        .WORD  37633\n$$TPA_ITEM.2:\n        .BYTE  1\n$$TPA_ITEM.3:\n        .LONG  7\n",
        // (SUB): the state's last, a subexpression, and a target.
        "        .WORD  7672\n$$TPA_ITEM.6:\n        .LONG  SUB\n",
        ".BYTE   72, 69, 76, 76, 79\n        .BYTE  255\n",
    ] {
        let want = want.replace("WORD_STB", "STB");
        assert!(asm.contains(&want), "{want}: {asm}");
    }
}

#[test]
fn vax_builtin_macros() {
    // vaxpunk's LIB gives back the VAX built-ins as macros: BUILTIN X
    // declares it for the block, X(...) then expands (tests/bliss/vaxbi32
    // and vaxbi64 run the string and bit ones).
    let out = with_lib(&module(
        "LIBRARY 'SYS$LIBRARY:LIB';\nOWN Q : VECTOR[2], L, R, E, T : VECTOR[256, BYTE];",
        "BUILTIN INSQUE, REMQUE, PROBER, MTPR, MFPR, CHMU, HALT, MOVPSL, EMUL, EDIV, ASHQ,\n\
         INDEX, MOVTUC, SPANC, CALLG, BISPSW;\n\
         INSQUE(E, Q); REMQUE(.Q[0], R); PROBER(%REF(0), %REF(8), Q);\n\
         MTPR(%REF(2), PR$_IPL); MFPR(PR$_IPL, L); MOVPSL(L); BISPSW(%REF(32));\n\
         EMUL(%REF(-3), %REF(4), %REF(5), Q); EDIV(%REF(7), Q, L, R); ASHQ(%REF(4), Q, Q);\n\
         INDEX(%REF(2), %REF(0), %REF(9), %REF(4), %REF(0), L);\n\
         MOVTUC(%REF(3), UPLIT('ABC'), %REF(%C'B'), T, %REF(3), Q);\n\
         SPANC(%REF(3), UPLIT('ABC'), T, %REF(1)); CALLG(Q, 0);\n\
         IF 0 THEN (CHMU(%REF(3)); HALT);",
    ));
    assert!(out.error().is_none(), "{:?}", out.error());
    let ir = out.ir.unwrap();
    for want in ["call $LIB$CALLG(", "pal "] {
        assert!(ir.contains(want), "{want}: {ir}");
    }
    // The interlocked queues are the PAL's calls, REMQHI's entry its R1.
    let insqhi = with_lib(&module(
        "LIBRARY 'SYS$LIBRARY:LIB';\nOWN Q : VECTOR[2], E : VECTOR[2], A;",
        "BUILTIN INSQHI, REMQHI; INSQHI(E, Q); REMQHI(Q, A);",
    ));
    assert!(insqhi.error().is_none(), "{:?}", insqhi.error());
    let ir = insqhi.ir.unwrap();
    for want in ["=l pal 135(", "=l pal 147(", "=l palr1"] {
        assert!(ir.contains(want), "{want}: {ir}");
    }
}
