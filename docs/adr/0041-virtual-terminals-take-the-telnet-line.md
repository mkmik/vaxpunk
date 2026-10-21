# ADR-0041 — A virtual terminal, VTAn, takes its TELNET terminal's connection, which moves to another one when the user connects to it

Oct 21, 2026 · @Marko Mikulicic

Proposed. While the system logical name `TCPIP$TELNET_VTA` is TRUE, as
on TCP/IP Services, a TELNET login runs on a virtual terminal, `VTAn:`,
and its `TNAn:` stays as the physical terminal, redirected to it. The
virtual terminal's UCB holds the TELNET connection itself, as `TNAn`'s
did. When the connection goes, a virtual terminal whose user is logged
in is disconnected instead of hung up: its process waits, for 900
seconds, VMS's `TTY_TIMEOUT`. `IO$_SETMODE!IO$M_TT_CONNECT`, which
`LOGINOUT` and `CONNECT` use, moves the connection from the caller's
virtual terminal to the disconnected one, and `IO$M_TT_DISCON`, which
`DISCONNECT` uses, closes it.

## Context

[t8y:/drivers/terminal#virtual-terminals](../map/drivers/terminal.md): a
remote session ended with its process when the connection closed, a
lost network or a closed laptop. VMS keeps it: a terminal with
`TT2$M_DISCONNECT` gets a virtual terminal at login, the job's
`SYS$COMMAND`, which the terminal driver links to the physical one.
When the line goes, the link breaks: the virtual terminal is detached,
`DEV$M_DET`, and its I/O waits, until `TTY_TIMEOUT` seconds later its
process is deleted. At the next login of the same user, LOGINOUT lists
the disconnected processes, `You have the following disconnected
process:`, and asks `Connect to above listed process [YES]:`; DCL has
`CONNECT` and `DISCONNECT`. Both are `IO$_SETMODE` with `IO$M_TT_CONNECT`
and `IO$M_TT_DISCON`, p1 the target's name, and the owner of the target
must have the caller's UIC (DEC's VAX/VMS V4.3 sources: `INTERACT.B32`,
`CONNECT.MAR`, `TTYFDT.MAR`, `TTYSUB.MAR`, read for what they do).
TCP/IP Services' TELNET gives its `TNAn:` logins virtual terminals while
`TCPIP$TELNET_VTA` is TRUE and `TTY_DEFCHAR2` has `TT2$M_DISCONNECT`.

VMS's driver keeps a logical UCB, `VTAn`, with the session's state, and
a physical UCB, the port's, and sends the logical one's I/O to the
physical one. vaxpunk's terminal driver keeps a terminal's session and
its port in one UCB ([ADR-0027](0027-terminals-are-ucbs-and-telnet-is-in-the-driver.md)):
the reads, type-ahead and characteristics next to the connection, its
Telnet state and what it has to send.

## Decision

- TELNETD passes `TT2$M_DISCONNECT` in p1 of `IO$_TTY_PORT` while
  `TCPIP$TELNET_VTA` is TRUE; `IO$_TTY_PORT` is vaxpunk's own function
  (ADR-0027), so p1 is ours to give a meaning. The driver then makes
  `VTAn` besides `TNAn`, moves the connection and the channel to it, and
  marks `TNAn` redirected, `UCB$V_TT_RED`: `$ASSIGN` of `TNAn:` assigns
  `VTAn:`, and `$GETDVI` shows `DEV$M_RED`. LOGINOUT runs on `VTAn`.
- A virtual terminal is a TELNET terminal with `UCB$V_TT_VT`, and
  `UCB$L_TT_PHY` its `TNAn`. Its connection, Telnet state, sends and
  receives on the port and what it has to send are the line; the rest of
  the UCB is the session.
- When the connection closes, and the owner is logged in, the virtual
  terminal is disconnected, `UCB$V_TT_DET`: the connection is closed,
  `TNAn` freed, what it had to send dropped, its writes done, as VMS's
  driver does, and the read in progress waits to start again, so that
  its prompt is written again on the next line. Reads and writes that
  come wait. Its timer, the one a timed read uses, hangs it up after
  `TTY$GL_TIMEOUT` seconds, which deletes the process.
- `IO$M_TT_CONNECT` on a connected virtual terminal moves the line to
  the disconnected one p1 names: the connection, the Telnet state, the
  sends and receives on the port (`NET$TTMOVE` gives their IRPs the
  other UCB), what is left to send, the window's size and `TNAn`. Then
  the caller's is disconnected, or with `IO$M_TT_DISCON` hung up, and
  the other's read starts again. `IO$M_TT_DISCON` alone disconnects the
  caller's.
- LOGINOUT, once the password is right and before the welcome, lists
  the user's processes on disconnected terminals, those of its username
  and UIC, asks, and connects; the process then deletes itself.
  `UAF$V_DISRECONNECT` turns that off. DCL's `CONNECT [/[NO]LOGOUT]
  [/CONTINUE]` and `DISCONNECT [/CONTINUE]` are VMS's.

## Alternatives considered

- **A logical UCB and a physical one, as VMS has**, with the driver
  sending the logical one's I/O to the physical one's port. Every routine
  that echoes, prompts, sends or receives would have to know which of
  the two it holds; moving the few fields that are the line is less.
- **No UCB for `TNAn`**: the virtual terminal only remembers the name.
  Its unit number would then have to be kept from `NEWTERM` some other
  way, and `SHOW DEVICES` wouldn't show it, as VMS's does.
- **Virtual terminals for every TELNET login, always.** A closed
  connection would no longer end a session, which surprises anyone who
  doesn't know about them, and VMS doesn't do it either by default.
- **`TTY_DEFCHAR2` as the switch**, as on VMS. vaxpunk has no SYSGEN to
  set it; TCP/IP Services' logical name is the one a system manager sets
  for TELNET anyway.
- **Connect from any terminal**, the console's too, as VMS does. The
  console's port is the UART, which can't move into another UCB; only a
  virtual terminal connects to another.

## Consequences

- A remote session survives its connection: SET HOST again, log in and
  answer YES, and the read it was in writes its prompt again.
- `SHOW TERMINAL` shows `Physical Terminal: _TNAn:`; `SHOW USERS/FULL`
  shows a virtual terminal's physical one, or `Disconnected`.
- At most 9 virtual terminals, as for TELNET terminals. `TTY_TIMEOUT` is
  fixed. Writes that come while it is disconnected wait, as on VMS, so a
  program that writes stops until the user comes back.
- Follow-ups: virtual terminals for the console and for pseudo
  terminals, `FTAn`, an SSH server's; `SET TERMINAL/DISCONNECT`; a
  SYSGEN for `TTY_TIMEOUT` and `TTY_DEFCHAR2`.
