# ADR-0033 — A pseudo terminal, FTAn, is a third port of the terminal driver, cloned from FTA0:, and the PTD$ routines are $QIO calls in the vector

Oct 13, 2026 · @Marko Mikulicic

Proposed; the second decision's `FTA0:` and unit numbers superseded by
[ADR-0044](0044-the-io-database.md). `PTD$CREATE` makes a pseudo terminal, `FTAn:`, which any
process uses as a terminal while the program that made it, its
controller, types into it with `PTD$WRITE` and reads what it prints with
`PTD$READ`. `FTAn` is a terminal UCB like `OPA0:` and `TNAn:`, with a
third port, a bit in the UCB: its output is kept for the controller's
reads instead of being sent. The PTD$ routines are code in the vector,
run in the caller's mode, that call `$ASSIGN` and `$QIO`: `PTD$CREATE`
assigns `FTA0:`, a template, and `IO$_SETMODE` there clones a new unit
and moves the channel to it; a read or write on that channel, from that
process, is the controller's.

## Context

[t8y:/drivers/pseudo-terminal](../map/drivers/pseudo-terminal.md): an
SSH server, the r-commands, DECterm and DTM all run a session on a
terminal whose other end is a program. VMS's pseudo terminal driver,
`FTDRIVER`, gives them `FTAn:` devices, and the control connection
routines of the I/O User's Reference Manual, Appendix D: `PTD$CREATE`,
`PTD$DELETE`, `PTD$READ`, `PTD$READW`, `PTD$WRITE`, `PTD$CANCEL` and
`PTD$SET_EVENT_NOTIFICATION`. Their buffers start with an I/O status
longword, a status word and a count word, the data four bytes past it,
and must lie in pages `PTD$CREATE` was given and locked.

[ADR-0027](0027-terminals-are-ucbs-and-telnet-is-in-the-driver.md) made
each terminal a UCB with its own state, and chose two ports, the
console's and TELNET's, chosen by a bit, over a port vector. A TELNET
terminal already keeps its echo and prompts in its UCB and queues its
writes, then hands them on in order: what a pseudo terminal does, with
the controller's reads taking them instead of sends on the port.

## Decision

- `UCB$V_TT_FT` marks a pseudo terminal. `TTPUTC` and `TNWRITE` treat it
  as a TELNET terminal without Telnet: echo and prompts go in
  `UCB$A_TT_OUT`, writes on `UCB$Q_TT_WRTQ`. Its `KICK`, `FTKICK`, gives
  them to the controller's first read on `UCB$Q_TT_PTDRD`, as much as it
  has room for, and completes a write once it is read whole.
- `FTA0:` is a template device among `IOC$AB_UCB`'s, as `BGA0:` is.
  `IO$_SETMODE` there, with the characteristics in p1 and p2, makes the
  first free `FTAn`, 1 to 9, and moves the channel to it, as
  `IO$_TTY_PORT` moves a socket's to `TNAn`. The UCB keeps that process's
  PID and channel: the control connection.
- On the control connection a read or write is the controller's, with
  p1 the buffer's status longword and p2 its data's length. A write puts
  the bytes in the type-ahead buffer as typed, CTRL/C and CTRL/Y too, as
  many as fit, and is done at once, `SS$_DATAOVERUN` if not all did. The
  status longword goes back in the IRP's buffered data, which `IOC$POST`
  copies to p1 before the IOSB, so no IOSB is used. Other functions,
  `IO$_SETMODE` and `IO$_SENSEMODE`, are the terminal's.
- Deassigning the control connection, `PTD$DELETE`, drops the
  controller's reads and hangs the terminal up, as a TELNET connection
  that closes does; it is freed once no channel has it.
- The PTD$ routines are `.ENTRY` routines in the vector, beside the
  system services, so programs link them from `SYS.STB`.

## Alternatives considered

- **PTD$ routines as system services**, `CHMK` into kernel routines
  that make IRPs themselves. That is `$QIO`'s argument checking,
  completion, event flag and AST written again.
- **Clone at `$ASSIGN`**, as VMS's template devices do. `$ASSIGN` finds
  devices by name and doesn't make them; cloning at `IO$_SETMODE` is
  what `BGA0:` already does (ADR-0016), and the caller is
  `PTD$CREATE`, not a person.
- **A function modifier for the controller's requests**, instead of the
  PID and channel. Any process could set it on its own channel to
  `FTAn:` and read another's session.
- **Lock the buffers in `inadr`'s pages and check them**, as VMS does.
  VMS locks them so the driver can write them at interrupt level without
  a process context. Here the data goes through the IRP and is copied in
  the caller's process, as for every buffered request, so the pages
  needn't be locked.

## Consequences

- `PTD$CREATE`'s `inadr`, `acmode` and its AST, and `PTD$WRITE`'s echo
  buffer are taken but not used: the echo is read with `PTD$READ`.
  `PTD$SET_EVENT_NOTIFICATION` succeeds and delivers nothing; the driver
  has no XON, XOFF or bell to tell of yet.
- At most 9 pseudo terminals, as for TELNET: a device's name has 4
  characters. Output the controller hasn't read past
  `UCB$K_TT_OUTSIZE` bytes of echo and prompts is lost; writes wait.
- `PTDTEST`, which STARTUP starts, checks a prompt, a line typed and
  its echo, a write, and the hangup.
- Follow-ups: an SSH server or DECterm on it; event notification once
  the driver has CTRL/O and XON/XOFF.
