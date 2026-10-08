# ADR-0027 — Each terminal is a UCB of its own, and a remote login is a TELNET terminal, TNAn, that the terminal driver drives over the TCP connection

Oct 8, 2026 · @Marko Mikulicic

Proposed. The terminal driver keeps each terminal's state in its UCB:
its characteristics, type-ahead buffer, line being read, recall buffer,
CTRL/C and CTRL/Y ASTs and a timer for timed reads. The console, `OPA0:`,
is one; a remote login is another, `TNAn:`, which TELNETD makes of the
connection it accepts with `IO$_TTY_PORT` on the socket's channel. The
driver speaks Telnet itself, sending and receiving on the connection
through the network driver, so DCL, EDIT and every other program see a
remote terminal as they see the console. RTPAD, `SET HOST`'s client,
goes a character at a time once the other end offers to echo.

## Context

[PRD-0003](../prd/0003-multi-user-vms.md)'s backlog item 3: there was one
terminal. `ttdriver.mar` kept the line being read, the type-ahead and
recall buffers, the CTRL/C and CTRL/Y queues and the characteristics in
its own data, for the console. A `SET HOST` session's DCL read and wrote
its network unit, `BGnn:`, which the network driver made look a little
like a terminal: `IO$_READPROMPT` sent the prompt and received a line,
and the AST modifiers did nothing. RTPAD sent a line at a time, edited
on the local console. So `SET TERMINAL` and `SHOW TERMINAL` worked on
the console only, and EDIT, which assigned `OPA0:` by name, wrote on the
console in a remote login. EDIT's keypad mode, and TPU after it
([PRD-0006](../prd/0006-tpu-and-eve.md)), need a terminal that reads a
key at a time, with its terminators and escape sequences, and knows its
size.

On VMS the terminal driver is a class driver, `TTDRIVER`, whose UCB per
terminal holds this state, over port drivers for each kind of line: the
console's, serial lines', LAT's, and TCP/IP Services' `TNDRIVER`, whose
`TNAn:` devices its TELNET server makes inside the system, one for each
connection, before `LOGINOUT` runs on them. The terminal driver's `$QIO`
interface (the I/O User's Reference Manual, the terminal chapter) is
what screen programs use: `IO$M_NOFILTR`, `IO$M_TIMED`, `IO$M_ESCAPE`,
`IO$M_TRMNOECHO`, terminator masks, `IO$M_TYPEAHDCNT`, and PASTHRU,
which passes every key, CTRL/C and CTRL/Y too, as `SET HOST` uses it.

## Decision

- A terminal's state lives in its UCB (`$UCBDEF`'s `UCB$x_TT_` fields).
  The driver's routines take the UCB in R5, as a VMS class driver's do;
  the terminals are a list, `TT$GL_UNITS`, the console first.
- Two ports, chosen by a bit in the UCB, not a port vector: the console's
  output is written at once, waiting for it, and its input comes from its
  receive interrupt; a TELNET terminal's output goes to its connection
  in sends, one on the port at a time so that they stay in order, and
  its input comes from a receive it keeps on the port, as large as its
  type-ahead buffer has room for.
- `IO$_TTY_PORT` on a connected TCP socket moves the connection and the
  channel to a new `TNAn` unit. TCP/IP Services makes its TNA devices
  inside its TELNET server, with no interface a program sees; this one
  is vaxpunk's own, with VMS's name for a port driver function.
- The driver speaks Telnet (RFC 854): it offers to echo and to suppress
  go ahead, asks for the window size (NAWS, RFC 1073), which becomes the
  terminal's width and page, and refuses other options. A connection that
  closes hangs the terminal up: reads and writes end with `SS$_HANGUP`.
- The read side is VMS's: terminator masks, the default terminators,
  line editing with insert and overstrike, `IO$M_TIMED` with a timer
  queue entry in the UCB, `IO$M_ESCAPE`, `IO$M_PURGE`,
  `IO$M_TRMNOECHO`, `IO$M_CVTLOW`, `IO$M_TYPEAHDCNT`, PASSALL and
  PASTHRU.
- RTPAD starts a line at a time, as before, and goes a character at a
  time, PASTHRU and NOECHO, when the other end says `WILL ECHO`: so it
  still works with a server that doesn't speak Telnet. CTRL/] ends a
  session.

## Alternatives considered

- **A pseudo-terminal pair, as VMS's `PTD$` services make for DECterm,
  and TELNETD passing bytes between it and the socket in user mode.**
  Telnet's parsing would be in TELNETD, in user mode, where a mistake is
  cheaper. But it needs a process, or AST-driven code serving every
  connection, copying each key and each line twice and switching
  processes for each, in a system with 4 MB of memory; `PTD$`'s interface
  is page-aligned buffers with status words, a lot to build for one
  user. TCP/IP Services itself did it inside the system.
- **Keep the network unit as the remote terminal**, with more terminal
  functions in the network driver. That is the terminal driver written
  twice, and every terminal feature, the characteristics, timed reads,
  CTRL/Y, twice more.
- **A port vector in each UCB**, as VMS's class and port drivers have.
  With two ports a bit and two branches are less; a third port, a serial
  line, would make it worth it.
- **Line mode Telnet (RFC 1184)**, editing on the client. EDIT's keypad
  mode and TPU need every key at once; character mode with remote echo
  is what VMS's TELNET does.

## Consequences

- EDIT's keypad mode works in a `SET HOST` session, on a screen of the
  client's size; `SET TERMINAL` and `SHOW TERMINAL` work on any terminal;
  `$GETDVI` and `$DEVICE_SCAN` see the `TNAn` units; the recall buffer is
  each terminal's own.
- The network driver gains a way to send and receive for the terminal
  driver (`NET$TTIO`, `TT$NETDONE`, `NET$TTCLOSE`), and loses its
  terminal imitation.
- Telnet's parsing runs at `IPL$_SYNCH`, in the executive.
- At most 9 TELNET terminals: a device name has 4 characters.
- Follow-ups: `LOGINOUT` on a new terminal instead of DCL (PRD-0003, step
  4); a hangup that deletes the process, as VMS's does; `$BRKTHRU`;
  CTRL/O, CTRL/T and XON/XOFF; the recall buffer DCL's, with `RECALL`;
  TELNET's command mode at CTRL/].
