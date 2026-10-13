//! What vbliss writes for what no test program can run or show: the PAL
//! built-ins (`docs/bliss64.md` *Built-ins*), which only the executive can
//! run, the attributes that name psects and linker symbols, and require
//! files in VMS directories.

const SOURCE: &str = "
MODULE PALS =
BEGIN
BUILTIN
    PAL_MTPR_IPL, PAL_MFPR_IPL, PAL_PROBER, PAL_CHMK, CALL_PAL, PAL_BPT, RPCC, PAL_HALT, RETURNADDRESS;

GLOBAL ROUTINE RAISE (IPL) =
    PAL_MTPR_IPL(.IPL);

GLOBAL ROUTINE LEVEL =
    PAL_MFPR_IPL();

GLOBAL ROUTINE READABLE (A, N) =
    PAL_PROBER(.A, .N, 3);

GLOBAL ROUTINE SERVICE (CODE, P) =
    PAL_CHMK(.CODE, .P);

GLOBAL ROUTINE OTHERS : NOVALUE =
    BEGIN
    CALL_PAL(%X'3E');
    CALL_PAL(%X'46', 1, 2, 3, 4);
    PAL_CHMK(7);
    PAL_BPT();
    RPCC();
    RETURNADDRESS();
    PAL_HALT()
    END;
END
ELUDOM
";

fn translate(source: &str) -> vbliss::Output {
    vbliss::translate(
        std::path::Path::new("pals.b64"),
        source,
        &vbliss::Options::default(),
    )
}

#[test]
fn pal_calls() {
    let out = translate(SOURCE);
    assert!(out.error().is_none(), "{:?}", out.error());
    let ir = out.ir.unwrap();
    for want in [
        "=l pal 15(%",
        "=l pal 14()",
        "=l pal 143(%",
        ", 3)",
        "=l pal 62()",
        "=l pal 70(1, 2, 3, 4)",
        "=l pal 458883()",
        "bpt",
        "=l rpcc",
        "=l retaddr",
        "=l pal 0()",
    ] {
        assert!(ir.contains(want), "no {want:?} in\n{ir}");
    }
    // CHMK's code goes in x7's bits 31:16, computed when it isn't constant.
    assert!(ir.contains("=l or %"), "{ir}");
    let asm = out.asm.unwrap();
    assert!(asm.contains("svc     #0"));
    assert!(asm.contains("mrs") && asm.contains("CNTVCT_EL0"));
    assert!(asm.contains("[x29, #8]"), "the return address");
    vasm::assemble_with(&asm, &vasm::Options::default(), "test", None).expect("vasm assembles it");
}

#[test]
fn missing_pal_calls_are_errors() {
    let out = translate(
        "MODULE M = BEGIN BUILTIN PAL_INSQHIL; ROUTINE R = PAL_INSQHIL(0, 0); END ELUDOM",
    );
    let e = out.error().expect("an error");
    assert!(e.msg.contains("DESIGN-0001"), "{}", e.msg);
}

#[test]
fn psect_and_external_name() {
    let out = translate(
        "MODULE M =
BEGIN
PSECT OWN = DATA2 (WRITE, NOEXECUTE);
GLOBAL G : EXTERNAL_NAME('my_g') PSECT(SPECIAL);
EXTERNAL E : EXTERNAL_NAME('Their_E');
EXTERNAL ROUTINE F : EXTERNAL_NAME('f_c');
GLOBAL ROUTINE R : EXTERNAL_NAME('r_c') PSECT(MYCODE) = F(.E);
END
ELUDOM",
    );
    assert!(out.error().is_none(), "{:?}", out.error());
    let asm = out.asm.unwrap();
    for want in [
        ".PSECT  SPECIAL",
        "MY_G::",
        ".EXTERNAL THEIR_E",
        "bl      F_C",
        ".PSECT  MYCODE",
        "R_C::",
    ] {
        assert!(asm.contains(want), "no {want:?} in\n{asm}");
    }
}

#[test]
fn plit_psect() {
    let out = translate(
        "MODULE M = BEGIN PSECT PLIT = TABLES (NOWRITE); BIND T = UPLIT (1, 2), U = PLIT PSECT(OTHER) (3); OWN P : VECTOR[2] INITIAL(T, U); END ELUDOM",
    );
    assert!(out.error().is_none(), "{:?}", out.error());
    let ir = out.ir.unwrap();
    assert!(ir.contains("in TABLES") && ir.contains("in OTHER"), "{ir}");
}

#[test]
fn require_directories() {
    let root = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("require-dirs");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("top/sub")).unwrap();
    std::fs::create_dir_all(root.join("other")).unwrap();
    std::fs::write(root.join("top/sub/a.r64"), "LITERAL A = 1;").unwrap();
    std::fs::write(root.join("other/b.r64"), "LITERAL B = 2;").unwrap();
    std::fs::write(root.join("top/c.r64"), "LITERAL C = 3;").unwrap();
    let source = "MODULE M = BEGIN
REQUIRE '[.SUB]A';
REQUIRE '[-.OTHER]B';
REQUIRE 'SRC$:[TOP]C.R64';
OWN X : INITIAL(A + B + C);
END ELUDOM";
    let opts = vbliss::Options {
        include: vec![root.clone()],
        ..vbliss::Options::default()
    };
    let out = vbliss::translate(&root.join("top/m.b64"), source, &opts);
    assert!(out.error().is_none(), "{:?}", out.error());
    assert!(out.ir.unwrap().contains("8 6"));
}
