//! Builds roottask.elf into OUT_DIR with the kernel's toolchain and libsel4,
//! and the boot volume, sys.vol: EXEC.EXE, linked from exec/*.mar, and an
//! image for each sysexe/*.mar, linked against SYS.STB, the executive's
//! symbols.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const CFLAGS: &str = "-O2 -g -Wall -Wextra -ffreestanding -fno-pie -fno-stack-protector \
    -fno-asynchronous-unwind-tables";
const LDFLAGS: &str = "-nostdlib -static -no-pie -T linker.ld -Wl,--build-id=none \
    -Wl,-z,max-page-size=4096";

fn main() {
    for path in ["src", "exec", "sysexe", "linker.ld", LIB] {
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
        .args(sources("src", &["c", "S"])));

    let mut modules: Vec<_> = sources("exec", &["mar"]).iter().map(compile).collect();
    modules.push(compile(Path::new(LIB).join("consolio.mar")));
    let exec = link("EXEC", vlink::DEFAULT_BASE, Some("EXEC$START"), &modules);
    fs::write(out.join("exec.map"), &exec.map).unwrap();
    let stb = symbol_table(&exec.map);
    let mut files = vec![("EXEC.EXE".to_string(), exec.image.write())];
    // Processes share the executive's address space, so each image gets
    // its own 1 MB from 16 MB up.
    for (i, source) in sources("sysexe", &["mar"]).iter().enumerate() {
        let name = source.file_stem().unwrap().to_str().unwrap().to_uppercase();
        let base = 0x0100_0000 + 0x10_0000 * i as u64;
        let image = link(&name, base, None, &[compile(source), stb.clone()]);
        files.push((format!("{name}.EXE"), image.image.write()));
    }
    fs::write(out.join("sys.vol"), volume(&files)).unwrap();
}

/// Where `.LIBRARY` finds lib.mlb and starlet.mlb.
const LIB: &str = "../vtools/lib";

/// Compiles a MACRO-32 source into an object module: (file name, bytes).
fn compile(source: impl AsRef<Path>) -> (String, Vec<u8>) {
    let source = source.as_ref();
    let name = source.file_stem().unwrap().to_str().unwrap().to_uppercase();
    let opts = vasm::Options {
        name,
        path: Some(source.into()),
        include: vec![LIB.into()],
        ..Default::default()
    };
    let text = fs::read_to_string(source).unwrap();
    let object = vmacro::compile(&text, &opts).unwrap_or_else(|diags| {
        let diags: Vec<_> = diags
            .iter()
            .map(|d| format!("{}:{}:{}: {}", d.file, d.line, d.col, d.msg))
            .collect();
        panic!("vmacro failed:\n{}", diags.join("\n"))
    });
    for d in &object.warnings {
        println!("cargo::warning={}:{}: {}", d.file, d.line, d.msg);
    }
    let file = source.display().to_string();
    (file, vms_obj::obj::write(&object.records))
}

/// Links object modules into an image at `base`.
fn link(
    name: &str,
    base: u64,
    transfer: Option<&str>,
    modules: &[(String, Vec<u8>)],
) -> vlink::Linked {
    let opts = vlink::Options {
        base,
        name: name.into(),
        transfer: transfer.map(Into::into),
        link_time: 0,
        relocatable: false,
    };
    let linked = vlink::link(modules, &opts)
        .unwrap_or_else(|msgs| panic!("vlink {name} failed:\n{}", msgs.join("\n")));
    for w in &linked.warnings {
        println!("cargo::warning={w}");
    }
    linked
}

/// SYS.STB: an object module defining each of the executive's global
/// symbols as a constant, from its link map, which kernel-mode images link
/// against as they did on VMS.
fn symbol_table(map: &str) -> (String, Vec<u8>) {
    let symbols = map
        .split("Symbols By Name")
        .nth(1)
        .and_then(|s| s.split("Symbols By Value").next())
        .expect("no symbols in the executive's map");
    let mut source = String::from("\t.TITLE\tSYS\tThe executive's global symbols\n");
    for line in symbols.lines().skip(3) {
        let mut words = line.split_whitespace();
        if let (Some(name), Some(value)) = (words.next(), words.next()) {
            source += &format!("{name} == ^X{value}\n");
        }
    }
    source += "\t.END\n";
    let opts = vasm::Options {
        name: "SYS".into(),
        ..Default::default()
    };
    let object = vmacro::compile(&source, &opts)
        .unwrap_or_else(|_| panic!("SYS.STB doesn't compile:\n{source}"));
    ("SYS.STB".into(), vms_obj::obj::write(&object.records))
}

/// The boot volume ($BVDDEF in vtools/lib/lib.mlb): a directory block of
/// 32-byte entries, a .ASCIC name, the first block and the size, then each
/// file from a 512-byte block.
fn volume(files: &[(String, Vec<u8>)]) -> Vec<u8> {
    const BLOCK: usize = 512;
    assert!(
        files.len() <= BLOCK / 32,
        "too many files for the boot volume"
    );
    let mut vol = vec![0; BLOCK];
    for (i, (name, data)) in files.iter().enumerate() {
        assert!(
            name.len() <= 23,
            "{name}: a boot volume name is at most 23 characters"
        );
        let lbn = (vol.len() / BLOCK) as u32;
        let e = &mut vol[32 * i..32 * (i + 1)];
        e[0] = name.len() as u8;
        e[1..=name.len()].copy_from_slice(name.as_bytes());
        e[24..28].copy_from_slice(&lbn.to_le_bytes());
        e[28..32].copy_from_slice(&(data.len() as u32).to_le_bytes());
        vol.extend(data);
        vol.resize(vol.len().next_multiple_of(BLOCK), 0);
    }
    vol
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
