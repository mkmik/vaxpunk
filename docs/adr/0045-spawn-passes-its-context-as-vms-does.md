# ADR-0045 — SPAWN's subprocess runs LOGINOUT, a known image with CMKRNL, which copies its owner's process logical names, and DCL passes the rest of the context in a mailbox that is the subprocess's SYS$ERROR

Oct 27, 2026 · @Marko Mikulicic

Proposed. DCL's `SPAWN` makes a subprocess with `$CREPRC` that runs
`LOGINOUT.EXE`, as VMS's does. In a subprocess `LOGINOUT` asks for no
username: it calls `EXE$LOGIN` without a UAF record, which gives the
process DCL and a copy of its owner's process logical names, and keeps
the username, UIC and privileges `$CREPRC` gave it. To call it,
`LOGINOUT` is a known image with `CMKRNL`, as VMS installs it. The rest
of the context, the symbols, the prompt, `SET VERIFY` and the command,
goes through a mailbox that is the subprocess's `SYS$ERROR`, as on VMS;
the subprocess's DCL reads it, makes `SYS$ERROR` `SYS$OUTPUT`'s again
and says so with an empty message in the termination mailbox, where
`SPAWN` then waits for its termination message.

## Context

[t8y:/dcl/spawn](../map/dcl/spawn.md): `SPAWN [command]` makes a
subprocess that runs DCL with a copy of the parent's symbols, process
logical names, default directory, privileges, prompt and verify state,
which does the command and logs out, or reads the terminal until
`LOGOUT`, while the parent waits. `PIPE`'s pipelines need subprocesses
too ([t8y:/dcl/pipe#pipelines](../map/dcl/pipe.md)).

In DEC's VAX/VMS V4.3 sources (`DCL/SRC/SPAWN.MAR`, `INITIAL.MAR` and
`LOGIN/SRC/LOGIN.B32`, read for what they do, not copied), `SPAWN`
creates a mailbox, writes its name as the new process's `SYS$ERROR` and
creates it with `$CREPRC` on `SYS$SYSTEM:LOGINOUT`, named `USERNAME_1`,
the first free number, with the parent's base priority and a termination
mailbox. It writes the context to the mailbox, a record at a time: a
header with the privileges, the prompt and the verify flag, the command,
the process logical names and tables, and the global and the current
procedure's local symbols. Its writes wait for the reader. `LOGINOUT`
sees it is a subprocess (it has an owner), reads no UAF, maps the CLI,
and makes the executive mode `SYS$ERROR` `SYS$OUTPUT`'s, keeping the
mailbox's name in a supervisor mode one, which DCL reads at its start
and deletes. DCL skips `LOGIN.COM`, and after the command logs out
without a word. The parent hibernates until the subprocess's
termination message comes, prints `%DCL-S-RETURNED`, and its final
status becomes `$STATUS`. VMS's DCL reads the process logical names
itself: they are in P1, which supervisor mode can read.

vaxpunk has `$CREPRC` with subprocesses, termination mailboxes and
mailboxes that take an end of file ([ADR-0019](0019-mailboxes.md)), and
`LOGINOUT` with `EXE$LOGIN` ([ADR-0029](0029-loginout-and-sysuaf.md)).
`$CREPRC` already gives a subprocess its creator's username, UIC,
privileges, rights, quotas and default directory. The logical names are
in nonpaged pool, which only kernel mode reads; the only way to list
them, `LNM$NAMES`, is a kernel routine for `$CMKRNL`, which DCL's
supervisor mode can't call without the privilege. A mailbox write is
done at once, and a temporary mailbox goes with its last channel.

## Decision

1. `SPAWN` makes the context mailbox and a termination mailbox, writes
   the whole context, then an end of file, and creates the subprocess:
   `LOGINOUT.EXE`, `SYS$INPUT` `/INPUT`'s file or device, else the
   terminal, `SYS$OUTPUT` the parent's, `SYS$ERROR` the context
   mailbox's name, `USERNAME_n` or `/PROCESS`'s name, the parent's base
   priority, and the termination mailbox's unit.
2. `LOGINOUT`, a known image with `CMKRNL` in the executive's `KNOWN`
   list, calls `EXE$LOGIN` with no record in a subprocess. `EXE$LOGIN`
   then copies the owner's `LNM$PROCESS_TABLE` but the process-permanent
   names (`LNM$SPAWN`) and activates `DCL.EXE`, and refuses if the process
   has a command interpreter already. With a record it now wants the
   process authorized `CMKRNL`, as the executive's and `TELNETD`'s
   `LOGINOUT` processes are, so that the known image's privilege can't
   log a user in from a `SYSUAF` of their own.
3. The subprocess's DCL, owned and with a mailbox for `SYS$ERROR`, reads
   the records to the end of file, defines `SYS$ERROR` as `SYS$OUTPUT`'s
   string, and writes an empty message to the termination mailbox, whose
   name the header has. `SPAWN` reads that mailbox: the empty message
   lets it deassign the context mailbox, then, with `/WAIT`, it reads on
   until the termination message, with its CTRL/Y AST disabled.
4. DCL's `LOGOUT` sets the final status: it calls `$EXIT` with `$STATUS`,
   and deletes the process when the executive calls it again.

## Alternatives considered

- **The subprocess runs `DCL.EXE` itself**, as `RUN/PROCESS` of DCL
  does now. No privilege needed, but no one can copy the logical names:
  DCL's supervisor mode can't read them.
- **A system service that lists the caller's process logical names**,
  for DCL. A service VMS doesn't have, for one caller; `LNM$NAMES`
  serves `SHOW.EXE`, a known image, the same way.
- **`$CREPRC` copies the logical names into every subprocess.** VMS's
  doesn't: a `RUN` subprocess starts with only its process-permanent
  names.
- **Logical names in the context mailbox, read in kernel mode by a
  known image the parent runs.** Running an image from DCL runs the
  current one down, which a `SPAWN` at the CTRL/Y prompt must not.
- **The parent waits for the mailbox to empty**, polling its message
  count, or with VMS's writes that wait for the reader. Either changes
  the mailbox driver or burns time; the empty message costs a write.

## Consequences

- `SPAWN`, with `/WAIT` and `/NOWAIT`, `/INPUT`, `/PROCESS`, `/PROMPT`,
  `/[NO]LOG` and `/[NO]SYMBOLS`. Not yet: `ATTACH`, `LIB$SPAWN`,
  `/OUTPUT` (no process-permanent files, t8y:/rms/ppf), `/NOTIFY`,
  `/KEYPAD`, `/[NO]LOGICAL_NAMES`, which `LOGINOUT` can't see, the
  process's own logical name tables, and `SET COMMAND`'s verbs.
- A user without privileges can spawn: `LOGINOUT`'s known `CMKRNL` does
  only the subprocess's half of `EXE$LOGIN`.
- The parent waits in a `$QIOW` on the termination mailbox, state LEF,
  where VMS's hibernates; `ATTACH` will need it to wait for an attach
  request too.
- `USERNAME_n` stops at 99, and the name must fit in 15 characters.
- A `/NOWAIT` subprocess's termination message goes to a mailbox that is
  gone, or, after 9,999 more mailboxes, to another with that unit.
