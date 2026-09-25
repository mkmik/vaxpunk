//! Assembles, links and runs every program in tests/run under vrun, and
//! checks it against the files next to it: NAME.stdout (exact output,
//! default empty), NAME.status (exit code, default 0) and NAME.stderr (a
//! line stderr must contain). A directory NAME/ is a program of several
//! modules, linked in name order. VRUN_FLAGS adds vrun options.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
fn programs() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/run");
    let mut names: Vec<String> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.is_dir() || p.extension().is_some_and(|e| e == "mar"))
        .map(|p| p.file_stem().unwrap().to_string_lossy().into_owned())
        .collect();
    names.sort();
    let failures: Vec<String> = names
        .iter()
        .filter_map(|n| run(&dir, n).err().map(|e| format!("{n}: {e}")))
        .collect();
    assert!(
        failures.is_empty(),
        "{} of {} programs failed:\n{}",
        failures.len(),
        names.len(),
        failures.join("\n")
    );
}

/// The same image anywhere in the lower half: here 64 TB up.
#[test]
fn high_base() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/run");
    let text = fs::read_to_string(dir.join("hello.mar")).unwrap();
    let records = vasm::assemble(&text, "HELLO", *b"25-SEP-2026 00:00").unwrap();
    let objects = [("hello.mar".to_string(), vms_obj::obj::write(&records))];
    let opts = vlink::Options {
        base: 0x4000_0000_0000,
        name: "HELLO".into(),
        transfer: None,
        link_time: 0,
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

fn run(dir: &Path, name: &str) -> Result<(), String> {
    let sources: Vec<PathBuf> = if dir.join(name).is_dir() {
        let mut s: Vec<PathBuf> = fs::read_dir(dir.join(name))
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        s.sort();
        s
    } else {
        vec![dir.join(format!("{name}.mar"))]
    };
    let mut objects = Vec::new();
    for source in &sources {
        let text = fs::read_to_string(source).unwrap();
        let module = source.file_stem().unwrap().to_string_lossy().to_uppercase();
        let records = vasm::assemble(&text, &module, *b"25-SEP-2026 00:00")
            .map_err(|d| format!("{}: {d:?}", source.display()))?;
        objects.push((source.display().to_string(), vms_obj::obj::write(&records)));
    }
    let opts = vlink::Options {
        base: vlink::DEFAULT_BASE,
        name: name.to_uppercase(),
        transfer: None,
        link_time: 0,
    };
    let linked = vlink::link(&objects, &opts).map_err(|e| e.join("\n"))?;
    let exe = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}.exe"));
    fs::write(&exe, linked.image.write()).unwrap();

    let flags = std::env::var("VRUN_FLAGS").unwrap_or_default();
    let out = Command::new(env!("CARGO_BIN_EXE_vrun"))
        .args(["--timeout", "20"])
        .args(flags.split_whitespace())
        .arg(&exe)
        .output()
        .unwrap();
    let (stdout, stderr) = (
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let expect = |ext: &str| fs::read_to_string(dir.join(format!("{name}.{ext}"))).ok();
    let status: i32 = expect("status").map_or(0, |s| s.trim().parse().unwrap());
    if out.status.code() != Some(status) {
        return Err(format!(
            "exit {:?}, expected {status}\nstdout: {stdout}\nstderr: {stderr}",
            out.status.code()
        ));
    }
    let want = expect("stdout").unwrap_or_default();
    if stdout != want {
        return Err(format!(
            "stdout {stdout:?}, expected {want:?}\nstderr: {stderr}"
        ));
    }
    if let Some(line) = expect("stderr")
        && !stderr.contains(line.trim())
    {
        return Err(format!(
            "stderr doesn't contain {:?}:\n{stderr}",
            line.trim()
        ));
    }
    Ok(())
}
