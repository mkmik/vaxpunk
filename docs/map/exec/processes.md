+++
title = "Processes"
summary = "PCBs, $CREPRC and $DELPRC, PIDs, hibernation and suspension, jobs"
aka = ["PCB", "PID", "$CREPRC", "$DELPRC", "$HIBER", "$WAKE", "$SUSPND", "$RESUME", "$SETPRI", "$SETPRN", "subprocess", "detached process", "JIB", "swapper", "process.mar"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["docs/design/0002-executive-processes.md", "docs/adr/0003-one-cpu-many-threads.md", "docs/adr/0005-access-modes-are-threads.md", "docs/adr/0006-cli-in-p1-runs-images-in-its-process.md", "docs/adr/0011-asts-on-the-kernel-stack.md", "docs/adr/0019-mailboxes.md", "docs/adr/0029-loginout-and-sysuaf.md", "docs/prd/0003-multi-user-vms.md", "vms/exec/process.mar", "vms/exec/exec.mar"]

[[feature]]
id = "create-delete"
title = "$CREPRC and $DELPRC: a PCB, a kernel stack, an image; deletion by a kernel AST"
status = "done"
milestone = "m0"
refs = ["docs/adr/0003-one-cpu-many-threads.md", "docs/adr/0011-asts-on-the-kernel-stack.md"]

[[feature]]
id = "control"
title = "$HIBER, $WAKE, $SUSPND, $RESUME, $SETPRI, $SETPRN"
status = "done"
milestone = "m0"

[[feature]]
id = "cli"
title = "A command interpreter in P1 that outlives the images it runs"
status = "done"
milestone = "m0"
refs = ["docs/adr/0006-cli-in-p1-runs-images-in-its-process.md"]

[[feature]]
id = "termination-mailbox"
title = "The termination message, MSG$_DELPROC, to $CREPRC's mailbox"
status = "done"
milestone = "m0"
needs = ["/exec/mailboxes"]
refs = ["docs/adr/0019-mailboxes.md"]

[[feature]]
id = "uic-privileges"
title = "A UIC and privileges each; GROUP and WORLD to touch another process"
status = "done"
milestone = "m1"
needs = ["/exec/privileges", "/exec/uic-protection"]
note = "PRD-0003 step 1"

[[feature]]
id = "login"
title = "EXE$LOGIN: LOGINOUT makes a terminal's process a user's"
status = "done"
milestone = "m1"
note = "PRD-0003 step 4, ADR-0029"

[[feature]]
id = "quotas"
title = "Process quotas from $CREPRC and the UAF; PRCLM on subprocesses"
status = "todo"
milestone = "m1"
note = "PRD-0003 tier 1 #4, step 5; $CREPRC ignores its quota list (process.mar); with t8y:/exec/quotas#login, :creprc and :fillm-prclm; t8y:/exec/quotas needs this component, so no need back"

[[feature]]
id = "jobs"
title = "Jobs: a JIB with the job's pooled quotas and its logical name table"
status = "todo"
milestone = "m1"
needs = ["/exec/logical-names#job-group"]
note = "PRD-0003 *Privileges, UICs and protection*: LNM$JOB, each logged-in job's; pooled quotas with t8y:/exec/quotas#job-pooled, which keeps them here"

[[feature]]
id = "altpri"
title = "$SETPRI above the base priority takes ALTPRI"
status = "todo"
note = "process.mar's $SETPRI checks GROUP and WORLD only"

[[feature]]
id = "subprocesses"
title = "Subprocesses and detached processes: PRC$M_DETACH, deleted with their owner"
status = "todo"
note = "$CREPRC ignores stsflg (process.mar); SPAWN and batch jobs need it"

[[feature]]
id = "more-processes"
title = "More than 31 processes: MAXPROCESSCNT instead of SCH$C_MAXPIX"
status = "todo"
note = "lib.mlb: 32 slots, index 0 unused"

[[feature]]
id = "modes"
title = "A process's mode, interactive, batch or network: $CREPRC's PRC$M_BATCH and PRC$M_NETWRK, $GETJPI's JPI$_MODE"
status = "todo"
note = "getjpi.mar has no JPI$_MODE; F$MODE() guesses from SYS$INPUT (t8y:/dcl/lexicals#mode)"

[[needs]]
on = "/exec/scheduler"
why = "Its states, queues and priority"

[[needs]]
on = "/exec/memory"
why = "Its own P0 and P1"

[[needs]]
on = "/exec/pool"
why = "The PCB and the kernel stack come from nonpaged pool"

[[needs]]
on = "/exec/asts"
why = "$DELPRC of another process is a kernel mode AST in it"

[[needs]]
on = "/exec/image-activator"
why = "A new process starts by activating its image"

[[needs]]
on = "/exec/logical-names"
why = "Its process table, with SYS$INPUT, SYS$OUTPUT and SYS$ERROR"

[[needs]]
on = "/platform/access-mode-threads"
why = "Each of its access modes is a seL4 thread with an address space"

[[needs]]
on = "/platform/pal"
why = "SWPCTX starts its threads, DELCTX deletes them"
+++

A VMS process is what owns things: a process control block (PCB) in
nonpaged pool, a process header with its page tables and working set
list, P0 for the image, P1 for the command interpreter, its stacks and
per-process data, and its channels, locks, logical names and quotas.
`$CREPRC` makes either a subprocess, which shares its creator's job and
quotas and goes when the creator goes, or a detached process, a job of
its own: `LOGINOUT` runs in one for each terminal, and the job
controller makes one for each batch job. A process keeps its command
interpreter for its life and runs images one after another. Process
names are unique within a UIC group; a PID names one process.

vaxpunk's processes are in `vms/exec/process.mar`. A process is a PCB
and a 16 KB kernel stack from pool, its image and stacks in its own P0
and P1, and the seL4 threads the PAL makes for it, one per access mode it
enters ([ADR-0003](../../adr/0003-one-cpu-many-threads.md),
[ADR-0005](../../adr/0005-access-modes-are-threads.md)). Only one
runs at a time. DCL lives in P1 in supervisor mode and runs images in
the process ([ADR-0006](../../adr/0006-cli-in-p1-runs-images-in-its-process.md)).
Deleting another process is a kernel mode AST in it
([ADR-0011](../../adr/0011-asts-on-the-kernel-stack.md)), and
rundown sends the termination message to a mailbox
([ADR-0019](../../adr/0019-mailboxes.md)). Since PRD-0003's steps 1
and 4 each process has a UIC, privileges and a username, which
`LOGINOUT` sets ([ADR-0029](../../adr/0029-loginout-and-sysuaf.md)).
Process 1 is the swapper, as on VMS, but it only reaps deleted processes
and starts `LOGINOUT` on the console. Missing: quotas, jobs and the JIB,
subprocesses that go with their owner, and more than 31 processes.
