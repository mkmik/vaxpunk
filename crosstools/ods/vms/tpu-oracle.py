#!/usr/bin/env python3
"""The TPU oracle (PRD-0006): runs TPU command files with DEC's TPU on
OpenVMS Alpha in AXPbox and writes back what they printed.

usage: tpu-oracle.py [--log LOG] FILE...

Each FILE.TPU is run with
    EDIT/TPU/NODISPLAY/NOSECTION/NOINITIALIZATION/NOJOURNAL/COMMAND=FILE.TPU
and should end the session itself, with EXIT or QUIT: TPU with no
section file and no display otherwise reads its SYS$INPUT, which the
oracle points at NL:. Other files (.TXT...)
are only copied in, for READ_FILE. Next to each FILE.TPU it writes:

    FILE.stdout  what TPU printed: MESSAGE's lines and its own messages
    FILE.out     FILE.OUT, if the program wrote one (WRITE_FILE)

All files go in one boot of the oracle's system disk, $TPU_ORACLE_DISK or
the BLISS oracle's (bliss-oracle.py says how to make one), cloned so that
the disk itself never changes (run-vms.py --system). AXPbox hangs in about
one run in six; a run that fails is retried twice. The console log goes to
logs/; --log LOG reads the results from a log instead of running again.
"""
import os, re, subprocess, sys, tempfile, time

HERE = os.path.dirname(os.path.abspath(__file__))
DISK = os.environ.get("TPU_ORACLE_DISK", os.path.expanduser(
    "~/Library/Caches/vaxpunk/bliss-oracle/golden-sys.img"))
TPU = "EDIT/TPU/NODISPLAY/NOSECTION/NOINITIALIZATION/NOJOURNAL/COMMAND="
MARK = "@@ORACLE"


def mark(what, name):
    # Split so that the echo of the command doesn't match.
    return f'WRITE SYS$OUTPUT "@@","ORACLE {what} {name}"'


def script(files):
    cmds = ["SET TERMINAL/WIDTH=255/PAGE=0",
            "CREATE/DIRECTORY SYS$SYSDEVICE:[TPUORACLE]",
            "SET DEFAULT SYS$SYSDEVICE:[TPUORACLE]",
            "DEFINE SYS$SCRATCH SYS$SYSDEVICE:[TPUORACLE]"]
    for path in files:
        with open(path, encoding="latin-1") as f:
            lines = f.read().splitlines()
        cmds += [f"CREATE {os.path.basename(path)}", *lines, "@@CTRLZ"]
    for path in files:
        name, ext = os.path.splitext(os.path.basename(path))
        if ext.upper() != ".TPU":
            continue
        out = f"{name}.OUT"
        # SYS$INPUT the null device: a program that doesn't end the
        # session reads end of file instead of the next DCL command.
        cmds += [mark("BEGIN OUT", name),
                 "DEFINE/USER_MODE SYS$INPUT NL:",
                 f"{TPU}{name}.TPU",
                 mark("BEGIN FILE", name),
                 f'IF F$SEARCH("{out}") .NES. "" THEN TYPE {out}',
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


def boot(files):
    """Runs the files on the oracle's system; returns the console log."""
    os.makedirs(os.path.join(HERE, "logs"), exist_ok=True)
    log = os.path.join(HERE, "logs", time.strftime("tpu-oracle-%Y%m%d-%H%M%S.log"))
    with tempfile.TemporaryDirectory() as tmp:
        cmd = os.path.join(tmp, "oracle.dcl")
        with open(cmd, "w", encoding="latin-1") as f:
            f.write("\n".join(script(files)) + "\n")
        for attempt in range(3):
            run = subprocess.run([sys.executable, os.path.join(HERE, "run-vms.py"),
                                  "--system", DISK, cmd, log])
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
        if ext.upper() != ".TPU":
            continue
        if ("OUT", key) not in found:
            sys.exit(f"{path}: no output in the console log")
        with open(name + ".stdout", "w") as f:
            f.write("".join(line + "\n" for line in found[("OUT", key)]))
        written = found.get(("FILE", key), [])
        if written:
            with open(name + ".out", "w") as f:
                f.write("".join(line + "\n" for line in written))
        elif os.path.exists(name + ".out"):
            os.remove(name + ".out")
        print(f"{path}: {len(found[('OUT', key)])} lines")


if __name__ == "__main__":
    args, log = sys.argv[1:], None
    if args[:1] == ["--log"] and len(args) > 1:
        log, args = args[1], args[2:]
    if not args or args[0].startswith("-"):
        sys.exit(__doc__)
    main(args, log)
