//! Builds tcpip.elf into OUT_DIR with the kernel's toolchain and libsel4's
//! headers: src/, the component's own code, and lwIP's core, IPv4 and
//! Ethernet, from the lwip submodule, configured by include/lwipopts.h.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const CFLAGS: &str = "-O2 -g -Wall -ffreestanding -fno-pie -fno-stack-protector \
    -fno-asynchronous-unwind-tables -mgeneral-regs-only -Iinclude -Ilwip/src/include";
const LDFLAGS: &str = "-nostdlib -static -no-pie -T linker.ld -Wl,--build-id=none \
    -Wl,-z,max-page-size=4096";
const LWIP: &[&str] = &["lwip/src/core", "lwip/src/core/ipv4"];

fn main() {
    for path in ["src", "include", "linker.ld", "lwip/src"] {
        println!("cargo::rerun-if-changed={path}");
    }
    // Fresh clones and worktrees leave submodules empty.
    if !Path::new("lwip/src/core").exists() {
        let _ = Command::new("git")
            .args(["submodule", "update", "--init", "lwip"])
            .status();
    }
    assert!(
        Path::new("lwip/src/core").exists(),
        "tcpip/lwip is missing, run: git submodule update --init"
    );
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let cc = format!("{}gcc", env::var("DEP_SEL4_CROSS_COMPILE").unwrap());
    let mut srcs = sources("src", &["c", "S"]);
    for dir in LWIP {
        srcs.extend(sources(dir, &["c"]));
    }
    srcs.push("lwip/src/netif/ethernet.c".into());
    let elf = out.join("tcpip.elf");
    let mut cmd = Command::new(cc);
    cmd.args(CFLAGS.split_whitespace())
        .arg(format!("-I{}", env::var("DEP_SEL4_INCLUDE").unwrap()))
        .args(LDFLAGS.split_whitespace())
        .arg("-o")
        .arg(&elf)
        .args(&srcs)
        .arg("-lgcc");
    let res = cmd.output().unwrap_or_else(|e| panic!("{cmd:?}: {e}"));
    let diagnostics = String::from_utf8_lossy(&res.stderr);
    assert!(res.status.success(), "{cmd:?} failed:\n{diagnostics}");
    for line in diagnostics.lines() {
        println!("cargo::warning={line}");
    }
    println!("cargo::metadata=elf={}", elf.display());
}

/// The files in `dir` with one of `exts`, sorted.
fn sources(dir: &str, exts: &[&str]) -> Vec<PathBuf> {
    let mut srcs: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension()
                .is_some_and(|ext| exts.iter().any(|e| ext == *e))
        })
        .collect();
    srcs.sort();
    srcs
}
