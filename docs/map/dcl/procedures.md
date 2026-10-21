+++
title = "Command procedures"
summary = "@file: .COM files of DCL commands, with parameters P1 to P8"
aka = ["@", ".COM", "command procedure", "P1", "SET VERIFY", "INQUIRE", "DECK", "EOD", "WAIT", "data lines", "SET RESTART_VALUE"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["vms/sysexe/dcl/proc.mar", "vms/sysexe/dcl.mar", "docs/prd/0003-multi-user-vms.md", "docs/adr/0029-loginout-and-sysuaf.md", "vms/sysmgr/dcltest.com"]

[[feature]]
id = "at"
title = "@file [p1 ... p8]: nested procedures, local symbols per level"
status = "done"
milestone = "m0"

[[feature]]
id = "data-lines"
title = "Lines without $ are the image's SYS$INPUT, and READ SYS$INPUT's"
status = "done"
milestone = "m1"
note = "PRD-0003 step 8"

[[feature]]
id = "verify"
title = "SET [NO]VERIFY and F$VERIFY: lines shown as they run"
status = "done"
milestone = "m1"
note = "PRD-0003 step 8"

[[feature]]
id = "file-input"
title = "A file as SYS$INPUT runs as a procedure, then logs out: STARTUP and batch"
status = "done"
milestone = "m1"
note = "ADR-0029 decision 5"

[[feature]]
id = "ppf-input"
title = "Data lines read by RMS from the procedure, a process-permanent file, not a mailbox"
status = "todo"
milestone = "m1"
needs = ["/rms/ppf#procedure-input", "/exec/logical-names"]
note = "PRD-0003 tier 1 #6; proc.mar ponytail: SYS$INPUT replaced in LNM$PROCESS, TMPMBX needed"

[[feature]]
id = "read-as-run"
title = "Procedures read as they run, not whole into a 16 KB buffer"
status = "todo"
note = "dcl.mar ponytail: PBUF"

[[feature]]
id = "inquire"
title = "INQUIRE symbol [prompt]"
status = "todo"

[[feature]]
id = "deck"
title = "DECK[/DOLLARS] and EOD: data lines that start with $"
status = "done"
note = "As RMS's RM1INPSCN, in DCL$DATALINE"

[[feature]]
id = "wait"
title = "WAIT delta-time"
status = "todo"
needs = ["/exec/timers", "/exec/time-conversion"]
note = "Needs $BINTIM (PRD-0003 tier 1 #7)"

[[feature]]
id = "output"
title = "@file/OUTPUT=file"
status = "todo"

[[feature]]
id = "restart"
title = "SET RESTART_VALUE and BATCH$RESTART, for batch jobs that restart"
status = "todo"
needs = ["/utils/job-controller"]

[[needs]]
on = "/dcl/interpreter"
why = "Each procedure line is a command DCL does"

[[needs]]
on = "/dcl/symbols"
why = "P1 to P8 and local symbols belong to a procedure level"

[[needs]]
on = "/rms/sequential"
why = "Reads the .COM file with RMS"

[[needs]]
on = "/exec/mailboxes"
why = "Data lines reach the image through a mailbox (proc.mar)"
+++

A command procedure is a file of DCL commands, `.COM` by default, each
line starting with `$`. `@file p1 p2 ...` runs it, with up to eight
parameters as the local symbols `P1` to `P8`; procedures call
procedures, each a level with its own local symbols. Lines without `$`
are data: an image the procedure runs reads them as its `SYS$INPUT`.
`SET VERIFY` shows each line as it runs. `INQUIRE` asks the terminal
for a value, `DECK` and `EOD` mark data lines that themselves start
with `$`, and `WAIT` pauses. A batch job is a procedure run in a
process of its own, its output going to a log file; `SET
RESTART_VALUE` lets a job that was interrupted pick up where it was.
`SYS$MANAGER:SYSTARTUP_VMS.COM`, `SYLOGIN.COM` and `LOGIN.COM` are
procedures too.

vaxpunk's are in `vms/sysexe/dcl/proc.mar`. `@` reads the whole file
into a 16 KB buffer shared by the levels running. A procedure's data
lines go to the image through a mailbox that replaces `SYS$INPUT` while
it runs, since logical names have no access modes and RMS has no
process-permanent files yet (PRD-0003 tier 1 #6). Which lines are data
is decided as VMS's RMS decides it for an image reading the procedure:
a `$` line ends them, but after `DECK` only `$ EOD` does, or the
string `DECK/DOLLARS=string` gives; each `EOD` ends one set and is an
end of file in the mailbox, and the next set may follow. `STARTUP` runs
`SYSTARTUP_VMS.COM` as its `SYS$INPUT`, as a batch job would run
([ADR-0029](../../adr/0029-loginout-and-sysuaf.md)).
`vms/sysmgr/dcltest.com` checks procedures in the boot test.
