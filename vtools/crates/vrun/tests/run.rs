//! Runs hand-encoded images under vrun in QEMU (needs qemu-system-aarch64).
//! VRUN_FLAGS adds vrun options, for example `VRUN_FLAGS=--hvf cargo test`.

use std::process::{Command, Output};

use vms_obj::exe::{Eisd, Image, Section};

const BASE: u64 = 0x10000;

/// An image with one code section at 0x10000, entered at its start.
fn image(code: &[u32]) -> Image {
    Image {
        name: "TEST".into(),
        ident: "V1".into(),
        link_time: 0,
        transfer: BASE,
        sections: vec![code_section(code)],
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
fn hung_image_times_out() {
    // b .
    let out = vrun_with(&image(&[0x14000000]), &["--timeout", "1"], &[]);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("%VRUN-F-TIMEOUT"), "{}", stderr(&out));
}
