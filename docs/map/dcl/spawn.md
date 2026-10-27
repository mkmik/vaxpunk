+++
title = "SPAWN and ATTACH"
summary = "Subprocesses with DCL: SPAWN, ATTACH, and LIB$SPAWN under them"
aka = ["SPAWN", "ATTACH", "subprocess", "LIB$SPAWN", "SPAWN/NOWAIT", "termination mailbox", "USERNAME_1", "%DCL-S-SPAWNED"]
kind = "core"
intent = "goal"
research = "done"
refs = ["docs/adr/0045-spawn-passes-its-context-as-vms-does.md", "docs/adr/0019-mailboxes.md", "vms/sysexe/dcl/spawn.mar", "vms/sysexe/loginout.mar", "vms/exec/lnm.mar"]

[[feature]]
id = "spawn"
title = "SPAWN [command]: a subprocess with the parent's symbols, prompt and verify, which does the command or reads the terminal; /WAIT, /NOWAIT, /INPUT, /PROCESS, /PROMPT, /LOG, /SYMBOLS"
status = "done"
note = "ADR-0045: the context goes through a mailbox that is the subprocess's SYS$ERROR"

[[feature]]
id = "logical-names"
title = "The parent's process logical names, copied into the subprocess"
status = "done"
note = "ADR-0045: LOGINOUT, a known image with CMKRNL, has EXE$LOGIN copy them (LNM$SPAWN); not the process's own tables"

[[feature]]
id = "attach"
title = "ATTACH: the terminal to another process of the job, which this one waits for"
status = "todo"
note = "SPAWN's parent waits in a $QIOW on the termination mailbox; ATTACH needs it to wait for an attach request too, as VMS's attach mailbox does"

[[feature]]
id = "lib-spawn"
title = "LIB$SPAWN: SPAWN for a program, through DCL"
status = "todo"
note = "The CLI callback, SYS$CLI, which the image calls and DCL serves in supervisor mode; t8y:/rtl/librtl#processes"

[[feature]]
id = "output"
title = "/OUTPUT: the subprocess's SYS$OUTPUT a file, .LOG by default"
status = "todo"
needs = ["/rms/ppf#log-file"]

[[feature]]
id = "notify"
title = "/NOTIFY: a broadcast when a /NOWAIT subprocess ends"
status = "todo"
note = "Needs a termination AST on the mailbox, as VMS's SPAWN has"

[[feature]]
id = "more-qualifiers"
title = "/NOLOGICAL_NAMES, /KEYPAD, /CLI, /TABLE, /CARRIAGE_CONTROL, and SET COMMAND's verbs in the subprocess"
status = "todo"
note = "LOGINOUT copies the names before DCL reads the context, so it can't be told /NOLOGICAL_NAMES yet"

[[needs]]
on = "/exec/processes"
why = "A subprocess is $CREPRC's, in the parent's job"

[[needs]]
on = "/exec/mailboxes"
why = "The parent hands its symbols and names over, and learns the end, through mailboxes"

[[needs]]
on = "/utils/loginout"
why = "The subprocess starts in LOGINOUT, which maps DCL and takes the parent's context"

[[needs]]
on = "/dcl/interpreter"
why = "SPAWN and ATTACH are DCL's commands"

[[needs]]
on = "/exec/logical-names"
why = "LNM$JOB is shared by the job; process names are copied"

[[needs]]
on = "/drivers/terminal"
why = "ATTACH moves the terminal from one process of the job to another"

[[needs]]
on = "/exec/quotas"
why = "PRCLM limits the subprocesses"
soft = true
+++

`SPAWN [command]` creates a subprocess that runs DCL, with a copy of the
parent's symbols, logical names, default directory and key definitions.
It runs the command, or prompts on the same terminal until `LOGOUT`.
`/NOWAIT` lets the parent go on; `/INPUT`, `/OUTPUT`, `/PROCESS` and
`/NOTIFY` say where it reads and writes and what it is called.
`ATTACH process` moves the terminal to another process of the same job
and suspends this one; a spawned DCL's `ATTACH` goes back to its parent.
`LIB$SPAWN` does the same for programs, and is how the C library's
`system()` and editors' `SPAWN` commands work. `PIPE`'s pipelines and
subshells are subprocesses too.

vaxpunk's `SPAWN` is in `vms/sysexe/dcl/spawn.mar`
([ADR-0045](../../adr/0045-spawn-passes-its-context-as-vms-does.md)).
The subprocess, `USERNAME_1` or the next free number, runs `LOGINOUT`,
which gives it DCL and a copy of the process logical names in kernel
mode; DCL reads the rest, the symbols, prompt, verify state and
command, from a mailbox that is its `SYS$ERROR`, then makes that
`SYS$OUTPUT`'s again. It skips `LOGIN.COM`, does the command or
`/INPUT`'s procedure and logs out without a word, or reads the terminal
until `LOGOUT`, which says `Process USERNAME_1 logged out`. The parent
says `%DCL-S-SPAWNED` and `%DCL-S-ATTACHED`, waits for the termination
message with its CTRL/Y disabled, says `%DCL-S-RETURNED`, and takes the
subprocess's final status as `$STATUS`. Missing: `ATTACH`, `LIB$SPAWN`,
`/OUTPUT`, `/NOTIFY` and the rarer qualifiers.
