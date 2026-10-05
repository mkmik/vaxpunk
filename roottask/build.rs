//! Builds roottask.elf into OUT_DIR with the kernel's toolchain and libsel4,
//! with the TCP/IP component's tcpip.elf in it,
//! and the system disk, sysdisk.img, a Files-11 ODS-2 volume: in [SYSEXE],
//! EXEC.EXE, linked from exec/*.mar, and an image for each sysexe/*.mar,
//! linked with sysexe/lib/*.mar, its command table if there is a
//! sysexe/NAME.cld, and against SYS.STB, the executive's symbols; DCL and
//! HELP with DCL$TABLES, from cld/*.cld, and DCL with sysexe/dcl/*.mar,
//! its CDU; in [SYSMGR], the files in sysmgr/,
//! as text.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const CFLAGS: &str = "-O2 -g -Wall -Wextra -ffreestanding -fno-pie -fno-stack-protector \
    -fno-asynchronous-unwind-tables";
const LDFLAGS: &str = "-nostdlib -static -no-pie -T linker.ld -Wl,--build-id=none \
    -Wl,-z,max-page-size=4096";

fn main() {
    for path in ["src", "exec", "sysexe", "sysmgr", "cld", "linker.ld", LIB] {
        println!("cargo::rerun-if-changed={path}");
    }
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let cc = format!("{}gcc", env::var("DEP_SEL4_CROSS_COMPILE").unwrap());
    // The TCP/IP component, which the PAL embeds (src/tcpip.S) and starts.
    let tcpip = env::var("DEP_TCPIP_ELF").unwrap();
    println!("cargo::rerun-if-changed={tcpip}");
    gcc(Command::new(cc)
        .args(CFLAGS.split_whitespace())
        .arg(format!("-I{}", env::var("DEP_SEL4_INCLUDE").unwrap()))
        .arg("-I../tcpip/include")
        .arg(format!("-DTCPIP_ELF=\"{tcpip}\""))
        .args(LDFLAGS.split_whitespace())
        .arg("-o")
        .arg(out.join("roottask.elf"))
        .args(sources("src", &["c", "S"])));

    let mut modules: Vec<_> = sources("exec", &["mar"]).iter().map(compile).collect();
    modules.push(compile(Path::new(LIB).join("consolio.mar")));
    // The executive goes in S0, the system space every process shares.
    let exec = link("EXEC", 0x4001_0000, Some("EXEC$START"), &modules);
    fs::write(out.join("exec.map"), &exec.map).unwrap();
    let stb = symbol_table(&exec.map);
    let libs: Vec<_> = sources("sysexe/lib", &["mar"])
        .iter()
        .map(compile)
        .collect();
    let mut files = vec![("EXEC.EXE".to_string(), exec.image.write())];
    // Each process has its own P0, so every image goes at the same address,
    // but DCL: linked in P1, at VA$C_CLI, it is a command interpreter, which
    // stays while the images it runs come and go in P0.
    for source in sources("sysexe", &["mar"]) {
        let name = source.file_stem().unwrap().to_str().unwrap().to_uppercase();
        let mut modules = vec![compile(&source)];
        let cld = source.with_extension("cld");
        if cld.exists() {
            modules.push(tables(&name, &[cld]));
        }
        if name == "DCL" || name == "HELP" {
            modules.push(tables("DCL$TABLES", &sources("cld", &["cld"])));
        }
        if name == "DCL" {
            modules.extend(sources("sysexe/dcl", &["mar"]).iter().map(compile));
        }
        modules.extend(libs.iter().cloned());
        modules.push(stb.clone());
        let base = if name == "DCL" {
            0x7FF0_0000
        } else {
            vlink::DEFAULT_BASE
        };
        let image = link(&name, base, None, &modules);
        files.push((format!("{name}.EXE"), image.image.write()));
    }
    disk(&out.join("sysdisk.img"), &files);
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

/// Compiles `.CLD` files with vcdu into one command table, an object
/// module whose global symbol is `name`, unless they say MODULE.
fn tables(name: &str, clds: &[PathBuf]) -> (String, Vec<u8>) {
    let texts: Vec<_> = clds
        .iter()
        .map(|p| (p.display().to_string(), fs::read_to_string(p).unwrap()))
        .collect();
    let source = vcdu::compile(name, &texts)
        .unwrap_or_else(|msgs| panic!("vcdu failed:\n{}", msgs.join("\n")));
    let opts = vasm::Options {
        name: name.into(),
        ..Default::default()
    };
    let object = vmacro::compile(&source, &opts)
        .unwrap_or_else(|_| panic!("{name}'s tables don't compile:\n{source}"));
    (format!("{name}.CLD"), vms_obj::obj::write(&object.records))
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

/// The system disk: an ODS-2 volume labelled VAXPUNK with the images in
/// [SYSEXE], fixed 512-byte records as VMS's are, and sysmgr/'s files in
/// [SYSMGR], their lines variable-length records, with names in capitals.
/// [1,4] owns them, and all may read and run them: (S:RWED,O:RWED,G:RE,W:RE).
fn disk(path: &Path, images: &[(String, Vec<u8>)]) {
    use ods_image::{Conversion, Image, InitParams, RecordAttrs, rfm};
    let _ = fs::remove_file(path);
    let params = InitParams {
        label: b"VAXPUNK".to_vec(),
        file_protection: 0xaa00,
        ..Default::default()
    };
    fn ok<T>(r: ods_image::Result<T>) -> T {
        r.unwrap_or_else(|e| panic!("the system disk: {e}"))
    }
    let mut vol = ok(Image::create(path, 4096, &params));
    ok(vol.mkdir("[SYSEXE]"));
    ok(vol.mkdir("[SYSMGR]"));
    let image = RecordAttrs {
        rtype: rfm::FIX,
        rsize: 512,
        maxrec: 512,
        ..Default::default()
    };
    for (name, data) in images {
        let spec = format!("[SYSEXE]{name}");
        let size = Some(data.len() as u64);
        ok(vol.copy_in(&mut &data[..], &spec, Conversion::Binary, size, Some(image)));
    }
    for source in sources("sysmgr", &["txt", "com", "cld"]) {
        let name = source.file_name().unwrap().to_str().unwrap().to_uppercase();
        let text = fs::read(&source).unwrap();
        let spec = format!("[SYSMGR]{name}");
        let (size, lines) = (Some(text.len() as u64), Conversion::LinesToRecords);
        ok(vol.copy_in(&mut &text[..], &spec, lines, size, None));
    }
    ok(vol.flush());
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
