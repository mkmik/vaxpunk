//! cargo test -p boot: boots without a terminal, types commands at the
//! console's DCL prompt once the executive has started, and waits until the
//! processes have printed their last lines (roottask/sysexe/), DIRECTORY,
//! TYPE and EDIT theirs from the system disk, EDIT's EXIT that can't write
//! there, and DIRECTORY its own from the ramdisk, MDA0:, made the default
//! device, around a COPY to it from SYS$SYSDEVICE:, a logical name, an EDIT
//! that writes a second version and DELETEs, or the root task is
//! done, which is only on a halt or a fault. DCL reads what was typed ahead a
//! line at a time. Then it types a CONTINUE with nothing stopped, and stops
//! SPIN and SLEEPER with CTRL/Y, twice each, with a CONTINUE in between, a
//! step at a time: each waits until a line has come so many times, the echo
//! of what it typed before included, and types.
//! STARTUP's SLEEPER and SVCTEST's NAPPER say they hibernate before SLEEPER
//! does. The CPU then idles, taking clock interrupts.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::Duration;

/// What the serial log must hold.
const LINES: &[&str] = &[
    "%MOUNT-I-MOUNTED, VAXPUNK mounted on _DKA0:",
    "STARTUP: done",
    "SVCTEST: ok",
    "process NOSUCH exited",
    "HOG: NUDGE ran",
    "TIMETEST: ok",
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
    "%NONAME-F-NOMSG, Message number 000184CC",
    "%MOUNT-I-MOUNTED, RAM mounted on _MDA0:",
    "  MDA0:[000000]",
    "Directory MDA0:[000000]",
    "MDA0:[000000]RAM.TXT;2",
    "RAM.TXT;2           RAM.TXT;1",
    "%DIRECT-W-NOFILES, no files found",
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
    let mut qemu = Command::new(env!("CARGO_BIN_EXE_boot"))
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut console = qemu.stdin.take().unwrap();
    let mut type_ = |s: &str| console.write_all(s.as_bytes()).unwrap();
    let log = || String::from_utf8_lossy(&fs::read(&log_path).unwrap_or_default()).into_owned();

    let mut typed = 0;
    let mut step = 0;
    for _ in 0..120 {
        let text = log();
        if typed == 0 && text.contains("%EXEC-I-START") {
            type_("RUN STARTUP\rRUN SNOOP\rFOO\rDIR [SYSEXE]P%NG\rTYPE WELCOME.TXT\r");
            type_("EDIT WELCOME.TXT\r\"index\"\r\"zzz\"\rEXIT\rQUIT\r");
            typed = 1;
        } else if typed == 4 && step < STEPS.len() {
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
        if typed == 2 && text.contains("Message number 000184CC") {
            type_("INIT MDA0: RAM\rMOUNT MDA0: RAM\rSET DEFAULT MDA0:[000000]\rSHOW DEFAULT\r");
            type_("COPY SYS$SYSDEVICE:[SYSMGR]WELCOME.TXT RAM.TXT\r");
            type_("EDIT RAM.TXT\rD 3:END\rI\rEdited with EDT.\r\x1aEXIT\r");
            type_("TYPE RAM.TXT\r");
            typed = 3;
        }
        if typed == 3 && text.contains("MDA0:[000000]RAM.TXT;2") {
            type_("DIR *.TXT\rDELETE RAM.TXT;1\rDELETE RAM.TXT;2\rDIR *.TXT\r");
            type_("CONTINUE\rRUN SPIN\r");
            typed = 4;
        }
        if LINES.iter().all(|l| text.contains(l)) || text.contains("root task done") {
            break;
        }
        sleep(Duration::from_secs(1));
    }
    let esp = root.join("out/esp.img");
    let _ = Command::new("pkill")
        .arg("-f")
        .arg(format!("qemu-system-aarch64.*{}", esp.display()))
        .status();
    drop(console);
    let _ = qemu.wait();

    let text = log().replace('\r', "");
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
}
