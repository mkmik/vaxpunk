+++
title = "Pseudo terminals"
summary = "FTAn: terminals a program drives with the PTD$ routines"
aka = ["FTA", "FTA0:", "FTDRIVER", "PTD$CREATE", "PTD$READ", "PTD$WRITE", "PTD$DELETE", "PTD$SET_EVENT_NOTIFICATION", "pseudoterminal", "pty"]
kind = "core"
intent = "aspiration"
research = "outline"
status = "todo"
refs = ["docs/adr/0027-terminals-are-ucbs-and-telnet-is-in-the-driver.md", "https://docs.vmssoftware.com/vsi-openvms-io-user-s-reference-manual/"]

[[needs]]
on = "/drivers/terminal"
why = "A pseudo terminal is a terminal UCB with a port that a program drives"

[[needs]]
on = "/exec/memory"
why = "PTD$ buffers are pages the caller gives, locked while in use"

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

vaxpunk has none. [ADR-0027](../../adr/0027-terminals-are-ucbs-and-telnet-is-in-the-driver.md)
weighed a pseudo terminal pair for `SET HOST` and chose to speak Telnet
inside the terminal driver instead, for memory and copies. An SSH server
or DECterm would bring them back (t8y:/net/ssh, t8y:/gui/decterm).
