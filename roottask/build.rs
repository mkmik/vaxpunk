//! Builds roottask.elf into OUT_DIR with the kernel's toolchain and libsel4.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const CFLAGS: &str = "-O2 -g -Wall -Wextra -ffreestanding -fno-pie -fno-stack-protector \
    -fno-asynchronous-unwind-tables";
const LDFLAGS: &str = "-nostdlib -static -no-pie -T linker.ld -Wl,--build-id=none \
    -Wl,-z,max-page-size=4096";

fn main() {
    for path in ["src", "linker.ld"] {
        println!("cargo::rerun-if-changed={path}");
    }
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let cc = format!("{}gcc", env::var("DEP_SEL4_CROSS_COMPILE").unwrap());
    gcc(Command::new(cc)
        .args(CFLAGS.split_whitespace())
        .arg(format!("-I{}", env::var("DEP_SEL4_INCLUDE").unwrap()))
        .args(LDFLAGS.split_whitespace())
        .arg("-o")
        .arg(out.join("roottask.elf"))
        .args(sources()));
}

/// src/*.c and src/*.S.
fn sources() -> Vec<PathBuf> {
    let mut srcs: Vec<_> = fs::read_dir("src")
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "c" || ext == "S"))
        .collect();
    srcs.sort();
    srcs
}

/// Runs gcc and hands its diagnostics to cargo as warnings: cargo shows a
/// build script's own output only when it fails.
fn gcc(cmd: &mut Command) {
    let out = cmd.output().unwrap_or_else(|e| panic!("{cmd:?}: {e}"));
    let diagnostics = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{cmd:?} failed:\n{diagnostics}");
    for line in diagnostics.lines() {
        println!("cargo::warning={line}");
    }
}
