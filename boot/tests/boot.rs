//! cargo test -p boot: boots without a terminal, logs in at the console
//! and types commands at DCL's prompt, a phase at a time. Each phase types its steps,
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

/// The executive starts and mounts the system disk; STARTUP's
/// SYSTARTUP_VMS.COM mounts the ramdisk, MDA0:, starts OPCOM, which
/// enables the console as an operator terminal, and can't mount the data
/// disk, DKB0:, made afresh in out/check-datadisk.img, not the one you
/// keep, and defines SYS$ANNOUNCE. Then, at a key, LOGINOUT writes ANNOUNCE.TXT and asks for a username: one that isn't there fails,
/// SYSTEM with the wrong password too, and with the right one, in lower
/// case, logs in, with a failure to report.
const BOOT: Phase = Phase {
    name: "boot",
    secs: 60,
    steps: &[
        // The console waits for a key once STARTUP is gone.
        ("  SYSTEM       job terminated at ", 1, "\r"),
        ("Username:", 1, "NOBODY\r"),
        ("Password:", 1, "nobodys\r"),
        ("Username:", 2, "SYSTEM\r"),
        ("Password:", 2, "wrongpw\r"),
        ("Username:", 3, "system\r"),
        ("Password:", 3, "manager\r"),
        // SYLOGIN.COM's TTSIZE asks the console for its size.
        ("\x1b[6n", 1, "\x1b[24;80R"),
    ],
    lines: &[
        "%EXEC-I-START",
        "%MOUNT-I-MOUNTED, VAXPUNK mounted on _DKA0:",
        "%MOUNT-I-MOUNTED, RAM mounted on _MDA0:",
        "%RUN-S-PROC_ID, identification of created process is ",
        "%%%%%%%%%%%  OPCOM  ",
        "\nOperator _OPA0: has been enabled, username SYSTEM\n",
        "Operator status for operator _OPA0:\nCENTRAL, PRINTER, TAPES, DISKS, DEVICES, CARDS, NETWORK, CLUSTER, SECURITY,\n",
        "%SYSTEM-W-NOHOMEBLK, Files-11 home block not found on volume",
        "  SYSTEM       job terminated at ",
        "  vaxpunk, an OpenVMS clone for arm64. Log in as SYSTEM, password MANAGER\n\nUsername: NOBODY\nPassword: \nUser authorization failure\n",
        "\tWelcome to vaxpunk arm64 Operating System, Version V0.1\n\t1 failure since last successful login\n",
    ],
};

/// SYSTEM's default, SYS$SYSROOT:[SYSMGR], and its first SET DEFAULT, to a
/// search list of the ramdisk and SYS$MANAGER, then back to SYS$MANAGER:, and
/// STARTUP_P1 and STARTUP_P8 from F$GETSYI. RUN STARTUP, whose processes
/// print theirs by the end, and SNOOP; then
/// DIRECTORY, of LIBRTL.EXE in SYS$SHARE too, TYPE, TYPE/HEAD, TYPE/TAIL
/// and EDIT on the system disk,
/// LOGIN.COM's line, DCLTEST.COM's and DCL's lines with symbols, a PIPE whose && skips and
/// || runs after a failure, those SYLOGIN.COM defined too, CLITEST's words as a
/// foreign command, EDIT's EXIT that can't write there, a logical name,
/// SHOW LOGICAL, SET DEFAULT HOME's SYS$SYSROOT:[SYSMGR] and SET DEFAULT
/// on the system disk, until SET DEFAULT [-] fails in [000000].
const SYSTEM_DISK: Phase = Phase {
    name: "system disk",
    secs: 30,
    steps: &[
        (
            "1 failure since last successful login",
            1,
            concat!(
                "SHOW DEFAULT\rDEFINE HOME MDA0:[000000],SYS$MANAGER\rSET DEFAULT HOME:\rSHOW DEFAULT\r",
                "SET DEFAULT SYS$MANAGER:\rDEASSIGN HOME\r",
                "WRITE SYS$OUTPUT \"[\", F$GETSYI(\"STARTUP_P1\"), \"|\", F$GETSYI(\"STARTUP_P8\"), \"]\"\r",
            ),
        ),
        (
            "  =   DKA0:[SYSMGR]",
            1,
            concat!(
                "RUN STARTUP\rRUN SNOOP\rFOO\rDIR [SYSEXE]P%NG\rDIR SYS$SHARE:\rTYPE WELCOME.TXT\r",
                "TYPE/HEAD=3 WELCOME.TXT\rTYPE/TAIL=2 WELCOME.TXT\r",
                "@DCLTEST 3 \"Two words\"\r@DCLTEST FAIL\rSHOW SYMBOL $STATUS\r",
                "X = 6 * 7\rWRITE SYS$OUTPUT \"X is \", X\rSHOW SYMBOL HOME\rHOME\r",
                "PIPE P = \"PIPE: \" ; WRITE SYS$OUTPUT P, \"a;b\" ; TYPE NOSUCH.TXT && ",
                "WRITE SYS$OUTPUT P, \"two\" || WRITE SYS$OUTPUT P, \"three\"\r",
                "EDIT/EDT WELCOME.TXT\r\"index\"\r\"zzz\"\rEXIT\rQUIT\r",
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
        // boot()'s --p1 and --p8, blank-padded to 4 characters.
        "[TEST|8   ]",
        "\n  =   MDA0:[000000]\n  =   MDA0:[SYSMGR]\n  =   DKA0:[SYSMGR]\n",
        " \\FOO\\",
        "PING.EXE;1          PONG.EXE;1",
        "Total of 2 files.",
        // Apart: STARTUP's processes may print between them.
        "Directory DKA0:[SYSLIB]",
        "EVE$SECTION.TPU$SECTION;1",
        "LIBRTL.EXE;1",
        "and the rest of what INITIALIZE made.",
        "CLITEST: foreign ONE \"Two\" 3",
        "CLITEST: foreign world",
        "DCLTEST: ok, 3 and 4",
        // LOGIN.COM's, at a terminal, F$MODE() INTERACTIVE.
        "SYSTEM on _OPA0:, ",
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
        concat!(
            "  \"SYS$SYSROOT\" = \"SYS$SPECIFIC:\"\n        = \"SYS$COMMON:\"\n",
            "  \"SYS$SYSTEM\" = \"SYS$SYSROOT:[SYSEXE]\"\n",
        ),
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
/// have, HELP/NOPROMPT SHOW LOGICAL from HELPLIB.HLB, and with
/// HLP$LIBRARY PHONEHELP, HELP REJECT, which only it has, UAFHELP's
/// topics and the user libraries after them (not HELPLIB's, whose
/// CONTINUE the steps below count), HELP @PHONE ANSWER and HELP/OUTPUT to a
/// file, a logical name in the system table, and a line edited and one recalled with the up
/// arrow. Then a search list, HOME, of the ramdisk and SYS$MANAGER: SET
/// DEFAULT to it, which keeps the default
/// directory, a COPY that makes the file in the first, a DIRECTORY that
/// finds it in both, a TYPE that finds a file in the second, SET DEFAULT
/// to it without a colon, a DIRECTORY of a search list with one inside it,
/// a TYPE of a logical name that is a file's specification, as on
/// OpenVMS, and a COPY to SYS$MANAGER:, which SYS$SYSROOT puts in the
/// ramdisk's [SYSMGR], and a DIRECTORY that finds it there and on DKA0:.
/// Then logical name tables: CREATE/NAME_TABLE of MYTAB, twice, a name in
/// it that SHOW LOGICAL finds once the process has an LNM$FILE_DEV of its
/// own that lists MYTAB, the process's directory, a table under MYTAB
/// that goes when /ATTRIBUTES=SUPERSEDE replaces MYTAB, and DEASSIGN of
/// the tables.
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
                "EDIT/EDT RAM.TXT\rD 3:END\rI\rEdited with EDT.\r\x1a",
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
                "RUN CLITEST\rDIR/XYZZY\rHELP/NOPROMPT SHOW LOGICAL\r",
                "DEFINE HLP$LIBRARY SYS$HELP:PHONEHELP\rHELP/NOPROMPT REJECT\r",
                "HELP/NOPROMPT/NOINSTRUCTIONS/LIBRARY=UAFHELP\rHELP/NOPROMPT @PHONE ANSWER\r",
                "HELP/OUTPUT=HELP.LIS TYPE\rTYPE HELP.LIS\rDEASSIGN HLP$LIBRARY\r",
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
        (
            "no translation for logical name HOME",
            1,
            concat!(
                "CREATE/NAME_TABLE MYTAB\rCREATE/NAME_TABLE MYTAB\rDEFINE/TABLE=MYTAB FOO BAR\r",
                "SHOW LOGICAL FOO\r",
                "DEFINE/TABLE=LNM$PROCESS_DIRECTORY LNM$FILE_DEV LNM$PROCESS,MYTAB,LNM$SYSTEM\r",
                "SHOW LOGICAL FOO\r",
            ),
        ),
        (
            "\"FOO\" = \"BAR\" (MYTAB)",
            1,
            concat!(
                "SHOW LOGICAL/TABLE=LNM$PROCESS_DIRECTORY\r",
                "DEASSIGN/TABLE=LNM$PROCESS_DIRECTORY LNM$FILE_DEV\r",
                "CREATE/NAME_TABLE/PARENT=MYTAB SUB\rDEFINE/TABLE=SUB X Y\r",
                "CREATE/NAME_TABLE/ATTRIBUTES=SUPERSEDE MYTAB\rSHOW LOGICAL/TABLE=SUB X\r",
            ),
        ),
        (
            "previous table MYTAB has been superseded",
            1,
            concat!(
                "CREATE/NAME_TABLE/ATTRIBUTES=SUPERSEDE NEWTAB\r",
                "DEASSIGN/TABLE=LNM$PROCESS_DIRECTORY MYTAB\r",
                "DEASSIGN/TABLE=LNM$PROCESS_DIRECTORY NEWTAB\rSHOW LOGICAL/TABLE=MYTAB FOO\r",
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
        " \\XYZZY\\",
        "    SHOW LOGICAL [logical-name]",
        "  With no name, or *, it lists every logical name.",
        "  Turns down the call ringing your phone. The caller is told.",
        "  Additional help libraries available (type @name for topics):\n\n  PHONEHELP",
        "  Picks up the call ringing your phone, and adds a viewport for the",
        "$ TYPE HELP.LIS\n\nTYPE\n",
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
        "%DCL-I-TABEXIST, MYTAB already exists - not superseded",
        "$ SHOW LOGICAL FOO\n%SHOW-S-NOTRAN, no translation for logical name FOO",
        "$ SHOW LOGICAL FOO\n   \"FOO\" = \"BAR\" (MYTAB)",
        concat!(
            "(LNM$PROCESS_DIRECTORY)\n\n",
            "  \"LNM$FILE_DEV\" = \"LNM$PROCESS\"\n        = \"MYTAB\"\n        = \"LNM$SYSTEM\"\n",
            "  \"LNM$PROCESS\" = \"LNM$PROCESS_TABLE\"\n",
            "  \"LNM$PROCESS_DIRECTORY\" [table] = \"\"\n",
            "  \"LNM$PROCESS_TABLE\" [table] = \"\"\n",
            "  \"MYTAB\" [table] = \"\"\n",
        ),
        "$ SHOW LOGICAL/TABLE=SUB X\n%SYSTEM-W-IVLOGTAB, invalid logical name table\n%SHOW-S-NOTRAN, no translation for logical name X",
        "%DCL-I-TABNOTFND, previous table NEWTAB was not found - new table created",
        "$ SHOW LOGICAL/TABLE=MYTAB FOO\n%SYSTEM-W-IVLOGTAB",
    ],
};

/// INITIALIZE/SIZE of DKB0:, which only the ramdisk takes, and of MDA0:,
/// SYSTARTUP_VMS.COM's, which made it 2,048 blocks, more than it has free;
/// INITIALIZE of DKB0:'s 64 MB, three blocks a cluster, MOUNT and COPY
/// there, MOUNT's DISK$DATA and the system disk's DISK$VAXPUNK, a
/// CREATE/DIRECTORY two levels deep
/// there and a COPY into it, a CREATE/LOG of a text file there from two
/// lines and CTRL/Z, an APPEND/LOG to the file copied there and
/// one to the system disk that fails, COPY of a list into one file, of a
/// wildcard into a file each, and of a wildcard into one file that it
/// finds too, which it passes over, SHOW DEVICES, SHOW DEVICES/FULL of
/// DKB0: and of OPA0:, the mailboxes, MBA2: and BG0:, the template, and
/// F$GETDVI of devices named as VMS's search takes them, a DISMOUNT of DKA0:
/// that fails and one of DKB0: that doesn't, which deletes DISK$DATA, a
/// DIRECTORY there that fails, a MOUNT that names its logical name WORK:,
/// and a SHOW DEVICES of DKB0: after each MOUNT, a DIRECTORY of the new directory and a DELETE
/// of the one above it, which has a file in it and stays, SHOW PROCESS/QUOTAS,
/// SHOW SYSTEM, SHOW USERS, of a user who isn't logged in too, SHOW MEMORY,
/// SHOW ERROR, SHOW QUOTA and SHOW CPU.
const DATA_DISK: Phase = Phase {
    name: "data disk",
    secs: 20,
    steps: &[
        // Once the ramdisk's last command has run: the type-ahead buffer is empty.
        (
            "no translation for logical name HOME",
            1,
            concat!(
                "INIT/SIZE=100 DKB0: DATA\r",
                "INIT/PROTECTION=(S:RWED,O:RWED,G:RWED,W:RWED) DKB0: DATA\rMOUNT/SYSTEM DKB0: DATA\r",
                "SHOW DEVICES DKB0\r",
                "WRITE SYS$OUTPUT \"RAM BIG \", F$GETDVI(\"MDA0\",\"FREEBLOCKS\") .GT. 1024\r",
                "SHOW LOGICAL DISK$DATA\rSHOW LOGICAL DISK$VAXPUNK\r",
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
                "COPY/LOG DKB0:[SUB.DEEP]DEEP.TXT,NOTE.TXT DKB0:[SUB.DEEP]BOTH.TXT\r",
                "COPY/LOG DKB0:[SUB.DEEP]*.TXT DKB0:[SUB]*.OLD\r",
                "COPY/LOG DKB0:[SUB.DEEP]*.TXT DKB0:[SUB.DEEP]ZALL.TXT\r",
            ),
        ),
        (
            "%RMS-E-WLK",
            1,
            concat!(
                "DIR DKB0:[000000]\rSHOW DEVICES\rSHOW DEVICES/FULL DKB0\rSHOW DEVICES/FULL OPA0:\r",
                "SHOW DEVICES MB\rSHOW DEVICES/FULL MBA2\rSHOW DEVICES/FULL BG0\r",
                "WRITE SYS$OUTPUT F$GETDVI(\"OP0\",\"DEVNAM\"), F$GETDVI(\"DKB\",\"DEVNAM\"), F$GETDVI(\"MB2\",\"DEVCLASS\"), F$GETDVI(\"FOO_BAR\",\"EXISTS\")\r",
                "DISMOUNT DKA0:\rDISMOUNT DKB0:\rSHOW LOGICAL DISK$DATA\r",
                "DIR DKB0:[000000]\rMOUNT/SYSTEM DKB0: DATA WORK:\rSHOW DEVICES DKB0\rSHOW LOGICAL WORK\r",
                "DIR DKB0:[SUB.DEEP]\rDELETE DKB0:[000000]SUB.DIR;1\r",
                "SHOW PROCESS/QUOTAS\rSHOW SYSTEM\r",
            ),
        ),
        (
            "CUR     4 SHOW.EXE",
            1,
            concat!(
                "SHOW USERS/FULL\rSHOW USERS NOBODY\rSHOW MEMORY/SLOTS\r",
                "SHOW ERROR\rSHOW QUOTA\rSHOW CPU/BRIEF\r",
            ),
        ),
    ],
    lines: &[
        "$ INIT/SIZE=100 DKB0: DATA\n%SYSTEM-F-BADPARAM, bad parameter value",
        "%MOUNT-I-MOUNTED, DATA mounted on _DKB0:",
        "RAM BIG 1",
        "\"DISK$DATA\" = \"_DKB0:\" (LNM$SYSTEM_TABLE)",
        "\"DISK$VAXPUNK\" = \"_DKA0:\" (LNM$SYSTEM_TABLE)",
        "Directory DKB0:[SUB]",
        "DEEP.DIR;1",
        "%CREATE-I-CREATED, DKB0:[SUB.DEEP]NOTE.TXT;1 created",
        "%APPEND-S-APPENDED, DKA0:[SYSMGR]WELCOME.TXT;1 appended to DKB0:[000000]DATA.TXT;1 (10 records)",
        "%RMS-E-WLK, device currently write locked",
        "%COPY-S-COPIED, DKB0:[SUB.DEEP]DEEP.TXT;1 copied to DKB0:[SUB.DEEP]BOTH.TXT;1 (10 records)",
        "%COPY-S-APPENDED, DKB0:[SUB.DEEP]NOTE.TXT;1 appended to DKB0:[SUB.DEEP]BOTH.TXT;1 (2 records)",
        "%COPY-S-COPIED, DKB0:[SUB.DEEP]BOTH.TXT;1 copied to DKB0:[SUB]BOTH.OLD;1 (12 records)",
        "%COPY-S-COPIED, DKB0:[SUB.DEEP]NOTE.TXT;1 copied to DKB0:[SUB]NOTE.OLD;1 (2 records)",
        "%COPY-S-COPIED, DKB0:[SUB.DEEP]BOTH.TXT;1 copied to DKB0:[SUB.DEEP]ZALL.TXT;1 (12 records)",
        "%COPY-S-APPENDED, DKB0:[SUB.DEEP]NOTE.TXT;1 appended to DKB0:[SUB.DEEP]ZALL.TXT;1 (2 records)",
        "Directory DKB0:[000000]",
        "DATA.TXT;1",
        "SUB.DIR;1",
        "DKA0:                   Mounted wrtlck       0  VAXPUNK",
        "DKB0:                   Mounted              0  DATA",
        "MDA0:                   Mounted              0  RAM",
        "OPA0:                   Online               0\nFTA0:                   Offline              0\nTNA0:                   Online               0\nVTA0:                   Online               0\n$ ",
        "MBA2:                   Online               0\n",
        "\nDevice MBA2:, device type local memory mailbox, is online, record-oriented\n    device, shareable, mailbox device.\n",
        "\nDevice BG0:, device type unknown, is online, record-oriented device, network\n    device, mailbox device, device is a template only.\n",
        "_OPA0:_DKB0:160FALSE\n",
        "\nDisk DKB0:, device type unknown, is online, mounted, file-oriented device,\n    shareable.\n\n    Error count                    0\n    Reference count",
        "    Default buffer size                 512\n\n    Volume label              \"DATA\"    Cluster size                          3\n    Free blocks ",
        "\nTerminal OPA0:, device type VT100, is online, record-oriented device, carriage\n    control.\n\n    Error count                    0\n    Reference count",
        "    Default buffer size                  80\n\n$ ",
        "%SYSTEM-F-DEVACTIVE, device is active",
        "no translation for logical name DISK$DATA",
        concat!(
            "%DIRECT-E-OPENIN, error opening DKB0:[000000]*.*;* as input\n",
            "-RMS-E-DNR, device not ready, not mounted, or unavailable",
        ),
        "\"WORK\" = \"_DKB0:\" (LNM$SYSTEM_TABLE)",
        "Directory DKB0:[SUB.DEEP]",
        "DEEP.TXT;1",
        "NOTE.TXT;1",
        "%RMS-E-MKD, ACP could not mark file for deletion",
        "Process name:       \"SYSTEM\"",
        "UIC:                [1,4]",
        " CPU limit:                      Infinite  Direct I/O limit:        150\n Buffered I/O byte count quota:    128000  Buffered I/O limit:      150",
        "00010001 SWAPPER",
        "SYSTEM          CUR     4 SHOW.EXE",
        "    Total number of users = 1,  number of processes = 1\n\n Username    Process Name    PID       Terminal\n SYSTEM      SYSTEM          ",
        "%SHOW-I-NOSUCHUSER, no processes found for user NOBODY",
        " Process Entry Slots               1024",
        "%SHOW-S-NOERRORS, no device errors found",
        "%SYSTEM-F-QFNOTACT, disk quotas not enabled on this volume",
        "System: QEMU virt arm64\nCPU 0    State: RUN\n       Process: SYSTEM",
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
        // HELP's own text and HELPLIB's topics fill the screen, then it
        // asks for a topic: Return leaves.
        ("Press RETURN to continue ...", 1, "\r"),
        ("Topic?", 1, "\rEDIT/EDT SYS$MANAGER:WELCOME.TXT\r"),
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
        "  INQUIRE       LEXICALS      LINE_EDITING  LOGOUT        MCR",
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

/// A second OPCOM can't start, its name taken; REQUEST's messages come
/// on the console through the first, for every class and for OPER12 only,
/// after the console's name.
const OPERATOR: Phase = Phase {
    name: "operator",
    secs: 20,
    steps: &[(
        "%DCL-W-IVCHAR",
        1,
        concat!(
            "RUN/DETACHED/PROCESS_NAME=OPCOM SYS$SYSTEM:OPCOM\r",
            "REQUEST \"Disk full, please help\"\rREQUEST/TO=OPER12 \"Twelve only\"\r",
        ),
    )],
    lines: &[
        "%RUN-F-CREPRC, process creation failed\n-SYSTEM-F-DUPLNAM, duplicate name\n",
        "\nMessage from user SYSTEM\n_OPA0:, Disk full, please help\n",
        "\nMessage from user SYSTEM\n_OPA0:, Twelve only\n",
    ],
};

/// SET PROCESS/PRIVILEGES takes CMKRNL and SYSNAM away, which SHOW
/// PROCESS/PRIVILEGES shows, without SHOW.EXE's own; SHOW LOGICAL still
/// lists the tables, sorted, with SHOW.EXE's CMKRNL, and DEFINE/SYSTEM
/// works, as the system table's protection lets SYSTEM's UIC group, a
/// system one, do, as on OpenVMS; a privilege it doesn't know fails, and
/// ALL gives them back.
const PRIVILEGES: Phase = Phase {
    name: "privileges",
    secs: 20,
    steps: &[(
        "_OPA0:, Twelve only",
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
        "  \"ZOO\" = \"TWO\"\n\n(LNM$SYSTEM_TABLE)\n\n  \"DISK$DATA\" = \"_DKB0:\"\n  \"DISK$RAM\"",
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
            "DISMOUNT DKB0:\rMOUNT/SYSTEM/PROTECTION=(W) DKB0: DATA\rRUN PROTTEST\r",
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
        // COPY's qualifiers, in a directory of BACKUP's *.TXT's way.
        (
            "NEW.TXT;2            (RWED,RWED,RE,RE)",
            1,
            concat!(
                "CREATE/DIRECTORY DKB0:[CQ]\r",
                "COPY/PROTECTION=(W:R)/ALLOCATION=20 SYS$SYSDEVICE:[SYSMGR]WELCOME.TXT DKB0:[CQ]Q.TXT\r",
                "DIRECTORY/FULL DKB0:[CQ]Q.TXT\r",
                "COPY SYS$SYSDEVICE:[SYSMGR]WELCOME.TXT DKB0:[CQ]Q.TXT;1\r",
                "COPY/REPLACE/LOG SYS$SYSDEVICE:[SYSMGR]WELCOME.TXT DKB0:[CQ]Q.TXT;1\r",
                "COPY/OVERLAY/LOG DKB0:[SUB.DEEP]NOTE.TXT DKB0:[CQ]Q.TXT\r",
                "TYPE DKB0:[CQ]Q.TXT\r",
                "COPY/CONFIRM SYS$SYSDEVICE:[SYSMGR]WELCOME.TXT DKB0:[CQ]C.TXT\r",
            ),
        ),
        ("? [N]: ", 1, "maybe\r"),
        (
            "? [N]: ",
            2,
            "n\rCOPY/CONFIRM/LOG SYS$SYSDEVICE:[SYSMGR]WELCOME.TXT DKB0:[CQ]C.TXT\r",
        ),
        (
            "? [N]: ",
            3,
            "Y\rCOPY/CONFIRM/LOG DKB0:[CQ]*.TXT DKB0:[CQ]*.NEW\r",
        ),
        (
            "? [N]: ",
            4,
            "all\rCOPY/CONFIRM DKB0:[CQ]*.TXT DKB0:[CQ]*.OLD\r",
        ),
        (
            "? [N]: ",
            5,
            "q\rDIRECTORY DKB0:[CQ]\rWRITE SYS$OUTPUT \"COPYQ: \", \"done\"\r",
        ),
    ],
    lines: &[
        "%RMS-E-FEX, file already exists, not superseded",
        "%COPY-I-REPLACED, DKB0:[CQ]Q.TXT;1 being replaced",
        "%COPY-S-COPIED, DKA0:[SYSMGR]WELCOME.TXT;1 copied to DKB0:[CQ]Q.TXT;1 (10 records)",
        "%COPY-I-OVERLAY, DKB0:[CQ]Q.TXT;1 being overwritten",
        "%COPY-S-COPIED, DKB0:[SUB.DEEP]NOTE.TXT;1 copied to DKB0:[CQ]Q.TXT;1 (2 records)",
        "Copy DKA0:[SYSMGR]WELCOME.TXT;1 to DKB0:[CQ]C.TXT;? [N]: ",
        "%COPY-S-COPIED, DKA0:[SYSMGR]WELCOME.TXT;1 copied to DKB0:[CQ]C.TXT;1 (10 records)",
        "%COPY-S-COPIED, DKB0:[CQ]C.TXT;1 copied to DKB0:[CQ]C.NEW;1 (10 records)",
        "%COPY-S-COPIED, DKB0:[CQ]Q.TXT;1 copied to DKB0:[CQ]Q.NEW;1 (2 records)",
        "Total of 4 files.",
        "COPYQ: done",
        "%RMS-E-PRV, insufficient privilege or file protection violation",
        "PROTTEST: [200,1] read DATA.TXT, and may not give it away, delete it or make a file there",
        "DATA.TXT;1           [SYSTEM]                         (RWED,RWED,RE,R)",
        "DATA.TXT;1           [200,1]                          (RWED,RWED,RE,)",
        "NEW.TXT;2            (RWED,RWED,RE,RE)",
    ],
};

/// SET SECURITY gives DATA.TXT, SYSTEM's again, with no access for the
/// world, ACEs (ADR-0040): one for PROTCHILD's UIC, [200,1], lets PROTTEST
/// read it; one denying its group, put before it, doesn't; then one for
/// anyone, *, does. SHOW SECURITY and DIRECTORY/SECURITY show them, and the
/// owner by name. A directory's default ACEs and its PROTECTED default
/// protection go to a new file in it, and a new version gets the one
/// before's ACL, but its NOPROPAGATE ACEs. A name that isn't an
/// identifier, ACCESS before IDENTIFIER, /LIKE without NAME, an ACE that
/// isn't there to delete, /AFTER, /REPLACE of two ACEs, where the last
/// was, and of one that isn't there, which changes nothing, and /LIKE,
/// which keeps a PROTECTED ACE first; then DATA.TXT goes back to [200,1],
/// and DKB0: to /PROTECTION=(W).
const ACLS: Phase = Phase {
    name: "acls",
    secs: 30,
    steps: &[
        (
            "COPYQ: done",
            1,
            concat!(
                "DISMOUNT DKB0:\rMOUNT/SYSTEM DKB0: DATA\rSET SECURITY/OWNER=[1,4] DKB0:[000000]DATA.TXT\r",
                "RUN PROTTEST\r",
                "SET SECURITY/ACL=(IDENTIFIER=[200,1],ACCESS=READ) DKB0:[000000]DATA.TXT\r",
                "RUN PROTTEST\r",
            ),
        ),
        (
            "PROTTEST: [200,1] read",
            3,
            concat!(
                "SET SECURITY/ACL=(ID=[200,*],ACC=NONE) DKB0:[000000]DATA.TXT\r",
                "SHOW SECURITY DKB0:[000000]DATA.TXT\rRUN PROTTEST\r",
            ),
        ),
        (
            "%RMS-E-PRV",
            4,
            concat!(
                "SET SECURITY/ACL/DELETE/LOG DKB0:[000000]DATA.TXT\r",
                "SET SECURITY/ACL=(IDENTIFIER=*,ACCESS=READ) DKB0:[000000]DATA.TXT\r",
                "RUN PROTTEST\r",
            ),
        ),
        (
            "PROTTEST: [200,1] read",
            4,
            concat!(
                "DIRECTORY/SECURITY DKB0:[000000]DATA.TXT\r",
                "CREATE/DIRECTORY DKB0:[ACL]\r",
                "SET SECURITY/ACL=((ID=[200,*],OPT=DEFAULT,ACC=R+W),(DEFAULT_PROTECTION,OPT=PROTECTED,S:RWED,O:RWED,G:RE,W:R)) DKB0:[000000]ACL.DIR\r",
                "COPY SYS$SYSDEVICE:[SYSMGR]WELCOME.TXT DKB0:[ACL]A.TXT\r",
                "SET SECURITY/ACL=((ID=INTERACTIVE,ACC=R+W,OPT=NOPROPAGATE),(ID=[200,2],ACC=R)) DKB0:[ACL]A.TXT\r",
                "COPY SYS$SYSDEVICE:[SYSMGR]WELCOME.TXT DKB0:[ACL]A.TXT\r",
                "DIRECTORY/ACL DKB0:[ACL]\r",
                "SHOW SECURITY DKB0:[ACL]A.TXT;2\r",
                "SET SECURITY/ACL=(IDENTIFIER=NOSUCH,ACCESS=READ) DKB0:[ACL]A.TXT\r",
                "SET SECURITY/ACL=(IDENTIFIER=LOCAL,ACCESS=FOO) DKB0:[ACL]A.TXT\r",
                "SET SECURITY/ACL=(ACCESS=READ,IDENTIFIER=LOCAL) DKB0:[ACL]A.TXT\r",
                "SET SECURITY/LIKE=CLASS=FILE DKB0:[ACL]A.TXT\r",
                "SET SECURITY/ACL=(IDENTIFIER=LOCAL,ACCESS=READ)/DELETE DKB0:[ACL]A.TXT;2\r",
                "SET SECURITY/ACL=(ID=LOCAL,ACC=R)/AFTER=(ID=[200,2]) DKB0:[ACL]A.TXT;2\r",
                "SET SECURITY/ACL=((ID=[200,*]),(ID=[200,2]))/REPLACE=(ID=REMOTE,ACC=E) DKB0:[ACL]A.TXT;2\r",
                "SET SECURITY/ACL=((ID=LOCAL),(ID=NETWORK))/REPLACE=(ID=BATCH,ACC=R) DKB0:[ACL]A.TXT;2\r",
                "SET SECURITY/ACL=(ID=BATCH,ACC=NONE,OPT=PROTECTED) DKB0:[ACL]A.TXT;1\r",
                "SET SECURITY/LIKE=NAME=DKB0:[ACL]A.TXT;2/LOG DKB0:[ACL]A.TXT;1\r",
                "SHOW SECURITY DKB0:[ACL]A.TXT;1\r",
                "SET SECURITY/ACL/DELETE DKB0:[000000]DATA.TXT\r",
                "SET SECURITY/OWNER=[200,1] DKB0:[000000]DATA.TXT\r",
                "DISMOUNT DKB0:\rMOUNT/SYSTEM/PROTECTION=(W) DKB0: DATA\r",
                "WRITE SYS$OUTPUT \"ACLS: \", \"done\"\r",
            ),
        ),
    ],
    lines: &[
        "\nDKB0:[000000]DATA.TXT;1 object of class FILE\n     Owner: [SYSTEM]\n     Protection: (System: RWED, Owner: RWED, Group: RE, World)\n     Access Control List:\n          (IDENTIFIER=[200,*],ACCESS=NONE)\n          (IDENTIFIER=[200,1],ACCESS=READ)\n",
        "%SET-I-MODIFIED, DKB0:[000000]DATA.TXT;1 modified",
        "DATA.TXT;1           [SYSTEM]                         (RWED,RWED,RE,)\n          (IDENTIFIER=*,ACCESS=READ)\n",
        "A.TXT;2\n          (IDENTIFIER=[200,2],ACCESS=READ)\n          (IDENTIFIER=[200,*],ACCESS=READ+WRITE)\nA.TXT;1\n          (IDENTIFIER=INTERACTIVE,OPTIONS=NOPROPAGATE,ACCESS=READ+WRITE)\n",
        "     Protection: (System: RWED, Owner: RWED, Group: RE, World: R)",
        "%SET-F-SYNTAX, error parsing 'NOSUCH,ACCESS=READ)'\n-SYSTEM-F-NOSUCHID, unknown rights identifier",
        "%SET-F-SYNTAX, error parsing 'ACCESS=FOO)'\n-SYSTEM-F-IVACL, invalid access control list entry syntax",
        "%SET-F-SYNTAX, error parsing 'ACCESS=READ,IDENTIFIER=LOCAL)'\n-SYSTEM-F-IVACL, invalid access control list entry syntax",
        "$ SET SECURITY/LIKE=CLASS=FILE DKB0:[ACL]A.TXT\n%CLI-W-VALREQ, missing qualifier or keyword value - supply all required values\n$ ",
        "%SET-W-NOSUCHACE, ACE for object DKB0:[ACL]A.TXT;2, does not exist\n    (IDENTIFIER=LOCAL,ACCESS=READ)\n%SET-F-WRITEERR, error writing DKB0:[ACL]A.TXT;2",
        "%SET-W-NOSUCHACE, ACE for object DKB0:[ACL]A.TXT;2, does not exist\n    (IDENTIFIER=NETWORK,ACCESS=NONE)\n%SET-F-WRITEERR, error writing DKB0:[ACL]A.TXT;2\n$ ",
        "%SET-I-MODIFIED, DKB0:[ACL]A.TXT;1 modified",
        "\nDKB0:[ACL]A.TXT;1 object of class FILE\n     Owner: [SYSTEM]\n     Protection: (System: RWED, Owner: RWED, Group: RE, World: R)\n     Access Control List:\n          (IDENTIFIER=BATCH,OPTIONS=PROTECTED,ACCESS=NONE)\n          (IDENTIFIER=REMOTE,ACCESS=EXECUTE)\n          (IDENTIFIER=LOCAL,ACCESS=READ)\n",
        "ACLS: done",
    ],
};

/// While DCL has SHARE.TXT open for writing, TYPE and OPEN/READ can't open
/// it: %RMS-E-FLK. Once it is closed, two readers can; DELETE takes its
/// directory entry, but the second reader still reads it, and the last
/// CLOSE deletes it, which the data disk's check finds no lost file of.
const SHARING: Phase = Phase {
    name: "sharing",
    secs: 20,
    steps: &[
        (
            "ACLS: done",
            1,
            concat!(
                "OPEN/WRITE OUT DKB0:[000000]SHARE.TXT\rWRITE OUT \"shared line\"\r",
                "TYPE DKB0:[000000]SHARE.TXT\r",
            ),
        ),
        ("RMS-E-FLK", 1, "OPEN/READ IN DKB0:[000000]SHARE.TXT\r"),
        (
            "RMS-E-FLK",
            2,
            concat!(
                "CLOSE OUT\rOPEN/READ IN DKB0:[000000]SHARE.TXT\r",
                "OPEN/READ IN2 DKB0:[000000]SHARE.TXT\rDELETE DKB0:[000000]SHARE.TXT;1\r",
                "WRITE SYS$OUTPUT \"SHARING: \", \"deleted\"\r",
            ),
        ),
        (
            "SHARING: deleted",
            1,
            concat!(
                "DIRECTORY DKB0:[000000]SHARE.TXT\rREAD IN2 LINE\r",
                "WRITE SYS$OUTPUT \"SHARING: \", LINE\rCLOSE IN\rCLOSE IN2\r",
            ),
        ),
    ],
    lines: &[
        "%RMS-E-FLK, file currently locked by another user",
        "%DCL-E-OPENIN, error opening DKB0:[000000]SHARE.TXT as input\n-RMS-E-FLK, file currently locked by another user",
        "SHARING: shared line",
    ],
};

/// SET DIRECTORY/VERSION_LIMIT=2 gives DKB0:[VL] a default limit, so
/// the third COPY of A.TXT purges ;1; SET FILE/VERSION_LIMIT=1 gives A.TXT
/// its own, which DIRECTORY/FULL shows, and the next COPY purges one more.
/// SET FILE/ATTRIBUTE and /EXPIRATION_DATE change its record format and
/// expiration date, the time through $BINTIM, which refuses 31-FEB.
/// SET VOLUME/LABEL relabels DKB0:'s volume, on the disk, as MOUNT finds,
/// and back.
const SET_FILE: Phase = Phase {
    name: "set file",
    secs: 30,
    steps: &[
        (
            "SHARING: shared line",
            1,
            concat!(
                "CREATE/DIRECTORY DKB0:[VL]\rSET DIRECTORY/VERSION_LIMIT=2 DKB0:[VL]\r",
                "COPY SYS$SYSDEVICE:[SYSMGR]WELCOME.TXT DKB0:[VL]A.TXT\r",
                "COPY SYS$SYSDEVICE:[SYSMGR]WELCOME.TXT DKB0:[VL]A.TXT\r",
                "COPY SYS$SYSDEVICE:[SYSMGR]WELCOME.TXT DKB0:[VL]A.TXT\r",
                "WRITE SYS$OUTPUT \"SETFILE: \", \"three\"\r",
            ),
        ),
        (
            "SETFILE: three",
            1,
            concat!(
                "DIRECTORY DKB0:[VL]\rDIRECTORY/FULL DKB0:[000000]VL.DIR\r",
                "SET FILE/VERSION_LIMIT=1 DKB0:[VL]A.TXT\r",
                "COPY SYS$SYSDEVICE:[SYSMGR]WELCOME.TXT DKB0:[VL]A.TXT\r",
                "WRITE SYS$OUTPUT \"SETFILE: \", \"four\"\r",
            ),
        ),
        (
            "SETFILE: four",
            1,
            concat!(
                "DIRECTORY DKB0:[VL]\r",
                "SET FILE/ATTRIBUTE=(RFM:FIX,LRL:80,MRS:80,RAT:NONE) DKB0:[VL]A.TXT\r",
                "SET FILE/EXPIRATION_DATE=1-JAN-2030:12:00 DKB0:[VL]A.TXT\r",
                "DIRECTORY/FULL DKB0:[VL]A.TXT;4\r",
                "WRITE SYS$OUTPUT \"SETFILE: \", \"full\"\r",
            ),
        ),
        (
            "SETFILE: full",
            1,
            concat!(
                "SET FILE/EXPIRATION_DATE=31-FEB-2030 DKB0:[VL]A.TXT\r",
                "SET FILE/ATTRIBUTE=(RFM:ODD) DKB0:[VL]A.TXT\r",
                "SET VOLUME/LABEL=SCRATCH DKB0:\rDISMOUNT DKB0:\rMOUNT/SYSTEM DKB0: SCRATCH\r",
                "WRITE SYS$OUTPUT \"SETFILE: \", \"label\"\r",
            ),
        ),
        (
            "SETFILE: label",
            1,
            concat!(
                "SET VOLUME/LABEL=DATA DKB0:\rDISMOUNT DKB0:\rMOUNT/SYSTEM/PROTECTION=(W) DKB0: DATA\r",
                "WRITE SYS$OUTPUT \"SETFILE: \", \"done\"\r",
            ),
        ),
    ],
    lines: &[
        "\nA.TXT;3             A.TXT;2\n",
        "Global buffer count: 0, Default version limit: 2, Contiguous, Directory file",
        "\nA.TXT;4             A.TXT;3\n",
        "Expires:     1-JAN-2030 12:00:00.00",
        "Global buffer count: 0, Version limit: 1\nRecord format:      Fixed length 80 byte records\nRecord attributes:  No carriage control",
        "$ SET FILE/EXPIRATION_DATE=31-FEB-2030 DKB0:[VL]A.TXT\n%SYSTEM-F-IVTIME, invalid time",
        "$ SET FILE/ATTRIBUTE=(RFM:ODD) DKB0:[VL]A.TXT\n%SYSTEM-F-BADPARAM, bad parameter value",
        "%MOUNT-I-MOUNTED, SCRATCH mounted on _DKB0:",
        "SETFILE: done",
    ],
};

/// SET TERMINAL changes the console's width, page, BROADCAST and PASTHRU,
/// which SHOW TERMINAL shows, refuses a width of 600, and after /NOECHO
/// the command typed next isn't echoed; then it puts them back. TTTEST
/// checks timed reads, the type-ahead count of the keys typed for it, a
/// read an escape sequence ends and one with terminators of its own.
/// TYPE/PAGE writes WELCOME.TXT on a page of 8 rows, 7 lines, asks, and
/// on RETURN scrolls on to its end; with /PAGE=CLEAR_SCREEN CTRL/Z at
/// the prompt stops it. INQUIRE asks for a global symbol's value.
const TERMINAL: Phase = Phase {
    name: "terminal",
    secs: 20,
    steps: &[
        (
            "SETFILE: done",
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
        (
            "TTTEST: ok",
            1,
            "SET TERMINAL/PAGE=8\rTYPE/PAGE SYS$MANAGER:WELCOME.TXT\r",
        ),
        // HELP's "Press RETURN to continue ..." was the first.
        (
            "Press RETURN to continue",
            2,
            "\rTYPE/PAGE=CLEAR_SCREEN SYS$MANAGER:WELCOME.TXT\r",
        ),
        (
            "Press RETURN to continue",
            3,
            "\x1aSET TERMINAL/PAGE=24\rIQS = \"ok\"\rINQUIRE/GLOBAL IQ say\r",
        ),
        (
            "SAY: ",
            1,
            "  hi  'IQS'  \"Mixed  case\" ! not this\rSHOW SYMBOL IQ\rWRITE SYS$OUTPUT \"TYPE/PAGE\", \" done\"\r",
        ),
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
        // Rows 1 to 7 scroll; the prompt is on row 8, centred.
        "\x1b7\x1b[1;7r\x1b8",
        "\x1b[7m\x1b[8;29fPress RETURN to continue\x1b[0m",
        // RETURN: the prompt erased, back to row 7 to scroll on.
        "\x1b[8;29f\x1b[K\x1b[7;1f\nDIRECTORY lists the files",
        "and the rest of what INITIALIZE made.\x1b7\x1b[r\x1b8\n$ ",
        // CTRL/Z: the prompt erased, and every row scrolls again.
        "*EXIT*\n\x1b[8;29f\x1b[K\x1b7\x1b[r\x1b8$ SET TERMINAL/PAGE=24",
        // INQUIRE's answer, IQS substituted, as := makes a string.
        "\nSAY: ",
        "  IQ == \"HI OK Mixed  case\"",
        "TYPE/PAGE done",
    ],
};

/// SET TERMINAL/DEVICE_TYPE gives the console a VT200's characteristics,
/// /SPEED its speeds and /COLOR ANSI colour, which SHOW TERMINAL and
/// F$GETDVI show; it refuses a type and a speed VMS doesn't know, as VMS
/// does. /INQUIRE asks what the terminal is, here a VT400 with a printer
/// port, sixel graphics and colour, then its size; asked again, no answer
/// comes, and it says so after 3 seconds. /PERMANENT sets the permanent
/// characteristics, VT100's, which SHOW TERMINAL/PERMANENT shows, then
/// puts them back as they were.
const TERMINAL_TYPES: Phase = Phase {
    name: "terminal types",
    secs: 20,
    steps: &[
        (
            "TYPE/PAGE done",
            1,
            concat!(
                "SET TERMINAL/DEVICE_TYPE=VT200/SPEED=(1200,300)/COLOR\rSHOW TERMINAL\r",
                "WRITE SYS$OUTPUT \"DEPEND3=\", F$GETDVI(\"SYS$COMMAND\",\"DEVDEPEND3\")\r",
                "SET TERMINAL/DEVICE_TYPE=FOO\rSET TERMINAL/SPEED=1234\rSET TERMINAL/INQUIRE\r",
            ),
        ),
        ("\x1b[c", 1, "\x1b[?64;1;2;4;22c"),
        (
            "\x1b[255;255H",
            1,
            "\x1b[40;100RSHOW TERMINAL\rSET TERMINAL/INQUIRE\r",
        ),
        (
            "-SET-I-UNKTERM",
            1,
            concat!(
                "SET TERMINAL/PERMANENT/WIDTH=90/SPEED=4800\rSHOW TERMINAL/PERMANENT\r",
                "SET TERMINAL/PERMANENT/DEVICE_TYPE=VT100/SPEED=9600/NOCOLOR/EIGHTBIT\r",
                "WRITE SYS$OUTPUT \"TERMINAL\", \" types done\"\r",
            ),
        ),
    ],
    lines: &[
        "Terminal: _OPA0:      Device_Type: VT200_Series  Owner: SYSTEM",
        "   Input:    1200     LFfill:  0      Width:  80      Parity: None",
        "   Output:    300     CRfill:  0      Page:   24",
        "   No DEC_CRT3        No DEC_CRT4        No DEC_CRT5        Ansi_Color",
        "   VMS Style Input    <CTRL-H> Backspace",
        "DEPEND3=2",
        "%SET-W-NOTSET, error modifying OPA0:",
        "-SET-E-INVQUAVAL, value 'FOO' invalid for /DEVICE_TYPE qualifier",
        "%SET-E-INVQUAVAL, value '1234' invalid for /SPEED qualifier",
        "Terminal: _OPA0:      Device_Type: VT400_Series  Owner: SYSTEM",
        "   Input:    1200     LFfill:  0      Width: 100      Parity: None",
        "   Output:    300     CRfill:  0      Page:   40",
        "   No Syspassword     SIXEL Graphics     No Soft Characters Printer port",
        "   DEC_CRT3           DEC_CRT4           No DEC_CRT5        Ansi_Color",
        "-SET-I-UNKTERM, unknown terminal type",
        "   Input:    4800     LFfill:  0      Width:  90      Parity: None",
        "TERMINAL types done",
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
        "DATA.TXT;1           [200,1]                          (RWED,RWED,RE,)",
        "NEW.TXT;2            [SYSTEM]                         (RWED,RWED,RE,RE)",
    ],
};

/// SPILLTEST.COM, a name with more versions than one directory record
/// holds, on DKB0:; then RMSTEST in a directory of its own there, whose
/// output must be OpenVMS's, vms/sysexe/rmstest.out (checked after the
/// session). SET RMS_DEFAULT sets the process's multiblock count and
/// indexed files' buffers, and the system's extension; a prologue that
/// isn't one sets nothing; SHOW RMS_DEFAULT shows them as OpenVMS's does.
/// A prologue default of 2 makes CONVERT/FDL's indexed file, whose FDL
/// names none, prologue 1, as its key is a string.
const RMS: Phase = Phase {
    name: "rms",
    secs: 90,
    steps: &[
        (
            "NEW.TXT;2            [SYSTEM]                         (RWED,RWED,RE,RE)",
            1,
            concat!(
                "@SYS$MANAGER:SPILLTEST\r",
                "CREATE/DIRECTORY DKB0:[RMS]\rSET DEFAULT DKB0:[RMS]\rRUN RMSTEST\rSET DEFAULT DKB0:[000000]\r",
            ),
        ),
        (
            "RMSTEST done",
            1,
            concat!(
                "SET RMS_DEFAULT/BLOCK_COUNT=24/BUFFER_COUNT=-5/INDEXED\r",
                "SET RMS_DEFAULT/EXTEND_QUANTITY=100/PROLOG=1\r",
                "SET RMS_DEFAULT/SYSTEM/EXTEND_QUANTITY=7\r",
            ),
        ),
        (
            "%SET-E-VALERR, specified value is out of legal range",
            1,
            concat!(
                "SHOW RMS_DEFAULT\r",
                "SET RMS_DEFAULT/BLOCK_COUNT=0/BUFFER_COUNT=0/INDEXED\r",
                "SET RMS_DEFAULT/SYSTEM/EXTEND_QUANTITY=0\r",
            ),
        ),
        (
            "System      0              7                 10",
            1,
            "CREATE DKB0:[RMS]P.FDL\rFILE\rORGANIZATION indexed\rKEY 0\rDUPLICATES yes\rSEG0_LENGTH 1\r\x1a",
        ),
        (
            "SEG0_LENGTH 1",
            1,
            concat!(
                "SET DEFAULT DKB0:[RMS]\rSET RMS_DEFAULT/PROLOG=2\rCONVERT/FDL=P.FDL P.FDL P2.IDX\r",
                "DIRECTORY/FULL P2.IDX\rSET RMS_DEFAULT/PROLOG=0\rSET DEFAULT DKB0:[000000]\r",
            ),
        ),
    ],
    lines: &[
        "SPILLTEST: ok",
        "RMSTEST done",
        "%SET-E-VALERR, specified value is out of legal range",
        concat!(
            "Process    24     |    5         0        0       0         0       |    0\n",
            "System     32     |    0         0        0       0         0       |    8\n\n",
            "          Prolog    Extend Quantity     PATH_TIMEOUT\n",
            "Process     0              0                  0\n",
            "System      0              7                 10\n",
        ),
        "File organization:  Indexed, Prolog: 1, Using 1 key",
    ],
};

/// EDIT/TPU/NODISPLAY runs a command file CREATE makes on DKB0:: TPU
/// (PRD-0006) reads WELCOME.TXT into a buffer, finds a pattern in it, puts
/// a line before it and writes it to a file, whose head TYPE shows. Then
/// EDIT/TPU on the console's screen: a window on a buffer, its status
/// line, a line typed into it, and CTRL/Z, which writes it to a file TYPE
/// shows. Then EDIT, which is EVE, from its section file in SYS$SHARE, on
/// a new file: a line typed, and CTRL/Z, which writes it.
const TPU: Phase = Phase {
    name: "tpu",
    secs: 90,
    steps: &[
        (
            "RMSTEST done",
            1,
            concat!(
                "CREATE TPUTEST.TPU\r",
                "b := CREATE_BUFFER (\"t\", \"SYS$SYSDEVICE:[SYSMGR]WELCOME.TXT\");\r",
                "POSITION (BEGINNING_OF (b));\r",
                "r := SEARCH_QUIETLY (\"Files-11\" + ARB (6), FORWARD);\r",
                "MESSAGE (\"TPU found \" + STR (r) + \" in \" + ",
                "STR (GET_INFO (b, \"record_count\")) + \" lines\");\r",
                "COPY_TEXT (\"Edited with TPU\");\r",
                "SPLIT_LINE;\r",
                "WRITE_FILE (b, \"TPUTEST.OUT\");\r",
                "QUIT (OFF);\r\x1a",
                "EDIT/TPU/NODISPLAY/NOSECTION/COMMAND=TPUTEST.TPU\r",
                "TYPE/HEAD=2 TPUTEST.OUT\r",
            ),
        ),
        (
            "Edited with TPU",
            1,
            concat!(
                "CREATE TPUSCR.TPU\r",
                "b := CREATE_BUFFER (\"scr\");\r",
                "w := CREATE_WINDOW (1, 20, ON);\r",
                "MAP (w, b);\r",
                "DEFINE_KEY (\"WRITE_FILE (b, 'TPUSCR.OUT'); QUIT (OFF)\", CTRL_Z_KEY);\r\x1a",
                "EDIT/TPU/NOSECTION/COMMAND=TPUSCR.TPU\r",
            ),
        ),
        ("Buffer : SCR", 1, "typed on vaxpunk\x1aTYPE TPUSCR.OUT\r"),
        ("$ TYPE TPUSCR.OUT", 1, "EDIT TPUEVE.TXT\r"),
        (
            "Buffer: TPUEVE.TXT",
            1,
            "Edited with EVE\x1aTYPE TPUEVE.TXT\r",
        ),
    ],
    lines: &[
        "%TPU-S-FILEIN, 10 lines read from file SYS$SYSDEVICE:[SYSMGR]WELCOME.TXT",
        "TPU found Files-11 ODS-2 in 10 lines",
        "%TPU-S-FILEOUT, 11 lines written to file TPUTEST.OUT",
        "Edited with TPU",
        "1 line written to file TPUSCR.OUT",
        "$ TYPE TPUSCR.OUT",
        "1 line written to file TPUEVE.TXT",
        "$ TYPE TPUEVE.TXT",
        "Edited with EVE",
    ],
};

/// CDEMO, a BLISS-64 program that calls a C library through the library's
/// BLISS-64 adapter (crosstools/vtools/examples/c), prints what it prints
/// under vrun, crosstools/vtools/tests/examples/c/cdemo.stdout (checked
/// after the session); CRTLTEST, a C program, finds the C run-time
/// library's routines right; and PALTEST, a BLISS-64 one, puts the PAL's
/// queue, FEN, MCES and processor number calls to work through vbliss's
/// PAL_ built-ins.
const C: Phase = Phase {
    name: "c",
    secs: 30,
    steps: &[
        ("$ TYPE TPUEVE.TXT", 1, "RUN CDEMO\r"),
        ("C calls 7", 1, "RUN CRTLTEST\r"),
        ("CRTLTEST: ok", 1, "RUN PALTEST\r"),
    ],
    lines: &["C calls 7", "CRTLTEST: ok", "PALTEST: ok"],
};

/// LBRTEST, in DKB0:[LBR], puts the LBR$ routines to work on a help
/// library and an object library of its own, which vms-obj reads after
/// the session, and answers LBR$OUTPUT_HELP's prompts: the topic COPY,
/// its subtopic /LOG, then Return twice, up a level and out.
const LBR: Phase = Phase {
    name: "lbr",
    secs: 30,
    steps: &[
        (
            "PALTEST: ok",
            1,
            "CREATE/DIRECTORY DKB0:[LBR]\rDEFINE LBRTEST DKB0:[LBR]\rRUN LBRTEST\r",
        ),
        // The first Topic? was HELP's, in CONTROL_KEYS.
        ("Topic?", 2, "COPY\r"),
        ("COPY Subtopic?", 1, "/LOG\r"),
        ("COPY Subtopic?", 2, "\r"),
        ("Topic?", 3, "\r"),
    ],
    lines: &[
        "LBRTEST: modules 00000002",
        "LBRTEST:   Makes a file from the lines typed.",
        "LBRTEST: TYPE",
        "  Shows each file copied.",
        "Sorry, no documentation on COPY /XYZ",
        "  /LOG       Parameters",
        "LBRTEST: symbols 0000003D",
        "LBRTEST: STRING",
        "  COPY TYPE",
        "COPY Subtopic? /LOG",
        "LBRTEST: ok",
    ],
};

/// Wildcard directories on DKB0:, in a tree made for them, [W] with
/// [W.A], [W.A.X] and [W.B], a file in each: DIRECTORY of the tree,
/// [W...], of the directories in [W], [W.*], of one anywhere under it,
/// [W...X], of [*], which is [000000] too, of every directory, and of
/// [...] from [W.A]; two F$SEARCH streams, one inside the other;
/// DIRECTORY's layouts (t8y:/utils/directory#columns), as OpenVMS Alpha's:
/// a name two columns wide, /COLUMNS, /VERSIONS, /SIZE=ALL with /OWNER and
/// /PROTECTION, /TOTAL, /DATE's dates Files-11 has none of, lists of
/// files (t8y:/utils/directory#lists), and a /VERSIONS it can't parse; PURGE
/// (t8y:/utils/delete#purge) of two more versions, /KEEP=2 then 1, with a
/// /KEEP and a version it won't take; and a DELETE/LOG of the tree's files,
/// after which there are none.
const WILDCARDS: Phase = Phase {
    name: "wildcards",
    secs: 60,
    steps: &[
        (
            "LBRTEST: ok",
            1,
            concat!(
                "CREATE/DIRECTORY DKB0:[W.A.X]\rCREATE/DIRECTORY DKB0:[W.B]\r",
                "COPY SYS$SYSDEVICE:[SYSMGR]WELCOME.TXT DKB0:[W]TOP.TMP\r",
                "COPY SYS$SYSDEVICE:[SYSMGR]WELCOME.TXT DKB0:[W.A]ONE.TMP\r",
                "COPY SYS$SYSDEVICE:[SYSMGR]WELCOME.TXT DKB0:[W.A.X]DEEP.TMP\r",
                "COPY SYS$SYSDEVICE:[SYSMGR]WELCOME.TXT DKB0:[W.B]TWO.TMP\r",
                "DIR DKB0:[W...]*.TMP\r",
            ),
        ),
        (
            "Grand total of 4 directories, 4 files.",
            1,
            "DIR DKB0:[W.*]\rDIR DKB0:[W...X]\rDIR DKB0:[*]SUB.DIR\r",
        ),
        (
            "$ DIR DKB0:[*]SUB.DIR",
            1,
            "DIR DKB0:[000000...]NOTE.TXT\rSET DEFAULT DKB0:[W.A]\rDIR [...]\r",
        ),
        (
            "Grand total of 2 directories, 3 files.",
            2,
            concat!(
                "A = F$SEARCH(\"DKB0:[W...]*.TMP\", 1)\r",
                "B = F$SEARCH(\"DKB0:[W]*.DIR\", 2)\r",
                "C = F$SEARCH(\"DKB0:[W...]*.TMP\", 1)\r",
                "WRITE SYS$OUTPUT \"STREAMS: \", A, \" \", B, \" \", C\r",
                "SET DEFAULT DKB0:[000000]\r",
            ),
        ),
        (
            "STREAMS: DKB0:",
            1,
            concat!(
                "COPY SYS$SYSDEVICE:[SYSMGR]WELCOME.TXT DKB0:[W]TOP.TMP\r",
                "COPY SYS$SYSDEVICE:[SYSMGR]WELCOME.TXT DKB0:[W]A_RATHER_LONG_NAME.TMP\r",
                "DIR DKB0:[W]\rDIR/COLUMNS=2 DKB0:[W]\rDIR/VERSIONS=1/COLUMNS=1 DKB0:[W]\r",
                "DIR/SIZE=ALL/OWNER/PROTECTION DKB0:[W]*.TMP\rDIR/TOTAL/SIZE DKB0:[W...]*.TMP\r",
                "DIR/DATE=(BACKUP,EXPIRED)/SIZE=ALLOCATION DKB0:[W]TOP.TMP;1\r",
                "DIR DKB0:[W]TOP.TMP;1,A.DIR\rDIR DKB0:[W.A]ONE.TMP,[W.B]TWO.TMP,NONE.TMP\r",
                "DIR DKB0:[W]NONE.TMP,NONE.DIR\rSHOW SYMBOL $STATUS\r",
                "DIR DKB0:[W]TOP.TMP,[W]TOP.TMP;1\rDIR DKB0:[W]A.DIR;1,TOP.TMP\r",
                "DIR DKB0:[W]TOP.TMP;1,[W.B],*.TMP\rDIR DKB0:[W...]ONE.TMP,TWO.TMP\r",
                "DIR DKB0:[W.A]ONE.TMP,[.W.B]*.TMP\rDIR/VERSIONS=1 DKB0:[W]TOP.TMP,[W]TOP.TMP\r",
                "DIR DKB0:[W]A.DIR,[NOSUCH]Y.TMP,TOP.TMP\rSHOW SYMBOL $STATUS\r",
                "DIR DKB0:[W]A.DIR,Y.TXT;ABC,TOP.TMP\rDIR DKB0:[W]A.DIR,NOSUCHDEV:Y\r",
                "DIR/VERSIONS=0\r",
            ),
        ),
        (
            "error parsing '0'",
            1,
            concat!(
                "COPY SYS$SYSDEVICE:[SYSMGR]WELCOME.TXT DKB0:[W]TOP.TMP\r",
                "COPY SYS$SYSDEVICE:[SYSMGR]WELCOME.TXT DKB0:[W]TOP.TMP\r",
                "PURGE/KEEP=0 DKB0:[W]\rPURGE DKB0:[W]*.TMP;*\r",
                "PURGE/LOG/KEEP=2 DKB0:[W...]*.TMP\rPURGE/LOG DKB0:[W...]*.TMP\r",
                "PURGE/LOG DKB0:[W...]*.TMP\r",
            ),
        ),
        (
            "no files purged",
            1,
            "DELETE/LOG DKB0:[W...]*.TMP;*\rDIR DKB0:[W...]*.TMP\r",
        ),
    ],
    lines: &[
        concat!(
            "Directory DKB0:[W]\n\nTOP.TMP;1\n\nTotal of 1 file.\n\n",
            "Directory DKB0:[W.A]\n\nONE.TMP;1\n\nTotal of 1 file.\n\n",
            "Directory DKB0:[W.A.X]\n\nDEEP.TMP;1\n\nTotal of 1 file.\n\n",
            "Directory DKB0:[W.B]\n\nTWO.TMP;1\n\nTotal of 1 file.\n\n",
            "Grand total of 4 directories, 4 files.",
        ),
        concat!(
            "$ DIR DKB0:[W.*]\n\n",
            "Directory DKB0:[W.A]\n\nONE.TMP;1           X.DIR;1\n\nTotal of 2 files.\n\n",
            "Directory DKB0:[W.B]\n\nTWO.TMP;1\n\nTotal of 1 file.\n\n",
            "Grand total of 2 directories, 3 files.",
        ),
        "$ DIR DKB0:[W...X]\n\nDirectory DKB0:[W.A.X]\n\nDEEP.TMP;1\n\nTotal of 1 file.\n",
        "$ DIR DKB0:[*]SUB.DIR\n\nDirectory DKB0:[000000]\n\nSUB.DIR;1\n\nTotal of 1 file.\n",
        concat!(
            "$ DIR DKB0:[000000...]NOTE.TXT\n\n",
            "Directory DKB0:[SUB.DEEP]\n\nNOTE.TXT;1\n\nTotal of 1 file.\n",
        ),
        concat!(
            "$ DIR [...]\n\n",
            "Directory DKB0:[W.A]\n\nONE.TMP;1           X.DIR;1\n\nTotal of 2 files.\n\n",
            "Directory DKB0:[W.A.X]\n\nDEEP.TMP;1\n\nTotal of 1 file.\n\n",
            "Grand total of 2 directories, 3 files.",
        ),
        "STREAMS: DKB0:[W]TOP.TMP;1 DKB0:[W]A.DIR;1 DKB0:[W.A]ONE.TMP;1",
        concat!(
            "A.DIR;1             A_RATHER_LONG_NAME.TMP;1                B.DIR;1\n",
            "TOP.TMP;2           TOP.TMP;1\n\nTotal of 5 files.",
        ),
        concat!(
            "$ DIR/COLUMNS=2 DKB0:[W]\n\nDirectory DKB0:[W]\n\n",
            "A.DIR;1             A_RATHER_LONG_NAME.TMP;1\n",
            "B.DIR;1             TOP.TMP;2\nTOP.TMP;1\n",
        ),
        "A.DIR;1\nA_RATHER_LONG_NAME.TMP;1\nB.DIR;1\nTOP.TMP;2\n\nTotal of 4 files.",
        concat!(
            "A_RATHER_LONG_NAME.TMP;1\n",
            "                              1/3           [SYSTEM]                         (RWED,RWED,RE,)\n",
            "TOP.TMP;2                     1/3           [SYSTEM]                         (RWED,RWED,RE,)\n",
            "TOP.TMP;1                     1/3           [SYSTEM]                         (RWED,RWED,RE,)\n",
            "\nTotal of 3 files, 3/9 blocks.",
        ),
        concat!(
            "$ DIR/TOTAL/SIZE DKB0:[W...]*.TMP\n\nDirectory DKB0:[W]\n\nTotal of 3 files, 3 blocks.\n\n",
            "Directory DKB0:[W.A]\n\nTotal of 1 file, 1 block.\n",
        ),
        "Grand total of 4 directories, 6 files, 6 blocks.",
        "TOP.TMP;1                     3   <None specified>   <No backup recorded>",
        concat!(
            "$ DIR DKB0:[W]TOP.TMP;1,A.DIR\n\nDirectory DKB0:[W]\n\n",
            "TOP.TMP;1           A.DIR;1\n\nTotal of 2 files.\n",
        ),
        concat!(
            "$ DIR DKB0:[W.A]ONE.TMP,[W.B]TWO.TMP,NONE.TMP\n\n",
            "Directory DKB0:[W.A]\n\nONE.TMP;1\n\nTotal of 1 file.\n\n",
            "Directory DKB0:[W.B]\n\nTWO.TMP;1\n\nTotal of 1 file.\n\n",
            "Grand total of 2 directories, 2 files.",
        ),
        "$ DIR DKB0:[W]NONE.TMP,NONE.DIR\n%DIRECT-W-NOFILES, no files found",
        "Hex = 10018290",
        concat!(
            "$ DIR DKB0:[W]TOP.TMP,[W]TOP.TMP;1\n\nDirectory DKB0:[W]\n\n",
            "TOP.TMP;2           TOP.TMP;1\n\nTotal of 2 files.\n\n",
            "Directory DKB0:[W]\n\nTOP.TMP;1\n\nTotal of 1 file.\n\n",
            "Grand total of 2 directories, 3 files.",
        ),
        concat!(
            "$ DIR DKB0:[W]A.DIR;1,TOP.TMP\n\nDirectory DKB0:[W]\n\n",
            "A.DIR;1             TOP.TMP;2\n\nTotal of 2 files.\n",
        ),
        concat!(
            "$ DIR DKB0:[W]TOP.TMP;1,[W.B],*.TMP\n\nDirectory DKB0:[W]\n\n",
            "TOP.TMP;1\n\nTotal of 1 file.\n\n",
            "Directory DKB0:[W.B]\n\nTWO.TMP;1\n\nTotal of 1 file.\n\n",
            "Grand total of 2 directories, 2 files.",
        ),
        concat!(
            "$ DIR DKB0:[W...]ONE.TMP,TWO.TMP\n\n",
            "Directory DKB0:[W.A]\n\nONE.TMP;1\n\nTotal of 1 file.\n\n",
            "Directory DKB0:[W.B]\n\nTWO.TMP;1\n\nTotal of 1 file.\n\n",
            "Grand total of 2 directories, 2 files.",
        ),
        concat!(
            "$ DIR DKB0:[W.A]ONE.TMP,[.W.B]*.TMP\n\n",
            "Directory DKB0:[W.A]\n\nONE.TMP;1\n\nTotal of 1 file.\n\n",
            "Directory DKB0:[W.B]\n\nTWO.TMP;1\n\nTotal of 1 file.\n\n",
            "Grand total of 2 directories, 2 files.",
        ),
        concat!(
            "$ DIR/VERSIONS=1 DKB0:[W]TOP.TMP,[W]TOP.TMP\n\n",
            "Directory DKB0:[W]\n\nTOP.TMP;2\n\nTotal of 1 file.\n\n",
            "Directory DKB0:[W]\n\nTOP.TMP;2\n\nTotal of 1 file.\n\n",
            "Grand total of 2 directories, 2 files.",
        ),
        concat!(
            "$ DIR DKB0:[W]A.DIR,[NOSUCH]Y.TMP,TOP.TMP\n\n",
            "Directory DKB0:[W]\n\nA.DIR;1\n\nTotal of 1 file.\n",
            "%DIRECT-E-OPENIN, error opening DKB0:[NOSUCH]Y.TMP;* as input\n",
            "-RMS-E-DNF, directory not found\n-SYSTEM-W-NOSUCHFILE, no such file\n",
            "%DIRECT-E-OPENIN, error opening DKB0:[NOSUCH]TOP.TMP;* as input\n",
            "-RMS-E-DNF, directory not found\n-SYSTEM-W-NOSUCHFILE, no such file\n",
        ),
        "Hex = 1001C04A",
        concat!(
            "$ DIR DKB0:[W]A.DIR,Y.TXT;ABC,TOP.TMP\n\nDirectory DKB0:[W]\n\n",
            "%DIRECT-E-OPENIN, error opening Y.TXT;ABC as input\n",
            "-RMS-F-SYN, file specification syntax error\n",
            "%DIRECT-E-OPENIN, error opening TOP.TMP as input\n",
            "-RMS-F-SYN, file specification syntax error\n",
            "A.DIR;1\n\nTotal of 1 file.\n",
        ),
        concat!(
            "$ DIR DKB0:[W]A.DIR,NOSUCHDEV:Y\n\nDirectory DKB0:[W]\n\n",
            "A.DIR;1\n\nTotal of 1 file.\n",
            "%DIRECT-E-OPENIN, error opening NOSUCHDEV:[W]Y.DIR;* as input\n",
            "-RMS-F-DEV, error in device name or inappropriate device type for operation\n",
        ),
        "$ DIR/VERSIONS=0\n%DIRECT-F-SYNTAX, error parsing '0'",
        "$ PURGE/KEEP=0 DKB0:[W]\n%PURGE-F-BADVALUE, '0' is an invalid keyword value",
        "$ PURGE DKB0:[W]*.TMP;*\n%PURGE-E-PURGEVER, version numbers not permitted",
        concat!(
            "$ PURGE/LOG/KEEP=2 DKB0:[W...]*.TMP\n",
            "%PURGE-I-FILPURG, DKB0:[W]TOP.TMP;2 deleted\n",
            "%PURGE-I-FILPURG, DKB0:[W]TOP.TMP;1 deleted\n",
            "%PURGE-I-TOTAL, 2 files deleted\n",
        ),
        "$ PURGE/LOG DKB0:[W...]*.TMP\n%PURGE-I-FILPURG, DKB0:[W]TOP.TMP;3 deleted\n$",
        "$ PURGE/LOG DKB0:[W...]*.TMP\n%PURGE-I-NOFILPURG, no files purged",
        concat!(
            "%DELETE-I-FILDEL, DKB0:[W]TOP.TMP;4 deleted\n",
            "%DELETE-I-FILDEL, DKB0:[W.A]ONE.TMP;1 deleted\n",
            "%DELETE-I-FILDEL, DKB0:[W.A.X]DEEP.TMP;1 deleted\n",
            "%DELETE-I-FILDEL, DKB0:[W.B]TWO.TMP;1 deleted\n",
        ),
        "$ DIR DKB0:[W...]*.TMP\n%DIRECT-W-NOFILES, no files found",
    ],
};

/// FORMS (t8y:/layered/decforms#ifdl) on DKB0:: FORMS TRANSLATE makes a
/// .FORM file of a form CREATE makes, which ods-image reads after the
/// session, and FORMS BACK_TRANSLATE makes IFDL of it again, which TYPE
/// shows; then a file that isn't there.
const FORMS: Phase = Phase {
    name: "forms",
    secs: 60,
    steps: &[(
        "%DIRECT-W-NOFILES, no files found",
        1,
        concat!(
            "CREATE FORMTEST.IFDL\r",
            "Form FORMTEST\rForm Data\rNAME Character(20)\rEnd Data\r",
            "Layout VT Device Terminal Type %VT100 End Device\rSize 24 Lines By 80 Columns\r",
            "Panel MAIN\rLiteral Text Line 1 Column 1 Value \"Name:\" End Literal\r",
            "Field NAME Line 1 Column 7 End Field\rEnd Panel\rEnd Layout\rEnd Form\r\x1a",
            "FORMS TRANSLATE/LOG FORMTEST\r",
            "FORMS BACK_TRANSLATE/LOG/OUTPUT=FORMBACK FORMTEST\r",
            "TYPE FORMBACK.IFDL\r",
            "FORMS TRANSLATE NOSUCH\r",
        ),
    )],
    lines: &[
        "%FORMS-S-TRANSLATED, file FORMTEST.IFDL translated to FORMTEST.FORM.",
        "%FORMS-S-BACKXLATED, file FORMTEST.FORM back-translated to FORMBACK.IFDL",
        "        Size 24 Lines By 80 Columns\n        Panel MAIN\n            Literal Text\n",
        "                Value \"Name:\"\n            End Literal\n            Field NAME\n",
        "%FORMS-F-OPENIN, error opening NOSUCH.IFDL as input.",
    ],
};

/// MEMTEST grows P0 to its end, 1 GB, and P1 to its start, below DCL, and
/// checks every page (ADR-0037); `MT P1`, with `MT :== $SYS$SYSTEM:MEMTEST`,
/// finds as much P1 again, since image rundown deleted the first run's.
/// `MT 4` makes four more processes do P0 at once, 5 GB, too long for here.
const MEMORY: Phase = Phase {
    name: "memory",
    secs: 120,
    steps: &[
        ("%FORMS-F-OPENIN", 1, "RUN SYS$SYSTEM:MEMTEST\r"),
        ("MEMTEST: ok", 1, "MT :== $SYS$SYSTEM:MEMTEST\rMT P1\r"),
    ],
    lines: &[
        "MEMTEST: P0 pages 0003F000\nMEMTEST: P1 pages 0001F000\nMEMTEST: ok",
        "$ MT P1\nMEMTEST: P1 pages 0001F000\nMEMTEST: ok",
    ],
};

/// CREATE/MAILBOX/LOG makes a temporary mailbox, which outlives CREATE on
/// DCL's channel: its logical name is there after, as is a permanent
/// one's with /MESSAGE_SIZE.
const MAILBOXES: Phase = Phase {
    name: "mailboxes",
    secs: 10,
    steps: &[(
        "MEMTEST: ok",
        2,
        concat!(
            "CREATE/MAILBOX/LOG MY_MBX\rSHOW LOGICAL MY_MBX\r",
            "CREATE/MAILBOX/PERMANENT/MESSAGE_SIZE=512 PRM_MBX\rSHOW LOGICAL PRM_MBX\r",
        ),
    )],
    lines: &[
        "$ CREATE/MAILBOX/LOG MY_MBX\n%CREATE-I-CREATED, MB",
        "\"MY_MBX\" = \"_MB",
        "\"PRM_MBX\" = \"_MB",
    ],
};

/// RUN's qualifiers for a process of its own are RUNDET's: an image that
/// isn't there, a privilege or a UIC it doesn't know, fail before
/// $CREPRC; then DCL in a new process, with a name, a priority, the UIC
/// [1,5], no privileges but two and its own I/O quotas, IO_DIRECT's
/// raised to the least $CREPRC gives, runs a procedure CREATE made, on
/// the console, and logs out.
const DETACHED: Phase = Phase {
    name: "detached",
    secs: 20,
    steps: &[(
        "\"PRM_MBX\" = \"_MB",
        1,
        concat!(
            "RUN/DETACHED NOSUCH\rRUN/PRIVILEGES=XYZZY SYS$SYSTEM:HOG\r",
            "RUN/UIC=[1,9] SYS$SYSTEM:HOG\r",
            "CREATE MDA0:[000000]DETACH.COM\r",
            "$ WRITE SYS$OUTPUT \"DETACH: \", F$GETJPI(\"\",\"PRCNAM\"), \" \", F$GETJPI(\"\",\"UIC\"), \" \", F$GETJPI(\"\",\"PRIB\"), \" \", F$GETJPI(\"\",\"CURPRIV\")\r",
            "$ SHOW PROCESS/QUOTAS\r$ WRITE SYS$OUTPUT \"DETACH: done\"\r\x1a",
            "RUN/PROCESS_NAME=DETACHER/PRIORITY=3/UIC=[1,5]/PRIVILEGES=(NOSAME,NETMBX,TMPMBX)",
            "/INPUT=MDA0:[000000]DETACH.COM/OUTPUT=OPA0:/ERROR=OPA0:",
            "/IO_BUFFERED=7/IO_DIRECT=2/BUFFER_LIMIT=%X10000 SYS$SYSTEM:DCL\r",
        ),
    )],
    lines: &[
        "%RUN-F-PARSEFAIL, error parsing NOSUCH\n-RMS-E-FNF, file not found\n",
        "%RUN-E-INVQUAVAL, value 'XYZZY' invalid for /PRIVILEGES qualifier\n",
        "%RUN-F-INVUIC, [1,9] is an invalid UIC specification\n",
        "DETACH: DETACHER [1,5] 3 TMPMBX,NETMBX\n",
        "Direct I/O limit:          4\n Buffered I/O byte count quota:     65536  Buffered I/O limit:        7\nDETACH: done",
    ],
};

/// SPAWN makes SYSTEM_1, a subprocess that runs DCL with a copy of the
/// symbols and process logical names, SYS$ERROR the terminal again once it
/// has read them, which reads the terminal until LOGOUT, and control
/// returns to SYSTEM; one named SPAWNER with a prompt of its own; from a
/// procedure, with its local symbols, and a command's status, which the
/// subprocess's final status makes $STATUS; /INPUT that isn't there, and
/// one that is, after the command; and /NOWAIT.
const SPAWN: Phase = Phase {
    name: "spawn",
    secs: 30,
    steps: &[
        (
            "DETACH: done",
            1,
            concat!(
                "SPWNX = 7\rSPWNY == \"glob\"\rDEFINE SPWNFOO BAR\rSPAWN\r",
                "SHOW SYMBOL SPWNX\rSHOW SYMBOL SPWNY\rSHOW LOGICAL SPWNFOO\rSHOW LOGICAL SYS$ERROR\r",
                "WRITE SYS$OUTPUT \"SPAWNED: \", F$GETJPI(\"\",\"PRCNAM\"), \" \", F$MODE()\rLOGOUT\r",
            ),
        ),
        (
            "  Process SYSTEM_1 logged out at ",
            1,
            concat!(
                "SPAWN/NOLOG/PROCESS=SPAWNER/PROMPT=\"SPAWNER$ \"\r",
                "WRITE SYS$OUTPUT F$GETJPI(\"\",\"PRCNAM\")\rLOGOUT\r",
                "CREATE MDA0:[000000]SPAWN.COM\r$ SPWNL = 5\r$ SPAWN/NOLOG SHOW SYMBOL SPWNL\r",
                "$ SPAWN/NOLOG EXIT 5\r$ WRITE SYS$OUTPUT \"SPAWN: status \", $STATUS\r\x1a",
                "@MDA0:[000000]SPAWN\rSPAWN/NOLOG/INPUT=NOSUCH\r",
                "CREATE MDA0:[000000]SPAWNIN.COM\r$ WRITE SYS$OUTPUT \"SPAWN: from the file\"\r\x1a",
                "SPAWN/NOLOG/INPUT=MDA0:[000000]SPAWNIN WRITE SYS$OUTPUT \"SPAWN: first\"\r",
                "SPAWN/NOWAIT WRITE SYS$OUTPUT \"SPAWN: \", F$GETJPI(\"\",\"PRCNAM\"), \" nowait\"\r",
            ),
        ),
    ],
    lines: &[
        "%DCL-S-SPAWNED, process SYSTEM_1 spawned\n%DCL-S-ATTACHED, terminal now attached to process SYSTEM_1\n",
        "  SPWNX = 7   Hex = 00000007",
        "  SPWNY == \"glob\"",
        "\"SPWNFOO\" = \"BAR\" (LNM$PROCESS_TABLE)",
        "\"SYS$ERROR\" = \"_OPA0:\" (LNM$PROCESS_TABLE)",
        "SPAWNED: SYSTEM_1 INTERACTIVE",
        "%DCL-S-RETURNED, control returned to process SYSTEM\n",
        "SPAWNER$ WRITE SYS$OUTPUT F$GETJPI(\"\",\"PRCNAM\")\nSPAWNER\n",
        "  Process SPAWNER logged out at ",
        "  SPWNL = 5   Hex = 00000005",
        "SPAWN: status 5\n",
        "$ SPAWN/NOLOG/INPUT=NOSUCH\n%RMS-E-FNF, file not found",
        "SPAWN: first\nSPAWN: from the file\n",
        "%DCL-S-SPAWNED, process SYSTEM_1 spawned\n",
        "SPAWN: SYSTEM_1 nowait",
    ],
};

/// MOUNT/SYSTEM/OVERRIDE=IDENTIFICATION conflicts; MOUNT without a label
/// asks for one, and without /SYSTEM mounts the volume for the job: the
/// disk is allocated, DISK$DATA is in the process's table, and another
/// job, a process RUN/UIC makes, can't list it, dismount it or mount it;
/// DISMOUNT deletes the name. MOUNT/NOWRITE write-locks it, so a COPY
/// there fails; MOUNT/OVERRIDE=IDENTIFICATION mounts it whatever label it
/// is given, defining WORK as well as DISK$DATA. A process of its own that
/// mounts it and goes leaves it dismounted. Then MOUNT/SYSTEM, for USERS.
const MOUNTS: Phase = Phase {
    name: "mounts",
    secs: 30,
    steps: &[
        (
            "SPAWN: SYSTEM_1 nowait",
            1,
            "DISMOUNT DKB0:\rMOUNT/SYSTEM/OVERRIDE=IDENTIFICATION DKB0:\rMOUNT DKB0:\r",
        ),
        (
            "_Label: ",
            1,
            concat!(
                "DATA\rSHOW DEVICES DKB0\rSHOW LOGICAL DISK$DATA\r",
                "CREATE MDA0:[000000]OTHER.COM\r$ SET NOON\r",
                "$ WRITE SYS$OUTPUT \"OTHER: \", \"dir\"\r$ DIRECTORY DKB0:[000000]\r",
                "$ WRITE SYS$OUTPUT \"OTHER: \", \"dismount\"\r$ DISMOUNT DKB0:\r",
                "$ WRITE SYS$OUTPUT \"OTHER: \", \"mount\"\r$ MOUNT DKB0: DATA\r",
                "$ WRITE SYS$OUTPUT \"OTHER: \", \"done\"\r\x1a",
                "RUN/UIC=[1,5]/INPUT=MDA0:[000000]OTHER.COM/OUTPUT=OPA0:/ERROR=OPA0: SYS$SYSTEM:DCL\r",
            ),
        ),
        (
            "OTHER: done",
            1,
            concat!(
                "DISMOUNT DKB0:\rSHOW LOGICAL/PROCESS DISK$DATA\r",
                "MOUNT/SYSTEM/NOWRITE DKB0: DATA\rSHOW DEVICES DKB0\r",
                "COPY SYS$SYSDEVICE:[SYSMGR]WELCOME.TXT DKB0:[000000]RO.TXT\rDISMOUNT DKB0:\r",
                "MOUNT/OVERRIDE=IDENTIFICATION DKB0: XYZ WORK\rSHOW LOGICAL WORK\rDISMOUNT DKB0:\r",
                "WRITE SYS$OUTPUT \"MOUNTS: \", \"mounter\"\r",
            ),
        ),
        (
            "MOUNTS: mounter",
            1,
            concat!(
                "CREATE MDA0:[000000]MOUNTER.COM\r$ MOUNT DKB0: DATA\r$ SHOW DEVICES DKB0\r\x1a",
                "CREATE MDA0:[000000]GONE.COM\r$ AGAIN:\r$ IF F$GETDVI(\"DKB0:\", \"MNT\") THEN GOTO AGAIN\r",
                "$ SHOW DEVICES DKB0\r\x1a",
                "RUN/UIC=[1,5]/INPUT=MDA0:[000000]MOUNTER.COM/OUTPUT=OPA0:/ERROR=OPA0: SYS$SYSTEM:DCL\r",
            ),
        ),
        // The first was SYSTEM's own SHOW DEVICES.
        (
            "Mounted alloc",
            2,
            "@MDA0:[000000]GONE\rMOUNT/SYSTEM DKB0: DATA\r",
        ),
    ],
    lines: &[
        "%MOUNT-F-CONFQUAL, conflicting qualifiers",
        "DKB0:                   Mounted alloc        0  DATA",
        "\"DISK$DATA\" = \"_DKB0:\" (LNM$PROCESS_TABLE)",
        "OTHER: dir\n%DIRECT-E-OPENIN, error opening DKB0:[000000]*.*;* as input\n-RMS-E-DNR, device not ready, not mounted, or unavailable",
        "OTHER: dismount\n%SYSTEM-W-DEVALLOC, device already allocated to another user",
        "OTHER: mount\n%SYSTEM-W-DEVALLOC, device already allocated to another user",
        "$ SHOW LOGICAL/PROCESS DISK$DATA\n%SHOW-S-NOTRAN, no translation for logical name DISK$DATA",
        "DKB0:                   Mounted wrtlck       0  DATA",
        "DKB0:[000000]RO.TXT\n%RMS-E-WLK, device currently write locked",
        "\"WORK\" = \"_DKB0:\" (LNM$PROCESS_TABLE)",
        "DKB0:                   Online               0",
    ],
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
        "GBLTEST: ok",
        "PTDTEST: ok",
        "LOCKTEST: ok",
        "%SYSTEM-W-NOSIGNAL, no signal currently active",
        "%SYSTEM-F-NOSIGNAL, no signal currently active",
        "CHFTEST: ok",
        "QUOTATEST: a read past BIOLM waited for the quota",
        "QUOTATEST: ok",
        "%SYSTEM-F-ACCVIO, access violation, reason mask=00, virtual address=40010000, PC=",
        "%SYSTEM-F-OPCDEC, opcode reserved to DIGITAL fault at PC=",
        "process SNOOP exited with status 1000000C",
        "process USURP exited with status 1000043C",
    ],
};

/// AUTHORIZE adds JOE, in another UIC group, with a few privileges and
/// his default directory on DKB0:, gives HELP SHOW from UAFHELP.HLB, and
/// Return twice, shows him, whole and on a line, and
/// lists every user in SYSUAF.LIS; SYSTEM adds the identifier PAYROLL,
/// grants it to JOE and gives a file an ACE that lets PAYROLL read it,
/// which the world may not; SYSTEM changes its password with SET
/// PASSWORD, after getting the old one wrong, defines SYS$WELCOME, which
/// each login after says in place of Welcome to vaxpunk, and logs out. JOE logs in,
/// stops SET PASSWORD with CTRL/Y, where the privileges are JOE's
/// permanent ones, not SETPWD's SYSPRV, and continues it, is shown
/// with his UIC, privileges, directory and rights, PAYROLL among them,
/// lists his process's logical names without CMKRNL,
/// reads the file, can't run AUTHORIZE or
/// OPCCRASH, nor DEFINE/SYSTEM or make a table in the system's directory,
/// but can in his own, spawns JOE_1, whose LOGINOUT, a known image, gives
/// it his SYS$LOGIN, and logs out; SYSTEM logs in with its new password, and
/// PHNTEST logs JOE in on a pseudo terminal: SYSTEM's PHONE rings him, he
/// answers, each says something, SYSTEM lists who can be called with
/// %DIRECTORY, reads %HELP DIAL from PHONEHELP.HLB, hangs up and leaves
/// PHONE, and JOE's PHONE sees the hang
/// up; then SYSTEM shuts the system down.
const USERS: Phase = Phase {
    name: "users",
    secs: 90,
    steps: &[
        (
            "process USURP exited with status 1000043C",
            1,
            concat!(
                "MCR AUTHORIZE ADD JOE /PASSWORD=joespw /UIC=[200,1] /DEVICE=DKB0",
                " /DIRECTORY=[000000] /PRIVILEGES=(TMPMBX,OPER,NETMBX) /OWNER=JOE /ACCOUNT=USERS\r",
                "MCR AUTHORIZE ADD JOE\r",
            ),
        ),
        ("%UAF-W-UAEERR", 1, "MCR AUTHORIZE\rHELP SHOW\r"),
        ("SHOW Subtopic?", 1, "\r"),
        // The first three were HELP's in CONTROL_KEYS and LBRTEST's.
        (
            "Topic?",
            4,
            concat!(
                "\rSHOW JOE\rMODIFY JOE /DEFPRIVILEGES=(NOALL,TMPMBX,OPER)\r",
                "SHOW/BRIEF *\rLIST\rEXIT\rTYPE SYSUAF.LIS\r",
            ),
        ),
        (
            "%UAF-I-LSTMSG2",
            1,
            concat!(
                "MCR AUTHORIZE ADD/IDENTIFIER PAYROLL\rMCR AUTHORIZE GRANT/IDENTIFIER PAYROLL JOE\r",
                "MCR AUTHORIZE SHOW/IDENTIFIER/FULL PAYROLL\rMCR AUTHORIZE SHOW/RIGHTS JOE\r",
                "OPEN/WRITE F DKB0:[000000]PAY.TXT\rWRITE F \"PAYROLL: \", \"readable\"\rCLOSE F\r",
                "SET SECURITY/PROTECTION=(W)/ACL=(IDENTIFIER=PAYROLL,ACCESS=READ) DKB0:[000000]PAY.TXT\r",
                "DISMOUNT DKB0:\rMOUNT/SYSTEM DKB0: DATA\r",
                "SET PASSWORD\rwrongpw\rSET PASSWORD\rmanager\rnewpw\rnewpw\r",
                "DEFINE/SYSTEM SYS$WELCOME \"Have a nice day\"\rLOGOUT\r",
            ),
        ),
        ("  SYSTEM       logged out at ", 1, "\r"),
        // AUTHORIZE's listings have Username: too: LOGINOUT's ANNOUNCE.TXT counts.
        ("Log in as SYSTEM, password MANAGER", 2, "JOE\r"),
        ("Password:", 4, "joespw\r"),
        // The first two were TTSIZE's at SYSTEM's login and SET TERMINAL/INQUIRE's.
        ("\x1b[6n", 3, "\x1b[24;80R"),
        ("Have a nice day", 2, "SET PASSWORD\r"),
        ("Old password:", 3, "\x19"),
        // The first nine were CONTROL_KEYS'.
        (
            "*INTERRUPT*",
            10,
            concat!(
                "WRITE SYS$OUTPUT \"At CTRL/Y: \", F$GETJPI(\"\", \"CURPRIV\")\rCONTINUE\r",
                "SHOW PROCESS/PRIVILEGES\rSHOW DEFAULT\rSHOW LOGICAL SYS$LOGIN\rSHOW LOGICAL/PROCESS\r",
                "SHOW PROCESS/RIGHTS\rTYPE DKB0:[000000]PAY.TXT\r",
                "MCR AUTHORIZE SHOW JOE\rMCR OPCCRASH\r",
            ),
        ),
        (
            "%SYSTEM-F-NOPRIV",
            1,
            concat!(
                "DEFINE/SYSTEM/NOLOG QQQ JOE\r",
                "CREATE/NAME_TABLE/PARENT=LNM$SYSTEM_DIRECTORY JOETAB\r",
                "CREATE/NAME_TABLE JOETAB\rDEFINE/TABLE=JOETAB A B\rSHOW LOGICAL/TABLE=JOETAB A\r",
                "SPAWN/NOLOG WRITE SYS$OUTPUT \"SPAWN: \", F$GETJPI(\"\",\"PRCNAM\"), \" \", F$TRNLNM(\"SYS$LOGIN\")\r",
                "LOGOUT\r",
            ),
        ),
        ("  JOE          logged out at ", 1, "\r"),
        ("Log in as SYSTEM, password MANAGER", 3, "SYSTEM\r"),
        ("Password:", 5, "newpw\r"),
        ("\x1b[6n", 4, "\x1b[50;100R"),
        (
            "Have a nice day",
            3,
            "SHOW PROCESS\rSHOW TERMINAL\rRUN PHNTEST\r",
        ),
        ("PHNTEST: JOE logged in", 1, "PHONE JOE\r"),
        ("That person has answered your call.", 1, "hello\r"),
        ("hi there", 1, "%DIRECTORY\r"),
        // A key ends the listing, and the help; CTRL/Z hangs up, and
        // again leaves PHONE.
        ("2 persons listed.", 1, "x%HELP DIAL\r"),
        ("a node name, as in NODE::USER, is refused", 1, "x\x1a\x1a"),
        ("PHNTEST: ok", 1, "SHUTDOWN\r"),
    ],
    lines: &[
        "%UAF-I-ADDMSG, user record successfully added",
        "  Shows a user's record from the system authorization file, or a line",
        "SHOW Subtopic?",
        "%UAF-W-UAEERR, invalid user name, user name already exists",
        "Username: JOE                              Owner:  JOE",
        "Account:  USERS                            UIC:    [200,1]",
        "Default:  DKB0:[000000]",
        "Authorized Privileges: \n  TMPMBX       OPER         NETMBX",
        "%UAF-I-MDFYMSG, user record(s) updated",
        "%UAF-I-RDBADDMSGU, identifier JOE value [000200,000001] added to rights database",
        "%UAF-I-RDBADDMSG, identifier PAYROLL value %X80010000 added to rights database",
        "%UAF-I-GRANTMSG, identifier PAYROLL granted to JOE",
        "  PAYROLL                          %X80010000      \n    Holder                           Attributes\n    JOE",
        "Identifier                         Value           Attributes\n  PAYROLL                          %X80010000",
        "Process rights:\n JOE                               resource\n INTERACTIVE",
        " LOCAL                             \n PAYROLL",
        "PAYROLL: readable",
        " JOE                   JOE             [200,1]       USERS      Normal   4 DKB0:[000000]",
        " SYSTEM MANAGER        SYSTEM          [1,4]         SYSTEM     All      4 SYS$SYSROOT:[SYSMGR]",
        "%UAF-I-LSTMSG2, listing file SYSUAF.LIS complete",
        "%SET-E-PWDNOTVAL, old password validation error - password not changed",
        "  SYSTEM       logged out at ",
        "At CTRL/Y: TMPMBX,OPER\n",
        "Process privileges:\n TMPMBX       OPER\n",
        "UIC:                [200,1]",
        "\n  DKB0:[000000]\n",
        "\"SYS$LOGIN\" = \"DKB0:[000000]\" (LNM$PROCESS_TABLE)",
        "  \"SYS$INPUT\" = \"_OPA0:\"\n  \"SYS$LOGIN\" = \"DKB0:[000000]\"\n  \"SYS$LOGIN_DEVICE\"",
        "%UAF-E-NAOFIL, unable to open system authorization file (SYSUAF.DAT)",
        "$ DEFINE/SYSTEM/NOLOG QQQ JOE\n%SYSTEM-F-NOPRIV",
        "$ CREATE/NAME_TABLE/PARENT=LNM$SYSTEM_DIRECTORY JOETAB\n%SYSTEM-F-NOPRIV",
        "   \"A\" = \"B\" (JOETAB)",
        "SPAWN: JOE_1 DKB0:[000000]",
        "  JOE          logged out at ",
        "Process name:       \"SYSTEM\"",
        // TTSIZE's, from the size the console said.
        "   Input:    9600     LFfill:  0      Width: 100      Parity: None",
        "   Output:   9600     CRfill:  0      Page:   50",
        " vaxpunk Phone Facility ",
        "Ringing JOE...",
        "Process Name    User Name       Terminal        Phone Status",
        "SHUTDOWN -- Perform an Orderly System Shutdown",
        "%PAL-I-POWEROFF",
    ],
};

const PHASES: &[Phase] = &[
    BOOT,
    SYSTEM_DISK,
    RAMDISK,
    DATA_DISK,
    CONTROL_KEYS,
    STOP_ID,
    OPERATOR,
    PRIVILEGES,
    PROTECTION,
    ACLS,
    SHARING,
    SET_FILE,
    TERMINAL,
    TERMINAL_TYPES,
    BACKUP,
    RMS,
    TPU,
    C,
    LBR,
    WILDCARDS,
    FORMS,
    MEMORY,
    MAILBOXES,
    DETACHED,
    SPAWN,
    MOUNTS,
    STARTUP,
    USERS,
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
    // FORMS's form has nothing wrong with it.
    "%FORMS-E-",
    "%FORMS-W-",
    // The passwords, which aren't echoed.
    "nobodys",
    "wrongpw",
    "manager",
    "newpw",
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

/// The log without its escape sequences, as a screen program such as PHONE
/// shows it: what it writes a character at a time, moving the cursor
/// between, reads as a whole.
fn screen_text(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\x1b' {
            out.push(c);
        } else if chars.next() == Some('[') {
            for c in chars.by_ref() {
                if c.is_ascii_alphabetic() {
                    break;
                }
            }
        }
    }
    out
}

/// How many times its `secs` a phase may take. CI's x86 runners emulate
/// the arm64 guest, and a phase there can take twice as long as on another
/// run: the system disk took 18 to 28 s of its 30 there, or ran out. A hang
/// still stalls, only later.
const SLACK: u64 = 3;

/// Runs the phases in order; Err says where it stalled.
fn session(log: impl Fn() -> String, mut type_: impl FnMut(&str)) -> Result<(), String> {
    for phase in PHASES {
        let start = Instant::now();
        let mut step = 0;
        loop {
            let text = log();
            let plain = screen_text(&text);
            let seen = |line: &str| {
                let count = |t: &str| t.lines().filter(|l| l.contains(line)).count();
                count(&text).max(count(&plain))
            };
            if let Some(&(line, times, keys)) = phase.steps.get(step)
                && seen(line) >= times
            {
                if keys.contains("{TELNET}") {
                    type_(&keys.replace("{TELNET}", telnet_pid(&text)));
                } else {
                    type_(keys);
                }
                step += 1;
                continue;
            }
            // STARTUP's processes print when they get there, which can be
            // in the middle of a command's lines: without theirs too. So
            // can DCL's prompt, after a RUN whose process prints on OPA0:
            // too: without the prompts that start a line.
            let unprompted = |s: &str| -> String {
                s.lines()
                    .map(|l| l.strip_prefix("$ ").unwrap_or(l))
                    .flat_map(|l| [l, "\n"])
                    .collect()
            };
            let quiet = unprompted(
                &plain
                    .lines()
                    .filter(|l| !STARTUP.lines.iter().any(|s| l.contains(s)))
                    .flat_map(|l| [l, "\n"])
                    .collect::<String>(),
            );
            let missing: Vec<_> = phase
                .lines
                .iter()
                .filter(|l| {
                    !text.contains(*l)
                        && !plain.contains(*l)
                        && !quiet.contains(unprompted(l).trim_end_matches('\n'))
                })
                .collect();
            if step == phase.steps.len() && missing.is_empty() {
                println!("{}: {:?}", phase.name, start.elapsed());
                break;
            }
            if start.elapsed() > Duration::from_secs(SLACK * phase.secs)
                || text.contains("root task done")
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
            .args(["--p1=TEST", "--p8=8"])
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
        3,
        "JOE's OPCCRASH, DEFINE/SYSTEM and table in LNM$SYSTEM_DIRECTORY"
    );
    assert_eq!(
        text.matches("%RMS-E-PRV").count(),
        5,
        "PROTTEST before SET PROTECTION=(W:R), after MOUNT/PROTECTION=(W), with no ACE \
         and with one denying its group, JOE's AUTHORIZE"
    );
    assert_eq!(
        text.matches("PROTTEST: [200,1] read").count(),
        4,
        "PROTTEST by the world's access, the owner's, its UIC's ACE and *'s"
    );
    assert_eq!(
        text.matches("*INTERRUPT*").count(),
        10,
        "CTRL/Y or CTRL/C interrupted after SET NOCONTROL"
    );
    assert_eq!(
        text.matches("This file is SYS$MANAGER:WELCOME.TXT").count(),
        3,
        "TYPE WELCOME.TXT and the two TYPE/PAGE, and not TYPE/HEAD=3's 4th line"
    );
    assert_eq!(
        (
            text.matches("DIRECTORY lists the files in").count(),
            text.matches("and DIRECTORY [000000] the volume").count()
        ),
        (2, 4),
        "TYPE/TAIL=2 WELCOME.TXT: its 9th line, not its 8th; TYPE/PAGE's \
         RETURN wrote both, its CTRL/Z neither"
    );
    assert!(
        !text.contains("SPIN: a register changed"),
        "SPIN's registers changed"
    );

    // RMSTEST's output must be OpenVMS's, and CDEMO's what vrun printed.
    for (program, file) in [
        ("RMSTEST", "vms/sysexe/rmstest.out"),
        ("CDEMO", "crosstools/vtools/tests/examples/c/cdemo.stdout"),
    ] {
        let want = fs::read_to_string(root.join(file)).unwrap();
        let first = want.lines().next().unwrap();
        let got: Vec<_> = text[text.find(first).expect(program)..]
            .lines()
            .take(want.lines().count())
            .collect();
        for (n, (w, g)) in want.lines().zip(&got).enumerate() {
            assert_eq!(w, *g, "{program}'s line {} isn't {file}'s", n + 1);
        }
    }

    // DKB0:'s free blocks: once the volume is new, then what the writes
    // left in the VCB's count, then what the bitmap says after a MOUNT.
    let free: Vec<u64> = text
        .lines()
        .filter(|l| l.starts_with("DKB0:") && l.contains(" DATA "))
        .take(3)
        .map(|l| l.split_whitespace().last().unwrap().parse().unwrap())
        .collect();
    assert!(
        free.len() == 3 && free[1] < free[0] && free[1] == free[2],
        "DKB0:'s free blocks {free:?}"
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
    assert_eq!(data.matches("Welcome to vaxpunk",).count(), 2, "{data}");
    assert!(img.lookup("[SUB.DEEP]DEEP.TXT").is_ok());
    assert!(
        img.lookup("[000000]SHARE.TXT").is_err(),
        "SHARE.TXT deleted"
    );
    let fid = img.lookup("[SUB.DEEP]NOTE.TXT").unwrap();
    let mut note = Vec::new();
    img.copy_out(fid, &mut note, Conversion::RecordsToLines)
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&note), "First line\nSecond line\n");
    // COPY/OVERLAY cut WELCOME.TXT's records from Q.TXT;1 and wrote NOTE.TXT's.
    let fid = img.lookup("[CQ]Q.TXT;1").unwrap();
    let mut q = Vec::new();
    img.copy_out(fid, &mut q, Conversion::RecordsToLines)
        .unwrap();
    assert_eq!(q, note);
    for name in ["DATA.TXT;1", "NEW.TXT;1", "NEW.TXT;2"] {
        let saved = img.lookup(&format!("[000000]{name}")).unwrap();
        let restored = img.lookup(&format!("[RESTORED]{name}")).unwrap();
        let (saved, restored) = (bytes(&mut img, saved), bytes(&mut img, restored));
        assert_eq!(saved, restored, "[RESTORED]{name}");
    }
    assert_eq!(
        img.lookup("[SPILL]V.TXT").ok(),
        img.lookup("[SPILL]V.TXT;69").ok()
    );
    assert!(img.lookup("[SPILL]V.TXT;1").is_ok() && img.lookup("[SPILL]V.TXT;70").is_err());
    for file in ["[RMS]REL.REL", "[RMS]RELF.REL"] {
        let fid = img.lookup(file).unwrap();
        let report = img.check_file(fid).unwrap();
        assert!(report.is_sound(), "{file}: {:?}", report.findings);
    }
    let fid = img.lookup("[000000]TXT.BCK").unwrap();
    save_set(&bytes(&mut img, fid));
    let fid = img.lookup("[LBR]LBRTEST.OLB").unwrap();
    lbrtest_library(&bytes(&mut img, fid));
    // FORMS's .FORM file: its bytes in FIX 512 records, the last padded.
    let fid = img.lookup("[000000]FORMTEST.FORM").unwrap();
    let form = bytes(&mut img, fid);
    assert!(form.starts_with(b"VAXPUNK FORM"), "{:?}", &form[..16]);
    let used = u32::from_le_bytes(form[16..20].try_into().unwrap()) as usize;
    assert!(
        used <= form.len() && form.len() % 512 == 0,
        "{used} of {}",
        form.len()
    );
}

/// The object library LBRTEST made, as vlink and vlib read one: HELLO,
/// with its records and its 60 symbols, and WORLD, with one.
fn lbrtest_library(file: &[u8]) {
    let lib = vms_obj::olb::Library::parse(file).expect("LBRTEST.OLB");
    let names: Vec<_> = lib.modules.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(names, ["HELLO", "WORLD"]);
    let (hello, world) = (&lib.modules[0], &lib.modules[1]);
    assert_eq!(hello.object, b"\0\0\x08\0MHD!\x01\0\x06\0GS\x02\0\x04\0");
    assert_eq!(world.object, b"\0\0\x06\0WD");
    let symbols: Vec<_> = (0..60)
        .map(|i| format!("LBRTEST_SYMBOL_{i:03}_WITH_A_LONG_NAME_FOR_THE_INDEX"))
        .collect();
    assert_eq!(hello.symbols, symbols);
    assert_eq!(world.symbols, ["WORLD_SYMBOL"]);
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
