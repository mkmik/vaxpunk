//! cargo test -p boot --test rms: RMS on relative and indexed files
//! OpenVMS made (docs/prd/0008-rms-record-and-indexed-files.md). A data
//! disk made here holds ods/fixtures/rms's files, each with the record
//! attributes its FDL file gives; vaxpunk boots with it as DKB0:, and
//! RMSDUMP prints each file's records along each of its keys, which must
//! be what OpenVMS printed, the fixture's dump of that key.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

use ods_image::rms::fdl;
use ods_image::{Conversion, Image, InitParams};

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

/// A vaxpunk in QEMU, its console and its log.
struct Vax {
    qemu: Child,
    console: ChildStdin,
    log: PathBuf,
}

impl Vax {
    fn boot(disk: &Path, log: &Path) -> Vax {
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
        self.console.write_all(format!("{line}\r").as_bytes()).unwrap();
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
    let fixtures = root.join("ods/fixtures/rms");
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
            let got: Vec<_> = out.lines().skip(1).take_while(|l| !l.starts_with("RMSDUMP: ")).collect();
            let tail = out.lines().find(|l| l.starts_with("RMSDUMP: ")).unwrap_or("no tail");
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
