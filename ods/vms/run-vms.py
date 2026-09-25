#!/usr/bin/env python3
"""Boots OpenVMS Alpha V8.4-2L1 from its install CD in AXPbox, types DCL
commands at its prompt, and shuts it down.

usage: run-vms.py CMDFILE [LOG]      (run-vms.py --selftest checks the console filter)

CMDFILE (UTF-8, Latin-1 characters only): one line is typed per DCL prompt.
After `CREATE file` (not /DIRECTORY) the following lines are typed as the
file's text, up to a line starting with @@CTRLZ, which sends Ctrl-Z.

Flow: SRM `boot dka400` -> the date prompt -> menu option 8 (DCL) -> `$$$`
-> SET TERMINAL/EIGHTBIT/WIDTH=132 -> CMDFILE -> LOGOUT -> option 9
(shutdown) -> SRM prompt -> SIGINT to the emulator, whose graceful exit
flushes the disk files (never kill it harder). The console goes to LOG
(default logs/<cmdfile>-<time>.log), the emulator's own output to LOG.emu.
Disks (es40.cfg): DKA0=disk1.img DKA100=disk2.img DKA200=disk3.img.
Environment: AXPBOX (emulator binary), AXPBOX_CFG (config), AXPBOX_BOOT
(SRM name of the CD).
"""
import os, re, select, signal, socket, subprocess, sys, time

HERE = os.path.dirname(os.path.abspath(__file__))
EMU = os.environ.get("AXPBOX", os.path.join(HERE, "axpbox", "build", "axpbox"))
PORT = 21264                       # serial0 port in es40.cfg
CONFIG = os.environ.get("AXPBOX_CFG", "es40.cfg")
BOOT = os.environ.get("AXPBOX_BOOT", "dka400")  # SRM name of the CD
PROMPT = "$$$ "
CMD_TIMEOUT = 3600                 # per DCL command, seconds
CREATE_FILE = re.compile(r"\s*\$?\s*CREA(T|TE)?\b(?!.*/DIR)", re.I)
MONTHS = "JAN FEB MAR APR MAY JUN JUL AUG SEP OCT NOV DEC".split()


class Console:
    """Telnet console: strips telnet commands, NUL fill and CR; logs as UTF-8."""

    def __init__(self, sock, log):
        self.sock, self.log = sock, log
        self.buf, self.mark, self.skip = "", 0, 0

    def clean(self, data):
        out = bytearray()
        for b in data:
            if self.skip == 1:                # byte after IAC
                if b == 0xFF:                 # IAC IAC = literal 0xFF
                    out.append(b)
                self.skip = 2 if 251 <= b <= 254 else 0  # WILL/WONT/DO/DONT + option
            elif self.skip == 2:              # option byte
                self.skip = 0
            elif b == 0xFF:
                self.skip = 1
            elif b not in (0, 13):
                out.append(b)
        return out.decode("latin-1")

    def pump(self, secs):
        if select.select([self.sock], [], [], secs)[0]:
            data = self.sock.recv(65536)
            if not data:
                raise EOFError("console connection closed (emulator died?)")
            text = self.clean(data)
            self.buf += text
            self.log.write(text)
            self.log.flush()

    def since(self):
        return self.buf[self.mark:]

    def send(self, s):
        self.mark = len(self.buf)
        self.sock.sendall(s.encode("latin-1"))

    def wait(self, cond, secs, what):
        end = time.time() + secs
        while not cond():
            if time.time() > end:
                raise TimeoutError(f"timed out after {secs}s waiting for {what}")
            self.pump(0.5)

    def poll(self, cond, secs):
        """Like wait() but returns False instead of raising."""
        try:
            self.wait(cond, secs, "")
            return True
        except TimeoutError:
            return False

    def seen(self, s):
        return lambda: s in self.since()

    def at_prompt(self):
        return self.since().endswith(PROMPT)

    def dcl(self, line):
        self.send(line + "\r")
        self.wait(self.at_prompt, CMD_TIMEOUT, repr(line))


def run(cmds, logpath):
    with socket.socket() as probe:
        if probe.connect_ex(("127.0.0.1", PORT)) == 0:
            sys.exit(f"port {PORT} busy: another axpbox running? (pkill -INT -x axpbox)")
    emu_log = open(logpath + ".emu", "wb")
    emu = subprocess.Popen([EMU, "run", CONFIG], cwd=HERE, stdout=emu_log,
                           stderr=subprocess.STDOUT, stdin=subprocess.DEVNULL)
    t0 = time.time()
    try:
        for _ in range(300):
            try:
                sock = socket.create_connection(("127.0.0.1", PORT))
                break
            except ConnectionRefusedError:
                if emu.poll() is not None:
                    raise RuntimeError(f"emulator exited early, see {logpath}.emu")
                time.sleep(0.2)
        else:
            raise RuntimeError("could not connect to the console port")
        with sock, open(logpath, "w", encoding="utf-8") as log:
            c = Console(sock, log)
            c.wait(c.seen("P00>>>"), 300, "SRM prompt")
            c.send("show device\r")
            c.wait(c.seen("P00>>>"), 60, "show device")
            # Booting from an IDE CD, about half the boots die loading images
            # (status 54, CTRLERR) and fall back to SRM; booting again works.
            # Not seen with SCSI; retry anyway.
            for attempt in range(1, 11):
                c.send(f"boot {BOOT}\r")
                c.wait(lambda: any(s in c.since() for s in
                                   ("date and time", "Enter CHOICE", "P00>>>")),
                       1800, "VMS boot")
                if "P00>>>" not in c.since():
                    break
                print(f"boot attempt {attempt} fell back to SRM, retrying", flush=True)
            else:
                raise RuntimeError("VMS did not boot from the CD")
            if "date and time" in c.since():
                t = time.localtime()
                c.send(f"{t.tm_mday:02}-{MONTHS[t.tm_mon - 1]}-{t.tm_year} "
                       f"{t.tm_hour:02}:{t.tm_min:02}\r")
                c.wait(c.seen("Enter CHOICE"), 1800, "install menu")
            c.send("8\r")
            c.wait(c.at_prompt, 600, "$$$ prompt")
            print(f"$$$ prompt after {time.time() - t0:.0f}s", flush=True)
            c.dcl("SET TERMINAL/EIGHTBIT/WIDTH=132")

            text = skip = False
            for line in cmds:
                if line.startswith("@@CTRLZ"):
                    if not skip:
                        c.send("\x1a")
                        c.wait(c.at_prompt, CMD_TIMEOUT, "prompt after Ctrl-Z")
                    text = skip = False
                elif skip:
                    continue
                elif text:  # file content: wait for the echo of the line terminator
                    c.send(line + "\r")
                    c.wait(c.seen("\n"), CMD_TIMEOUT, f"echo of {line!r}")
                elif CREATE_FILE.match(line):
                    c.send(line + "\r")
                    c.wait(c.seen("\n"), CMD_TIMEOUT, f"echo of {line!r}")
                    # A prompt right away means CREATE failed: skip its text block.
                    skip = c.poll(c.at_prompt, 3)
                    text = not skip
                    if skip:
                        print(f"WARNING: {line!r} failed, skipping its text", flush=True)
                else:
                    c.dcl(line)
            if text:
                print("WARNING: command file ended inside a text block, sending Ctrl-Z")
                c.send("\x1a")
                c.wait(c.at_prompt, CMD_TIMEOUT, "prompt after Ctrl-Z")
            print(f"commands done after {time.time() - t0:.0f}s, shutting down", flush=True)
            c.send("LOGOUT\r")
            c.wait(c.seen("Enter CHOICE"), 600, "menu after LOGOUT")
            c.send("9\r")
            c.wait(c.seen("P00>>>"), 600, "SRM prompt after shutdown")
    finally:
        emu.send_signal(signal.SIGINT)  # graceful: flushes and closes the disk files
        try:
            emu.wait(60)
        except subprocess.TimeoutExpired:
            emu.kill()
            print("WARNING: emulator did not exit on SIGINT; killed", flush=True)
    print(f"done in {time.time() - t0:.0f}s; console log: {logpath}", flush=True)


def selftest():
    c = Console(None, None)
    assert c.clean(b"\xff\xfd\x01\xff\xfb\x03A\r\n\x00B\xff\xffC\xff\xf1D") == "A\nB\xffCD"
    assert c.clean(b"\xff") == "" and c.clean(b"\xfb") == "" and c.clean(b"\x01E") == "E"
    assert CREATE_FILE.match("CREATE a.txt") and CREATE_FILE.match("$ create/log x")
    assert not CREATE_FILE.match("CREATE/DIRECTORY [.x]") and not CREATE_FILE.match("CREATED")
    print("selftest ok")


if __name__ == "__main__":
    if sys.argv[1:] == ["--selftest"]:
        selftest()
        sys.exit()
    if len(sys.argv) not in (2, 3):
        sys.exit(__doc__)
    with open(sys.argv[1], encoding="utf-8") as f:
        cmds = f.read().splitlines()
    stem = os.path.splitext(os.path.basename(sys.argv[1]))[0]
    log = sys.argv[2] if len(sys.argv) == 3 else os.path.join(
        HERE, "logs", f"{stem}-{time.strftime('%Y%m%d-%H%M%S')}.log")
    os.makedirs(os.path.dirname(os.path.abspath(log)), exist_ok=True)
    run(cmds, log)
