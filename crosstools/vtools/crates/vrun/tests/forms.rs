//! FORMS, DECforms's translator (t8y:/layered/decforms#ifdl), under vrun:
//! builds FORMS.EXE from vms/sysexe/forms's BLISS-64 and the I/O module,
//! crosstools/vtools/bliss/fio.mar, then runs `FORMS TRANSLATE NAME/LIST`
//! on each NAME.ifdl in vms/sysexe/forms/tests. What it prints must be
//! NAME.stdout, and its listing NAME.lis where there is one. A form it
//! writes, BACK_TRANSLATE must make NAME.back of, and that, translated and
//! back translated again, must come back the same. FORMS_BLESS=1 writes
//! the files the tests compare with: there is no DECforms to compare with,
//! so read what changed before committing it.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use vms_obj::obj;

fn repo() -> PathBuf {
    fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../..")).unwrap()
}

/// Links FORMS.EXE into `out`.
fn build(out: &Path) -> PathBuf {
    let repo = repo();
    let forms = repo.join("vms/sysexe/forms");
    let bliss = repo.join("crosstools/vtools/bliss");
    let mut objects = Vec::new();
    let mut sources: Vec<_> = fs::read_dir(&forms)
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
                forms.clone(),
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
        name: "FORMS".into(),
        transfer: None,
        link_time: 0,
        relocatable: false,
        shareable: None,
    };
    let image = vlink::link(&objects, &link)
        .unwrap_or_else(|e| panic!("{e:#?}"))
        .image;
    let exe = out.join("forms.exe");
    fs::write(&exe, image.write()).unwrap();
    exe
}

/// FORMS with the command line `args`, in `dir`: what it printed.
fn forms(exe: &Path, dir: &Path, args: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_vrun"))
        .args(["--timeout", "60", "--files"])
        .arg(dir)
        .arg(exe)
        .arg(args)
        .output()
        .unwrap();
    assert!(
        out.stderr.is_empty(),
        "FORMS {args}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Compares `got` with the file `want`, or writes it with FORMS_BLESS.
fn same(name: &str, what: &str, got: &str, want: &Path) -> Result<(), String> {
    if std::env::var("FORMS_BLESS").is_ok() {
        fs::write(want, got).unwrap();
        return Ok(());
    }
    let want = fs::read_to_string(want).unwrap_or_default();
    if got == want {
        Ok(())
    } else {
        Err(format!("{name}: {what}\n{got}\nexpected\n{want}"))
    }
}

/// Runs one test; Err says how it differs.
fn run(exe: &Path, tests: &Path, name: &str) -> Result<(), String> {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("forms-{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let up = name.to_uppercase();
    fs::copy(
        tests.join(format!("{name}.ifdl")),
        dir.join(format!("{up}.IFDL")),
    )
    .unwrap();
    let said = forms(exe, &dir, &format!("TRANSLATE {up}/LIST"));
    same(
        name,
        "printed",
        &said,
        &tests.join(format!("{name}.stdout")),
    )?;
    let lis = tests.join(format!("{name}.lis"));
    if lis.exists() {
        let got = fs::read_to_string(dir.join(format!("{up}.LIS"))).unwrap();
        same(name, "listed", &got, &lis)?;
    }
    if !dir.join(format!("{up}.FORM")).exists() {
        return Ok(());
    }
    let said = forms(exe, &dir, &format!("BACK_TRANSLATE {up}/OUTPUT={up}.BACK"));
    if !said.is_empty() {
        return Err(format!("{name}: BACK_TRANSLATE printed\n{said}"));
    }
    let back = fs::read_to_string(dir.join(format!("{up}.BACK"))).unwrap();
    same(
        name,
        "back translated",
        &back,
        &tests.join(format!("{name}.back")),
    )?;
    // The IFDL BACK_TRANSLATE wrote makes the same form again.
    forms(
        exe,
        &dir,
        &format!("TRANSLATE {up}.BACK/NOWARNINGS/OUTPUT=AGAIN"),
    );
    forms(exe, &dir, "BACK_TRANSLATE AGAIN/OUTPUT=AGAIN.BACK");
    let again = fs::read_to_string(dir.join("AGAIN.BACK")).unwrap_or_default();
    if again != back {
        return Err(format!(
            "{name}: translated again, back translated\n{again}"
        ));
    }
    Ok(())
}

#[test]
fn translate() {
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR")).join("forms");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    let exe = build(&tmp);
    let tests = repo().join("vms/sysexe/forms/tests");
    let mut names: Vec<_> = fs::read_dir(&tests)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "ifdl"))
        .map(|p| p.file_stem().unwrap().to_string_lossy().into_owned())
        .collect();
    names.sort();
    let failures: Vec<_> = names
        .iter()
        .filter_map(|name| run(&exe, &tests, name).err())
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// What FORMS says about the rest: a file that isn't a form, one that
/// isn't there, a function it doesn't have, and the error limit.
#[test]
fn messages() {
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR")).join("forms-messages");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    let exe = build(&tmp);
    fs::write(tmp.join("JUNK.FORM"), [0u8; 512]).unwrap();
    fs::copy(
        repo().join("vms/sysexe/forms/tests/refs.ifdl"),
        tmp.join("REFS.IFDL"),
    )
    .unwrap();
    for (args, want) in [
        (
            "BACK_TRANSLATE JUNK",
            "%FORMS-F-BADFORM, form file is invalid.\n",
        ),
        (
            "BACK_TRANSLATE NOSUCH",
            "%FORMS-F-OPENIN, error opening NOSUCH.FORM as input.\n",
        ),
        (
            "TRANSLATE NOSUCH/LOG",
            "%FORMS-F-OPENIN, error opening NOSUCH.IFDL as input.\n",
        ),
        (
            "EDIT NOSUCH",
            "%FORMS-E-NOTYETIMP, feature not yet implemented.\n",
        ),
    ] {
        assert_eq!(forms(&exe, &tmp, args), want, "FORMS {args}");
    }
    // Two errors, then the limit: what comes after isn't said.
    let said = forms(&exe, &tmp, "TRANSLATE REFS/ERROR_LIMIT=2");
    let lines: Vec<_> = said.lines().filter(|l| l.starts_with(['%', '-'])).collect();
    assert_eq!(
        lines,
        [
            "%FORMS-E-VIEENDLIN, viewport end line not within layout size.",
            "%FORMS-E-ENDLINGTR, ending viewport line must be greater than beginning viewport line.",
            "-FORMS-I-ERRLIMREA, error limit of 2 has been reached.",
            "%FORMS-F-TRANABORT, translation terminated before completely processing the IFDL file.",
        ],
        "{said}"
    );
}
