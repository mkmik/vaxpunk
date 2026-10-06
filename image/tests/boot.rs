//! cargo test -p boot: boots without a terminal, types commands at the
//! console's DCL prompt once the executive has started, and waits until the
//! processes have printed their last lines (roottask/sysexe/), DIRECTORY,
//! TYPE and EDIT theirs from the system disk, DCLTEST.COM's and DCL's
//! with symbols, those SYLOGIN.COM defined too, CLITEST's words as a
//! foreign command and as a verb SET COMMAND added, EDIT's EXIT that can't write
//! there, and DIRECTORY its own from the ramdisk, MDA0:, made the default
//! device, around a COPY/LOG to it from SYS$SYSDEVICE:, which prompts for
//! its parameters, a DELETE/LOG, CLITEST's checks of the command parser, a
//! qualifier DIRECTORY doesn't have, HELP SHOW, a logical name in the
//! system table, a logical name, an EDIT
//! in line mode and keypad mode that writes a second version and DELETEs,
//! a line edited and one recalled with the up arrow, then INITIALIZE,
//! MOUNT and COPY on the data disk, DKB0:, made afresh in
//! out/check-datadisk.img, not the one you keep, which SYSTARTUP_VMS.COM
//! couldn't mount at boot, a CREATE/DIRECTORY two levels deep there and a
//! COPY into it, an APPEND/LOG to the file copied there and one to the
//! system disk that fails, SHOW DEVICES, a DISMOUNT of DKA0: that fails and one of
//! DKB0: that doesn't, a DIRECTORY there that fails, a MOUNT without a
//! label, a DIRECTORY of the new directory and a DELETE of the one above
//! it, which has a file in it and stays, SHOW PROCESS
//! and SHOW SYSTEM, or the root task is done, which is only on a halt or a fault. DCL reads what was
//! typed ahead a line at a time. Then it types a CONTINUE with nothing stopped, a CTRL/C
//! for CTRLC's AST, which cancels its read, a CTRL/Y at the prompt, and stops SPIN,
//! first with a CTRL/C no AST takes, and SLEEPER with CTRL/Y, twice each, with a
//! CONTINUE in between, and
//! EDIT while it reads, whose read CTRL/Y ends, so that DCL reads SHOW
//! DEFAULT and CONTINUE, and EDIT takes the empty line as RETURN. A
//! CTRL/Y and STOP end EDIT, and $STATUS shows SS$_ABORT; STOP NOSUCH fails;
//! and SLEEPER, stopped with CTRL/Y, exits with EXIT 3. After SET
//! NOCONTROL=Y, CTRL/Y and CTRL/C at the prompt do nothing, and after SET
//! CONTROL CTRL/Y interrupts again. STOP/IDENTIFICATION deletes
//! TCPIP$TELNET, by the PID SHOW SYSTEM listed, and then fails, as does a
//! PID that isn't hex. SET PROCESS/PRIVILEGES takes CMKRNL and SYSNAM
//! away, which SHOW PROCESS/PRIVILEGES shows, so SHOW LOGICAL, which needs
//! $CMKRNL, and DEFINE/SYSTEM fail with NOPRIV, a privilege it doesn't
//! know fails, and ALL gives them back. PROTTEST's [200,1] process can't
//! read DKB0:[000000]DATA.TXT; after SET PROTECTION=(W:R) it can, and
//! after SET FILE/OWNER_UIC=[200,1] and SET PROTECTION=(W) too, as the
//! owner, which DIRECTORY/OWNER/PROTECTION shows each time, but it can't
//! once DKB0: is mounted /PROTECTION=(W), whose world may do nothing;
//! PROTTEST's own $CREATE gives a file an owner and protection with a
//! protection XAB. A COPY of a new version of a file SET PROTECTION=(W:RE)
//! has that protection too. All a
//! step at a time: each waits until a line has come so many times, the echo
//! of what it typed before included, and types.
//! STARTUP's SLEEPER and SVCTEST's NAPPER say they hibernate before SLEEPER
//! does. The CPU then idles, taking clock interrupts. Once this QEMU is
//! gone, ods-image checks the data disk's volume and finds the files there,
//! WELCOME.TXT twice in the one APPEND added to.

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
    "%SYSTEM-W-NOHOMEBLK, Files-11 home block not found on volume",
    "STARTUP: done",
    "SVCTEST: ok",
    "process NAPPER exited with status 0000217C",
    "process NOSUCH exited",
    "HOG: NUDGE ran",
    "TIMETEST: ok",
    "FSTEST: ok",
    "ASTTEST: ok",
    "MBXTEST: ok",
    "%SYSTEM-W-NOSIGNAL, no signal currently active",
    "%SYSTEM-F-NOSIGNAL, no signal currently active",
    "CHFTEST: ok",
    "%SYSTEM-F-ACCVIO, access violation, reason mask=00, virtual address=40010000, PC=",
    "%SYSTEM-F-OPCDEC, opcode reserved to DIGITAL fault at PC=",
    "process SNOOP exited with status 1000000C",
    "process USURP exited with status 1000043C",
    " \\FOO\\",
    "PING.EXE;1          PONG.EXE;1",
    "Total of 2 files.",
    "and the rest of what INITIALIZE made.",
    "    9\tand DIRECTORY [000000]",
    "String was not found",
    "Unable to write the file, status 000182BA",
    "\"FOO\" = \"SYS$INPUT\" (LNM$PROCESS_TABLE)",
    "no translation for logical name FOO",
    "\"ZOO\" = \"TWO\" (LNM$PROCESS_TABLE)",
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
    "_From: SYS$SYSDEVICE:[SYSMGR]WELCOME.TXT",
    "_To: RAM.TXT",
    "%COPY-S-COPIED, DKA0:[SYSMGR]WELCOME.TXT;1 copied to MDA0:[000000]RAM.TXT;1 (10 records)",
    "%DELETE-I-FILDEL, MDA0:[000000]RAM.TXT;1 deleted",
    "CLITEST: ok",
    "%DCL-W-IVQUAL, unrecognized qualifier - check validity, spelling, and placement",
    " \\BRIEFLY\\",
    "%MOUNT-I-MOUNTED, DATA mounted on _DKB0:",
    "Directory DKB0:[000000]",
    "DATA.TXT;1",
    "SUB.DIR;1",
    "Directory DKB0:[SUB]",
    "DEEP.DIR;1",
    "%APPEND-S-APPENDED, DKA0:[SYSMGR]WELCOME.TXT;1 appended to DKB0:[000000]DATA.TXT;1 (10 records)",
    "%RMS-E-WLK, device currently write locked",
    "%SYSTEM-F-DEVACTIVE, device is active",
    "%RMS-E-DNR, device not ready, not mounted, or unavailable",
    "Directory DKB0:[SUB.DEEP]",
    "DEEP.TXT;1",
    "%RMS-E-MKD, ACP could not mark file for deletion",
    "DKA0:                   Mounted wrtlck       0  VAXPUNK",
    "DKB0:                   Mounted              0  DATA",
    "MDA0:                   Mounted              0  RAM",
    "OPA0:                   Online               0",
    "Process name:       \"SYSTEM\"",
    "00010001 SWAPPER",
    "SYSTEM          CUR     4 SHOW.EXE",
    "  INITIALIZE device label",
    "  SHOW LOGICAL [logical_name]",
    "    /[NO]MOUNTED",
    "   \"ZZZ\" = \"YYY\" (LNM$SYSTEM_TABLE)",
    "*CANCEL*",
    "CTRLC: ok",
    "CLITEST: foreign ONE \"Two\" 3",
    "CLITEST: foreign world",
    "DCLTEST: ok, 3 and 4",
    "%RMS-E-FNF, file not found",
    "  $STATUS == 268534418   Hex = 10018292  Octal = 02000301222",
    "X is 42",
    "  HOME == \"SET DEFAULT SYS$MANAGER:\"",
    "    2\t        Welcome to vaxpunk, an OpenVMS clone for arm64",
    "$STATUS == 268435500   Hex = 1000002C",
    "%SYSTEM-W-NONEXPR, nonexistent process",
    "$STATUS == 3   Hex = 00000003",
    "Y is on",
    "$STATUS == 268437736   Hex = 100008E8",
    "%DCL-W-IVCHAR, invalid numeric value - check for invalid digits",
    "UIC:                [1,4]",
    "Authorized privileges:\n CMKRNL       CMEXEC       SYSNAM       GRPNAM",
    "Process privileges:\n CMEXEC       GRPNAM       ALLSPOOL     DETACH",
    "%SYSTEM-F-NOPRIV, insufficient privilege or object protection violation",
    " \\XYZZY\\",
    "   \"QQQ\" = \"RRR\" (LNM$SYSTEM_TABLE)",
    "%RMS-E-PRV, insufficient privilege or file protection violation",
    "PROTTEST: [200,1] read DATA.TXT, and may not give it away, delete it or make a file there",
    "DATA.TXT;1          [1,4]               (RWED,RWED,RE,R)",
    "DATA.TXT;1          [200,1]             (RWED,RWED,RE,)",
    "NEW.TXT;2           (RWED,RWED,RE,RE)",
];

/// What it must not: the lines of DCLTEST.COM's a failure skips or reaches,
/// FSTEST's when two processes in the file system got in each other's way,
/// and CHFTEST's when a check failed.
const ABSENT: &[&str] = &[
    "DCLTEST: not here",
    "DCLTEST: failed",
    "FSTEST: a count changed",
    "CHFTEST: exited",
    // RUN SNOOP's ACCVIO is written once, not again by DCL.
    "%NONAME-F-NOMSG, Message number 0000000C",
];

/// Once the ramdisk is done: when a line has come so many times, type.
const STEPS: &[(&str, usize, &str)] = &[
    ("CTRLC: waiting for CTRL/C", 1, "\x03"),
    ("CTRLC: ok", 1, "\x19"),
    ("*INTERRUPT*", 1, "RUN SPIN\r"),
    ("SPIN: spinning", 1, "\x03"),
    ("*INTERRUPT*", 2, "CONTINUE\r"),
    ("CONTINUE", 2, "\x19"),
    ("*INTERRUPT*", 3, "RUN SLEEPER\r"),
    ("SLEEPER: hibernating", 3, "\x19"),
    ("*INTERRUPT*", 4, "CONTINUE\r"),
    ("CONTINUE", 3, "\x19"),
    ("*INTERRUPT*", 5, "HELP\r"),
    (
        "HELP verb describes a verb",
        1,
        "EDIT SYS$MANAGER:WELCOME.TXT\r",
    ),
    ("    1\t", 3, "\x19"),
    ("*INTERRUPT*", 6, "SHOW DEFAULT\rCONTINUE\r"),
    (
        "    2\t        Welcome to vaxpunk, an OpenVMS clone for arm64",
        1,
        "\x19",
    ),
    (
        "*INTERRUPT*",
        7,
        "STOP\rSHOW SYMBOL $STATUS\rSTOP NOSUCH\rRUN SLEEPER\r",
    ),
    ("SLEEPER: hibernating", 4, "\x19"),
    ("*INTERRUPT*", 8, "EXIT 3\rSHOW SYMBOL $STATUS\r"),
    (
        "$STATUS == 3   Hex = 00000003",
        1,
        "SET NOCONTROL=Y\rWRITE SYS$OUTPUT \"Y is\", \" off\"\r",
    ),
    (
        "Y is off",
        1,
        "\x19\x03SET CONTROL\rWRITE SYS$OUTPUT \"Y is\", \" on\"\r",
    ),
    ("Y is on", 1, "\x19"),
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
            type_("@DCLTEST 3 \"Two words\"\r@DCLTEST FAIL\rSHOW SYMBOL $STATUS\r");
            type_("X = 6 * 7\rWRITE SYS$OUTPUT \"X is \", X\rSHOW SYMBOL HOME\rHOME\r");
            type_("EDIT WELCOME.TXT\r\"index\"\r\"zzz\"\rEXIT\rQUIT\r");
            // More than the type-ahead buffer's 255: the console holds back the rest.
            type_("DEFINE FOO SYS$INPUT\rSHOW LOGICAL FOO\rSHOW LOGICAL\rDEASSIGN FOO\r");
            type_("SHOW LOGICAL FOO\rSHOW DEFAULT\rSET DEFAULT [SYSEXE]\rSHOW DEFAULT\rDIR D*\r");
            type_("SET DEFAULT [NOSUCH]\rSHOW DEFAULT\rSET DEFAULT [-]\rSHOW DEFAULT\r");
            type_("DIR [.SYSMGR]W*\rSET DEFAULT [-]\r");
            typed = 2;
        } else if typed == 7 && step < STEPS.len() {
            let (line, times, keys) = STEPS[step];
            if text.lines().filter(|l| l.contains(line)).count() >= times {
                type_(keys);
                step += 1;
            }
        } else if typed == 7 && text.matches("*INTERRUPT*").count() == 9 {
            // TCPIP$TELNET, by its PID in SHOW SYSTEM's list.
            let pid = text
                .lines()
                .map(|l| l.split_whitespace().collect::<Vec<_>>())
                .find(|w| {
                    w.len() > 1
                        && w[1] == "TCPIP$TELNET"
                        && w[0].len() == 8
                        && w[0].chars().all(|c| c.is_ascii_hexdigit())
                })
                .unwrap()[0];
            type_(&format!("STOP/IDENTIFICATION={pid}\rSTOP/ID={pid}\r"));
            type_("SHOW SYMBOL $STATUS\rSTOP/ID=XYZ\r");
            typed = 8;
        } else if typed == 8 && text.contains("%DCL-W-IVCHAR") {
            type_("SET PROCESS/PRIVILEGES=(NOCMKRNL,NOSYSNAM)\rSHOW PROCESS/PRIVILEGES\r");
            type_("SHOW LOGICAL\rDEFINE/SYSTEM/NOLOG QQQ RRR\rSET PROCESS/PRIV=XYZZY\r");
            type_("SET PROCESS/PRIVILEGES=ALL\rDEFINE/SYSTEM/NOLOG QQQ RRR\r");
            type_("SHOW LOGICAL/SYSTEM QQQ\r");
            typed = 9;
        } else if typed == 9 && text.contains("\"QQQ\" = \"RRR\" (LNM$SYSTEM_TABLE)") {
            // File protection, from PROTTEST's process of another UIC.
            type_("RUN PROTTEST\rSET PROTECTION=(W:R) DKB0:[000000]DATA.TXT\rRUN PROTTEST\r");
            type_("DIRECTORY/OWNER/PROTECTION DKB0:[000000]DATA.TXT\r");
            typed = 10;
        } else if typed == 10 && text.contains("(RWED,RWED,RE,R)") {
            type_("SET FILE/OWNER_UIC=[200,1] DKB0:[000000]DATA.TXT\r");
            type_("SET PROTECTION=(W) DKB0:[000000]DATA.TXT\r");
            type_("DIRECTORY/OWNER/PROTECTION DKB0:[000000]DATA.TXT\rRUN PROTTEST\r");
            typed = 11;
        } else if typed == 11 && text.matches("PROTTEST: [200,1] read").count() == 2 {
            type_("DISMOUNT DKB0:\rMOUNT/PROTECTION=(W) DKB0:\rRUN PROTTEST\r");
            typed = 12;
        } else if typed == 12 && text.matches("%RMS-E-PRV").count() == 2 {
            // A new version has its predecessor's protection.
            type_("COPY SYS$SYSDEVICE:[SYSMGR]WELCOME.TXT DKB0:[000000]NEW.TXT\r");
            type_("SET PROTECTION=(W:RE) DKB0:[000000]NEW.TXT\r");
            type_("COPY SYS$SYSDEVICE:[SYSMGR]WELCOME.TXT DKB0:[000000]NEW.TXT\r");
            type_("DIRECTORY/PROTECTION DKB0:[000000]NEW.TXT;2\r");
            typed = 13;
        }
        // Then, once SET DEFAULT [-] has failed in [000000], the ramdisk.
        if typed == 2 && text.contains("error in directory name") {
            type_("INIT MDA0: RAM\rMOUNT MDA0: RAM\rSET DEFAULT MDA0:[000000]\rSHOW DEFAULT\r");
            type_("COPY/LOG\rSYS$SYSDEVICE:[SYSMGR]WELCOME.TXT\rRAM.TXT\r");
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
            type_("DIR *.TXT\rDELETE/LOG RAM.TXT;1\rDELETE RAM.TXT;2\rDIR *.TXT\r");
            type_("RUN CLITEST\rDIR/BRIEFLY\rHELP SHOW\r");
            type_("DEFINE/SYSTEM/NOLOG ZZZ YYY\rSHOW LOGICAL/SYSTEM ZZZ\r");
            // Line editing: the up arrow recalls DEFINE, three DELs and TWO
            // change it, and the arrows, CTRL/H and CTRL/E make SHOW LOGICAL ZOO.
            type_("DEFINE ZOO ONE\r\x1b[A\x7f\x7f\x7fTWO\r");
            type_("LOGICAL OO\x1b[D\x1b[DZ\x08SHOW \x05\r");
            typed = 4;
        }
        // Then the data disk.
        // Once step 3's last command has run: the type-ahead buffer is empty.
        if typed == 4 && text.contains("\"ZOO\" = \"TWO\" (LNM$PROCESS_TABLE)") {
            type_("INIT/PROTECTION=(S:RWED,O:RWED,G:RWED,W:RWED) DKB0: DATA\rMOUNT DKB0: DATA\r");
            type_("COPY SYS$SYSDEVICE:[SYSMGR]WELCOME.TXT DKB0:[000000]DATA.TXT\r");
            type_("CREATE/DIRECTORY DKB0:[SUB.DEEP]\r");
            type_(
                "COPY SYS$SYSDEVICE:[SYSMGR]WELCOME.TXT DKB0:[SUB.DEEP]DEEP.TXT\rDIR DKB0:[SUB]\r",
            );
            typed = 5;
        }
        if typed == 5 && text.contains("Directory DKB0:[SUB]") {
            type_("APPEND/LOG SYS$SYSDEVICE:[SYSMGR]WELCOME.TXT DKB0:[000000]DATA.TXT\r");
            type_("APPEND DKB0:[000000]DATA.TXT SYS$SYSDEVICE:[SYSMGR]WELCOME.TXT\r");
            typed = 6;
        }
        if typed == 6 && text.contains("%RMS-E-WLK") {
            type_("DIR DKB0:[000000]\rSHOW DEVICES\rDISMOUNT DKA0:\rDISMOUNT DKB0:\r");
            type_(
                "DIR DKB0:[000000]\rMOUNT DKB0:\rDIR DKB0:[SUB.DEEP]\rDELETE DKB0:[000000]SUB.DIR;1\r",
            );
            type_("SHOW PROCESS\rSHOW SYSTEM\rCONTINUE\rRUN CTRLC\r");
            typed = 7;
        }
        let done = text.matches("%RMS-E-PRV").count() == 2;
        if done && LINES.iter().all(|l| text.contains(l)) || text.contains("root task done") {
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
    let present: Vec<_> = ABSENT.iter().filter(|l| text.contains(*l)).collect();
    assert!(present.is_empty(), "{present:?}");
    assert_eq!(
        text.matches("%SYSTEM-F-NOPRIV").count(),
        2,
        "SHOW LOGICAL and DEFINE/SYSTEM without CMKRNL and SYSNAM"
    );
    assert_eq!(
        text.matches("%RMS-E-PRV").count(),
        2,
        "PROTTEST before SET PROTECTION=(W:R) and after MOUNT/PROTECTION=(W)"
    );
    assert_eq!(
        text.matches("PROTTEST: [200,1] read").count(),
        2,
        "PROTTEST by the world's access and by the owner's"
    );
    assert_eq!(
        text.matches("*INTERRUPT*").count(),
        9,
        "CTRL/Y or CTRL/C interrupted after SET NOCONTROL"
    );
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
    let data = String::from_utf8_lossy(&data);
    assert!(data.contains("and the rest of what INITIALIZE made."));
    assert_eq!(data.matches("Welcome to vaxpunk").count(), 2, "{data}");
    assert!(img.lookup("[SUB.DEEP]DEEP.TXT").is_ok());
}
