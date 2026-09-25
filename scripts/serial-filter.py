#!/usr/bin/env python3
"""Copy the QEMU console to stdout minus the screen control of EDK2 and Limine.

Both treat the console as an 80x25 screen: EDK2 clears it (ESC[2J) and sets a
DOS text mode (ESC[=3h), and Limine draws each character at an absolute
position (ESC[row;colH). Until the shim prints its banner, this drops clears
and mode changes and turns each move to a new row into a newline, so the
firmware's output reads as plain appended lines and leaves the terminal's
scrollback alone. From the banner on, the console belongs to our code and
passes through untouched.
"""
import os
import re
import sys

CSI = re.compile(rb"\x1b\[([0-9;=?]*)([@-~])")
BANNER = b"vaxpunk shim:"

row = None
pending = tail = b""
passthrough = False
while chunk := os.read(0, 4096):
    data, pending = pending + chunk, b""
    if passthrough:
        sys.stdout.buffer.write(data)
        sys.stdout.buffer.flush()
        continue
    esc = data.rfind(b"\x1b")
    if esc != -1 and not CSI.match(data, esc) and len(data) - esc < 16:
        data, pending = data[:esc], data[esc:]  # sequence split across reads
    out, pos = bytearray(), 0
    for m in CSI.finditer(data):
        out += data[pos : m.start()]
        pos = m.end()
        if m[2] == b"H":
            r = m[1].split(b";")[0]
            if row is not None and r != row:
                out += b"\r\n"
            row = r
        elif m[2] not in (b"J", b"h", b"l"):
            out += m[0]
    out += data[pos:]
    seen = tail + out  # reads can split the banner
    passthrough, tail = BANNER in seen, bytes(seen[-len(BANNER) :])
    sys.stdout.buffer.write(out)
    sys.stdout.buffer.flush()
