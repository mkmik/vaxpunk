//! Builds shim.elf into OUT_DIR with the kernel's toolchain, for the RAM the
//! kernel was built for.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const CFLAGS: &str = "-O2 -g -Wall -Wextra -ffreestanding -mgeneral-regs-only -fno-pie \
    -fno-stack-protector -fno-asynchronous-unwind-tables -fno-tree-loop-distribute-patterns \
    -I. -Iinclude";
const LDFLAGS: &str = "-nostdlib -static -no-pie -T linker.ld -Wl,--build-id=none \
    -Wl,-z,max-page-size=4096";

fn main() {
    for path in ["src", "include", "limine.h", "linker.ld"] {
        println!("cargo::rerun-if-changed={path}");
    }
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());

    // seL4's RAM ranges as C initializers, from the platform the kernel was built for.
    let platform = fs::read(env::var("DEP_SEL4_PLATFORM").unwrap()).unwrap();
    let platform: serde_json::Value = serde_json::from_slice(&platform).unwrap();
    let ram: String = platform["memory"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            let (start, end) = (r["start"].as_u64().unwrap(), r["end"].as_u64().unwrap());
            format!("{{ {start:#x}, {end:#x} }},\n")
        })
        .collect();
    fs::write(out.join("sel4_ram.h"), ram).unwrap();

    let cc = format!("{}gcc", env::var("DEP_SEL4_CROSS_COMPILE").unwrap());
    gcc(Command::new(cc)
        .args(CFLAGS.split_whitespace())
        .arg(format!("-I{}", out.display()))
        .args(LDFLAGS.split_whitespace())
        .arg("-o")
        .arg(out.join("shim.elf"))
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
