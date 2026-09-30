//! Assembles, links and runs every program in tests/run and examples under
//! vrun, and checks it against its expected files, next to it in tests/run
//! and in tests/examples for examples: NAME.stdout (exact output,
//! default empty), NAME.status (exit code, default 0) and NAME.stderr (a
//! line stderr must contain). A directory NAME/ is a program of several
//! modules, linked in name order, then the object library vlib makes of the
//! modules in NAME/lib/, if there is one. VRUN_FLAGS adds vrun options.
//! Every object, library and image made on the way must parse and write back
//! to the same bytes.
//!
//! Each program links /RELOCATABLE, and runs the same at its link base and
//! moved far away. Linked again at that base, it must differ exactly where
//! its fixups say. Without /RELOCATABLE, it is the same image minus them.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use vms_obj::exe::Image;
use vms_obj::obj;
use vms_obj::olb::Library;

#[test]
fn programs() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    // (sources, expected files)
    let dirs = [("tests/run", "tests/run"), ("examples", "tests/examples")];
    let mut programs: Vec<(PathBuf, PathBuf, String)> = dirs
        .iter()
        .flat_map(|(src, exp)| {
            let (src, exp) = (root.join(src), root.join(exp));
            fs::read_dir(&src)
                .unwrap()
                .map(|e| e.unwrap().path())
                .filter(|p| p.is_dir() || p.extension().is_some_and(|e| e == "mar"))
                .map(move |p| {
                    let name = p.file_stem().unwrap().to_string_lossy().into_owned();
                    (src.clone(), exp.clone(), name)
                })
        })
        .collect();
    programs.sort();
    let failures: Vec<String> = programs
        .iter()
        .filter_map(|(src, exp, n)| run(src, exp, n).err().map(|e| format!("{n}: {e}")))
        .collect();
    assert!(
        failures.is_empty(),
        "{} of {} programs failed:\n{}",
        failures.len(),
        programs.len(),
        failures.join("\n")
    );
}

/// The same image anywhere in the lower half: here 64 TB up.
#[test]
fn high_base() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let text = fs::read_to_string(dir.join("hello.mar")).unwrap();
    let records = vasm::assemble(&text, &options("HELLO", &dir.join("hello.mar")))
        .unwrap()
        .records;
    let objects = [("hello.mar".to_string(), vms_obj::obj::write(&records))];
    let opts = vlink::Options {
        base: 0x4000_0000_0000,
        name: "HELLO".into(),
        transfer: None,
        link_time: 0,
        relocatable: false,
    };
    let exe = Path::new(env!("CARGO_TARGET_TMPDIR")).join("hello-high.exe");
    fs::write(&exe, vlink::link(&objects, &opts).unwrap().image.write()).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vrun"))
        .arg(&exe)
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "Hello, world!\n",
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(out.status.code(), Some(0));
}

/// Assembler options for a test program: macro libraries come from vtools/lib.
fn options(module: &str, source: &Path) -> vasm::Options {
    vasm::Options {
        name: module.into(),
        date: *b"25-SEP-2026 00:00",
        path: Some(source.to_path_buf()),
        include: vec![Path::new(env!("CARGO_MANIFEST_DIR")).join("../../lib")],
    }
}

fn run(dir: &Path, expected: &Path, name: &str) -> Result<(), String> {
    let program = dir.join(name);
    let mut objects = Vec::new();
    if program.is_dir() {
        for source in sources(&program) {
            objects.push(assemble(&source)?);
        }
        if program.join("lib").is_dir() {
            let mut lib = vlib::new(0);
            for source in sources(&program.join("lib")) {
                let (file, object) = assemble(&source)?;
                vlib::replace(&mut lib, &file, &object, 0)?;
            }
            let bytes = lib.write();
            assert_eq!(Library::parse(&bytes).unwrap().write(), bytes, "round trip");
            objects.push(("LIB.OLB".into(), bytes));
        }
    } else {
        objects.push(assemble(&dir.join(format!("{name}.mar")))?);
    }
    let link = |base, relocatable| {
        let opts = vlink::Options {
            base,
            name: name.to_uppercase(),
            transfer: None,
            link_time: 0,
            relocatable,
        };
        vlink::link(&objects, &opts).map_err(|e| e.join("\n"))
    };
    let linked = link(vlink::DEFAULT_BASE, true)?;
    let image = &linked.image;
    let plain = Image {
        fixups: None,
        ..image.clone()
    };
    if plain != link(vlink::DEFAULT_BASE, false)?.image {
        return Err("/RELOCATABLE changed more than the fixup section".into());
    }
    let far = far_base(image);
    vlink::check_fixups(image, &link(far, true)?.image)
        .map_err(|e| format!("linked at {far:#x}: {e}"))?;

    let exe = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}.exe"));
    let bytes = image.write();
    assert_eq!(Image::parse(&bytes).unwrap().write(), bytes, "round trip");
    fs::write(&exe, bytes).unwrap();
    let map = exe.with_extension("map");
    fs::write(&map, &linked.map).unwrap();

    let expect = |ext: &str| fs::read_to_string(expected.join(format!("{name}.{ext}"))).ok();
    let status: i32 = expect("status").map_or(0, |s| s.trim().parse().unwrap());
    let want = expect("stdout").unwrap_or_default();
    let fault = expect("stderr").unwrap_or_default();
    for base in [None, Some(far)] {
        let flags = std::env::var("VRUN_FLAGS").unwrap_or_default();
        let base_flags = base.map(|b| ["--base".to_string(), format!("{b:#x}")]);
        let out = Command::new(env!("CARGO_BIN_EXE_vrun"))
            .args(["--timeout", "20", "--map"])
            .arg(&map)
            .args(flags.split_whitespace())
            .args(base_flags.iter().flatten())
            .arg(&exe)
            .output()
            .unwrap();
        let (stdout, stderr) = (
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr),
        );
        let at = base.map_or(String::new(), |b| format!(" at {b:#x}"));
        if out.status.code() != Some(status) {
            return Err(format!(
                "exit {:?}{at}, expected {status}\nstdout: {stdout}\nstderr: {stderr}",
                out.status.code()
            ));
        }
        if stdout != want {
            return Err(format!(
                "stdout{at} {stdout:?}, expected {want:?}\nstderr: {stderr}"
            ));
        }
        // Addresses change when the image moves; where they are in it doesn't.
        let lines: Vec<&str> = match base {
            None => vec![fault.trim()],
            Some(_) => fault.lines().filter(|l| l.starts_with('-')).collect(),
        };
        if let Some(line) = lines.iter().find(|l| !stderr.contains(*l)) {
            return Err(format!("stderr{at} doesn't contain {line:?}:\n{stderr}"));
        }
    }
    Ok(())
}

/// Where else to run an image: far above 4 GB, unless it has longword
/// addresses, which must stay below 2 GB. Then as high as vrun's own range,
/// from 7FF00000, lets it go.
fn far_base(image: &Image) -> u64 {
    let f = image.fixups.as_ref().unwrap();
    if f.long.is_empty() {
        return 0x1234_5678_0000;
    }
    let start = image.sections[0].vaddr;
    let end = image.sections.iter().map(|s| s.vaddr + u64::from(s.size));
    let span = end.max().unwrap().next_multiple_of(0x1000) - start;
    let limit = start + (i32::MAX - f.long_max) as u64;
    (0x7ff0_0000 - span).min(limit) & !0xffff
}

/// The .mar files in `dir`, in name order.
fn sources(dir: &Path) -> Vec<PathBuf> {
    let mut s: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "mar"))
        .collect();
    s.sort();
    s
}

/// Assembles `source`; returns its file name and the object.
fn assemble(source: &Path) -> Result<(String, Vec<u8>), String> {
    let text = fs::read_to_string(source).unwrap();
    let module = source.file_stem().unwrap().to_string_lossy().to_uppercase();
    let records = vasm::assemble(&text, &options(&module, source))
        .map_err(|d| {
            let msgs: Vec<String> = d
                .iter()
                .map(|d| format!("{}:{}:{}: {}", d.file, d.line, d.col, d.msg))
                .collect();
            msgs.join("\n")
        })?
        .records;
    let bytes = obj::write(&records);
    assert_eq!(
        obj::write(&obj::parse(&bytes).unwrap()),
        bytes,
        "round trip"
    );
    Ok((source.display().to_string(), bytes))
}
