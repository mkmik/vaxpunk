//! Assembles the boot stub into a raw binary for `include_bytes!`, with the
//! repo's cross binutils (the same `CROSS_COMPILE` convention as the shim).

use std::env;
use std::path::PathBuf;
use std::process::Command;

include!("src/layout.rs");

fn main() {
    println!("cargo::rerun-if-changed=stub/stub.S");
    println!("cargo::rerun-if-changed=src/layout.rs");
    println!("cargo::rerun-if-env-changed=CROSS_COMPILE");

    let cross = env::var("CROSS_COMPILE").unwrap_or_else(|_| {
        let elf = Command::new("aarch64-elf-as").arg("--version").output();
        if elf.is_ok() {
            "aarch64-elf-"
        } else {
            "aarch64-linux-gnu-"
        }
        .into()
    });
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let (obj, elf, bin) = (
        out.join("stub.o"),
        out.join("stub.elf"),
        out.join("stub.bin"),
    );

    let mut ld = Command::new(format!("{cross}ld"));
    ld.arg(format!("-Ttext={LOAD_BASE:#x}"))
        .arg("-o")
        .arg(&elf)
        .arg(&obj);
    let mut objcopy = Command::new(format!("{cross}objcopy"));
    objcopy.args(["-O", "binary"]).arg(&elf).arg(&bin);
    let mut asm = Command::new(format!("{cross}as"));
    for (name, value) in [("UART", UART), ("BOOT", BOOT), ("EL1_STACK", EL1_STACK)] {
        asm.arg(format!("--defsym={name}={value:#x}"));
    }
    asm.arg("-o").arg(&obj).arg("stub/stub.S");

    for cmd in [&mut asm, &mut ld, &mut objcopy] {
        let status = cmd.status().unwrap_or_else(|e| panic!("{cmd:?}: {e}"));
        assert!(status.success(), "{cmd:?} failed");
    }
    let size = std::fs::metadata(&bin).unwrap().len();
    assert!(
        size <= STUB_MAX,
        "boot stub is {size} bytes, more than STUB_MAX"
    );
    // RAM_BASE is only used by vrun itself.
    let _ = RAM_BASE;
}
