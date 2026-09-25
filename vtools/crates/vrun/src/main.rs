//! vrun: runs a vaxpunk EXE at EL0 in a bare-metal QEMU virt machine.
//! docs/runner-abi.md describes what the image sees.

mod layout;
mod plan;

use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Command, ExitCode, Stdio};
use std::time::{Duration, Instant};
use std::{env, fs, io, process, thread};

use vms_obj::exe::Image;

use crate::layout::LOAD_BASE;

const USAGE: &str = "usage: vrun [--hvf] [--gdb] [--map FILE] [--timeout SECONDS] [--verbose] \
                     IMAGE [ARGUMENTS...]";
const STUB: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/stub.bin"));

/// VMS condition values for image faults.
const SS_ACCVIO: u32 = 0x0c;
const SS_ABORT: u32 = 0x2c;
const SS_OPCDEC: u32 = 0x43c;

struct Options {
    hvf: bool,
    gdb: bool,
    /// The image's link map, for symbols in fault messages.
    map: Option<PathBuf>,
    timeout: Option<Duration>,
    verbose: bool,
    image: PathBuf,
    args: String,
}

/// How the image ended, from the stub's "!vrun" line.
#[derive(Debug, PartialEq)]
enum Outcome {
    Exit(u64),
    Fault { esr: u64, pc: u64, far: u64 },
    StubFault { esr: u64, pc: u64, far: u64 },
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(msg) => {
            eprintln!("%VRUN-F-{msg}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<u8, String> {
    let opt = options(env::args().skip(1))?;
    let file = fs::read(&opt.image)
        .map_err(|e| format!("OPENIN, cannot read {}: {e}", opt.image.display()))?;
    let image = Image::parse(&file).map_err(|e| {
        format!(
            "IMGFMT, {} is not a vaxpunk image: {e}",
            opt.image.display()
        )
    })?;
    let symbols = match &opt.map {
        Some(path) => map_symbols(
            &fs::read_to_string(path)
                .map_err(|e| format!("OPENIN, cannot read {}: {e}", path.display()))?,
        ),
        None => Vec::new(),
    };
    let plan = plan::plan(&image, opt.args.as_bytes(), STUB)?;
    if opt.verbose {
        for r in &plan.map {
            eprintln!(
                "%VRUN-I-MAP, {:016X}-{:016X} {:?} at physical {:08X}",
                r.va,
                r.va + r.size - 1,
                r.prot,
                r.pa
            );
        }
    }

    let ram = TempFile::new(&plan.ram)?;
    let mut qemu = Command::new("qemu-system-aarch64");
    qemu.args([
        "-M", "virt", "-display", "none", "-monitor", "none", "-serial", "stdio",
    ])
    .args(["-no-reboot", "-m", &format!("{}M", plan.ram_mb)])
    .args(if opt.hvf {
        ["-accel", "hvf", "-cpu", "host"]
    } else {
        ["-accel", "tcg", "-cpu", "max"]
    })
    .arg("-device")
    .arg(format!(
        "loader,file={},addr={LOAD_BASE:#x},force-raw=on",
        ram.0.display().to_string().replace(',', ",,")
    ))
    .arg("-device")
    .arg(format!("loader,addr={LOAD_BASE:#x},cpu-num=0"));
    if opt.gdb {
        let start = image.transfer;
        qemu.args(["-s", "-S"]);
        eprintln!(
            "%VRUN-I-GDB, waiting for a debugger on port 1234; the image starts at {start:016X}"
        );
        eprintln!("  lldb -o 'gdb-remote 1234' -o 'breakpoint set -a {start:#x}' -o continue");
        eprintln!("  gdb-multiarch -ex 'target remote :1234' -ex 'break *{start:#x}' -ex continue");
    }
    if opt.verbose {
        eprintln!("%VRUN-I-QEMU, {qemu:?}");
    }

    let mut child = qemu
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| format!("QEMU, cannot start qemu-system-aarch64: {e}"))?;
    let console = child.stdout.take().unwrap();
    let relay = thread::spawn(move || relay(console, io::stdout()));

    let deadline = opt.timeout.filter(|_| !opt.gdb).map(|t| Instant::now() + t);
    loop {
        if child
            .try_wait()
            .map_err(|e| format!("QEMU, {e}"))?
            .is_some()
        {
            break;
        }
        if deadline.is_some_and(|d| Instant::now() >= d) {
            let _ = child.kill();
            let _ = child.wait();
            let secs = opt.timeout.unwrap().as_secs();
            return Err(format!(
                "TIMEOUT, the image did not finish within {secs} seconds"
            ));
        }
        thread::sleep(Duration::from_millis(10));
    }

    match relay.join().unwrap() {
        Some(Outcome::Exit(x0)) => {
            let status = x0 as u32;
            if opt.verbose {
                eprintln!("%VRUN-I-EXIT, image exited with status %X{status:08X}");
            }
            Ok(host_code(status))
        }
        Some(Outcome::Fault { esr, pc, far }) => {
            let (msg, status) = fault(esr, pc, far, &image, &symbols);
            eprintln!("{msg}");
            Ok(host_code(status))
        }
        Some(Outcome::StubFault { esr, pc, far }) => Err(format!(
            "STUBFAULT, the boot stub faulted: ESR={esr:016X}, PC={pc:016X}, virtual address={far:016X}"
        )),
        None if opt.gdb => Err("NOSTATUS, the debugger ended the run".into()),
        None => Err("NOSTATUS, QEMU exited without the image's status".into()),
    }
}

fn options(mut args: impl Iterator<Item = String>) -> Result<Options, String> {
    let mut opt = Options {
        hvf: false,
        gdb: false,
        map: None,
        timeout: Some(Duration::from_secs(30)),
        verbose: false,
        image: PathBuf::new(),
        args: String::new(),
    };
    let usage = || format!("USAGE, {USAGE}");
    loop {
        match args.next().ok_or_else(usage)?.as_str() {
            "--hvf" => opt.hvf = true,
            "--gdb" => opt.gdb = true,
            "--verbose" => opt.verbose = true,
            "--map" => opt.map = Some(args.next().ok_or_else(usage)?.into()),
            "--timeout" => {
                let secs: u64 = args.next().and_then(|s| s.parse().ok()).ok_or_else(usage)?;
                opt.timeout = (secs > 0).then(|| Duration::from_secs(secs));
            }
            flag if flag.starts_with("--") => return Err(usage()),
            image => {
                opt.image = image.into();
                opt.args = args.collect::<Vec<_>>().join(" ");
                return Ok(opt);
            }
        }
    }
}

/// Copies the guest console to `output` and returns the stub's "!vrun"
/// report, which it leaves out. The report may follow the image's output on
/// the same line; everything before it is relayed exactly. Binary-safe.
fn relay(console: impl Read, mut output: impl Write) -> Option<Outcome> {
    const MARK: &[u8] = b"!vrun ";
    let mut console = BufReader::new(console);
    let mut line = Vec::new();
    let mut outcome = None;
    loop {
        line.clear();
        match console.read_until(b'\n', &mut line) {
            Ok(0) | Err(_) => return outcome,
            Ok(_) => {}
        }
        let mark = line.windows(MARK.len()).position(|w| w == MARK);
        let text = &line[..mark.unwrap_or(line.len())];
        let _ = output.write_all(text).and_then(|()| output.flush());
        if let Some(at) = mark {
            outcome = parse_report(&line[at + MARK.len()..]);
        }
    }
}

fn parse_report(report: &[u8]) -> Option<Outcome> {
    let report = std::str::from_utf8(report).ok()?;
    let mut words = report.split_ascii_whitespace();
    let kind = words.next()?;
    let mut hex = words.map(|w| u64::from_str_radix(w, 16).ok());
    let mut next = || hex.next().flatten();
    match kind {
        "exit" => Some(Outcome::Exit(next()?)),
        "fault" => Some(Outcome::Fault {
            esr: next()?,
            pc: next()?,
            far: next()?,
        }),
        "stubfault" => Some(Outcome::StubFault {
            esr: next()?,
            pc: next()?,
            far: next()?,
        }),
        _ => None,
    }
}

/// The message and VMS condition value for a fault, from its exception
/// syndrome, then lines that say where the PC and the address are.
fn fault(esr: u64, pc: u64, far: u64, image: &Image, symbols: &[(u64, String)]) -> (String, u32) {
    let at =
        place(pc, image, symbols).map_or(String::new(), |p| format!("\n-VRUN-I-PC, PC is {p}"));
    match esr >> 26 {
        0x20 | 0x21 | 0x24 | 0x25 => {
            let address = match place(far, image, symbols) {
                Some(p) => format!("\n-VRUN-I-ADDRESS, virtual address is {p}"),
                None if plan::GUARD.contains(&far) => {
                    "\n-VRUN-I-STACKOVF, virtual address is below the stack: it overflowed".into()
                }
                None => String::new(),
            };
            (
                format!(
                    "%VRUN-F-ACCVIO, access violation, virtual address={far:016X}, \
                     PC={pc:016X}{at}{address}"
                ),
                SS_ACCVIO,
            )
        }
        0x00 => (
            format!("%VRUN-F-OPCDEC, reserved or privileged instruction, PC={pc:016X}{at}"),
            SS_OPCDEC,
        ),
        0x01 | 0x18 => (
            format!("%VRUN-F-OPCDEC, privileged instruction, PC={pc:016X}{at}"),
            SS_OPCDEC,
        ),
        0x15 => (
            format!(
                "%VRUN-F-OPCDEC, no monitor call SVC #{}, PC={pc:016X}{at}",
                esr & 0xffff
            ),
            SS_OPCDEC,
        ),
        ec => (
            format!(
                "%VRUN-F-EXCEPT, unexpected exception class {ec:02X}, ESR={esr:016X}, \
                 virtual address={far:016X}, PC={pc:016X}{at}"
            ),
            SS_ABORT,
        ),
    }
}

/// Where `addr` is in the image: the nearest symbol at or before it in the
/// same image section, if any, and the section and offset.
fn place(addr: u64, image: &Image, symbols: &[(u64, String)]) -> Option<String> {
    let (i, s) = image
        .sections
        .iter()
        .enumerate()
        .find(|(_, s)| (s.vaddr..s.vaddr + u64::from(s.size)).contains(&addr))?;
    let section = format!("image section {} + %X{:X}", i + 1, addr - s.vaddr);
    let symbol = symbols
        .iter()
        .filter(|(v, _)| (s.vaddr..=addr).contains(v))
        .max_by_key(|(v, _)| *v);
    Some(match symbol {
        Some((v, name)) if *v == addr => format!("{name} ({section})"),
        Some((v, name)) => format!("{name}+%X{:X} ({section})", addr - v),
        None => section,
    })
}

/// The symbols of a vlink map: its "Symbols By Value" list.
fn map_symbols(map: &str) -> Vec<(u64, String)> {
    map.lines()
        .skip_while(|l| l.trim() != "Symbols By Value")
        .take_while(|l| l.trim() != "Image Synopsis")
        .filter_map(|l| {
            let (value, name) = l.trim().split_once(' ')?;
            Some((
                u64::from_str_radix(value, 16).ok()?,
                name.trim().to_string(),
            ))
        })
        .collect()
}

/// Maps a VMS status to a host exit code: 0 if the low bit is set (success),
/// otherwise the low byte, and 1 if that is zero.
fn host_code(status: u32) -> u8 {
    match status {
        s if s & 1 != 0 => 0,
        s => (s as u8).max(1),
    }
}

/// A file holding guest RAM for QEMU's loader, removed when dropped.
struct TempFile(PathBuf);

impl TempFile {
    fn new(data: &[u8]) -> Result<Self, String> {
        let path = env::temp_dir().join(format!("vrun-{}.ram", process::id()));
        fs::write(&path, data).map_err(|e| format!("WRITERR, {}: {e}", path.display()))?;
        Ok(Self(path))
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports() {
        let mut output = Vec::new();
        let console = b"hello\nno newline!vrun exit 000000000000002c\n";
        assert_eq!(relay(&console[..], &mut output), Some(Outcome::Exit(0x2c)));
        assert_eq!(output, b"hello\nno newline");
        let fault = parse_report(b"fault 0000000092000006 0000000000010000 0000000000000008\n");
        assert_eq!(
            fault,
            Some(Outcome::Fault {
                esr: 0x9200_0006,
                pc: 0x10000,
                far: 8
            })
        );
        assert_eq!(parse_report(b"exit zz\n"), None);
    }

    #[test]
    fn statuses() {
        assert_eq!(host_code(1), 0);
        assert_eq!(host_code(0x2c), 44);
        assert_eq!(host_code(0x100), 1);
        let image = sample();
        let fault = |esr| fault(esr, 0, 0, &image, &[]).1;
        assert_eq!(fault(0x9200_0006), SS_ACCVIO, "data abort from EL0");
        assert_eq!(fault(0x0200_0000), SS_OPCDEC, "undefined instruction");
    }

    fn sample() -> Image {
        let section = |vaddr, size: u32| vms_obj::exe::Section {
            vaddr,
            size,
            flags: 0,
            data: vec![0; size as usize],
        };
        Image {
            name: "T".into(),
            ident: String::new(),
            link_time: 0,
            transfer: 0x10000,
            sections: vec![section(0x10000, 0x20), section(0x20000, 8)],
        }
    }

    #[test]
    fn symbols() {
        let map = "\
Symbols By Name

  Symbol                           Value             Module
  START                            0000000000010000  M

Symbols By Value

  Value             Symbol
  0000000000000005  FIVE
  0000000000010000  START
  0000000000010010  MID
  0000000000020000  MSG

Image Synopsis

  Transfer address  0000000000010000 (START)
";
        let symbols = map_symbols(map);
        assert_eq!(symbols.len(), 4, "{symbols:?}");
        let image = sample();
        let at = |addr| place(addr, &image, &symbols);
        assert_eq!(at(0x10000).unwrap(), "START (image section 1 + %X0)");
        assert_eq!(at(0x10014).unwrap(), "MID+%X4 (image section 1 + %X14)");
        assert_eq!(at(0x20004).unwrap(), "MSG+%X4 (image section 2 + %X4)");
        assert_eq!(at(5), None, "FIVE is a constant, outside the image");
        assert_eq!(
            place(0x10004, &image, &[]).unwrap(),
            "image section 1 + %X4"
        );
        let (msg, _) = fault(0x9200_0006, 0x10004, 0x7ff0_fff0, &image, &symbols);
        let lines: Vec<&str> = msg.lines().skip(1).collect();
        assert_eq!(
            lines,
            [
                "-VRUN-I-PC, PC is START+%X4 (image section 1 + %X4)",
                "-VRUN-I-STACKOVF, virtual address is below the stack: it overflowed"
            ]
        );
    }
}
