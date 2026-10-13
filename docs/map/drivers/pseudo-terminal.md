+++
title = "Pseudo terminals"
summary = "FTAn: terminals a program drives with the PTD$ routines"
aka = ["FTA", "FTA0:", "FTDRIVER", "PTD$CREATE", "PTD$READ", "PTD$WRITE", "PTD$DELETE", "PTD$SET_EVENT_NOTIFICATION", "pseudoterminal", "pty"]
kind = "core"
intent = "goal"
research = "done"
refs = ["docs/adr/0033-pseudo-terminals-are-a-port-of-the-terminal-driver.md", "docs/adr/0027-terminals-are-ucbs-and-telnet-is-in-the-driver.md", "vms/exec/ttdriver.mar", "vms/sysexe/ptdtest.mar", "https://docs.vmssoftware.com/vsi-openvms-io-user-s-reference-manual/"]

[[feature]]
id = "basic"
title = "PTD$CREATE, PTD$READ, PTD$READW, PTD$WRITE, PTD$CANCEL, PTD$DELETE; FTAn a terminal any process assigns"
status = "done"
note = "ADR-0033; PTDTEST"

[[feature]]
id = "events"
title = "PTD$SET_EVENT_NOTIFICATION's ASTs: XON, XOFF, bell, changed characteristics, reads"
status = "todo"
needs = ["/drivers/terminal#ctrl-o-t"]
note = "It succeeds and delivers nothing"

[[feature]]
id = "echo-buffer"
title = "PTD$WRITE's echo buffer, and PTD$CREATE's AST when the terminal's last channel goes"
status = "todo"
note = "The echo is read with PTD$READ"

[[feature]]
id = "inadr"
title = "Buffers only in PTD$CREATE's inadr pages, SS$_ACCVIO outside them"
status = "todo"
note = "Not needed for locking: the data is copied through the IRP"

[[needs]]
on = "/drivers/terminal"
why = "A pseudo terminal is a terminal UCB with a port that a program drives"

[[needs]]
on = "/exec/qio"
why = "The PTD$ routines are $QIO calls on the control connection"

[[needs]]
on = "/exec/asts"
why = "Reads and writes complete with ASTs; events notify the controller"
+++

A pseudoterminal is a terminal whose other end is a program instead of
a line. `PTD$CREATE` makes an `FTAn:` unit and returns a channel; the
controlling program writes what is typed with `PTD$WRITE` and reads what
the terminal prints with `PTD$READ`, in page-aligned buffers;
`PTD$SET_EVENT_NOTIFICATION` tells it of XON, XOFF, the bell and
changed characteristics; `PTD$DELETE` hangs it up. Meanwhile any process
uses `FTAn:` as an ordinary terminal, with `LOGINOUT`, DCL, EDT and
everything else. DECterm and the VMS port of xterm are built on them;
the I/O User's Reference Manual gives them a chapter of their own.

[ADR-0027](../../adr/0027-terminals-are-ucbs-and-telnet-is-in-the-driver.md)
weighed a pseudo terminal pair for `SET HOST` and chose to speak Telnet
inside the terminal driver instead, for memory and copies. vaxpunk's
pseudo terminals are for what comes after: an SSH server or DECterm
(t8y:/net/ssh, t8y:/gui/decterm). `FTAn` is a third port of
`ttdriver.mar`, cloned by `IO$_SETMODE` on `FTA0:`, and the PTD$
routines are `$QIO` calls in the vector
([ADR-0033](../../adr/0033-pseudo-terminals-are-a-port-of-the-terminal-driver.md)).
