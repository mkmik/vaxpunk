//! cargo test -p boot: boots without a terminal and types commands at the
//! console's DCL prompt, a phase at a time. Each phase types its steps,
//! each once a line has come so many times, the echo of what it typed
//! before included, then waits for the lines it must have printed, all
//! within its deadline; one that runs out says which phase stalled, on
//! what, and the log's last line. DCL reads what was typed ahead a line at
//! a time. Once this QEMU is gone, ods-image checks the data disk's volume
//! and finds the files there, WELCOME.TXT twice in the one APPEND added to,
//! and the files BACKUP restored as they were, from a save set whose
//! blocks' CRCs are right.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

use ods_image::{Conversion, Image, Mode, Severity};

/// A part of the session: once `after` is on so many lines of the log,
/// type `keys`, step by step, then wait for `lines`, all within `secs` of
/// the phase starting. `{TELNET}` in keys is TCPIP$TELNET's PID, from
/// SHOW SYSTEM's list.
struct Phase {
    name: &'static str,
    secs: u64,
    steps: &'static [(&'static str, usize, &'static str)],
    lines: &'static [&'static str],
}

/// The executive starts and mounts the system disk; SYSTARTUP_VMS.COM
/// mounts the ramdisk, MDA0:, and can't mount the data disk, DKB0:, made
/// afresh in out/check-datadisk.img, not the one you keep.
const BOOT: Phase = Phase {
    name: "boot",
    secs: 60,
    steps: &[],
    lines: &[
        "%EXEC-I-START",
        "%MOUNT-I-MOUNTED, VAXPUNK mounted on _DKA0:",
        "%MOUNT-I-MOUNTED, RAM mounted on _MDA0:",
        "%SYSTEM-W-NOHOMEBLK, Files-11 home block not found on volume",
    ],
};

/// SYSTEM's default, SYS$SYSROOT:[SYSMGR], and its first SET DEFAULT, to a
/// search list of the ramdisk and SYS$MANAGER, then back to SYS$MANAGER:. RUN STARTUP, whose processes
/// print theirs by the end, and SNOOP; then
/// DIRECTORY, TYPE, TYPE/HEAD, TYPE/TAIL and EDIT on the system disk,
/// DCLTEST.COM's and DCL's lines with symbols, a PIPE whose && skips and
/// || runs after a failure, those SYLOGIN.COM defined too, CLITEST's words as a
/// foreign command, EDIT's EXIT that can't write there, a logical name,
/// SHOW LOGICAL, SET DEFAULT HOME's SYS$SYSROOT:[SYSMGR] and SET DEFAULT
/// on the system disk, until SET DEFAULT [-] fails in [000000].
const SYSTEM_DISK: Phase = Phase {
    name: "system disk",
    secs: 30,
    steps: &[
        (
            "%EXEC-I-START",
            1,
            concat!(
                "SHOW DEFAULT\rDEFINE HOME MDA0:[000000],SYS$MANAGER\rSET DEFAULT HOME:\rSHOW DEFAULT\r",
                "SET DEFAULT SYS$MANAGER:\rDEASSIGN HOME\r",
            ),
        ),
        (
            "  =   DKA0:[SYSMGR]",
            1,
            concat!(
                "RUN STARTUP\rRUN SNOOP\rFOO\rDIR [SYSEXE]P%NG\rTYPE WELCOME.TXT\r",
                "TYPE/HEAD=3 WELCOME.TXT\rTYPE/TAIL=2 WELCOME.TXT\r",
                "@DCLTEST 3 \"Two words\"\r@DCLTEST FAIL\rSHOW SYMBOL $STATUS\r",
                "X = 6 * 7\rWRITE SYS$OUTPUT \"X is \", X\rSHOW SYMBOL HOME\rHOME\r",
                "PIPE P = \"PIPE: \" ; WRITE SYS$OUTPUT P, \"a;b\" ; TYPE NOSUCH.TXT && ",
                "WRITE SYS$OUTPUT P, \"two\" || WRITE SYS$OUTPUT P, \"three\"\r",
                "EDIT WELCOME.TXT\r\"index\"\r\"zzz\"\rEXIT\rQUIT\r",
                // More than the type-ahead buffer's 255: the console holds back the rest.
                "DEFINE FOO SYS$INPUT\rSHOW LOGICAL FOO\rSHOW LOGICAL\rDEASSIGN FOO\r",
                "SHOW LOGICAL FOO\rSHOW DEFAULT\rSET DEFAULT SYS$SYSDEVICE:[SYSMGR]\rSHOW DEFAULT\r",
                "SET DEFAULT [SYSEXE]\rSHOW DEFAULT\rDIR D*\r",
                "SET DEFAULT [NOSUCH]\rSHOW DEFAULT\rSET DEFAULT [-]\rSHOW DEFAULT\r",
                "DIR [.SYSMGR]W*\rSET DEFAULT [-]\r",
            ),
        ),
    ],
    lines: &[
        "$ SHOW DEFAULT\n  SYS$SYSROOT:[SYSMGR]\n  =   MDA0:[SYSMGR]\n  =   DKA0:[SYSMGR]\n",
        "\n  =   MDA0:[000000]\n  =   MDA0:[SYSMGR]\n  =   DKA0:[SYSMGR]\n",
        " \\FOO\\",
        "PING.EXE;1          PONG.EXE;1",
        "Total of 2 files.",
        "and the rest of what INITIALIZE made.",
        "CLITEST: foreign ONE \"Two\" 3",
        "CLITEST: foreign world",
        "DCLTEST: ok, 3 and 4",
        "%RMS-E-FNF, file not found",
        "  $STATUS == 268534418   Hex = 10018292  Octal = 02000301222",
        "X is 42",
        "PIPE: a;b",
        "PIPE: three",
        "  HOME == \"SET DEFAULT SYS$MANAGER:\"",
        "    9\tand DIRECTORY [000000]",
        "String was not found",
        "Unable to write the file, status 000182BA",
        "\"FOO\" = \"SYS$INPUT\" (LNM$PROCESS_TABLE)",
        "no translation for logical name FOO",
        "(LNM$SYSTEM_TABLE)",
        "  \"SYS$ERROR\" = \"_OPA0:\"",
        "  \"SYS$SYSTEM\" = \"SYS$SYSROOT:[SYSEXE]\"",
        "  \"SYS$SYSROOT\" = \"SYS$SPECIFIC:\"\n        = \"SYS$COMMON:\"",
        "  SYS$SYSROOT:[SYSMGR]\n  =   MDA0:[SYSMGR]\n  =   DKA0:[SYSMGR]\n",
        "  DKA0:[SYSMGR]",
        "  DKA0:[SYSEXE]",
        "DCL.EXE;1           DELETE.EXE;1        DIRECTORY.EXE;1",
        "%DCL-I-INVDEF, DKA0:[NOSUCH] does not exist",
        "  DKA0:[NOSUCH]",
        "  DKA0:[000000]",
        "Directory DKA0:[SYSMGR]",
        "WELCOME.TXT;1",
        "%RMS-F-DIR, error in directory name",
    ],
};

/// MDA0:, SYSTARTUP_VMS.COM's, made the default device, around a COPY/LOG to it from
/// SYS$SYSDEVICE:, which prompts for its parameters, an EDIT in line mode
/// and keypad mode that writes a second version, DIRECTORY, a DELETE/LOG,
/// CLITEST's checks of the command parser, a qualifier DIRECTORY doesn't
/// have, HELP SHOW, a logical name in the system table, and a line edited
/// and one recalled with the up arrow. Then a search list, HOME, of the
/// ramdisk and SYS$MANAGER: SET DEFAULT to it, which keeps the default
/// directory, a COPY that makes the file in the first, a DIRECTORY that
/// finds it in both, a TYPE that finds a file in the second, SET DEFAULT
/// to it without a colon, a DIRECTORY of a search list with one inside it,
/// a TYPE of a logical name that is a file's specification, as on
/// OpenVMS, and a COPY to SYS$MANAGER:, which SYS$SYSROOT puts in the
/// ramdisk's [SYSMGR], and a DIRECTORY that finds it there and on DKA0:.
const RAMDISK: Phase = Phase {
    name: "ramdisk",
    secs: 30,
    steps: &[
        (
            "error in directory name",
            1,
            concat!(
                "SET DEFAULT MDA0:[000000]\rSHOW DEFAULT\r",
                "COPY/LOG\rSYS$SYSDEVICE:[SYSMGR]WELCOME.TXT\rRAM.TXT\r",
                "EDIT RAM.TXT\rD 3:END\rI\rEdited with EDT.\r\x1a",
                // Keypad mode: GOLD 5 goes to the top, GOLD PF3 finds OpenVMS,
                // seven DEL Cs delete it, and EDT-edited goes in its place.
                "C\r\x1bOP\x1bOu\x1bOP\x1bOROpenVMS\r",
                "\x1bOl\x1bOl\x1bOl\x1bOl\x1bOl\x1bOl\x1bOl",
                "EDT-edited\x1aEXIT\r",
                "TYPE RAM.TXT\r",
            ),
        ),
        (
            "MDA0:[000000]RAM.TXT;2",
            1,
            concat!(
                "DIR *.TXT\rDELETE/LOG RAM.TXT;1\rDELETE RAM.TXT;2\rDIR *.TXT\r",
                "RUN CLITEST\rDIR/BRIEFLY\rHELP SHOW\r",
                "DEFINE/SYSTEM/NOLOG ZZZ YYY\rSHOW LOGICAL/SYSTEM ZZZ\r",
                // Line editing: the up arrow recalls DEFINE, three DELs and TWO
                // change it, and the arrows, CTRL/H and CTRL/E make SHOW LOGICAL ZOO.
                "DEFINE ZOO ONE\r\x1b[A\x7f\x7f\x7fTWO\r",
                "LOGICAL OO\x1b[D\x1b[DZ\x08SHOW \x05\r",
            ),
        ),
        (
            "\"ZOO\" = \"TWO\" (LNM$PROCESS_TABLE)",
            1,
            concat!(
                "DEFINE HOME MDA0:[000000],SYS$MANAGER\rSHOW LOGICAL HOME\r",
                "SET DEFAULT HOME:\rSHOW DEFAULT\r",
                "COPY SYS$SYSDEVICE:[SYSMGR]WELCOME.TXT WELCOME.TXT\rDIR WELCOME.TXT\r",
            ),
        ),
        (
            "Grand total of 2 directories",
            1,
            concat!(
                "TYPE DCLTEST.CLD\rSET DEFAULT HOME\rSHOW DEFAULT\r",
                "DEFINE INNER SYS$MANAGER,MDA0:[000000]\r",
                "DEFINE OUTER SYS$SYSDEVICE:[SYSEXE],INNER\rDIR OUTER:WELCOME.TXT\r",
            ),
        ),
        (
            "Grand total of 2 directories",
            2,
            concat!(
                "DEFINE F MDA0:[000000]WELCOME.TXT\rTYPE/HEAD=2 F\r",
                "SET DEFAULT MDA0:[000000]\rDELETE WELCOME.TXT;1\r",
                "COPY/LOG SYS$MANAGER:WELCOME.TXT SYS$MANAGER:WELCOME.TXT\r",
                "DIR SYS$MANAGER:WELCOME.TXT\rDELETE SYS$SPECIFIC:[SYSMGR]WELCOME.TXT;1\r",
                "DEASSIGN HOME\rSHOW LOGICAL HOME\r",
            ),
        ),
    ],
    lines: &[
        "  MDA0:[000000]",
        "_From: SYS$SYSDEVICE:[SYSMGR]WELCOME.TXT",
        "_To: RAM.TXT",
        "%COPY-S-COPIED, DKA0:[SYSMGR]WELCOME.TXT;1 copied to MDA0:[000000]RAM.TXT;1 (10 records)",
        "        Welcome to vaxpunk, an EDT-edited clone for arm64\n",
        "Directory MDA0:[000000]",
        "RAM.TXT;2           RAM.TXT;1",
        "%DELETE-I-FILDEL, MDA0:[000000]RAM.TXT;1 deleted",
        "%DIRECT-W-NOFILES, no files found",
        "CLITEST: ok",
        "%DCL-W-IVQUAL, unrecognized qualifier - check validity, spelling, and placement",
        " \\BRIEFLY\\",
        "  SHOW LOGICAL [logical_name]",
        "    /[NO]MOUNTED",
        "   \"ZZZ\" = \"YYY\" (LNM$SYSTEM_TABLE)",
        "\"ZOO\" = \"TWO\" (LNM$PROCESS_TABLE)",
        "   \"HOME\" = \"MDA0:[000000]\" (LNM$PROCESS_TABLE)\n        = \"SYS$MANAGER\"",
        concat!(
            "Directory MDA0:[000000]\n\nWELCOME.TXT;1\n\nTotal of 1 file.\n\n",
            "Directory DKA0:[SYSMGR]\n\nWELCOME.TXT;1\n\nTotal of 1 file.\n\n",
            "Grand total of 2 directories, 2 files.",
        ),
        "! DCLTEST.CLD: GREET, a verb DCLTEST.COM adds with SET COMMAND.",
        concat!(
            "$ SET DEFAULT HOME\n$ SHOW DEFAULT\n  HOME:[000000]\n  =   MDA0:[000000]\n",
            "  =   MDA0:[SYSMGR]\n  =   DKA0:[SYSMGR]\n",
        ),
        concat!(
            "Directory DKA0:[SYSMGR]\n\nWELCOME.TXT;1\n\nTotal of 1 file.\n\n",
            "Directory MDA0:[000000]\n\nWELCOME.TXT;1\n\nTotal of 1 file.\n\n",
            "Grand total of 2 directories, 2 files.",
        ),
        "$ TYPE/HEAD=2 F\n\n        Welcome to vaxpunk",
        "%COPY-S-COPIED, DKA0:[SYSMGR]WELCOME.TXT;1 copied to MDA0:[SYSMGR]WELCOME.TXT;1 (10 records)",
        concat!(
            "Directory MDA0:[SYSMGR]\n\nWELCOME.TXT;1\n\nTotal of 1 file.\n\n",
            "Directory DKA0:[SYSMGR]\n\nWELCOME.TXT;1\n\nTotal of 1 file.\n\n",
            "Grand total of 2 directories, 2 files.",
        ),
        "no translation for logical name HOME",
    ],
};

/// INITIALIZE, MOUNT and COPY on DKB0:, a CREATE/DIRECTORY two levels deep
/// there and a COPY into it, a CREATE/LOG of a text file there from two
/// lines and CTRL/Z, an APPEND/LOG to the file copied there and
/// one to the system disk that fails, SHOW DEVICES, a DISMOUNT of DKA0:
/// that fails and one of DKB0: that doesn't, a DIRECTORY there that fails,
/// a MOUNT without a label, a DIRECTORY of the new directory and a DELETE
/// of the one above it, which has a file in it and stays, SHOW PROCESS and
/// SHOW SYSTEM.
const DATA_DISK: Phase = Phase {
    name: "data disk",
    secs: 20,
    steps: &[
        // Once the ramdisk's last command has run: the type-ahead buffer is empty.
        (
            "no translation for logical name HOME",
            1,
            concat!(
                "INIT/PROTECTION=(S:RWED,O:RWED,G:RWED,W:RWED) DKB0: DATA\rMOUNT DKB0: DATA\r",
                "COPY SYS$SYSDEVICE:[SYSMGR]WELCOME.TXT DKB0:[000000]DATA.TXT\r",
                "CREATE/DIRECTORY DKB0:[SUB.DEEP]\r",
                "COPY SYS$SYSDEVICE:[SYSMGR]WELCOME.TXT DKB0:[SUB.DEEP]DEEP.TXT\rDIR DKB0:[SUB]\r",
            ),
        ),
        (
            "Directory DKB0:[SUB]",
            1,
            concat!(
                "CREATE/LOG DKB0:[SUB.DEEP]NOTE.TXT\rFirst line\rSecond line\r\x1a",
                "APPEND/LOG SYS$SYSDEVICE:[SYSMGR]WELCOME.TXT DKB0:[000000]DATA.TXT\r",
                "APPEND DKB0:[000000]DATA.TXT SYS$SYSDEVICE:[SYSMGR]WELCOME.TXT\r",
            ),
        ),
        (
            "%RMS-E-WLK",
            1,
            concat!(
                "DIR DKB0:[000000]\rSHOW DEVICES\rDISMOUNT DKA0:\rDISMOUNT DKB0:\r",
                "DIR DKB0:[000000]\rMOUNT DKB0:\rDIR DKB0:[SUB.DEEP]\rDELETE DKB0:[000000]SUB.DIR;1\r",
                "SHOW PROCESS\rSHOW SYSTEM\r",
            ),
        ),
    ],
    lines: &[
        "%MOUNT-I-MOUNTED, DATA mounted on _DKB0:",
        "Directory DKB0:[SUB]",
        "DEEP.DIR;1",
        "%CREATE-I-CREATED, DKB0:[SUB.DEEP]NOTE.TXT;1 created",
        "%APPEND-S-APPENDED, DKA0:[SYSMGR]WELCOME.TXT;1 appended to DKB0:[000000]DATA.TXT;1 (10 records)",
        "%RMS-E-WLK, device currently write locked",
        "Directory DKB0:[000000]",
        "DATA.TXT;1",
        "SUB.DIR;1",
        "DKA0:                   Mounted wrtlck       0  VAXPUNK",
        "DKB0:                   Mounted              0  DATA",
        "MDA0:                   Mounted              0  RAM",
        "OPA0:                   Online               0",
        "%SYSTEM-F-DEVACTIVE, device is active",
        "%RMS-E-DNR, device not ready, not mounted, or unavailable",
        "Directory DKB0:[SUB.DEEP]",
        "DEEP.TXT;1",
        "NOTE.TXT;1",
        "%RMS-E-MKD, ACP could not mark file for deletion",
        "Process name:       \"SYSTEM\"",
        "UIC:                [1,4]",
        "00010001 SWAPPER",
        "SYSTEM          CUR     4 SHOW.EXE",
    ],
};

/// A CONTINUE with nothing stopped, a CTRL/C for CTRLC's AST, which
/// cancels its read, a CTRL/Y at the prompt, and stops SPIN, first with a
/// CTRL/C no AST takes, and SLEEPER with CTRL/Y, twice each, with a
/// CONTINUE in between, and EDIT while it reads, whose read CTRL/Y ends, so
/// that DCL reads SHOW DEFAULT and CONTINUE, and EDIT takes the empty line
/// as RETURN. A CTRL/Y and STOP end EDIT, and $STATUS shows SS$_ABORT;
/// STOP NOSUCH fails; and SLEEPER, stopped with CTRL/Y, exits with EXIT 3.
/// After SET NOCONTROL=Y, CTRL/Y and CTRL/C at the prompt do nothing, and
/// after SET CONTROL CTRL/Y interrupts again.
const CONTROL_KEYS: Phase = Phase {
    name: "control keys",
    secs: 60,
    steps: &[
        (
            "SYSTEM          CUR     4 SHOW.EXE",
            1,
            "CONTINUE\rRUN CTRLC\r",
        ),
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
    ],
    lines: &[
        "*CANCEL*",
        "CTRLC: ok",
        "  INITIALIZE device label",
        "    2\t        Welcome to vaxpunk, an OpenVMS clone for arm64",
        "$STATUS == 268435500   Hex = 1000002C",
        "%SYSTEM-W-NONEXPR, nonexistent process",
        "$STATUS == 3   Hex = 00000003",
        "Y is on",
    ],
};

/// STOP/IDENTIFICATION deletes TCPIP$TELNET, by the PID SHOW SYSTEM
/// listed, and then fails, as does a PID that isn't hex.
const STOP_ID: Phase = Phase {
    name: "stop/id",
    secs: 20,
    steps: &[(
        "*INTERRUPT*",
        9,
        "STOP/IDENTIFICATION={TELNET}\rSTOP/ID={TELNET}\rSHOW SYMBOL $STATUS\rSTOP/ID=XYZ\r",
    )],
    lines: &[
        "$STATUS == 268437736   Hex = 100008E8",
        "%DCL-W-IVCHAR, invalid numeric value - check for invalid digits",
    ],
};

/// SET PROCESS/PRIVILEGES takes CMKRNL and SYSNAM away, which SHOW
/// PROCESS/PRIVILEGES shows, so SHOW LOGICAL, which needs $CMKRNL, and
/// DEFINE/SYSTEM fail with NOPRIV, a privilege it doesn't know fails, and
/// ALL gives them back.
const PRIVILEGES: Phase = Phase {
    name: "privileges",
    secs: 20,
    steps: &[(
        "%DCL-W-IVCHAR",
        1,
        concat!(
            "SET PROCESS/PRIVILEGES=(NOCMKRNL,NOSYSNAM)\rSHOW PROCESS/PRIVILEGES\r",
            "SHOW LOGICAL\rDEFINE/SYSTEM/NOLOG QQQ RRR\rSET PROCESS/PRIV=XYZZY\r",
            "SET PROCESS/PRIVILEGES=ALL\rDEFINE/SYSTEM/NOLOG QQQ RRR\r",
            "SHOW LOGICAL/SYSTEM QQQ\r",
        ),
    )],
    lines: &[
        "Authorized privileges:\n CMKRNL       CMEXEC       SYSNAM       GRPNAM",
        "Process privileges:\n CMEXEC       GRPNAM       ALLSPOOL     DETACH",
        "%SYSTEM-F-NOPRIV, insufficient privilege or object protection violation",
        " \\XYZZY\\",
        "   \"QQQ\" = \"RRR\" (LNM$SYSTEM_TABLE)",
    ],
};

/// PROTTEST's [200,1] process can't read DKB0:[000000]DATA.TXT; after SET
/// PROTECTION=(W:R) it can, and after SET FILE/OWNER_UIC=[200,1] and SET
/// PROTECTION=(W) too, as the owner, which DIRECTORY/OWNER/PROTECTION shows
/// each time, but it can't once DKB0: is mounted /PROTECTION=(W), whose
/// world may do nothing; PROTTEST's own $CREATE gives a file an owner and
/// protection with a protection XAB. A COPY of a new version of a file SET
/// PROTECTION=(W:RE) has that protection too.
const PROTECTION: Phase = Phase {
    name: "protection",
    secs: 20,
    steps: &[
        (
            "\"QQQ\" = \"RRR\" (LNM$SYSTEM_TABLE)",
            1,
            concat!(
                "RUN PROTTEST\rSET PROTECTION=(W:R) DKB0:[000000]DATA.TXT\rRUN PROTTEST\r",
                "DIRECTORY/OWNER/PROTECTION DKB0:[000000]DATA.TXT\r",
            ),
        ),
        (
            "(RWED,RWED,RE,R)",
            1,
            concat!(
                "SET FILE/OWNER_UIC=[200,1] DKB0:[000000]DATA.TXT\r",
                "SET PROTECTION=(W) DKB0:[000000]DATA.TXT\r",
                "DIRECTORY/OWNER/PROTECTION DKB0:[000000]DATA.TXT\rRUN PROTTEST\r",
            ),
        ),
        (
            "PROTTEST: [200,1] read",
            2,
            "DISMOUNT DKB0:\rMOUNT/PROTECTION=(W) DKB0:\rRUN PROTTEST\r",
        ),
        // A new version has its predecessor's protection.
        (
            "%RMS-E-PRV",
            2,
            concat!(
                "COPY SYS$SYSDEVICE:[SYSMGR]WELCOME.TXT DKB0:[000000]NEW.TXT\r",
                "SET PROTECTION=(W:RE) DKB0:[000000]NEW.TXT\r",
                "COPY SYS$SYSDEVICE:[SYSMGR]WELCOME.TXT DKB0:[000000]NEW.TXT\r",
                "DIRECTORY/PROTECTION DKB0:[000000]NEW.TXT;2\r",
            ),
        ),
    ],
    lines: &[
        "%RMS-E-PRV, insufficient privilege or file protection violation",
        "PROTTEST: [200,1] read DATA.TXT, and may not give it away, delete it or make a file there",
        "DATA.TXT;1          [1,4]               (RWED,RWED,RE,R)",
        "DATA.TXT;1          [200,1]             (RWED,RWED,RE,)",
        "NEW.TXT;2           (RWED,RWED,RE,RE)",
    ],
};

/// SET TERMINAL changes the console's width, page, BROADCAST and PASTHRU,
/// which SHOW TERMINAL shows, refuses a width of 600, and after /NOECHO
/// the command typed next isn't echoed; then it puts them back. TTTEST
/// checks timed reads, the type-ahead count of the keys typed for it, a
/// read an escape sequence ends and one with terminators of its own.
const TERMINAL: Phase = Phase {
    name: "terminal",
    secs: 20,
    steps: &[
        (
            "NEW.TXT;2           (RWED,RWED,RE,RE)",
            1,
            concat!(
                "SET TERMINAL/WIDTH=132/PAGE=48/NOBROADCAST/PASTHRU\rSHOW TERMINAL\r",
                "SET TERMINAL/WIDTH=600\rSET TERMINAL/NOECHO\r",
                "WRITE SYS$OUTPUT \"QUIET\", \"LY\"\r",
            ),
        ),
        (
            "QUIETLY",
            1,
            "SET TERMINAL/ECHO/WIDTH=80/PAGE=24/BROADCAST/NOPASTHRU OPA0:\rSHOW TERMINAL\r",
        ),
        (
            "   Broadcast          No Readsync        No Form            Fulldup",
            1,
            "RUN TTTEST\r",
        ),
        ("TTTEST: keys", 1, "ab\x1b[Ahellox"),
    ],
    lines: &[
        "Terminal: _OPA0:      Device_Type: VT100         Owner: SYSTEM",
        "   Input:    9600     LFfill:  0      Width: 132      Parity: None",
        "   Output:   9600     CRfill:  0      Page:   48",
        "   No Broadcast       No Readsync        No Form            Fulldup",
        "   No Dialup          No Secure server   No Disconnect      Pasthru",
        "%SYSTEM-F-BADPARAM, bad parameter value",
        "   Input:    9600     LFfill:  0      Width:  80      Parity: None",
        "   Interactive        Echo               Type_ahead         No Escape",
        "   Broadcast          No Readsync        No Form            Fulldup",
        "TTTEST: ok",
    ],
};

/// BACKUP saves DKB0:[000000]'s text files in a save set, lists it, and
/// restores them into [RESTORED], each with its version, owner and
/// protection: DATA.TXT is [200,1]'s now, with (W).
const BACKUP: Phase = Phase {
    name: "backup",
    secs: 20,
    steps: &[
        (
            "   Broadcast          No Readsync        No Form            Fulldup",
            1,
            concat!(
                "BACKUP/LOG DKB0:[000000]*.TXT DKB0:[000000]TXT.BCK/SAVE_SET\r",
                "BACKUP/LIST DKB0:[000000]TXT.BCK/SAVE_SET\r",
            ),
        ),
        (
            "End of save set",
            1,
            concat!(
                "CREATE/DIRECTORY DKB0:[RESTORED]\r",
                "BACKUP/LOG DKB0:[000000]TXT.BCK/SAVE_SET DKB0:[RESTORED]\r",
                "DIRECTORY/OWNER/PROTECTION DKB0:[RESTORED]\r",
            ),
        ),
    ],
    lines: &[
        "%BACKUP-S-COPIED, copied DKB0:[000000]DATA.TXT;1",
        "%BACKUP-S-COPIED, copied DKB0:[000000]NEW.TXT;2",
        "Command:           BACKUP/LOG DKB0:[000000]*.TXT DKB0:[000000]TXT.BCK/SAVE_SET",
        "[000000]NEW.TXT;1",
        "Total of 3 files",
        "%BACKUP-S-CREATED, created DKB0:[RESTORED]NEW.TXT;1",
        "Directory DKB0:[RESTORED]",
        "DATA.TXT;1          [200,1]             (RWED,RWED,RE,)",
        "NEW.TXT;2           [1,4]               (RWED,RWED,RE,RE)",
    ],
};

/// RMSTEST in a directory of its own on DKB0:, whose output must be
/// OpenVMS's, vms/sysexe/rmstest.out (checked after the session).
const RMS: Phase = Phase {
    name: "rms",
    secs: 60,
    steps: &[(
        "NEW.TXT;2           [1,4]               (RWED,RWED,RE,RE)",
        1,
        "CREATE/DIRECTORY DKB0:[RMS]\rSET DEFAULT DKB0:[RMS]\rRUN RMSTEST\rSET DEFAULT DKB0:[000000]\r",
    )],
    lines: &["RMSTEST done"],
};

/// The processes STARTUP and SNOOP ran, which print as they go, done long
/// before: STARTUP's SLEEPER and SVCTEST's NAPPER say they hibernate
/// before SLEEPER does, and the CPU then idles, taking clock interrupts.
const STARTUP: Phase = Phase {
    name: "startup",
    secs: 10,
    steps: &[],
    lines: &[
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
    ],
};

const PHASES: &[Phase] = &[
    BOOT,
    SYSTEM_DISK,
    RAMDISK,
    DATA_DISK,
    CONTROL_KEYS,
    STOP_ID,
    PRIVILEGES,
    PROTECTION,
    TERMINAL,
    BACKUP,
    RMS,
    STARTUP,
];

/// What the log must not hold: the lines of DCLTEST.COM's a failure skips
/// or reaches, FSTEST's when two processes in the file system got in each
/// other's way, and CHFTEST's when a check failed.
const ABSENT: &[&str] = &[
    "DCLTEST: not here",
    "DCLTEST: failed",
    // PIPE's && after TYPE fails.
    "PIPE: two",
    "FSTEST: a count changed",
    "CHFTEST: exited",
    // SET TERMINAL/NOECHO's: the command after it isn't echoed.
    "SYS$OUTPUT \"QUIET\"",
    // RUN SNOOP's ACCVIO is written once, not again by DCL.
    "%NONAME-F-NOMSG, Message number 0000000C",
];

/// The boot binary, killed with its QEMU when dropped, on a panic too.
struct Qemu(Child);

impl Drop for Qemu {
    fn drop(&mut self) {
        // boot became run-qemu.sh, QEMU's parent: this QEMU, not another on out/.
        let _ = Command::new("pkill")
            .arg("-P")
            .arg(self.0.id().to_string())
            .status();
        let _ = self.0.wait();
    }
}

/// TCPIP$TELNET's PID, from SHOW SYSTEM's list.
fn telnet_pid(text: &str) -> &str {
    text.lines()
        .map(|l| l.split_whitespace().collect::<Vec<_>>())
        .find(|w| {
            w.len() > 1
                && w[1] == "TCPIP$TELNET"
                && w[0].len() == 8
                && w[0].chars().all(|c| c.is_ascii_hexdigit())
        })
        .expect("TCPIP$TELNET in SHOW SYSTEM")[0]
}

/// Runs the phases in order; Err says where it stalled.
fn session(log: impl Fn() -> String, mut type_: impl FnMut(&str)) -> Result<(), String> {
    for phase in PHASES {
        let start = Instant::now();
        let mut step = 0;
        loop {
            let text = log();
            if let Some(&(line, times, keys)) = phase.steps.get(step)
                && text.lines().filter(|l| l.contains(line)).count() >= times
            {
                if keys.contains("{TELNET}") {
                    type_(&keys.replace("{TELNET}", telnet_pid(&text)));
                } else {
                    type_(keys);
                }
                step += 1;
                continue;
            }
            let missing: Vec<_> = phase.lines.iter().filter(|l| !text.contains(*l)).collect();
            if step == phase.steps.len() && missing.is_empty() {
                println!("{}: {:?}", phase.name, start.elapsed());
                break;
            }
            if start.elapsed() > Duration::from_secs(phase.secs) || text.contains("root task done")
            {
                let waiting = match phase.steps.get(step) {
                    Some((line, times, _)) => format!("{line:?} {times} times"),
                    None => format!("{missing:?}"),
                };
                // The last line that isn't empty or DCL's prompt.
                let last = text.lines().rfind(|l| !matches!(l.trim(), "" | "$"));
                return Err(format!(
                    "stalled at phase {} after line {:?}, waiting for {waiting}",
                    phase.name,
                    last.unwrap_or_default()
                ));
            }
            sleep(Duration::from_secs(1));
        }
    }
    Ok(())
}

#[test]
fn boot() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let log_path = root.join("out/serial.log");
    let _ = fs::remove_file(&log_path);
    let datadisk = root.join("out/check-datadisk.img");
    let _ = fs::remove_file(&datadisk);
    let mut qemu = Qemu(
        Command::new(env!("CARGO_BIN_EXE_boot"))
            .env("DATADISK", &datadisk)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let mut console = qemu.0.stdin.take().unwrap();
    let log =
        || String::from_utf8_lossy(&fs::read(&log_path).unwrap_or_default()).replace('\r', "");

    let result = session(log, |s| console.write_all(s.as_bytes()).unwrap());
    drop(console);
    drop(qemu);

    let text = log();
    if let Some(start) = text.find("EXEC.EXE:") {
        let start = text[..start].rfind('\n').map_or(0, |i| i + 1);
        print!("{}", &text[start..]);
    }
    if let Err(e) = result {
        panic!("{e}");
    }
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
    assert_eq!(
        text.matches("This file is SYS$MANAGER:WELCOME.TXT").count(),
        1,
        "TYPE WELCOME.TXT, and not TYPE/HEAD=3's 4th line"
    );
    assert_eq!(
        (
            text.matches("DIRECTORY lists the files in").count(),
            text.matches("and DIRECTORY [000000] the volume").count()
        ),
        (1, 3),
        "TYPE/TAIL=2 WELCOME.TXT: its 9th line, not its 8th"
    );
    assert!(
        !text.contains("SPIN: a register changed"),
        "SPIN's registers changed"
    );

    let want = fs::read_to_string(root.join("vms/sysexe/rmstest.out")).unwrap();
    let first = want.lines().next().unwrap();
    let got: Vec<_> = text[text.find(first).expect("RMSTEST's output")..]
        .lines()
        .take(want.lines().count())
        .collect();
    for (n, (w, g)) in want.lines().zip(&got).enumerate() {
        assert_eq!(w, *g, "RMSTEST's line {} isn't OpenVMS's", n + 1);
    }

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
    assert_eq!(data.matches("Welcome to vaxpunk",).count(), 2, "{data}");
    assert!(img.lookup("[SUB.DEEP]DEEP.TXT").is_ok());
    let fid = img.lookup("[SUB.DEEP]NOTE.TXT").unwrap();
    let mut note = Vec::new();
    img.copy_out(fid, &mut note, Conversion::RecordsToLines)
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&note), "First line\nSecond line\n");
    for name in ["DATA.TXT;1", "NEW.TXT;1", "NEW.TXT;2"] {
        let saved = img.lookup(&format!("[000000]{name}")).unwrap();
        let restored = img.lookup(&format!("[RESTORED]{name}")).unwrap();
        let (saved, restored) = (bytes(&mut img, saved), bytes(&mut img, restored));
        assert_eq!(saved, restored, "[RESTORED]{name}");
    }
    for file in ["[RMS]REL.REL", "[RMS]RELF.REL"] {
        let fid = img.lookup(file).unwrap();
        let report = img.check_file(fid).unwrap();
        assert!(report.is_sound(), "{file}: {:?}", report.findings);
    }
    let fid = img.lookup("[000000]TXT.BCK").unwrap();
    save_set(&bytes(&mut img, fid));
}

/// A file's bytes, up to its end of file.
fn bytes(img: &mut Image, fid: ods_image::Fid) -> Vec<u8> {
    let mut data = Vec::new();
    img.copy_out(fid, &mut data, Conversion::Binary).unwrap();
    data
}

/// Checks a save set's blocks as VMS's BACKUP does: each numbered, its
/// header's CRC-16 and the block's CRC-32 right, with both 0 for them.
fn save_set(data: &[u8]) {
    fn crc(bytes: &[u8], mut crc: u32, poly: u32) -> u32 {
        for b in bytes {
            crc ^= *b as u32;
            for _ in 0..8 {
                crc = if crc & 1 == 1 {
                    (crc >> 1) ^ poly
                } else {
                    crc >> 1
                };
            }
        }
        crc
    }
    assert!(
        !data.is_empty() && data.len().is_multiple_of(32256),
        "{} bytes",
        data.len()
    );
    for (n, block) in data.chunks(32256).enumerate() {
        let mut b = block.to_vec();
        let number = u32::from_le_bytes(b[8..12].try_into().unwrap());
        let block_crc = u32::from_le_bytes(b[36..40].try_into().unwrap());
        let header_crc = u16::from_le_bytes([b[254], b[255]]) as u32;
        b[36..40].fill(0);
        b[254..256].fill(0);
        assert_eq!(number as usize, n + 1);
        assert_eq!(
            header_crc,
            crc(&b[..256], 0, 0xA001),
            "block {number}'s header CRC"
        );
        assert_eq!(block_crc, !crc(&b, !0, 0xEDB88320), "block {number}'s CRC");
    }
}
