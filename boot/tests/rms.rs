//! cargo test -p boot --test rms: RMS on relative and indexed files
//! OpenVMS made (docs/prd/0008-rms-record-and-indexed-files.md). A data
//! disk made here holds crosstools/ods/fixtures/rms's files, each with the record
//! attributes its FDL file gives; vaxpunk boots with it as DKB0:, and
//! RMSDUMP prints each file's records along each of its keys, which must
//! be what OpenVMS printed, the fixture's dump of that key. CONVERT/FDL
//! makes those files again from the inputs and FDL files OpenVMS made them
//! from, with the same records, and ANALYZE/RMS_FILE finds them sound;
//! COPY copies one, and DIRECTORY/FULL describes it. A procedure,
//! IDXM.COM, makes IDXM.IDX with DCL's OPEN, READ and WRITE as MAKE.COM did
//! on OpenVMS, and must leave the same records.
//! Then RMSRAND runs a long script of random operations on an indexed
//! file, each of whose results a model here knows, and `ods` checks the
//! file after.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Mutex, MutexGuard};
use std::thread::sleep;
use std::time::{Duration, Instant};

use ods_image::rms::fdl;
use ods_image::{Conversion, Image, InitParams, Mode};

/// The fixtures: file, its keys.
const FILES: &[(&str, usize)] = &[
    ("idx1.idx", 2),
    ("idxc.idx", 2),
    ("idxf.idx", 3),
    ("idxb.idx", 1),
    ("comp.idx", 1),
    ("idxm.idx", 2),
    ("uaf.idx", 4),
    ("rel.rel", 1),
    ("relf.rel", 1),
];

/// A vaxpunk in QEMU, its console and its log. One at a time: they
/// share the system disk.
struct Vax {
    qemu: Child,
    console: ChildStdin,
    log: PathBuf,
    _one: MutexGuard<'static, ()>,
}

static ONE: Mutex<()> = Mutex::new(());

impl Vax {
    fn boot(disk: &Path, log: &Path) -> Vax {
        let one = ONE.lock().unwrap_or_else(|e| e.into_inner());
        let _ = fs::remove_file(log);
        let mut qemu = Command::new(env!("CARGO_BIN_EXE_boot"))
            .env("DATADISK", disk)
            .env("LOG", log)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let console = qemu.stdin.take().unwrap();
        let vax = Vax {
            qemu,
            console,
            log: log.to_path_buf(),
            _one: one,
        };
        vax.wait_for("%MOUNT-I-MOUNTED, DATA mounted on _DKB0:", 0, 60);
        vax.wait_for("\n$ ", 0, 60);
        vax
    }

    fn text(&self) -> String {
        String::from_utf8_lossy(&fs::read(&self.log).unwrap_or_default()).replace('\r', "")
    }

    /// Waits until the log has `what` past `from`; returns where it ends.
    fn wait_for(&self, what: &str, from: usize, secs: u64) -> usize {
        let end = Instant::now() + Duration::from_secs(secs);
        while Instant::now() < end {
            if let Some(i) = self.text().get(from..).and_then(|t| t.find(what)) {
                return from + i + what.len();
            }
            sleep(Duration::from_millis(200));
        }
        panic!("{}: no {what:?} after {from}", self.log.display());
    }

    /// Types a command and waits for `done` after it: the text from the
    /// command's echo to `done`.
    fn command(&mut self, line: &str, done: &str) -> String {
        let from = self.text().len();
        self.console
            .write_all(format!("{line}\r").as_bytes())
            .unwrap();
        let echo = self.wait_for(line, from, 20);
        let end = self.wait_for(done, echo, 120);
        self.text()[echo..end].to_string()
    }
}

impl Drop for Vax {
    fn drop(&mut self) {
        // boot and run-qemu.sh are QEMU's parents: this QEMU, not another.
        let _ = Command::new("pkill")
            .arg("-P")
            .arg(self.qemu.id().to_string())
            .status();
        let _ = self.qemu.kill();
        let _ = self.qemu.wait();
    }
}

/// A data disk, DATA, with the fixtures in [000000], each with its FDL's
/// record attributes.
fn fixture_disk(path: &Path, fixtures: &Path) {
    let _ = fs::remove_file(path);
    let params = InitParams {
        label: b"DATA".to_vec(),
        max_files: 64,
        ..Default::default()
    };
    let mut vol = Image::create(path, 4096, &params).unwrap();
    for (file, _) in FILES {
        let data = fs::read(fixtures.join(file)).unwrap();
        let stem = file.split('.').next().unwrap();
        let text = fs::read_to_string(fixtures.join(format!("{stem}.fdl"))).unwrap();
        let attrs = fdl::parse(&text).unwrap().record_attrs();
        vol.copy_in(
            &mut &data[..],
            &format!("[000000]{}", file.to_uppercase()),
            Conversion::Binary,
            Some(data.len() as u64),
            Some(attrs),
        )
        .unwrap();
    }
    vol.flush().unwrap();
}

#[test]
fn fixtures() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let fixtures = root.join("crosstools/ods/fixtures/rms");
    let disk = root.join("out/rms-datadisk.img");
    fixture_disk(&disk, &fixtures);
    let mut vax = Vax::boot(&disk, &root.join("out/rms.log"));
    vax.command("SET DEFAULT DKB0:[000000]", "\n$ ");
    vax.command("RMSDUMP :== $RMSDUMP", "\n$ ");
    let mut failed = Vec::new();
    for (file, keys) in FILES {
        let stem = file.split('.').next().unwrap();
        for key in 0..*keys {
            let out = vax.command(&format!("RMSDUMP {} {key}", file.to_uppercase()), "\n$ ");
            let want = fs::read_to_string(fixtures.join(format!("{stem}_key{key}.dump"))).unwrap();
            let got: Vec<_> = out
                .lines()
                .skip(1)
                .take_while(|l| !l.starts_with("RMSDUMP: "))
                .collect();
            let tail = out
                .lines()
                .find(|l| l.starts_with("RMSDUMP: "))
                .unwrap_or("no tail");
            if got != want.lines().collect::<Vec<_>>() || !tail.ends_with("0001827A") {
                let first = got.iter().zip(want.lines()).position(|(g, w)| *g != w);
                failed.push(format!(
                    "{file} key {key}: {} records of {}, {tail}, first difference at {first:?}",
                    got.len(),
                    want.lines().count()
                ));
            }
        }
    }
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

/// make.com's conversions: output, FDL, input, qualifiers, keys. Not
/// REL.REL's, which DCL's READ/DELETE and WRITE/UPDATE change after.
const CONVERSIONS: &[(&str, &str, &str, &str, usize)] = &[
    ("IDX1.IDX", "IDX1.FDL", "DATA.TXT", "", 2),
    ("IDXC.IDX", "IDXC.FDL", "DATA.TXT", "", 2),
    ("IDXF.IDX", "IDXF.FDL", "FIXD.TXT", "/PAD", 3),
    ("COMP.IDX", "COMP.FDL", "COMP.TXT", "", 1),
    ("RELF.REL", "RELF.FDL", "FIXD.TXT", "/PAD", 1),
];

#[test]
fn convert() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let fixtures = root.join("crosstools/ods/fixtures/rms");
    let disk = root.join("out/convert-datadisk.img");
    let _ = fs::remove_file(&disk);
    let params = InitParams {
        label: b"DATA".to_vec(),
        max_files: 64,
        ..Default::default()
    };
    let mut vol = Image::create(&disk, 4096, &params).unwrap();
    for (_, fdl, input, _, _) in CONVERSIONS {
        for file in [fdl, input] {
            let spec = format!("[000000]{file}");
            if vol.lookup(&spec).is_ok() {
                continue;
            }
            let data = fs::read(fixtures.join("make").join(file.to_lowercase())).unwrap();
            vol.copy_in(
                &mut &data[..],
                &spec,
                Conversion::LinesToRecords,
                Some(data.len() as u64),
                None,
            )
            .unwrap();
        }
    }
    let com = idxm_com();
    vol.copy_in(
        &mut com.as_bytes(),
        "[000000]IDXM.COM",
        Conversion::LinesToRecords,
        Some(com.len() as u64),
        None,
    )
    .unwrap();
    vol.flush().unwrap();
    drop(vol);
    let mut failed = Vec::new();
    {
        let mut vax = Vax::boot(&disk, &root.join("out/convert.log"));
        vax.command("SET DEFAULT DKB0:[000000]", "\n$ ");
        vax.command("RMSDUMP :== $RMSDUMP", "\n$ ");
        for (out, fdl, input, quals, keys) in CONVERSIONS {
            let said = vax.command(&format!("CONVERT/FDL={fdl}{quals} {input} {out}"), "\n$ ");
            if said.contains("-F-") {
                failed.push(format!("CONVERT {out}: {said}"));
            }
            let stem = out.split('.').next().unwrap().to_lowercase();
            for key in 0..*keys {
                let got = vax.command(&format!("RMSDUMP {out} {key}"), "\n$ ");
                let got: Vec<_> = got
                    .lines()
                    .skip(1)
                    .take_while(|l| !l.starts_with("RMSDUMP: "))
                    .collect();
                let want =
                    fs::read_to_string(fixtures.join(format!("{stem}_key{key}.dump"))).unwrap();
                if got != want.lines().collect::<Vec<_>>() {
                    failed.push(format!(
                        "{out} key {key}: {} records of {}",
                        got.len(),
                        want.lines().count()
                    ));
                }
            }
            let report = vax.command(&format!("ANALYZE/RMS_FILE {out}"), "\n$ ");
            if !report.contains("The analysis uncovered NO errors.") {
                failed.push(format!("ANALYZE/RMS_FILE {out}: {report}"));
            }
        }
        // COPY copies an indexed file block by block, records and keys.
        vax.command("COPY IDXF.IDX COPY.IDX", "\n$ ");
        let got = vax.command("RMSDUMP COPY.IDX 1", "\n$ ");
        let got: Vec<_> = got
            .lines()
            .skip(1)
            .take_while(|l| !l.starts_with("RMSDUMP: "))
            .collect();
        let want = fs::read_to_string(fixtures.join("idxf_key1.dump")).unwrap();
        if got != want.lines().collect::<Vec<_>>() {
            failed.push(format!("COPY.IDX key 1: {} records", got.len()));
        }
        let full = vax.command("DIRECTORY/FULL COPY.IDX", "\n$ ");
        if !full.contains("File organization:  Indexed, Prolog: 3, Using 3 keys") {
            failed.push(format!("DIRECTORY/FULL: {full}"));
        }
        // DCL's OPEN, READ and WRITE make IDXM.IDX as OpenVMS's did.
        vax.command("CONVERT/FDL=IDX1.FDL DATA.TXT IDXM.IDX", "\n$ ");
        let said = vax.command("@IDXM", "\n$ ");
        let want = [
            "IDXM: lookup 98994",
            "IDXM: generic K0000010 ZETA added record 3",
            "IDXM: key 1 K0000015 OMEGA updated, longer than it was before by a lot",
            "IDXM: next K0000004 ZETA added record 1",
            "IDXM: 257 records",
            "IDXM: written by DCL",
        ];
        let got: Vec<_> = said.lines().filter(|l| l.starts_with("IDXM: ")).collect();
        if got != want || said.contains("%DCL-") || said.contains("%RMS-") {
            failed.push(format!("@IDXM: {said}"));
        }
        for key in 0..2 {
            let got = vax.command(&format!("RMSDUMP IDXM.IDX {key}"), "\n$ ");
            let got: Vec<_> = got
                .lines()
                .skip(1)
                .take_while(|l| !l.starts_with("RMSDUMP: "))
                .collect();
            let want = fs::read_to_string(fixtures.join(format!("idxm_key{key}.dump"))).unwrap();
            if got != want.lines().collect::<Vec<_>>() {
                failed.push(format!("IDXM.IDX key {key}: {} records", got.len()));
            }
        }
    }
    let mut vol = Image::open(&disk, Mode::ReadOnly).unwrap();
    for out in CONVERSIONS
        .iter()
        .map(|c| c.0)
        .chain(["COPY.IDX", "IDXM.IDX"])
    {
        let fid = vol.lookup(&format!("[000000]{out}")).unwrap();
        let report = vol.check_file(fid).unwrap();
        if !report.is_sound() {
            failed.push(format!("{out}: {:?}", report.findings));
        }
    }
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

/// IDXM.COM: MAKE.COM's IDXM, without its lexical functions, then a
/// lookup that fails to /ERROR, a generic one, one by key 1 and the next
/// along it, a count to /END_OF_FILE, a sequential file written and read
/// back, and a CLOSE/NOLOG of a file not open.
fn idxm_com() -> String {
    let mut com = String::from("$ OPEN/READ/WRITE F IDXM.IDX\n");
    for i in 1..=60 {
        com += &format!("$ WRITE F \"K{:07} ZETA added record {i}\"\n", i * 3 + 1);
    }
    com += r#"$ READ/KEY="K0000009"/DELETE F R
$ READ/KEY="K0000012"/DELETE F R
$ READ/KEY="K0000300"/DELETE F R
$ READ/KEY="K0000015" F R
$ WRITE/UPDATE F "K0000015 OMEGA updated, longer than it was before by a lot"
$ READ/KEY="K0000018" F R
$ WRITE/UPDATE F "K0000018 ALPHA"
$ READ/KEY="K0000600" F R
$ WRITE/UPDATE F "K0000600 BETA changed"
$ READ/KEY="K0000001"/ERROR=NONE F R
$ WRITE SYS$OUTPUT "IDXM: found K0000001"
$ NONE:
$ WRITE SYS$OUTPUT "IDXM: lookup ", $STATUS
$ READ/KEY="K000001" F R
$ WRITE SYS$OUTPUT "IDXM: generic ", R
$ READ/INDEX=1/KEY="OMEGA" F R
$ WRITE SYS$OUTPUT "IDXM: key 1 ", R
$ READ F R
$ WRITE SYS$OUTPUT "IDXM: next ", R
$ CLOSE F
$ OPEN F IDXM.IDX
$ N = 0
$ COUNT:
$ READ/END_OF_FILE=DONE F R
$ N = N + 1
$ GOTO COUNT
$ DONE:
$ CLOSE F
$ WRITE SYS$OUTPUT "IDXM: ", N, " records"
$ OPEN/WRITE O NOTE
$ WRITE O "written by ", "DCL"
$ CLOSE O
$ OPEN/READ O NOTE.DAT
$ READ O R
$ CLOSE O
$ WRITE SYS$OUTPUT "IDXM: ", R
$ CLOSE/NOLOG O
"#;
    com
}

/// RMSRAND's file: its alternate keys' values. Key 2 follows key 0, so
/// that an $UPDATE, which may not change it, keeps it.
const ALT1: [&str; 6] = ["ALPHA", "BRAVO", "DELTA", "ECHO ", "GAMMA", "OMEGA"];
const NORMAL: &str = "00010001";
const OK_DUP: &str = "00018011";
const RNF: &str = "000182B2";
const DUP: &str = "000184EC";
const EOF: &str = "0001827A";

/// What RMSRAND prints of a record: its size, first 18 bytes and last.
fn shown(rec: &[u8]) -> String {
    format!(
        "{} {}{}",
        rec.len(),
        String::from_utf8_lossy(&rec[..18]),
        rec[rec.len() - 1] as char
    )
}

/// A random script for RMSRAND of `n` operations, and what it must print:
/// a model of the file, its records by key 0 and each alternate value's
/// records in the order they got it, says what each operation returns.
fn script(seed: u64, n: usize) -> (String, Vec<String>) {
    let mut x = seed;
    let mut rand = |m: u64| {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        (x % m) as usize
    };
    let mut recs: BTreeMap<Vec<u8>, Vec<u8>> = BTreeMap::new();
    let mut alts: [BTreeMap<Vec<u8>, Vec<Vec<u8>>>; 2] = Default::default();
    let (mut text, mut out) = (String::new(), Vec::new());
    let keys = (n as u64 / 3).max(250);
    for _ in 0..n {
        let k = rand(keys);
        let key = format!("R{k:07}").into_bytes();
        let a1 = ALT1[rand(ALT1.len() as u64)];
        let a2 = format!("{:03}", k % 7);
        let size = 20 + rand(181);
        let fill = (b'A' + rand(26) as u8) as char;
        let op = ["P", "P", "P", "P", "U", "U", "U", "D", "D", "G", "A"][rand(11)];
        text += &format!(
            "{op} {} {a1} {a2} {size:03} {fill}\n",
            String::from_utf8_lossy(&key)
        );
        let mut rec = format!("{} {a1} {a2}", String::from_utf8_lossy(&key)).into_bytes();
        rec.resize(size, fill as u8);
        let vals = |r: &[u8]| [r[9..14].to_vec(), r[15..18].to_vec()];
        let k8 = String::from_utf8_lossy(&key).to_string();
        let line = match (op, recs.get(&key).cloned()) {
            ("P", Some(_)) => format!("P {k8} {DUP}"),
            ("P", None) => {
                let mut dup = false;
                for (i, v) in vals(&rec).into_iter().enumerate() {
                    let list = alts[i].entry(v).or_default();
                    dup |= !list.is_empty();
                    list.push(key.clone());
                }
                recs.insert(key.clone(), rec);
                format!("P {k8} {}", if dup { OK_DUP } else { NORMAL })
            }
            ("U" | "D" | "G", None) => format!("{op} {k8} {RNF}"),
            ("U", Some(old)) => {
                let mut dup = false;
                for (i, (o, v)) in vals(&old).into_iter().zip(vals(&rec)).enumerate() {
                    if o != v {
                        alts[i].get_mut(&o).unwrap().retain(|p| *p != key);
                        let list = alts[i].entry(v).or_default();
                        dup |= !list.is_empty();
                        list.push(key.clone());
                    }
                }
                recs.insert(key.clone(), rec);
                format!("U {k8} {NORMAL} {}", if dup { OK_DUP } else { NORMAL })
            }
            ("D", Some(old)) => {
                for (i, v) in vals(&old).into_iter().enumerate() {
                    alts[i].get_mut(&v).unwrap().retain(|p| *p != key);
                }
                recs.remove(&key);
                format!("D {k8} {NORMAL} {NORMAL}")
            }
            ("G", Some(r)) => format!("G {k8} {NORMAL} {}", shown(&r)),
            _ => match alts[0].get(a1.as_bytes()).and_then(|l| l.first()) {
                Some(p) => format!("A {a1} {NORMAL} {}", shown(&recs[p])),
                None => format!("A {a1} {RNF}"),
            },
        };
        out.push(line);
    }
    out.extend(recs.values().map(|r| format!(" {}", shown(r))));
    out.push(format!("KEY 0 {} {EOF}", recs.len()));
    for (i, alt) in alts.iter().enumerate() {
        let along: Vec<_> = alt.values().flatten().collect();
        out.extend(along.iter().map(|p| format!(" {}", shown(&recs[*p]))));
        out.push(format!("KEY {} {} {EOF}", i + 1, along.len()));
    }
    out.push(format!("RMSRAND done {NORMAL}"));
    (text, out)
}

#[test]
fn random() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let disk = root.join("out/rmsrand-datadisk.img");
    // RMSRAND_SEED and RMSRAND_OPS run another script; out/rmsrand.want is
    // what it must print, to compare with OpenVMS's.
    let env = |name, default| {
        std::env::var(name)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    };
    let (text, want) = script(
        env("RMSRAND_SEED", 0x5eed_0008),
        env("RMSRAND_OPS", 6000) as usize,
    );
    fs::write(root.join("out/rmsrand.want"), want.join("\n") + "\n").unwrap();
    let _ = fs::remove_file(&disk);
    let params = InitParams {
        label: b"DATA".to_vec(),
        max_files: 16,
        ..Default::default()
    };
    let mut vol = Image::create(&disk, 4096, &params).unwrap();
    vol.copy_in(
        &mut text.as_bytes(),
        "[000000]RAND.TXT",
        Conversion::LinesToRecords,
        Some(text.len() as u64),
        None,
    )
    .unwrap();
    vol.flush().unwrap();
    drop(vol);
    let got = {
        let mut vax = Vax::boot(&disk, &root.join("out/rmsrand.log"));
        vax.command("SET DEFAULT DKB0:[000000]", "\n$ ");
        vax.command("RMSRAND :== $RMSRAND", "\n$ ");
        vax.command("RMSRAND", "\n$ ")
    };
    let got: Vec<_> = got
        .lines()
        .skip(1)
        .take_while(|l| !l.starts_with('$'))
        .map(str::to_string)
        .collect();
    for (n, (w, g)) in want.iter().zip(&got).enumerate() {
        assert_eq!(w, g, "RMSRAND's line {} isn't the model's", n + 1);
    }
    assert_eq!(want.len(), got.len(), "RMSRAND's lines");
    let mut vol = Image::open(&disk, Mode::ReadOnly).unwrap();
    let fid = vol.lookup("[000000]RAND.IDX").unwrap();
    let report = vol.check_file(fid).unwrap();
    assert!(report.is_sound(), "RAND.IDX: {:?}", report.findings);
}
