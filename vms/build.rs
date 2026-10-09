//! Builds the system disk, sysdisk.img, a Files-11 ODS-2 volume: in [SYSEXE],
//! EXEC.EXE, linked from exec/*.mar and, after them, exec/*.b64, which
//! vbliss compiles, and an image for each sysexe/*.mar or *.b64, the
//! two linked together when both are there, and with sysexe/lib/*.mar and
//! *.b64, its command table if there is a
//! sysexe/NAME.cld, its ARM64 if there is a sysexe/NAME.m64, which vasm
//! assembles, and against LIBRTL.EXE and SYS.STB, the executive's symbols;
//! in [SYSLIB], LIBRTL.EXE, the shareable image of sysexe/librtl/*.mar,
//! with the symbol vector librtl.opt gives, which DCL links in instead; DCL and
//! HELP with DCL$TABLES, from cld/*.cld, and DCL with sysexe/dcl/*.mar,
//! its CDU, and HELP and TCPIP with sysexe/help/*.mar, which describes
//! command tables, and CREATE, CONVERT and ANALYZRMS with sysexe/rms/*.mar,
//! FDL and their output; in [SYSMGR], the files in sysmgr/,
//! as text.

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    for path in ["exec", "sysexe", "sysmgr", "cld", LIB] {
        println!("cargo::rerun-if-changed={path}");
    }
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());

    let mut exec = sources("exec", &["mar"]);
    exec.push(Path::new(LIB).join("consolio.mar"));
    let mut modules = compile(&exec);
    // After the MACRO-32 modules, whose psects come first: the vector page
    // starts where syssrv.mar says.
    modules.extend(sources("exec", &["b64"]).iter().map(|s| bliss(s)));
    // The executive goes in S0, the system space every process shares.
    let exec = link("EXEC", 0x4001_0000, Some("EXEC$START"), &modules);
    fs::write(out.join("exec.map"), &exec.map).unwrap();
    let stb = symbol_table(&exec.map);
    // The programs and the modules they share, compiled together, as the
    // executive's modules are, so that their JSBs to each other know what
    // the routines they call keep.
    let sysexe: Vec<_> = [
        "sysexe",
        "sysexe/lib",
        "sysexe/librtl",
        "sysexe/dcl",
        "sysexe/help",
        "sysexe/rms",
    ]
    .iter()
    .flat_map(|d| sources(d, &["mar"]))
    .collect();
    let compiled: HashMap<_, _> = sysexe.iter().cloned().zip(compile(&sysexe)).collect();
    let module = |p: &PathBuf| compiled[p].clone();
    let mut libs: Vec<_> = sources("sysexe/lib", &["mar"]).iter().map(module).collect();
    libs.extend(sources("sysexe/lib", &["b64"]).iter().map(|s| bliss(s)));
    let mut files = vec![("[SYSEXE]EXEC.EXE".to_string(), exec.image.write())];
    // LIBRTL.EXE, which moves: linked at 0, as on Alpha.
    let librtl: Vec<_> = sources("sysexe/librtl", &["mar"])
        .iter()
        .map(module)
        .collect();
    let opt = fs::read_to_string("sysexe/librtl/librtl.opt").unwrap();
    let vector = vlink::options(&opt).unwrap_or_else(|e| panic!("librtl.opt: {e:?}"));
    let modules: Vec<_> = librtl.iter().cloned().chain([stb.clone()]).collect();
    let shr = link_shareable("LIBRTL", vector, &modules);
    files.push(("[SYSLIB]LIBRTL.EXE".into(), shr.image.write()));
    let librtl_exe = ("LIBRTL.EXE".to_string(), shr.image.write());
    // Each process has its own P0, so every image goes at the same address,
    // but DCL: linked in P1, at VA$C_CLI, it is a command interpreter, which
    // stays while the images it runs come and go in P0.
    let mut programs = sources("sysexe", &["mar", "b64"]);
    programs.dedup_by(|a, b| a.file_stem() == b.file_stem());
    for source in programs {
        let name = source.file_stem().unwrap().to_str().unwrap().to_uppercase();
        let mut modules = Vec::new();
        if source.with_extension("mar").exists() {
            modules.push(module(&source.with_extension("mar")));
        }
        // A program's BLISS-64, alone or with its MACRO-32.
        let b64 = source.with_extension("b64");
        if b64.exists() {
            modules.push(bliss(&b64));
        }
        let cld = source.with_extension("cld");
        if cld.exists() {
            modules.push(tables(&name, &[cld]));
        }
        // Hand-written ARM64 the program calls, as BLISS-64 would build it.
        let m64 = source.with_extension("m64");
        if m64.exists() {
            modules.push(assemble(&m64));
        }
        if name == "DCL" || name == "HELP" {
            modules.push(tables("DCL$TABLES", &sources("cld", &["cld"])));
        }
        if name == "DCL" {
            modules.extend(sources("sysexe/dcl", &["mar"]).iter().map(module));
        }
        if name == "HELP" || name == "TCPIP" {
            modules.extend(sources("sysexe/help", &["mar"]).iter().map(module));
        }
        if ["CREATE", "CONVERT", "ANALYZRMS"].contains(&name.as_str()) {
            modules.extend(sources("sysexe/rms", &["mar"]).iter().map(module));
        }
        modules.extend(libs.iter().cloned());
        // DCL, in P1, can't call a shareable image, which goes in P0.
        if name == "DCL" {
            modules.extend(librtl.iter().cloned());
        } else {
            modules.push(librtl_exe.clone());
        }
        modules.push(stb.clone());
        let base = if name == "DCL" {
            0x7FF0_0000
        } else {
            vlink::DEFAULT_BASE
        };
        let image = link(&name, base, None, &modules);
        files.push((format!("[SYSEXE]{name}.EXE"), image.image.write()));
    }
    disk(&out.join("sysdisk.img"), &files);
}

/// Where `.LIBRARY` finds lib.mlb and starlet.mlb.
const LIB: &str = "../crosstools/vtools/lib";

/// Assembles an ARM64 source with vasm into an object module: (file name,
/// bytes).
fn assemble(source: &Path) -> (String, Vec<u8>) {
    let opts = vasm::Options {
        name: source.file_stem().unwrap().to_str().unwrap().to_uppercase() + "_ARM",
        path: Some(source.into()),
        include: vec![LIB.into()],
        ..Default::default()
    };
    let text = fs::read_to_string(source).unwrap();
    let object = vasm::assemble(&text, &opts).unwrap_or_else(|diags| {
        let diags: Vec<_> = diags
            .iter()
            .map(|d| format!("{}:{}:{}: {}", d.file, d.line, d.col, d.msg))
            .collect();
        panic!("vasm failed:\n{}", diags.join("\n"))
    });
    (
        source.display().to_string(),
        vms_obj::obj::write(&object.records),
    )
}

/// Compiles a BLISS-64 source with vbliss into an object module: (file
/// name, bytes). Its warnings, the dot lint's included, are errors.
fn bliss(source: &Path) -> (String, Vec<u8>) {
    let text = fs::read_to_string(source).unwrap();
    let opts = vasm::Options {
        name: source.file_stem().unwrap().to_str().unwrap().to_uppercase(),
        path: Some(source.into()),
        include: vec![LIB.into()],
        ..Default::default()
    };
    let bliss = vbliss::Options {
        include: vec![LIB.into()],
        ..Default::default()
    };
    let (object, out) = vbliss::compile_with(&text, &opts, &bliss);
    let complaints: Vec<_> = out
        .diags
        .iter()
        .chain(&out.lints)
        .map(|d| format!("{}:{}: {}", d.file, d.line, d.msg))
        .collect();
    if !complaints.is_empty() {
        panic!("vbliss {}:\n{}", source.display(), complaints.join("\n"));
    }
    let object = object.unwrap_or_else(|diags| {
        let diags: Vec<_> = diags
            .iter()
            .map(|d| format!("{}:{}: {}", d.file, d.line, d.msg))
            .collect();
        panic!(
            "vbliss {} didn't assemble:\n{}",
            source.display(),
            diags.join("\n")
        )
    });
    (
        source.display().to_string(),
        vms_obj::obj::write(&object.records),
    )
}

/// Compiles MACRO-32 sources linked together into object modules: (file
/// name, bytes) each.
fn compile(sources: &[PathBuf]) -> Vec<(String, Vec<u8>)> {
    let texts: Vec<_> = sources
        .iter()
        .map(|s| fs::read_to_string(s).unwrap())
        .collect();
    let opts: Vec<_> = sources
        .iter()
        .map(|s| vasm::Options {
            name: s.file_stem().unwrap().to_str().unwrap().to_uppercase(),
            path: Some(s.into()),
            include: vec![LIB.into()],
            ..Default::default()
        })
        .collect();
    let modules: Vec<_> = texts.iter().map(String::as_str).zip(&opts).collect();
    let objects = vmacro::compile_modules(&modules);
    let mut failed = Vec::new();
    let mut out = Vec::new();
    for (source, object) in sources.iter().zip(objects) {
        match object {
            Ok(object) => {
                // A warning fails the build, as an error does: CI must not pass it.
                failed.extend(
                    object
                        .warnings
                        .iter()
                        .map(|d| format!("{}:{}: warning: {}", d.file, d.line, d.msg)),
                );
                let file = source.display().to_string();
                out.push((file, vms_obj::obj::write(&object.records)));
            }
            Err(diags) => failed.extend(
                diags
                    .iter()
                    .map(|d| format!("{}:{}:{}: {}", d.file, d.line, d.col, d.msg)),
            ),
        }
    }
    if !failed.is_empty() {
        panic!("vmacro failed:\n{}", failed.join("\n"));
    }
    out
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
        ..Default::default()
    };
    checked(name, vlink::link(modules, &opts))
}

/// Links object modules into a shareable image, at 0, with the symbol
/// vector `vector`.
fn link_shareable(name: &str, vector: Vec<String>, modules: &[(String, Vec<u8>)]) -> vlink::Linked {
    let opts = vlink::Options {
        name: name.into(),
        shareable: Some(vector),
        ..Default::default()
    };
    checked(name, vlink::link(modules, &opts))
}

/// The image, or the link's errors and warnings, which fail the build
/// alike; its information doesn't.
fn checked(name: &str, linked: Result<vlink::Linked, Vec<String>>) -> vlink::Linked {
    let linked = linked.unwrap_or_else(|msgs| panic!("vlink {name} failed:\n{}", msgs.join("\n")));
    let warnings: Vec<_> = linked
        .warnings
        .iter()
        .filter(|w| !w.contains("-I-"))
        .collect();
    if !warnings.is_empty() {
        panic!(
            "vlink {name} warned:\n{}",
            warnings
                .iter()
                .map(|w| w.as_str())
                .collect::<Vec<_>>()
                .join("\n")
        );
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
/// [SYSEXE] and [SYSLIB], as `images` names them, fixed 512-byte records as
/// VMS's are, and sysmgr/'s files in
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
    let mut vol = ok(Image::create(path, 8192, &params));
    ok(vol.mkdir("[SYSEXE]"));
    ok(vol.mkdir("[SYSLIB]"));
    ok(vol.mkdir("[SYSMGR]"));
    let image = RecordAttrs {
        rtype: rfm::FIX,
        rsize: 512,
        maxrec: 512,
        ..Default::default()
    };
    for (spec, data) in images {
        let size = Some(data.len() as u64);
        ok(vol.copy_in(&mut &data[..], spec, Conversion::Binary, size, Some(image)));
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
