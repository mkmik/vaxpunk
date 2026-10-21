+++
title = "Operator console"
summary = "OPA0:, the console terminal, and the operator's messages it shows"
aka = ["OPA0:", "console", "OPDRIVER", "TTY$RCVINT", "RXCS", "RXDB", "TXDB", "operator console", "console port"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["docs/design/0001-pal-interface.md", "docs/adr/0013-qio-irps-and-drivers.md", "docs/adr/0027-terminals-are-ucbs-and-telnet-is-in-the-driver.md", "docs/adr/0042-opcom-reads-the-operators-mailbox.md", "vms/exec/ttdriver.mar", "vms/sysexe/opcom.mar", "pal/src/main.c"]

[[feature]]
id = "port"
title = "The console port: receive interrupt into the type-ahead buffer, output by MTPR TXDB"
status = "done"
milestone = "m0"
note = "DESIGN-0001; ttdriver.mar TTY$RCVINT"

[[feature]]
id = "flow"
title = "A full type-ahead buffer leaves characters in the UART, as XOFF would"
status = "done"
note = "ttdriver.mar"

[[feature]]
id = "early-output"
title = "Executive messages written before processes run (EXE$OUTCHAR)"
status = "done"
milestone = "m0"

[[feature]]
id = "ctrl-y-waits"
title = "CTRL/Y gets through when the type-ahead buffer is full"
status = "todo"
note = "ttdriver.mar ponytail: a CTRL/Y typed then waits until a read takes a character"

[[feature]]
id = "opcom"
title = "OPCOM's messages on the console: it is an operator terminal"
status = "done"
needs = ["/utils/opcom#process"]
note = "ADR-0042; vms/sysexe/opcom.mar. For VMS's default classes; REPLY/ENABLE and /DISABLE are t8y:/utils/opcom#operator-terminals"

[[feature]]
id = "interrupt-uart"
title = "The UART's own interrupt and output queued, not polled each 10 ms tick"
status = "todo"
needs = ["/platform/console-uart"]
note = "PRD-0003 tier 4 #38; DESIGN-0001"

[[needs]]
on = "/drivers/terminal"
why = "OPA0: is a terminal UCB, the first of TT$GL_UNITS"

[[needs]]
on = "/platform/console-uart"
why = "The PAL reads and writes the UART behind RXDB and TXDB"

[[needs]]
on = "/utils/opcom"
why = "OPA0: as an operator console"
soft = true
+++

`OPA0:` is the console terminal of a VMS system: the one attached to
the console port, which shows boot messages, bugchecks and, once
OPCOM runs, operator messages, and where the system manager logs in
when nothing else works. Its port driver differs by machine (the VAX's
console registers, Alpha's SRM console port, a serial port or a
graphics console); above it, it is a terminal like the others.

vaxpunk's console is the UART QEMU's virt machine has, which the PAL
reads and writes behind the VAX's console registers: `RXCS`, `RXDB` and
`TXDB` ([DESIGN-0001](../../design/0001-pal-interface.md)). In
`vms/exec/ttdriver.mar` its receive interrupt, `TTY$RCVINT`, fills the
UCB's type-ahead buffer, and its output is written a character at a time
and waited for. The PAL looks at the UART on each tick (PRD-0003 tier 4
#38). OPCOM enables it as an operator terminal when it starts, and
writes the operators' messages there with `$BRKTHRU`
([ADR-0042](../../adr/0042-opcom-reads-the-operators-mailbox.md)).
