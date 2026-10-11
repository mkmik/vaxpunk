+++
title = "Console UART"
summary = "QEMU virt's PL011, driven by the PAL behind the VAX console registers"
aka = ["PL011", "UART", "serial console", "TXDB", "RXDB", "RXCS", "OPA0", "serial.log", "uart1"]
kind = "platform"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["docs/design/0001-pal-interface.md", "pal/src/main.c", "scripts/run-qemu.sh", "scripts/serial-filter.py", "README.md"]

[[feature]]
id = "transmit"
title = "MTPR_TXDB writes a character on the PL011"
status = "done"
milestone = "m0"
note = "DESIGN-0001; no debug kernel needed"

[[feature]]
id = "receive"
title = "RXCS and RXDB, and the console receive interrupt at IPL 20, vector 0xF8"
status = "done"
milestone = "m0"
note = "DESIGN-0001 Interrupts and exceptions"

[[feature]]
id = "uart-interrupt"
title = "The UART's own interrupt, not a look each 10 ms tick"
status = "todo"
note = "PRD-0003 tier 4 #38"

[[feature]]
id = "dtb-address"
title = "The UART's address from the DTB, not QEMU virt's 0x09000000"
status = "todo"
note = "PRD-0003 tier 4 #38; main.c ponytail"

[[feature]]
id = "second-uart"
title = "A driver for the second PL011, at 0x09040000, as a terminal"
status = "todo"
note = "README: --uart1 serves it on telnet localhost:4444; nothing in the guest drives it yet"

[[needs]]
on = "//platform/pal"
why = "The PAL maps the UART and serves the console registers"
+++

A VAX's console terminal was reached through processor registers:
`TXCS` and `TXDB` to send a character, `RXCS` and `RXDB` to receive one,
with an interrupt at IPL 20. VMS's console driver, `OPDRIVER`, makes it
`OPA0:`. Alpha had no such registers; its console went through SRM
firmware callbacks until the terminal driver took over.

vaxpunk keeps the VAX's registers. The PAL drives QEMU virt's PL011 and
serves `MTPR #PR$_TXDB`, `MFPR #PR$_RXCS` and `MFPR #PR$_RXDB` from it
([DESIGN-0001](../../docs/design/0001-pal-interface.md)). It looks at the
UART on each 10 ms tick and raises the console receive interrupt when a
character waits, so input can lag up to 10 ms. The address is QEMU's
fixed one. QEMU's second PL011 is always present, and `--uart1` serves it
on a TCP port, but no driver uses it yet.
