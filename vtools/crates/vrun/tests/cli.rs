//! Runs CLITEST, the system disk's check of the command parser
//! (roottask/sysexe/clitest.mar, with lib/cli.mar and its CLITEST.CLD
//! compiled by vcdu, and lib/getforeign.mar), under vrun, so that the
//! parser is checked where the system itself can't be built. A stub stands
//! in for PUT_LINE, which writes with vrun's put, SYS$EXIT, and
//! LIB$GET_INPUT, which CLITEST doesn't call.

use std::fs;
use std::path::Path;
use std::process::Command;

const STUB: &str = "
        .TITLE  STUB    PUT_LINE, SYS$EXIT and LIB$GET_INPUT, under vrun
        .LIBRARY \"vrun.mlb\"
        .PSECT  STUB_DATA, NOEXE, WRT, LONG
NL:     .ASCII  <10>
        .PSECT  STUB_CODE, EXE, LONG
        .ENTRY  PUT_LINE, ^M<R2>
        MOVL    4(AP), R1
        MOVZBL  (R1)+, R2
        $WRITE  x1, x19  ; R1, R2
        MOVAB   NL, R1
        MOVL    #1, R2
        $WRITE  x1, x19  ; R1, R2
        RET
        .ENTRY  SYS$EXIT, ^M<>
        MOVL    4(AP), R0
        svc     #1                      ; doesn't return
        RET
        .ENTRY  LIB$GET_INPUT, ^M<>
        MOVL    #44, R0
        RET
        .END
";

#[test]
fn clitest() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let sysexe = root.join("roottask/sysexe");
    let compile = |name: &str, source: &str| {
        let opts = vasm::Options {
            name: name.into(),
            include: vec![root.join("vtools/lib"), sysexe.clone()],
            ..Default::default()
        };
        let object = vmacro::compile(source, &opts).unwrap_or_else(|d| {
            panic!("{name}: {:?}", d.iter().map(|d| &d.msg).collect::<Vec<_>>())
        });
        (name.to_string(), vms_obj::obj::write(&object.records))
    };
    let read = |p: &str| fs::read_to_string(sysexe.join(p)).unwrap();
    let cld = vec![("clitest.cld".to_string(), read("clitest.cld"))];
    let tables = vcdu::compile("CLITEST_TABLES", &cld).unwrap();
    let modules = [
        compile("CLITEST", &read("clitest.mar")),
        compile("CLI", &read("lib/cli.mar")),
        compile("GETFOREIGN", &read("lib/getforeign.mar")),
        compile("TABLES", &tables),
        compile("STUB", STUB),
    ];
    let opts = vlink::Options {
        base: vlink::DEFAULT_BASE,
        name: "CLITEST".into(),
        transfer: None,
        link_time: 0,
        relocatable: false,
    };
    let linked = vlink::link(&modules, &opts).unwrap();
    let exe = Path::new(env!("CARGO_TARGET_TMPDIR")).join("clitest.exe");
    fs::write(&exe, linked.image.write()).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vrun"))
        .arg(&exe)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success() && stdout.contains("CLITEST: ok"),
        "exit {:?}, a failed check exits with 2 * its number\n{stdout}{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
}
