# ADR-0042 — OPCOM is a process that reads the operator's mailbox, MBA2:, which the executive makes at boot and $SNDOPR writes, and it writes on the console with $BRKTHRU

Oct 21, 2026 · @Marko Mikulicic

Proposed. As on VMS, the executive makes a permanent mailbox at boot,
`MBA2:`, the operator's, and `$SNDOPR` writes each message to it after a
header that says who sent it. OPCOM is a program, `OPCOM.EXE`, which
`SYSTARTUP_VMS.COM` starts with `RUN/DETACHED/PRIORITY=6/PROCESS_NAME=OPCOM`. It
reads `MBA2:` and writes each message on the operator terminals with
`$BRKTHRU`, under VMS's banner. The console, `OPA0:`, is one from the
start, for VMS's default classes; it is the only one for now.

## Context

[t8y:/drivers/console#opcom](../map/drivers/console.md): on VMS the
console is an operator terminal. Programs send OPCOM messages with
`$SNDOPR`: `REQUEST`, `MOUNT` asking for a volume, the job controller,
the audit server, a startup procedure's `%LICENSE-E-NOAUTH`. OPCOM
writes them on the terminals enabled for their class, and the console
is enabled when OPCOM starts. Real OpenVMS Alpha V8.4-2L1 in AXPbox
prints, at boot and after `REQUEST "hello"`:

```
%%%%%%%%%%%  OPCOM  21-OCT-2026 05:27:16.79  %%%%%%%%%%%
Operator _PUNK$OPA0: has been enabled, username SYSTEM

%%%%%%%%%%%  OPCOM  21-OCT-2026 05:27:52.50  %%%%%%%%%%%
Message from user SYSTEM on PUNK
_PUNK$OPA0:, hello
```

Each message is a bell, the banner, the lines and a line feed, written
on a line of its own. In DEC's VAX/VMS V4.3 sources (`SYSSNDMSG.MAR`,
`OPCOMINI.B32`, `OPCOMRQST.B32`, read for what they do, not copied),
`$SNDOPR` writes to `SYS$GL_OPRMBX`, the UCB of `MBA2:`, a mailbox the
executive owns, after a 38-byte header: `MSG$_OPRQST`, the reply
mailbox's unit, the sender's privileges, UIC, username, account and
base priority. With no reader on `MBA2:` it returns `OPC$_NOPERATOR`, a
success. OPCOM assigns a channel to `_MBA2:` by name.

vaxpunk has mailboxes ([ADR-0019](0019-mailboxes.md)), `$CREPRC`, and
`$BRKTHRU` in the terminal driver, which writes between a program's
reads and writes its prompt again.

## Decision

- `MB$INIT`, from `EXEC$START`, makes a permanent mailbox as `$CREMBX`
  would, named `MBA2`, unit 2, which takes messages of 1,024 bytes and
  holds 8,192, and puts its UCB's address in `SYS$GL_OPRMBX`.
- `$SNDOPR msgbuf, chan` builds VMS's header and the message, up to 986
  bytes, on the kernel stack, and writes them with `MB$SEND`. With no
  channel to `MBA2:` it returns `OPC$_NOPERATOR`. `chan`, if it is
  given, must be a mailbox's; its unit goes in the header.
- `OPCOM.EXE` assigns `_MBA2:` and reads it. For an `OPC$_RQ_RQST`
  whose classes are the console's, it writes `Message from user` and the
  sender's username, then the text, with `$BRKTHRU` to `_OPA0:`,
  `BRK$C_OPCOM`. When it starts it writes VMS's two messages that the
  console is enabled, and for which classes.
- `REQUEST "text" [/TO=(class,...)]` is a program, `REQUEST.EXE`, as on
  VMS: it sends `OPC$_RQ_RQST`, the documented `$SNDOPR` format, with the
  name of `SYS$COMMAND` before the text.
- DCL's `RUN` has `/DETACHED`, `/PROCESS_NAME` and `/PRIORITY`, which
  create the process with `$CREPRC` and print `%RUN-S-PROC_ID`, so that
  `SYSTARTUP_VMS.COM` starts OPCOM as VMS's startup does, at priority 6
  (`VMS$CONFIG-050_OPCOM.COM`): above STARTUP's 4, OPCOM says the
  console is enabled before STARTUP goes on and LOGINOUT asks for a
  username.

## Alternatives considered

- **OPCOM makes its mailbox with `$CREMBX` and a logical name**, which
  `$SNDOPR` translates. That needs the logical name looked up in kernel
  mode on each message, and a program could name a mailbox of its own
  `OPCOM`'s. A mailbox the executive makes can't be taken.
- **OPCOM writes on the console with `$QIO`** to `OPA0:`. Its messages
  would land in the middle of a line being typed. `$BRKTHRU` writes them
  between reads and writes the prompt and the line again, as VMS's does.
- **The executive starts OPCOM**, as it starts STARTUP. VMS starts it
  from `STARTUP.COM`, which a site can change, and `RUN/DETACHED` is
  wanted anyway ([t8y:/dcl/interpreter#run-detach](../map/dcl/interpreter.md)).

## Consequences

- `REQUEST` works, and any program can tell the operator something with
  `$SNDOPR`. The executive's own messages, such as `%MOUNT-I-MOUNTED`,
  still go straight to the console.
- The console is the only operator terminal, enabled for VMS's default
  classes, until `REPLY/ENABLE` and `/DISABLE`
  ([t8y:/utils/opcom#operator-terminals](../map/utils/opcom.md)). OPCOM
  shows a request that wants a reply as a message, and doesn't answer it,
  until `REPLY/TO` (`#requests`); there is no `OPERATOR.LOG` (`#log`).
- No node name: the banner's lines lack VMS's ` on NODE` and the
  terminal's `NODE$`, as everywhere else in vaxpunk
  ([t8y:/exec/sysgen-parameters](../map/exec/sysgen-parameters.md)).
- `$BRKTHRU` doesn't know where the cursor is, so a message at the start
  of a line gets an empty line before it, which VMS's doesn't.
- A full `MBA2:` fails `$SNDOPR` with `SS$_MBFULL` instead of waiting
  for room, as every vaxpunk mailbox write does.
- Accounts: the header's is blanks, as a process has none yet.
