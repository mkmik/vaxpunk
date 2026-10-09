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
    // Its modules, and the terminal it has under vrun (vrun/tt.b64).
    let mut sources: Vec<_> = fs::read_dir(&tpu)
        .unwrap()
        .chain(fs::read_dir(tpu.join("vrun")).unwrap())
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

/// What the oracle printed, its directory and versions taken out of file
/// names: a `;` and digits after a letter.
fn expected(text: &str) -> String {
    let text = text.replace("SYS$SYSDEVICE:[TPUORACLE]", "");
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == ';'
            && out.ends_with(|p: char| p.is_ascii_alphabetic())
            && chars.peek().is_some_and(char::is_ascii_digit)
        {
            while chars.peek().is_some_and(char::is_ascii_digit) {
                chars.next();
            }
        } else {
            out.push(c);
        }
    }
    out
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
    let keys = tests.join(format!("{name}.keys"));
    if keys.exists() {
        // On the screen: the keys in TT.IN, a burst a line; the screen is
        // TT.OUT, which must look as the oracle's did.
        fs::write(
            dir.join("TT.IN"),
            unescape(&fs::read_to_string(&keys).unwrap()),
        )
        .unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_vrun"))
            .args(["--timeout", "60", "--files"])
            .arg(&dir)
            .arg(exe)
            .arg(format!("/NOSECTION/COMMAND={up}.TPU"))
            .output()
            .unwrap();
        let got = render(&fs::read(dir.join("TT.OUT")).unwrap_or_default());
        let want = render(&fs::read(tests.join(format!("{name}.vt"))).unwrap());
        if got != want {
            return Err(format!(
                "{name}: showed\n{got}\nexpected\n{want}\nstderr: {}",
                String::from_utf8_lossy(&out.stderr)
            ));
        }
        return Ok(());
    }
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
        .filter(|p| p.extension().is_some_and(|x| x == "stdout" || x == "vt"))
        .map(|p| p.file_stem().unwrap().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names.dedup();
    let failures: Vec<_> = names
        .iter()
        .filter_map(|name| run(&exe, &tests, name).err())
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// EVE (eve/eve.tpu, saved as a section file) on each eve/tests/NAME.txt, NAME.eve
/// typed, a burst a line: the screen must end as the oracle's EVE left it,
/// NAME.vt, and the file must be NAME.out if there is one.
#[test]
fn eve() {
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR")).join("eve");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    let exe = build(&tmp);
    let eve = repo().join("vms/sysexe/tpu/eve");
    let tests = eve.join("tests");
    // EVE's section file, as a system manager makes one: EVE's source
    // run, then SAVE.
    let mk = tmp.join("section");
    fs::create_dir_all(&mk).unwrap();
    let source = fs::read_to_string(eve.join("eve.tpu")).unwrap();
    fs::write(
        mk.join("MKEVE.TPU"),
        format!("{source}SAVE (\"EVE\");\nQUIT;\n"),
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vrun"))
        .args(["--timeout", "60", "--files"])
        .arg(&mk)
        .arg(&exe)
        .arg("/NODISPLAY/NOSECTION/COMMAND=MKEVE.TPU")
        .output()
        .unwrap();
    let said = String::from_utf8_lossy(&out.stdout);
    assert!(
        said.starts_with("%TPU-S-SECTSAVED,") && said.lines().count() == 1,
        "making EVE's section said\n{said}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let section = fs::read(mk.join("EVE.TPU$SECTION")).unwrap();
    // The system disk's copy (vms/build.rs puts it in SYS$SHARE) must be
    // this one: TPU_BLESS=1 writes it.
    let disk = eve.join("eve.section");
    if std::env::var("TPU_BLESS").is_ok() {
        fs::write(&disk, &section).unwrap();
    }
    assert!(
        fs::read(&disk).ok() == Some(section.clone()),
        "{} isn't EVE's section as TPU saves it now: run TPU_BLESS=1 cargo test -p vrun --test tpu eve",
        disk.display()
    );
    let mut names: Vec<_> = fs::read_dir(&tests)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "eve") && p.with_extension("vt").exists())
        .map(|p| p.file_stem().unwrap().to_string_lossy().into_owned())
        .collect();
    names.sort();
    let mut failures = Vec::new();
    for name in &names {
        let dir = tmp.join(name);
        fs::create_dir_all(&dir).unwrap();
        let up = name.to_uppercase();
        fs::write(dir.join("EVE.TPU$SECTION"), &section).unwrap();
        // The file, empty if the session has none, as the oracle makes it.
        let txt = fs::read(tests.join(format!("{name}.txt"))).unwrap_or_default();
        fs::write(dir.join(format!("{up}.TXT")), txt).unwrap();
        fs::write(
            dir.join("TT.IN"),
            unescape(&fs::read_to_string(tests.join(format!("{name}.eve"))).unwrap()),
        )
        .unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_vrun"))
            .args(["--timeout", "60", "--files"])
            .arg(&dir)
            .arg(&exe)
            .arg(format!("/SECTION=EVE {up}.TXT"))
            .output()
            .unwrap();
        let got = render(&fs::read(dir.join("TT.OUT")).unwrap_or_default());
        let vt = fs::read(tests.join(format!("{name}.vt"))).unwrap();
        // The oracle's file names cut on the screen it showed, not in
        // what it sent, which wrote over what longer names covered.
        let want = expected(&render(&vt));
        if got != want {
            failures.push(format!(
                "{name}: showed\n{got}\nexpected\n{want}\nstderr: {}",
                String::from_utf8_lossy(&out.stderr)
            ));
            continue;
        }
        let want_out = tests.join(format!("{name}.out"));
        if want_out.exists() {
            let want = fs::read_to_string(&want_out).unwrap();
            let got = fs::read_to_string(dir.join(format!("{up}.TXT"))).unwrap_or_default();
            if got != want {
                failures.push(format!("{name}: wrote\n{got}\nexpected\n{want}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The screen a VT100 of 80 by 24 shows after `bytes`, up to where TPU
/// puts the terminal back as it found it at the end: its rows, blanks at
/// their ends cut, and where the cursor is.
fn render(bytes: &[u8]) -> String {
    const EXIT: &[u8] = b"\x1b>\x1b[?7h";
    let end = bytes
        .windows(EXIT.len())
        .rposition(|w| w == EXIT)
        .unwrap_or(bytes.len());
    let mut parser = vt100::Parser::new(24, 80, 0);
    parser.process(&home_after_decstbm(&bytes[..end]));
    let screen = parser.screen();
    let mut out: Vec<String> = screen
        .rows(0, 80)
        .map(|r| r.trim_end().to_string())
        .collect();
    // The video of each row with any: R reverse, B bold, U underline.
    for r in 0..24 {
        let mask: String = (0..80)
            .map(|c| match screen.cell(r, c) {
                // Erased cells are plain on a VT100, whatever the video.
                Some(x) if !x.has_contents() => ' ',
                Some(x) if x.inverse() => 'R',
                Some(x) if x.bold() => 'B',
                Some(x) if x.underline() => 'U',
                _ => ' ',
            })
            .collect();
        if mask.trim() != "" {
            out.push(format!("video {:2} {}", r + 1, mask.trim_end()));
        }
    }
    let (row, col) = screen.cursor_position();
    out.push(format!("cursor {},{}", row + 1, col + 1));
    out.join("\n")
}

/// `bytes` with a cursor home after each DECSTBM, `ESC [ t ; b r`, which
/// a VT100 does and the vt100 crate doesn't.
fn home_after_decstbm(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        out.push(bytes[i]);
        if bytes[i..].starts_with(b"\x1b[") {
            let n = bytes[i + 2..]
                .iter()
                .take_while(|c| c.is_ascii_digit() || **c == b';')
                .count();
            if bytes.get(i + 2 + n) == Some(&b'r') {
                out.extend_from_slice(&bytes[i + 1..i + 3 + n]);
                out.extend_from_slice(b"\x1b[H");
                i += 3 + n;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// Prints the oracle's screens: TPU_SHOW=1 cargo test -p vrun --test tpu show.
#[test]
fn show() {
    if std::env::var("TPU_SHOW").is_err() {
        return;
    }
    let tpu = repo().join("vms/sysexe/tpu");
    let mut vts: Vec<_> = fs::read_dir(tpu.join("tests"))
        .unwrap()
        .chain(fs::read_dir(tpu.join("eve/tests")).unwrap())
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "vt"))
        .collect();
    vts.sort();
    for vt in vts {
        println!(
            "===== {}\n{}",
            vt.display(),
            render(&fs::read(&vt).unwrap())
        );
    }
}

/// A .keys file's bytes: its lines' \\r, \\n, \\t, \\\\ and \\xHH escapes
/// decoded, as the oracle's Python decodes them, \\n as 255 and n; the
/// lines stay apart.
fn unescape(text: &str) -> Vec<u8> {
    let mut out = Vec::new();
    for line in text.lines() {
        let b = line.as_bytes();
        let mut i = 0;
        while i < b.len() {
            if b[i] == b'\\' && i + 1 < b.len() {
                match b[i + 1] {
                    b'r' => out.push(13),
                    // A line feed typed: vrun/tt.b64 reads it back.
                    b'n' => out.extend_from_slice(&[255, b'n']),
                    b't' => out.push(9),
                    b'x' if i + 3 < b.len() => {
                        let hex = std::str::from_utf8(&b[i + 2..i + 4]).unwrap();
                        out.push(u8::from_str_radix(hex, 16).unwrap());
                        i += 2;
                    }
                    c => out.push(c),
                }
                i += 2;
            } else {
                out.push(b[i]);
                i += 1;
            }
        }
        out.push(b'\n');
    }
    out
}
