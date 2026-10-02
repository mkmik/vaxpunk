//! cargo test -p boot: boots without a terminal, types commands at the
//! console's DCL prompt once the executive has started, and waits until the
//! processes have printed their last lines (roottask/sysexe/), DIRECTORY,
//! TYPE and EDIT theirs from the system disk, EDIT's EXIT that can't write
//! there, and DIRECTORY its own from the ramdisk, MDA0:, made the default
//! device, around a COPY to it from SYS$SYSDEVICE:, a logical name, an EDIT
//! in line mode and keypad mode that writes a second version and DELETEs,
//! then INITIALIZE, MOUNT and COPY on the data disk, DKB0:, made afresh in
//! out/check-datadisk.img, not the one you keep, and SHOW DEVICES, or the
//! root task is done, which is only on a halt or a fault. DCL reads what was
//! typed ahead a line at a time. Then it types a CONTINUE with nothing stopped, and stops
//! SPIN and SLEEPER with CTRL/Y, twice each, with a CONTINUE in between, a
//! step at a time: each waits until a line has come so many times, the echo
//! of what it typed before included, and types.
//! STARTUP's SLEEPER and SVCTEST's NAPPER say they hibernate before SLEEPER
//! does. The CPU then idles, taking clock interrupts. Once this QEMU is
//! gone, ods-image checks the data disk's volume and finds the file there.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::Duration;

use ods_image::{Conversion, Image, Mode, Severity};

/// What the serial log must hold.
const LINES: &[&str] = &[
    "%MOUNT-I-MOUNTED, VAXPUNK mounted on _DKA0:",
    "STARTUP: done",
    "SVCTEST: ok",
    "process NOSUCH exited",
    "HOG: NUDGE ran",
    "TIMETEST: ok",
    "ASTTEST: ok",
    "process SNOOP exited with status 0000000C",
    "process USURP exited with status 0000043C",
    "%NONAME-F-NOMSG, Message number 0000000C",
    " \\FOO\\",
    "PING.EXE;1          PONG.EXE;1",
    "Total of 2 files.",
    "and the rest of what INITIALIZE made.",
    "    9\tand DIRECTORY [000000]",
    "String was not found",
    "Unable to write the file, status 000182BA",
    "\"FOO\" = \"SYS$INPUT\" (LNM$PROCESS_TABLE)",
    "no translation for logical name FOO",
    "(LNM$SYSTEM_TABLE)",
    "  \"SYS$ERROR\" = \"_OPA0:\"",
    "  \"SYS$SYSTEM\" = \"SYS$SYSDEVICE:[SYSEXE]\"",
    "  DKA0:[SYSMGR]",
    "  DKA0:[SYSEXE]",
    "DCL.EXE;1           DELETE.EXE;1        DIRECTORY.EXE;1",
    "%DCL-I-INVDEF, DKA0:[NOSUCH] does not exist",
    "  DKA0:[NOSUCH]",
    "  DKA0:[000000]",
    "Directory DKA0:[SYSMGR]",
    "WELCOME.TXT;1",
    "%RMS-F-DIR, error in directory name",
    "%MOUNT-I-MOUNTED, RAM mounted on _MDA0:",
    "  MDA0:[000000]",
    "Directory MDA0:[000000]",
    "MDA0:[000000]RAM.TXT;2",
    "        Welcome to vaxpunk, an EDT-edited clone for arm64\n",
    "RAM.TXT;2           RAM.TXT;1",
    "%DIRECT-W-NOFILES, no files found",
    "%MOUNT-I-MOUNTED, DATA mounted on _DKB0:",
    "Directory DKB0:[000000]",
    "DATA.TXT;1",
    "DKA0:                   Mounted wrtlck       0  VAXPUNK",
    "DKB0:                   Mounted              0  DATA",
    "MDA0:                   Mounted              0  RAM",
    "OPA0:                   Online               0",
    "CONTINUE    goes back to the image CTRL/Y stopped",
];

/// Once the ramdisk is done: when a line has come so many times, type.
const STEPS: &[(&str, usize, &str)] = &[
    ("SPIN: spinning", 1, "\x19"),
    ("*INTERRUPT*", 1, "CONTINUE\r"),
    ("CONTINUE", 2, "\x19"),
    ("*INTERRUPT*", 2, "RUN SLEEPER\r"),
    ("SLEEPER: hibernating", 3, "\x19"),
    ("*INTERRUPT*", 3, "CONTINUE\r"),
    ("CONTINUE", 3, "\x19"),
    ("*INTERRUPT*", 4, "HELP\r"),
];

#[test]
fn boot() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let log_path = root.join("out/serial.log");
    let _ = fs::remove_file(&log_path);
    let datadisk = root.join("out/check-datadisk.img");
    let _ = fs::remove_file(&datadisk);
    let mut qemu = Command::new(env!("CARGO_BIN_EXE_boot"))
        .env("DATADISK", &datadisk)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut console = qemu.stdin.take().unwrap();
    let mut type_ = |s: &str| console.write_all(s.as_bytes()).unwrap();
    let log =
        || String::from_utf8_lossy(&fs::read(&log_path).unwrap_or_default()).replace('\r', "");

    let mut typed = 0;
    let mut step = 0;
    for _ in 0..120 {
        let text = log();
        if typed == 0 && text.contains("%EXEC-I-START") {
            type_("RUN STARTUP\rRUN SNOOP\rFOO\rDIR [SYSEXE]P%NG\rTYPE WELCOME.TXT\r");
            type_("EDIT WELCOME.TXT\r\"index\"\r\"zzz\"\rEXIT\rQUIT\r");
            typed = 1;
        } else if typed == 5 && step < STEPS.len() {
            let (line, times, keys) = STEPS[step];
            if text.lines().filter(|l| l.contains(line)).count() >= times {
                type_(keys);
                step += 1;
            }
        }
        // The rest once EDIT is done: the type-ahead buffer holds 255 characters.
        if typed == 1 && text.contains("String was not found") {
            type_("DEFINE FOO SYS$INPUT\rSHOW LOGICAL FOO\rSHOW LOGICAL\rDEASSIGN FOO\r");
            type_("SHOW LOGICAL FOO\rSHOW DEFAULT\rSET DEFAULT [SYSEXE]\rSHOW DEFAULT\rDIR D*\r");
            type_("SET DEFAULT [NOSUCH]\rSHOW DEFAULT\rSET DEFAULT [-]\rSHOW DEFAULT\r");
            type_("DIR [.SYSMGR]W*\rSET DEFAULT [-]\r");
            typed = 2;
        }
        // Then, once SET DEFAULT [-] has failed in [000000], the ramdisk.
        if typed == 2 && text.contains("error in directory name") {
            type_("INIT MDA0: RAM\rMOUNT MDA0: RAM\rSET DEFAULT MDA0:[000000]\rSHOW DEFAULT\r");
            type_("COPY SYS$SYSDEVICE:[SYSMGR]WELCOME.TXT RAM.TXT\r");
            type_("EDIT RAM.TXT\rD 3:END\rI\rEdited with EDT.\r\x1a");
            // Keypad mode: GOLD 5 goes to the top, GOLD PF3 finds OpenVMS,
            // seven DEL Cs delete it, and EDT-edited goes in its place.
            type_("C\r\x1bOP\x1bOu\x1bOP\x1bOROpenVMS\r");
            type_(&"\x1bOl".repeat(7));
            type_("EDT-edited\x1aEXIT\r");
            type_("TYPE RAM.TXT\r");
            typed = 3;
        }
        if typed == 3 && text.contains("MDA0:[000000]RAM.TXT;2") {
            type_("DIR *.TXT\rDELETE RAM.TXT;1\rDELETE RAM.TXT;2\rDIR *.TXT\r");
            typed = 4;
        }
        // Then the data disk.
        if typed == 4 && text.contains("no files found") {
            type_("INIT DKB0: DATA\rMOUNT DKB0: DATA\r");
            type_("COPY SYS$SYSDEVICE:[SYSMGR]WELCOME.TXT DKB0:[000000]DATA.TXT\r");
            type_("DIR DKB0:[000000]\rSHOW DEVICES\rCONTINUE\rRUN SPIN\r");
            typed = 5;
        }
        if LINES.iter().all(|l| text.contains(l)) || text.contains("root task done") {
            break;
        }
        sleep(Duration::from_secs(1));
    }
    // boot became run-qemu.sh, QEMU's parent: this QEMU, not another on out/.
    let _ = Command::new("pkill")
        .arg("-P")
        .arg(qemu.id().to_string())
        .status();
    drop(console);
    let _ = qemu.wait();

    let text = log();
    if let Some(start) = text.find("EXEC.EXE:") {
        let start = text[..start].rfind('\n').map_or(0, |i| i + 1);
        print!("{}", &text[start..]);
    }
    let missing: Vec<_> = LINES.iter().filter(|l| !text.contains(*l)).collect();
    assert!(missing.is_empty(), "no {missing:?}");
    assert!(
        !text.contains("SPIN: a register changed"),
        "SPIN's registers changed"
    );

    let mut img = Image::open(&datadisk, Mode::ReadOnly).unwrap();
    let report = img.verify().unwrap();
    assert_eq!(report.count(Severity::Error), 0, "{:?}", report.findings);
    assert_eq!(report.count(Severity::Leak), 0, "{:?}", report.findings);
    let fid = img.lookup("[000000]DATA.TXT").unwrap();
    let mut data = Vec::new();
    img.copy_out(fid, &mut data, Conversion::RecordsToLines)
        .unwrap();
    assert!(String::from_utf8_lossy(&data).contains("and the rest of what INITIALIZE made."));
}
