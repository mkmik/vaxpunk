#!/usr/bin/env python3
"""Has real VMS check the volumes ods writes.

usage: vms/check.py

Builds an ODS-2 and an ODS-5 volume with the ods CLI, putting everything
it can make hard for itself under [T]: nested and many-block directories,
seventy versions of a name, a file fragmented into several headers,
renames, purges, deletes, attribute changes, and on ODS-5 names with
spaces, dots, accents and lots of characters. Then OpenVMS Alpha, in
AXPbox (vms/setup.sh), runs ANALYZE/DISK_STRUCTURE on both and BACKUPs
their [T] trees onto a volume it initializes itself; ods verifies that
volume and compares every file VMS copied with the original: contents,
versions and attributes. Exits non-zero on any difference or complaint.
"""
import hashlib, json, os, random, re, shutil, subprocess, sys, time

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
ODS = os.path.join(ROOT, "target", "release", "ods")
WORK = os.path.join(HERE, "work")
SIZE = 200 * 1024 * 1024


def ods(*args):
    r = subprocess.run([ODS, *args], capture_output=True, text=True, encoding="latin-1")
    if r.returncode:
        sys.exit(f"ods {' '.join(args)}: {r.stderr.strip()}")
    return r.stdout


def lines(rng, n):
    return "".join(f"line {i} " + "x" * rng.randrange(0, 200) + "\n" for i in range(n))


def build(img, ods5):
    rng = random.Random(5 if ods5 else 2)
    host = os.path.join(WORK, "host5" if ods5 else "host2")
    shutil.rmtree(host, ignore_errors=True)
    os.makedirs(host)

    def put(path, data, mode="binary"):
        f = os.path.join(host, "f")
        with open(f, "wb" if isinstance(data, bytes) else "w", **({} if isinstance(data, bytes) else {"encoding": "latin-1"})) as w:
            w.write(data)
        ods("copy-in", img, f, path, "--mode", mode)

    if os.path.exists(img):
        os.remove(img)
    ods("init", img, "--size", str(SIZE // 512), "--label", "ODSCHK5" if ods5 else "ODSCHK2", "--ods5" if ods5 else "--ods2")
    for d in ["[T]", "[T.SUB]", "[T.SUB.DEEP]", "[T.OLD]", "[T.FRAG]"]:
        ods("mkdir", img, d)
    put("/T/README.TXT", lines(rng, 40), "lines-to-records")
    put("/T/DATA.BIN", rng.randbytes(300_000))
    put("/T/EMPTY.DAT", b"")
    put("/T/SUB/NOTES.TXT", lines(rng, 3), "lines-to-records")
    put("/T/SUB/DEEP/BOTTOM.TXT", lines(rng, 1000), "lines-to-records")
    put("/T/OLD/MOVED.TXT", lines(rng, 2), "lines-to-records")
    for v in range(70):
        put("/T/VERS.TXT", f"version {v + 1}\n", "lines-to-records")
    ods("purge", img, "[T]VERS.TXT", "--keep", "60")
    ods("delete", img, "[T]VERS.TXT;30")

    # A directory of 300 files, imported in one go.
    many = os.path.join(host, "many")
    os.makedirs(many)
    for i in range(300):
        with open(os.path.join(many, f"F{i:04}_{'Y' * rng.randrange(0, 20)}.DAT"), "wb") as w:
            w.write(rng.randbytes(rng.randrange(0, 2000)))
    ods("mkdir", img, "[T.MANY]")
    ods("import", img, many, "[T.MANY]")

    # Free space in one-cluster holes, then a file that has to use them.
    frag = os.path.join(host, "frag")
    os.makedirs(frag)
    for i in range(600):
        with open(os.path.join(frag, f"H{i:04}.TMP"), "wb") as w:
            w.write(b"h" * 1000)
    ods("import", img, frag, "[T.FRAG]")
    for d in "02468":
        ods("delete", img, f"[T.FRAG]H%%%{d}.TMP;*")
    put("/T/FRAG/BIG.BIN", rng.randbytes(800_000))
    headers = json.loads(ods("dump", img, "[T.FRAG]BIG.BIN", "--json"))
    if len(headers) < 2:
        sys.exit(f"{img}: BIG.BIN has {len(headers)} header, the test wants extension headers")

    ods("rename", img, "[T]OLD.DIR;1", "[T]NEW.DIR")
    ods("rename", img, "[T]README.TXT;1", "[T.SUB]README.TXT")
    ods("set-attr", img, "[T.SUB]NOTES.TXT", "--protection", "S:RWED,O:RWED,G:R,W:", "--owner", "[200,3]")
    ods("delete", img, "[T]EMPTY.DAT;1")

    if ods5:
        ods("mkdir", img, "[T.dir^.with^.dots]")
        put("/T/mixed Case name.Txt", lines(rng, 5), "lines-to-records")
        put("/T/dots.in.the.name.txt", lines(rng, 5), "lines-to-records")
        put("/T/caf\xe9 cr\xe8me.txt", lines(rng, 5), "lines-to-records")
        put("/T/" + "long" * 30 + ".data", rng.randbytes(1000))
        put("/T/dir.with.dots/inner.File.txt", lines(rng, 5), "lines-to-records")
        put("/T/lower.txt", lines(rng, 5), "lines-to-records")
        put("/T/lower.txt", lines(rng, 5), "lines-to-records")
        ods("rename", img, "[T]lower.txt;1", "[T.dir^.with^.dots]Renamed^ In^ Place.txt")

    report = ods("verify", img)
    if "0 errors, 0 leaks, 0 warnings" not in report:
        sys.exit(f"{img}: ods finds problems itself:\n{report}")


def vms_complaints(log):
    """Lines in the console log that are not VMS saying all is well."""
    bad = []
    fine = re.compile(r"%ANALDISK-I-OPENQUOTA|-SYSTEM-W-NOSUCHFILE|%MOUNT-I-MOUNTED|%BACKUP-I-")
    for line in log.splitlines():
        if re.match(r"\s*[%-][A-Z]+-[WEF]-", line) and not fine.search(line):
            bad.append(line.strip())
        elif re.match(r"\s*%ANALDISK-", line) and not fine.search(line):
            bad.append(line.strip())
    return bad


def manifest(img, spec, dest):
    shutil.rmtree(dest, ignore_errors=True)
    ods("export", img, spec, dest)
    with open(os.path.join(dest, "ods-manifest.json")) as f:
        entries = json.load(f)["entries"]
    out = {}
    for e in entries:
        if not e["directory"]:
            with open(os.path.join(dest, e["path"]), "rb") as f:
                e["sha256"] = hashlib.sha256(f.read()).hexdigest()
        out[e["path"]] = e
    return out


# What BACKUP must carry over. Allocation, file IDs, the highwater mark and
# the backup date legitimately differ, and directories BACKUP makes itself
# are dated when it makes them.
KEEP = ["directory", "name", "version", "bytes", "sha256", "rtype", "rattrib", "rsize", "vfcsize", "maxrec",
        "owner", "protection", "created", "revised", "expires", "version_limit"]
DIR_DATES = {"created", "revised"}


def compare(src, dst):
    diffs = []
    for path in sorted(set(src) | set(dst)):
        a, b = src.get(path), dst.get(path)
        if a is None or b is None:
            diffs.append(f"{path}: only in {'the copy' if a is None else 'the original'}")
            continue
        for k in KEEP:
            if a["directory"] and k in DIR_DATES:
                continue
            if a.get(k) != b.get(k):
                diffs.append(f"{path}: {k} {a.get(k)!r} became {b.get(k)!r}")
    return diffs


def main():
    subprocess.run([os.path.join(HERE, "setup.sh")], check=True)
    subprocess.run(["cargo", "build", "-q", "--release", "-p", "ods-cli"], cwd=ROOT, check=True)
    os.makedirs(WORK, exist_ok=True)
    disks = [os.path.join(HERE, f"disk{i}.img") for i in (1, 2, 3)]
    build(disks[0], False)
    build(disks[1], True)
    with open(disks[2], "wb") as f:
        f.truncate(SIZE)
    log = os.path.join(HERE, "logs", f"check-{time.strftime('%Y%m%d-%H%M%S')}.log")
    os.makedirs(os.path.dirname(log), exist_ok=True)
    subprocess.run([sys.executable, os.path.join(HERE, "run-vms.py"), os.path.join(HERE, "check.dcl"), log], check=True)
    with open(log, encoding="utf-8") as f:
        text = f.read()
    problems = [f"VMS: {l}" for l in vms_complaints(text)]
    analyzed = text.count("Analyze/Disk_Structure for")
    if analyzed != 3:
        problems.append(f"VMS analyzed {analyzed} volumes, not 3")
    report = ods("verify", disks[2])
    if " 0 errors" not in report:
        problems.append(f"ods on the volume VMS wrote: {report.strip()}")
    for src, dst, name in [(disks[0], "[ODS2]", "ods2"), (disks[1], "[ODS5]", "ods5")]:
        a = manifest(src, "[T]", os.path.join(WORK, f"{name}-original"))
        b = manifest(disks[2], dst, os.path.join(WORK, f"{name}-copy"))
        problems += [f"{name}: {d}" for d in compare(a, b)]
        print(f"{name}: {len(a)} files and directories compared")
    print(f"console log: {log}")
    if problems:
        print("\n".join(problems))
        sys.exit(f"FAILED: {len(problems)} problems")
    print("PASSED: VMS finds nothing wrong, and copies every file exactly")


if __name__ == "__main__":
    main()
