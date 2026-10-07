//! VASM, vasm in BLISS-64 (vtools/bliss/vasm, PRD-0004 step 13): builds
//! VASM.EXE with vbliss, then runs it under vrun on vasm's test programs
//! and examples, the encodings tests/encode.rs checks against GNU as, the
//! assembly vbliss makes of the BLISS-64 tests and of VASM itself, and the
//! cases in vtools/tests/vasm-port.txt. Each object module must be the
//! Rust vasm's byte for byte, and each message the same, or the same
//! failure.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use vms_obj::obj;

/// When vrun says it is: VMS's time 0, as it has no clock.
const EPOCH: [u8; 17] = *b"17-NOV-1858 00:00";

fn vtools() -> PathBuf {
    fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
}

/// Every file under `dir` with one of `exts`, in name order.
fn files(dir: &Path, exts: &[&str]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for e in fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        if p.is_dir() {
            out.extend(files(&p, exts));
        } else if p.extension().is_some_and(|x| exts.iter().any(|e| x == *e)) {
            out.push(p);
        }
    }
    out.sort();
    out
}

fn bliss_options(text: &str) -> vbliss::Options {
    let v = vtools();
    vbliss::Options {
        include: vec![
            v.join("lib"),
            v.join("tests/bliss"),
            v.join("bliss"),
            v.join("bliss/vasm"),
        ],
        ..vbliss::Options::from_source(text)
    }
}

/// Links VASM.EXE from vtools/bliss/vasm and the I/O module.
fn build(out: &Path) -> PathBuf {
    let dir = vtools().join("bliss/vasm");
    let mut objects = Vec::new();
    for source in files(&dir, &["b64"]) {
        let text = fs::read_to_string(&source).unwrap();
        let name = source.file_stem().unwrap().to_string_lossy().to_uppercase();
        let opts = vasm::Options {
            name,
            path: Some(source.clone()),
            ..Default::default()
        };
        let (object, output) = vbliss::compile_with(&text, &opts, &bliss_options(&text));
        let said: Vec<_> = output.diags.iter().chain(&output.lints).collect();
        assert!(said.is_empty(), "{}: {said:?}", source.display());
        let records = object
            .unwrap_or_else(|d| panic!("{}: {d:?}", source.display()))
            .records;
        objects.push((source.display().to_string(), obj::write(&records)));
    }
    let fio = vtools().join("bliss/fio.mar");
    let opts = vasm::Options {
        name: "FIO".into(),
        ..Default::default()
    };
    let records = vasm::assemble(&fs::read_to_string(&fio).unwrap(), &opts)
        .unwrap_or_else(|d| panic!("{d:?}"))
        .records;
    objects.push((fio.display().to_string(), obj::write(&records)));
    let link = vlink::Options {
        base: vlink::DEFAULT_BASE,
        name: "VASM".into(),
        transfer: None,
        link_time: 0,
        relocatable: false,
    };
    let image = vlink::link(&objects, &link)
        .unwrap_or_else(|e| panic!("{e:?}"))
        .image;
    let exe = out.join("vasm.exe");
    fs::write(&exe, image.write()).unwrap();
    exe
}

/// Writes the sources to assemble into `dir`, with the macro libraries in
/// dir/lib; returns their names.
fn corpus(dir: &Path) -> Vec<String> {
    let v = vtools();
    let repo = v.parent().unwrap().to_path_buf();
    fs::create_dir_all(dir.join("lib")).unwrap();
    for mlb in files(&v.join("lib"), &["mlb"]) {
        fs::copy(&mlb, dir.join("lib").join(mlb.file_name().unwrap())).unwrap();
    }
    fs::write(dir.join("lib/bad.mlb"), ".MACRO OK\n.ENDM\nnop\n").unwrap();
    let mut names = Vec::new();
    let mut add = |name: String, text: &str| {
        fs::write(dir.join(&name), text).unwrap();
        names.push(name);
    };
    // A file's name here: its path from the repository, flattened.
    let flat = |p: &Path| {
        p.strip_prefix(&repo)
            .unwrap()
            .to_string_lossy()
            .replace('/', "_")
    };
    let mut sources = files(&v.join("tests/run"), &["mar"]);
    sources.extend(files(&v.join("examples/vasm"), &["mar"]));
    sources.extend(files(&repo.join("roottask/sysexe"), &["m64"]));
    sources.push(v.join("crates/vasm/tests/expressions.mar"));
    sources.push(v.join("bliss/fio.mar"));
    for p in &sources {
        let name = flat(p).replace(".m64", ".mar");
        add(name, &fs::read_to_string(p).unwrap());
    }
    // Every form encode.rs checks against GNU as, and the NOTPIC warnings.
    let encode = fs::read_to_string(v.join("crates/vasm/tests/encode.rs")).unwrap();
    let text = encode.split("const SOURCE: &str = r#\"").nth(1).unwrap();
    add("encode.mar".into(), text.split("\"#;").next().unwrap());
    let warnings = fs::read_to_string(v.join("crates/vasm/tests/warnings.rs")).unwrap();
    let block = warnings
        .split("const SOURCE")
        .nth(1)
        .unwrap()
        .split("];")
        .next()
        .unwrap();
    let lines: Vec<&str> = block
        .lines()
        .filter_map(|l| l.trim().strip_prefix('"')?.strip_suffix("\","))
        .collect();
    add("notpic.mar".into(), &lines.join("\n"));
    // What vbliss makes of BLISS-64, VASM's own modules included.
    let mut bliss = files(&v.join("tests/bliss"), &["b64", "b32"]);
    bliss.extend(files(&v.join("lib"), &["b64"]));
    bliss.extend(files(&repo.join("roottask/exec"), &["b64"]));
    bliss.extend(files(&v.join("bliss/vasm"), &["b64"]));
    for p in &bliss {
        let text = fs::read_to_string(p).unwrap();
        let out = vbliss::translate(p, &text, &bliss_options(&text));
        let asm = out
            .asm
            .unwrap_or_else(|| panic!("{}: {:?}", p.display(), out.diags));
        add(format!("bliss_{}.mar", flat(&p.with_extension(""))), &asm);
    }
    let cases = fs::read_to_string(v.join("tests/vasm-port.txt")).unwrap();
    for (i, case) in cases.split("\n---\n").enumerate().skip(1) {
        add(format!("case{i:03}.mar"), &format!("{case}\n"));
    }
    add(
        "ascic.mar".into(),
        &format!(".ASCIC /{}/\n", "x".repeat(256)),
    );
    add(
        "ascid.mar".into(),
        &format!(".ASCID /{}/\n", "x".repeat(65536)),
    );
    add("tabs.mar".into(), "\t\tadd\tx0, x1, w9\n");
    add("warn.mar".into(), &format!(".WARN {}\n", "w".repeat(3000)));
    names
}

/// What the Rust vasm makes of `name`: the object module if it assembles,
/// and the messages its command line prints.
fn reference(dir: &Path, name: &str) -> (Option<Vec<u8>>, String) {
    let path = dir.join(name);
    let stem = name.strip_suffix(".mar").unwrap().to_uppercase();
    let opts = vasm::Options {
        name: stem.chars().take(31).collect(),
        date: EPOCH,
        path: Some(path.clone()),
        include: vec![dir.join("lib")],
    };
    let (object, diags) = match vasm::assemble(&fs::read_to_string(&path).unwrap(), &opts) {
        Ok(o) => (Some(obj::write(&o.records)), o.warnings),
        Err(d) => (None, d),
    };
    // As cli.rs prints them, with names relative to the directory, as
    // VASM sees them under vrun --files.
    let mut text = String::new();
    for d in diags {
        let level = if d.warning { "warning" } else { "error" };
        text += &format!("{}:{}:{}: {level}: {}\n", d.file, d.line, d.col, d.msg);
        let pad: String = d
            .text
            .chars()
            .take(d.col.saturating_sub(1))
            .map(|c| if c == '\t' { '\t' } else { ' ' })
            .collect();
        text += &format!("    {}\n    {pad}^\n", d.text);
        for c in &d.context {
            text += &format!("  {c}\n");
        }
    }
    let prefix = format!("{}/", dir.display());
    (object, text.replace(&prefix, ""))
}

/// What VASM.EXE makes of `name` under vrun.
fn vasm_exe(dir: &Path, exe: &Path, name: &str) -> (Option<Vec<u8>>, String) {
    let object = dir.join(name.replace(".mar", ".obj"));
    let flags = std::env::var("VRUN_FLAGS").unwrap_or_default();
    let out = Command::new(env!("CARGO_BIN_EXE_vrun"))
        .args(["--timeout", "600", "--files"])
        .arg(dir)
        .args(flags.split_whitespace())
        .arg(exe)
        .args(["-I", "lib", "-o"])
        .arg(object.file_name().unwrap())
        .arg(name)
        .output()
        .unwrap();
    let ok = out.status.code() == Some(0);
    assert!(
        ok || out.status.code() == Some(2),
        "{name}: {:?}\n{}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    (ok.then(|| fs::read(&object).unwrap()), text)
}

#[test]
fn vasm_port() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("vasm-port");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let exe = build(&dir);
    let names = corpus(&dir);
    let next = AtomicUsize::new(0);
    let failures = Mutex::new(Vec::new());
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get().min(8));
    std::thread::scope(|s| {
        for _ in 0..workers {
            s.spawn(|| {
                while let Some(name) = names.get(next.fetch_add(1, Ordering::Relaxed)) {
                    let want = reference(&dir, name);
                    let got = vasm_exe(&dir, &exe, name);
                    if got.0 != want.0 {
                        let what = match (&got.0, &want.0) {
                            (Some(_), Some(_)) => "the object modules differ",
                            (Some(_), None) => "VASM assembles what vasm doesn't",
                            _ => "vasm assembles what VASM doesn't",
                        };
                        failures
                            .lock()
                            .unwrap()
                            .push(format!("{name}: {what}\n{}", got.1));
                    } else if got.1 != want.1 {
                        let msg = format!("{name}: messages\n{}--- vasm says\n{}", got.1, want.1);
                        failures.lock().unwrap().push(msg);
                    }
                }
            });
        }
    });
    let failures = failures.into_inner().unwrap();
    assert!(
        failures.is_empty(),
        "{} of {} differ:\n{}",
        failures.len(),
        names.len(),
        failures.join("\n")
    );
}
