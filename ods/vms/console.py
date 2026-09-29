#!/usr/bin/env python3
"""Boots an installed OpenVMS in AXPbox and leaves you at the console
login prompt.

usage: console.py [DIR [BOOTDEV]]   (defaults: this directory, dka0)
       console.py --selftest

DIR holds es40.cfg (serial0 on port 21264) and axpbox or
axpbox/build/axpbox; the emulator's output goes to DIR/axpbox.log.
To quit: shut VMS down (@SYS$SYSTEM:SHUTDOWN), wait for P00>>>,
then Ctrl-] stops the emulator gracefully so the disk images are flushed.
Closing the terminal stops it the same way.

AXPbox hands the guest one input character per device tick and leaves the
rest queued in TCP, which buffers ~80 KB on loopback: a mouse wheel on the
alternate screen (arrow keys) could queue minutes of input. So stdin is
always read, never blocked on the guest (Ctrl-] always works), input is
paced to VMS_CPS characters per second, and a cursor or function key is
dropped while earlier input is still unsent.
"""
import os, re, select, signal, socket, subprocess, sys, termios, time, tty

PORT = 21264                          # serial0 port in es40.cfg
CPS = float(os.environ.get("VMS_CPS", 40))  # below AXPbox's ~50 chars/s intake
BURST = 16                            # typing bursts go out at once
QUIT = b"\x1d"                        # Ctrl-]
KEY = re.compile(rb"\x1b(?:\[[0-9;]*[@-~]|O.)")  # CSI or SS3 key sequence


def untelnet(data, skip):
    """Strips telnet commands; skip carries a split command across calls."""
    out = bytearray()
    for b in data:
        if skip == 1:                     # byte after IAC
            if b == 0xFF:                 # IAC IAC = literal 0xFF
                out.append(b)
            skip = 2 if 251 <= b <= 254 else 0  # WILL/WONT/DO/DONT + option
        elif skip == 2:                   # option byte
            skip = 0
        elif b == 0xFF:
            skip = 1
        else:
            out.append(b)
    return bytes(out), skip


def accept(pending, data):
    """What of the typed data to queue behind pending unsent bytes."""
    out, pos = bytearray(), 0
    for m in KEY.finditer(data):
        out += data[pos:m.start()]
        if not pending and not out:       # keys only onto an empty queue
            out += m.group()
        pos = m.end()
    out += data[pos:]
    return bytes(out).replace(b"\xff", b"\xff\xff")


def main(dir, dev):
    os.chdir(dir)
    exe = "./axpbox" if os.path.isfile("axpbox") and os.access("axpbox", os.X_OK) \
        else "axpbox/build/axpbox"
    for need in ("es40.cfg", exe):
        if not os.path.isfile(need):
            sys.exit(f"{dir}: no {need} (run ods/vms/setup.sh, or pass the dir that has it)")
    # AXPbox ignores bind() errors: a second one would listen on a random port.
    with socket.socket() as probe:
        if probe.connect_ex(("127.0.0.1", PORT)) == 0:
            sys.exit(f"port {PORT} busy: another axpbox running?")
    with open("axpbox.log", "wb") as log:
        # Own session: the terminal's signals must not reach it.
        emu = subprocess.Popen([exe, "run", "es40.cfg"], stdout=log, stderr=log,
                               stdin=subprocess.DEVNULL, start_new_session=True)
    for _ in range(250):
        try:
            sock = socket.create_connection(("127.0.0.1", PORT))
            break
        except ConnectionRefusedError:
            if emu.poll() is not None:
                sys.exit("emulator exited, see axpbox.log")
            time.sleep(0.2)
    else:
        emu.kill()                        # still waiting for a connection: safe
        sys.exit("emulator not listening, see axpbox.log")

    def hangup(*_):
        raise SystemExit
    signal.signal(signal.SIGHUP, hangup)
    signal.signal(signal.SIGTERM, hangup)
    saved = termios.tcgetattr(0)
    tty.setraw(0)
    try:
        session(sock, dev)
    except SystemExit:
        pass
    finally:
        try:
            termios.tcsetattr(0, termios.TCSAFLUSH, saved)
            print(f"\r\nstopping the emulator, pid {emu.pid} (flushing disk images)...",
                  flush=True)
        except OSError:                   # the terminal is gone
            pass
        # SIGINT while still connected: AXPbox ignores it in accept().
        emu.send_signal(signal.SIGINT)
        emu.wait()
        sock.close()


def session(sock, dev):
    sock.setblocking(False)               # a full TCP buffer must not stall us
    queue, budget, skip = b"", BURST, 0
    tail, booting = b"", True             # automate until the login prompt
    last_out = last_tick = time.monotonic()
    while True:
        wait = 0.02 if queue else 1
        for fd in select.select([0, sock], [], [], wait)[0]:
            if fd == 0:
                data = os.read(0, 4096)
                if QUIT in data:
                    return
                queue += accept(queue, data)
                continue
            data = sock.recv(65536)
            if not data:
                print("\r\nconsole connection closed, see axpbox.log\r")
                return
            data, skip = untelnet(data, skip)
            os.write(1, data)
            last_out = time.monotonic()
            if not booting:
                continue
            tail = (tail + data)[-256:]
            if b"P00>>>" in tail:
                queue += f"boot {dev}\r".encode()
                tail = b""
            elif b"date and time" in tail:
                queue += time.strftime("%d-%b-%Y %H:%M\r").upper().encode()
                tail = b""
            elif re.search(rb"\rUsername: $", tail):  # audit alarms print one after LF
                booting = False
        now = time.monotonic()
        # 30 s of silence: OPA0 needs Return to show Username: after startup.
        if booting and now - last_out > 30:
            queue += b"\r"
            last_out = now
        budget = min(BURST, budget + (now - last_tick) * CPS)
        last_tick = now
        if queue and budget >= 1:
            try:
                n = sock.send(queue[:int(budget)])
                queue, budget = queue[n:], budget - n
            except BlockingIOError:
                pass


def selftest():
    assert untelnet(b"\xff\xfd\x01A\r\n\xff\xffB\xff", 0) == (b"A\r\n\xffB", 1)
    assert untelnet(b"\xfb\x03C", 1) == (b"C", 0)
    up = b"\x1b[A"
    assert accept(b"", up * 5) == up           # wheel flood: one arrow
    assert accept(b"x", up) == b""             # still typing out: dropped
    assert accept(b"", b"ab" + up) == b"ab"    # text first, key dropped
    assert accept(b"x", b"dir\r") == b"dir\r"  # text is never dropped
    assert accept(b"", b"\x1bOP\xff") == b"\x1bOP\xff\xff"
    print("selftest ok")


if __name__ == "__main__":
    args = sys.argv[1:]
    if args == ["--selftest"]:
        selftest()
    elif len(args) > 2:
        sys.exit(__doc__)
    else:
        here = os.path.dirname(os.path.abspath(__file__))
        main(os.path.abspath(args[0] if args else here), args[1] if len(args) > 1 else "dka0")
