"""Reads a sequential VAR file's bytes on stdin and writes its records a
line each: printable ASCII as it is, a backslash and every other byte as
\\xNN. The record dumps the RMS fixtures are checked against."""
import sys

data = sys.stdin.buffer.read()
pos = 0
while pos + 2 <= len(data):
    n = int.from_bytes(data[pos:pos + 2], "little")
    if n == 0xFFFF:  # the rest of the block is unused
        pos = (pos // 512 + 1) * 512
        continue
    rec = data[pos + 2:pos + 2 + n]
    pos += 2 + n + (n & 1)
    print("".join(chr(b) if 32 <= b < 127 and b != 92 else "\\x%02X" % b for b in rec))
