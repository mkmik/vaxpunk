//! Runs hand-encoded images under vrun in QEMU (needs qemu-system-aarch64).
//! VRUN_FLAGS adds vrun options, for example `VRUN_FLAGS=--hvf cargo test`.

use std::process::{Command, Output};

use vms_obj::exe::{Eisd, Fixups, Image, Section};

const BASE: u64 = 0x10000;

/// An image with one code section at 0x10000, entered at its start.
fn image(code: &[u32]) -> Image {
    Image {
        name: "TEST".into(),
        ident: "V1".into(),
        link_time: 0,
        transfer: BASE,
        sections: vec![code_section(code)],
        fixups: None,
        shareables: Vec::new(),
        vector: None,
    }
}

fn code_section(code: &[u32]) -> Section {
    let data: Vec<u8> = code.iter().flat_map(|i| i.to_le_bytes()).collect();
    Section {
        vaddr: BASE,
        size: data.len() as u32,
        flags: Eisd::M_EXE,
        data,
    }
}

fn vrun(image: &Image, args: &[&str]) -> Output {
    vrun_with(image, &[], args)
}

/// Runs `image` with vrun `flags` before the image name and `args` after it.
fn vrun_with(image: &Image, flags: &[&str], args: &[&str]) -> Output {
    let path = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "{}-{:?}.exe",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::write(&path, image.write()).unwrap();
    let env_flags = std::env::var("VRUN_FLAGS").unwrap_or_default();
    let out = Command::new(env!("CARGO_BIN_EXE_vrun"))
        .args(["--timeout", "20"])
        .args(env_flags.split_whitespace())
        .args(flags)
        .arg(&path)
        .args(args)
        .output()
        .unwrap();
    std::fs::remove_file(&path).unwrap();
    out
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Code followed by data in the same section.
fn with_data(code: &[u32], data: &[u8]) -> Image {
    let mut image = image(code);
    let s = &mut image.sections[0];
    s.data.extend_from_slice(data);
    s.size = s.data.len() as u32;
    image
}

#[test]
fn prints_hello() {
    // adr x0, msg; mov x1, #6; svc #2; mov x0, #1; ret; msg: "hello\n"
    let code = [0x100000a0, 0xd28000c1, 0xd4000041, 0xd2800020, 0xd65f03c0];
    let out = vrun(&with_data(&code, b"hello\n"), &[]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert_eq!(stdout(&out), "hello\n");
}

#[test]
fn arguments() {
    // x0 is the info block: its descriptor holds the argument string.
    // ldrh w1, [x0, #16]; ldr w0, [x0, #20]; svc #2; mov x0, #1; ret
    let code = [0x79402001, 0xb9401400, 0xd4000041, 0xd2800020, 0xd65f03c0];
    let out = vrun(&image(&code), &["one", "two"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert_eq!(stdout(&out), "one two");
}

#[test]
fn dump_registers() {
    // svc #3; mov x0, #1; ret
    let out = vrun(&image(&[0xd4000061, 0xd2800020, 0xd65f03c0]), &[]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let dump = stdout(&out);
    assert!(
        dump.starts_with("x00 000000007ff00000  x01 0000000000000000"),
        "{dump}"
    );
    assert!(
        dump.contains("x30 000000007ff01000  sp  000000007fff0000\n"),
        "{dump}"
    );
    assert!(dump.ends_with("pc  0000000000010000\n"), "{dump}");
}

#[test]
fn bad_pointer_to_monitor_call() {
    // mov x0, #0; mov x1, #1; svc #2
    let out = vrun(&image(&[0xd2800000, 0xd2800021, 0xd4000041]), &[]);
    assert_eq!(out.status.code(), Some(12));
    let err = stderr(&out);
    assert!(
        err.contains("virtual address=0000000000000000, PC=0000000000010008"),
        "{err}"
    );
}

#[test]
fn returns_success() {
    // mov x0, #1; ret
    let out = vrun(&image(&[0xd2800020, 0xd65f03c0]), &[]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
}

#[test]
fn returns_failure_status() {
    // mov x0, #0x2c; ret
    let out = vrun(&image(&[0xd2800580, 0xd65f03c0]), &[]);
    assert_eq!(out.status.code(), Some(44), "{}", stderr(&out));
}

#[test]
fn access_violation() {
    // ldr x0, [x1] with x1 = 0 at entry
    let out = vrun(&image(&[0xf9400020]), &[]);
    assert_eq!(out.status.code(), Some(12));
    let err = stderr(&out);
    assert!(err.contains("%VRUN-F-ACCVIO"), "{err}");
    assert!(
        err.contains("virtual address=0000000000000000, PC=0000000000010000"),
        "{err}"
    );
}

#[test]
fn privileged_instruction() {
    // mrs x0, sctlr_el1
    let out = vrun(&image(&[0xd5381000]), &[]);
    assert_eq!(out.status.code(), Some(0x3c));
    assert!(stderr(&out).contains("%VRUN-F-OPCDEC"), "{}", stderr(&out));
}

#[test]
fn stack_overflow_hits_the_guard_page() {
    // 1: sub sp, sp, #4096; str xzr, [sp]; b 1b
    let out = vrun(&image(&[0xd14007ff, 0xf90003ff, 0x17fffffe]), &[]);
    assert_eq!(out.status.code(), Some(12));
    assert!(stderr(&out).contains("%VRUN-F-ACCVIO"), "{}", stderr(&out));
}

#[test]
fn code_is_not_writable() {
    // adr x1, .; str w0, [x1]
    let out = vrun(&image(&[0x10000001, 0xb9000020]), &[]);
    assert_eq!(out.status.code(), Some(12));
    assert!(
        stderr(&out).contains("virtual address=0000000000010000"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn unknown_monitor_call() {
    // svc #9
    let out = vrun(&image(&[0xd4000121]), &[]);
    assert_eq!(out.status.code(), Some(0x3c));
    assert!(
        stderr(&out).contains("SVC #9, PC=0000000000010000"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn moves_to_another_base() {
    // ldr x1, [x0, #24] (how far the image moved); svc #3; mov x0, #1; ret
    let code = [0xf9400c01, 0xd4000061, 0xd2800020, 0xd65f03c0];
    let mut image = image(&code);
    let out = vrun_with(&image, &["--base", "0x20000"], &[]);
    assert!(
        stderr(&out).contains("%VRUN-F-NOTRELOC"),
        "{}",
        stderr(&out)
    );

    // No addresses to fix, but a fixup section that says so.
    image.fixups = Some(Fixups::default());
    let out = vrun_with(&image, &["--base", "0x20000"], &[]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let dump = stdout(&out);
    assert!(dump.contains("x01 0000000000010000"), "{dump}");
    assert!(dump.ends_with("pc  0000000000020004\n"), "{dump}");

    for (base, why) in [
        ("0x21000", "not a multiple of 64 KB"),
        ("0x7ff00000", "overlaps vrun's range"),
    ] {
        let out = vrun_with(&image, &["--base", base], &[]);
        assert!(stderr(&out).contains(why), "{base}: {}", stderr(&out));
    }

    // A longword address after the code, which must stay below 2 GB.
    let mut image = with_data(&code, &0x10010u32.to_le_bytes());
    image.fixups = Some(Fixups {
        long: vec![16],
        long_min: 0x10010,
        long_max: 0x10010,
        ..Fixups::default()
    });
    let out = vrun_with(&image, &["--base", "0x80000000"], &[]);
    assert!(
        stderr(&out).contains("too far for the longword"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn hung_image_times_out() {
    // b .
    let out = vrun_with(&image(&[0x14000000]), &["--timeout", "1"], &[]);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("%VRUN-F-TIMEOUT"), "{}", stderr(&out));
}

/// Copies IN.TXT to OUT.TXT in --files' directory, five bytes at a time,
/// then checks the errors: a name outside it, a missing file, a bad channel.
const COPY: &str = r#"
        .TITLE  COPY    Copies a host file through the file monitor calls
        .LIBRARY "vrun.mlb"

        .MACRO  CHECK   STATUS, ?OK             ; exits with x0 unless STATUS
        cmp     x0, #STATUS
        b.eq    OK
        svc     #1
OK:
        .ENDM   CHECK

        .PSECT  $CODE$
START::
        adr     x19, in
        $FOPEN  x19, #6
        CHECK   1
        mov     x20, x1
        adr     x19, out
        $FOPEN  x19, #7, #1
        CHECK   1
        mov     x21, x1
        adrp    x19, buffer
        add     x19, x19, #:lo12:buffer
copy:   $FREAD  x20, x19, #5
        cmp     x0, #^X870              ; SS$_ENDOFFILE
        b.eq    done
        CHECK   1
        mov     x22, x1
        $FWRITE x21, x19, x22
        CHECK   1
        b       copy
done:   $FCLOSE x20
        CHECK   1
        $FCLOSE x21
        CHECK   1
        adr     x19, up
        $FOPEN  x19, #9
        CHECK   ^X24                    ; SS$_NOPRIV
        adr     x19, missing
        $FOPEN  x19, #7
        CHECK   ^X910                   ; SS$_NOSUCHFILE
        $FREAD  x20, x19, #1            ; closed
        CHECK   ^X13C                   ; SS$_IVCHAN
        $EXIT

        .PSECT  $LITERAL$
in:     .ASCII  "IN.TXT"
out:    .ASCII  "OUT.TXT"
up:     .ASCII  "../IN.TXT"
missing: .ASCII "MISSING"

        .PSECT  $BSS$
buffer: .BLKB   5

        .END    START
"#;

#[test]
fn copies_a_host_file() {
    let lib = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../lib");
    let opts = vasm::Options {
        name: "COPY".into(),
        include: vec![lib],
        ..Default::default()
    };
    let records = vasm::assemble(COPY, &opts)
        .unwrap_or_else(|d| panic!("{:?}", d.iter().map(|d| &d.msg).collect::<Vec<_>>()))
        .records;
    let objects = [("copy.mar".to_string(), vms_obj::obj::write(&records))];
    let link = vlink::Options {
        base: BASE,
        name: "COPY".into(),
        transfer: None,
        link_time: 0,
        relocatable: false,
        shareable: None,
    };
    let image = vlink::link(&objects, &link).unwrap().image;

    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("files");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let text: Vec<u8> = (0..=255).cycle().take(1000).collect();
    std::fs::write(dir.join("IN.TXT"), &text).unwrap();
    let out = vrun_with(&image, &["--files", dir.to_str().unwrap()], &[]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert_eq!(std::fs::read(dir.join("OUT.TXT")).unwrap(), text);

    // Without --files, the image may open nothing.
    let out = vrun(&image, &[]);
    assert_eq!(out.status.code(), Some(0x24), "{}", stderr(&out));
}

/// Copies the file its first argument names to the one its second names,
/// through the I/O module BLISS.EXE and VASM.EXE use under vrun.
const BLISS_COPY: &str = r#"
MODULE BCOPY (MAIN = BCOPY) =
BEGIN
REQUIRE 'FIO';

ROUTINE BCOPY (INFO) =
    BEGIN
    LOCAL ARGS, LEN, SPLIT, IN, OUT, COUNT, STATUS, BUFFER : VECTOR [5, BYTE];
    ! The argument string's static descriptor, in the runner info block.
    LEN = .(.INFO + 16)<0, 16>;
    ARGS = .(.INFO + 20)<0, 32>;
    SPLIT = 0;
    WHILE .SPLIT LSS .LEN AND .(.ARGS + .SPLIT)<0, 8> NEQ %C' ' DO
        SPLIT = .SPLIT + 1;
    STATUS = FIO$OPEN(.ARGS, .SPLIT, FIO$K_READ, IN);
    IF NOT .STATUS THEN RETURN .STATUS;
    STATUS = FIO$OPEN(.ARGS + .SPLIT + 1, .LEN - .SPLIT - 1, FIO$K_WRITE, OUT);
    IF NOT .STATUS THEN RETURN .STATUS;
    WHILE (STATUS = FIO$READ(.IN, BUFFER, 5, COUNT)) DO
        BEGIN
        STATUS = FIO$WRITE(.OUT, BUFFER, .COUNT);
        IF NOT .STATUS THEN RETURN .STATUS;
        END;
    IF .STATUS NEQ %X'870' THEN RETURN .STATUS;     ! SS$_ENDOFFILE
    FIO$CLOSE(.IN);
    FIO$CLOSE(.OUT)
    END;

END
ELUDOM
"#;

#[test]
fn bliss_copies_a_host_file() {
    let bliss = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../bliss");
    let opts = |name: &str| vasm::Options {
        name: name.into(),
        include: vec![bliss.clone()],
        ..Default::default()
    };
    let flags = vbliss::Options {
        include: vec![bliss.clone()],
        ..vbliss::Options::from_source(BLISS_COPY)
    };
    let fio = std::fs::read_to_string(bliss.join("fio.mar")).unwrap();
    let objects = [
        vbliss::compile_with(BLISS_COPY, &opts("BCOPY"), &flags).0,
        vasm::assemble(&fio, &opts("FIO")),
    ]
    .map(|r| {
        let records = r
            .unwrap_or_else(|d| panic!("{:?}", d.iter().map(|d| &d.msg).collect::<Vec<_>>()))
            .records;
        (String::new(), vms_obj::obj::write(&records))
    });
    let link = vlink::Options {
        base: BASE,
        name: "BCOPY".into(),
        transfer: None,
        link_time: 0,
        relocatable: false,
        shareable: None,
    };
    let image = vlink::link(&objects, &link).unwrap().image;

    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("bliss-files");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let text: Vec<u8> = (0..=255).cycle().take(1000).collect();
    std::fs::write(dir.join("IN.TXT"), &text).unwrap();
    let files = ["--files", dir.to_str().unwrap()];
    let out = vrun_with(&image, &files, &["IN.TXT", "OUT.TXT"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert_eq!(std::fs::read(dir.join("OUT.TXT")).unwrap(), text);

    // A missing file's status comes back as the exit status, cut to a byte.
    let out = vrun_with(&image, &files, &["MISSING", "OUT.TXT"]);
    assert_eq!(out.status.code(), Some(0x910 & 0xff), "{}", stderr(&out));
}
