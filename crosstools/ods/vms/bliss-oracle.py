#!/usr/bin/env python3
"""The BLISS oracle (PRD-0004): compiles BLISS programs with DEC's BLISSA64
on OpenVMS Alpha in AXPbox, runs them, and writes back what it said.

usage: bliss-oracle.py [--log LOG] FILE...

Each FILE.B64 is compiled with
    BLISS/A64/LIST/SOURCE_LIST=(EXPAND_MACROS,REQUIRE)/NOMACHINE_CODE
and each FILE.B32 the same with /A32 in place of /A64,
plus the qualifiers on its first line if it starts with `! BLISS:` (as
`! BLISS: /A32/VARIANT=3`), linked and run; a line `! LINK: OTHER` links
OTHER's object in too, from a program earlier on the command line. Other files (.R64, .REQ...) are
only copied in, for REQUIRE, unless their first line asks for /LIBRARY:
those are compiled first, `BLISS/A64/LIBRARY` (`/A32` for .R32 and .REQ),
for LIBRARY. Next to each program it writes:

    FILE.lis     the listing, its dates blanked
    FILE.stdout  what the program printed (LIB$PUT_OUTPUT's lines)
    FILE.oracle  the compiler's and linker's messages and the exit status
                 (for a library, the compiler's messages and status)

All files go in one boot of the system disk at $BLISS_ORACLE_DISK (default
~/Library/Caches/vaxpunk/bliss-oracle/golden-sys.img), an OpenVMS Alpha
system with BLISSA64 installed and SYSTEM's password "system": a clone, so
the disk itself never changes (run-vms.py --system). The files go into
[ORACLE] on a clone of that clone with ods (`$ODS`, by default the
workspace's target/release/ods), with ORACLE.COM, the commands, so that
one line is typed at the console, which loses characters of long input.
One way to make the disk is
an APFS clone of the playground's, while that VM is down:
    cp -c real_vms_playground/sys.img ~/Library/Caches/vaxpunk/bliss-oracle/golden-sys.img
AXPbox hangs in about one run in six; a run that fails is retried twice.
The console log goes to logs/; --log LOG reads the files' results from a
log instead of running them again.
"""
import os, re, subprocess, sys, tempfile, time

HERE = os.path.dirname(os.path.abspath(__file__))
DISK = os.environ.get("BLISS_ORACLE_DISK", os.path.expanduser(
    "~/Library/Caches/vaxpunk/bliss-oracle/golden-sys.img"))
BLISS = "BLISS/A64/LIST/SOURCE_LIST=(EXPAND_MACROS,REQUIRE)/NOMACHINE_CODE"
PROGRAMS = (".B64", ".B32")
ODS = os.environ.get("ODS", os.path.join(HERE, "..", "..", "..", "target", "release", "ods"))
MARK = "@@ORACLE"


def mark(what, name):
    # Split so that the echo of the command doesn't match.
    return f'WRITE SYS$OUTPUT "@@","ORACLE {what} {name}"'


def qualifiers(path):
    """The qualifiers on a file's first line, `! BLISS: /A32/VARIANT=3`."""
    with open(path, encoding="latin-1") as f:
        first = f.readline()
    return first.split(":", 1)[1].strip() if first.upper().startswith("! BLISS:") else ""


def script(files):
    """ORACLE.COM's commands: everything but starting it, which is the only
    line typed at the console."""
    # NOON: a program that fails doesn't end the procedure.
    cmds = ["SET NOON", "SET DEFAULT SYS$SYSDEVICE:[ORACLE]"]
    # Libraries first: a require file whose first line asks for /LIBRARY.
    for path in files:
        name, ext = os.path.splitext(os.path.basename(path))
        extra = qualifiers(path)
        if ext.upper() in PROGRAMS or "/LIBRARY" not in extra.upper():
            continue
        dialect = "/A32" if ext.upper() in (".R32", ".REQ") else "/A64"
        cmds += [mark("BEGIN LIB", name),
                 f"BLISS{dialect}{extra} {name}{ext.upper()}",
                 'WRITE SYS$OUTPUT "BLISS status ", $STATUS',
                 mark("END", name)]
    for path in files:
        name, ext = os.path.splitext(os.path.basename(path))
        if ext.upper() not in PROGRAMS:
            continue
        extra = qualifiers(path)
        bliss = BLISS.replace("/A64", "/A32") if ext.upper() == ".B32" else BLISS
        # `! LINK: OTHER` links OTHER's object too, compiled before.
        with open(path, encoding="latin-1") as f:
            other = re.search(r"^! LINK: (\w+)", f.read(), re.M)
        objects = name + ("," + other.group(1) if other else "")
        cmds += [mark("BEGIN LOG", name),
                 f"{bliss}{extra} {name}{ext.upper()}",
                 f'WRITE SYS$OUTPUT "BLISS status ", $STATUS',
                 f"LINK {objects}",
                 f'WRITE SYS$OUTPUT "LINK status ", $STATUS',
                 mark("BEGIN LIS", name),
                 f"TYPE {name}.LIS",
                 mark("BEGIN OUT", name),
                 f"RUN {name}",
                 mark("BEGIN STATUS", name),
                 'WRITE SYS$OUTPUT "RUN status ", $STATUS',
                 mark("END", name)]
    return cmds


def sections(log):
    """The text between markers: {(what, name): lines}, the commands'
    echoes ($ lines) dropped."""
    out, key = {}, None
    for line in log.splitlines():
        m = re.match(rf"{MARK} (BEGIN (\w+)|END) (\S+)$", line)
        if m:
            key = (m.group(2), m.group(3).upper()) if m.group(2) else None
            if key:
                out[key] = []
        elif key and not line.startswith("$ "):
            out[key].append(line)
    return out


def disk(files, tmp):
    """A clone of the oracle's disk with the files in [ORACLE], a VAR
    record a line, as CREATE would have made them, and ORACLE.COM, the
    commands that compile and run them."""
    img = os.path.join(tmp, "sys.img")
    subprocess.run(["cp", "-c", DISK, img], check=True)
    subprocess.run([ODS, "mkdir", img, "[ORACLE]"], check=True)
    com = os.path.join(tmp, "ORACLE.COM")
    with open(com, "w", encoding="latin-1") as f:
        f.write("".join(f"$ {c}\n" for c in script(files)))
    for path in files + [com]:
        subprocess.run([ODS, "copy-in", "--mode", "lines-to-records", img, path,
                        f"[ORACLE]{os.path.basename(path).upper()}"], check=True)
    return img


def boot(files):
    """Runs the files on the oracle's system; returns the console log."""
    os.makedirs(os.path.join(HERE, "logs"), exist_ok=True)
    log = os.path.join(HERE, "logs", time.strftime("oracle-%Y%m%d-%H%M%S.log"))
    # The clone sits next to the disk, on its file system, so cp -c can clone.
    with tempfile.TemporaryDirectory(dir=os.path.dirname(DISK)) as tmp:
        img = disk(files, tmp)
        # Only this is typed: the console loses characters of long input.
        cmd = os.path.join(tmp, "oracle.dcl")
        with open(cmd, "w", encoding="latin-1") as f:
            f.write("SET TERMINAL/WIDTH=255/PAGE=0/TAB\n@SYS$SYSDEVICE:[ORACLE]ORACLE\n")
        for attempt in range(3):
            run = subprocess.run([sys.executable, os.path.join(HERE, "run-vms.py"),
                                  "--system", img, cmd, log])
            if run.returncode == 0:
                return log
            print(f"oracle run {attempt + 1} failed", file=sys.stderr)
    sys.exit(f"the oracle failed three times; last log: {log}")


def main(files, log=None):
    with open(log or boot(files), encoding="utf-8") as f:
        found = sections(f.read())
    for path in files:
        name, ext = os.path.splitext(path)
        key = os.path.basename(name).upper()
        if ("LIB", key) in found:
            with open(name + ".oracle", "w") as f:
                f.write("\n".join(found[("LIB", key)]) + "\n")
            print(f"{path}: {' '.join(found[('LIB', key)][-1:])}")
        if ext.upper() not in PROGRAMS:
            continue
        if ("OUT", key) not in found:
            sys.exit(f"{path}: no output in the console log")
        with open(name + ".lis", "w") as f:
            # Without the time, so that the listing changes only with its text.
            text = "\n".join(found[("LIS", key)]) + "\n"
            f.write(re.sub(r"\d+-[A-Z]{3}-\d{4} \d\d:\d\d:\d\d", "DD-MMM-YYYY HH:MM:SS", text))
        with open(name + ".stdout", "w") as f:
            f.write("".join(line + "\n" for line in found[("OUT", key)]))
        with open(name + ".oracle", "w") as f:
            f.write("\n".join(found[("LOG", key)] + found[("STATUS", key)]) + "\n")
        print(f"{path}: {' '.join(found[('STATUS', key)])}")


if __name__ == "__main__":
    args, log = sys.argv[1:], None
    if args[:1] == ["--log"] and len(args) > 1:
        log, args = args[1], args[2:]
    if not args or args[0].startswith("-"):
        sys.exit(__doc__)
    main(args, log)
