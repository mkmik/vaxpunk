+++
title = "SPAWN and ATTACH"
summary = "Subprocesses with DCL: SPAWN, ATTACH, and LIB$SPAWN under them"
aka = ["SPAWN", "ATTACH", "subprocess", "LIB$SPAWN", "SPAWN/NOWAIT", "termination mailbox"]
kind = "core"
intent = "goal"
research = "outline"
status = "todo"
refs = ["docs/adr/0019-mailboxes.md", "vms/sysexe/dcl/pipe.mar"]

[[needs]]
on = "//exec/processes"
why = "A subprocess is $CREPRC's, in the parent's job"

[[needs]]
on = "//exec/mailboxes"
why = "The parent hands its symbols and names over, and learns the end, through mailboxes"

[[needs]]
on = "//utils/loginout"
why = "The subprocess starts in LOGINOUT, which maps DCL and takes the parent's context"

[[needs]]
on = "//dcl/interpreter"
why = "SPAWN and ATTACH are DCL's commands"

[[needs]]
on = "//exec/logical-names"
why = "LNM$JOB is shared by the job; process names are copied"

[[needs]]
on = "//drivers/terminal"
why = "ATTACH moves the terminal from one process of the job to another"

[[needs]]
on = "//exec/quotas"
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

Nothing is built. [ADR-0019](../../docs/adr/0019-mailboxes.md) lists
DCL's `SPAWN` as a follow-up of mailboxes, whose termination message it
uses; `pipe.mar` refuses pipelines for want of it.
