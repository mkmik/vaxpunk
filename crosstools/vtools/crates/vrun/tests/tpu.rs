//! TPU (PRD-0006) under vrun: builds TPU.EXE from vms/sysexe/tpu's
//! BLISS-64 and the I/O module, crosstools/vtools/bliss/fio.mar, then runs
//! each command file in vms/sysexe/tpu/tests as
//! `EDIT/TPU/NODISPLAY/NOSECTION/COMMAND=NAME.TPU`, with the other files
//! there beside it, and compares what it prints and the NAME.OUT it writes
//! with what DEC's TPU did on OpenVMS (NAME.stdout and NAME.out, from
//! crosstools/ods/vms/tpu-oracle.py). The oracle's file names, in its
//! directory, are cut to the name as given.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use vms_obj::obj;

fn repo() -> PathBuf {
    fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../..")).unwrap()
}

/// Links TPU.EXE into `out`.
fn build(out: &Path) -> PathBuf {
    let repo = repo();
    let tpu = repo.join("vms/sysexe/tpu");
    let bliss = repo.join("crosstools/vtools/bliss");
    let mut objects = Vec::new();
    let mut sources: Vec<_> = fs::read_dir(&tpu)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "b64"))
        .collect();
    sources.sort();
    let mut errors = Vec::new();
    for source in sources {
        let text = fs::read_to_string(&source).unwrap();
        let name = source.file_stem().unwrap().to_string_lossy().to_uppercase();
        let opts = vasm::Options {
            name,
            path: Some(source.clone()),
            ..Default::default()
        };
        let bopts = vbliss::Options {
            include: vec![
                tpu.clone(),
                bliss.clone(),
                repo.join("crosstools/vtools/lib"),
            ],
            ..vbliss::Options::from_source(&text)
        };
        let (object, output) = vbliss::compile_with(&text, &opts, &bopts);
        let file = source.file_name().unwrap().to_string_lossy().into_owned();
        for d in output.diags.iter().chain(&output.lints) {
            errors.push(format!("{file}:{}:{}: {}", d.line, d.col, d.msg));
        }
        match object {
            Ok(o) => objects.push((source.display().to_string(), obj::write(&o.records))),
            Err(d) => errors.push(format!("{file}: {d:?}")),
        }
    }
    assert!(errors.is_empty(), "{}", errors.join("\n"));
    let fio = bliss.join("fio.mar");
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
        name: "TPU".into(),
        transfer: None,
        link_time: 0,
        relocatable: false,
        shareable: None,
    };
    let image = vlink::link(&objects, &link)
        .unwrap_or_else(|e| panic!("{e:#?}"))
        .image;
    let exe = out.join("tpu.exe");
    fs::write(&exe, image.write()).unwrap();
    exe
}

/// What the oracle printed, its directory taken out of file names.
fn expected(text: &str) -> String {
    text.replace("SYS$SYSDEVICE:[TPUORACLE]", "")
        .replace(";1", "")
}

/// Runs one test; Err says how it differs.
fn run(exe: &Path, tests: &Path, name: &str) -> Result<(), String> {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("tpu-{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    // The other files, in capitals as VMS names them.
    for e in fs::read_dir(tests).unwrap().flatten() {
        let p = e.path();
        let ext = p.extension().map(|x| x.to_string_lossy().into_owned());
        if matches!(ext.as_deref(), Some("stdout" | "out")) {
            continue;
        }
        let up = p.file_name().unwrap().to_string_lossy().to_uppercase();
        fs::copy(&p, dir.join(up)).unwrap();
    }
    let up = name.to_uppercase();
    let out = Command::new(env!("CARGO_BIN_EXE_vrun"))
        .args(["--timeout", "60", "--files"])
        .arg(&dir)
        .arg(exe)
        .arg(format!("/NODISPLAY/NOSECTION/COMMAND={up}.TPU"))
        .output()
        .unwrap();
    let got = String::from_utf8_lossy(&out.stdout).into_owned();
    let want = expected(&fs::read_to_string(tests.join(format!("{name}.stdout"))).unwrap());
    if got != want {
        return Err(format!(
            "{name}: printed\n{got}\nexpected\n{want}\nstderr: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    let want_out = tests.join(format!("{name}.out"));
    if want_out.exists() {
        let want = fs::read_to_string(&want_out).unwrap();
        let got = fs::read_to_string(dir.join(format!("{up}.OUT"))).unwrap_or_default();
        if got != want {
            return Err(format!("{name}: wrote\n{got}\nexpected\n{want}"));
        }
    }
    Ok(())
}

#[test]
fn tpu() {
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR")).join("tpu");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    let exe = build(&tmp);
    let tests = repo().join("vms/sysexe/tpu/tests");
    let mut names: Vec<_> = fs::read_dir(&tests)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "stdout"))
        .map(|p| p.file_stem().unwrap().to_string_lossy().into_owned())
        .collect();
    names.sort();
    let failures: Vec<_> = names
        .iter()
        .filter_map(|name| run(&exe, &tests, name).err())
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
