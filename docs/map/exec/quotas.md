+++
title = "Quotas"
summary = "Per-process and per-job limits on pool, I/O, ASTs, timers, files, locks"
aka = ["BYTLM", "BIOLM", "DIOLM", "ASTLM", "TQELM", "PRCLM", "FILLM", "ENQLM", "BUFQUO", "PGFLQUOTA", "WSQUOTA", "WSEXTENT", "WSDEFAULT", "JTQUOTA", "CPU limit", "JIB", "SS$_EXQUOTA", "RSRCWAIT", "SHOW PROCESS/QUOTAS", "EXQUOTA privilege"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m1"
refs = ["docs/prd/0003-multi-user-vms.md", "docs/adr/0038-io-quotas-charged-with-the-irp.md", "docs/adr/0013-qio-irps-and-drivers.md", "docs/adr/0019-mailboxes.md", "docs/design/0002-executive-processes.md", "vms/exec/qio.mar", "vms/exec/mbdriver.mar", "vms/exec/process.mar", "vms/sysexe/authorize.mar", "vms/sysexe/loginout.mar"]

[[feature]]
id = "uaf-fields"
title = "The UAF holds them, and AUTHORIZE sets /ASTLM, /BIOLM, /BYTLM, /DIOLM, /ENQLM, /FILLM, /PRCLM, /TQELM"
status = "done"
milestone = "m1"
note = "PRD-0003 step 4; authorize.mar, $UAFDEF in lib.mlb"

[[feature]]
id = "login"
title = "LOGINOUT gives the process the UAF's quotas"
status = "wip"
milestone = "m1"
note = "PRD-0003 step 5; EXE$LOGIN sets BIOLM, DIOLM and BYTLM (ADR-0038), not the others yet. t8y:/utils/loginout's work, on this component"

[[feature]]
id = "pool-io"
title = "BYTLM, BIOLM and DIOLM charged per I/O request; a request past BIOLM waits"
status = "done"
milestone = "m1"
needs = ["/exec/qio"]
note = "ADR-0038: charged with the IRP, IOC$ALLOCIRP; QUOTATEST in the boot test"

[[feature]]
id = "asts-timers"
title = "ASTLM and TQELM charged for queued ASTs and timer queue entries"
status = "todo"
milestone = "m1"
needs = ["/exec/asts", "/exec/timers"]
note = "PRD-0003 tier 1 #4; DESIGN-0002 ASTs"

[[feature]]
id = "mailbox-bufquo"
title = "A mailbox's BUFQUO charged to its creator's BYTLM"
status = "todo"
milestone = "m1"
needs = ["/exec/mailboxes"]
note = "PRD-0003 tier 1 #4; ADR-0019"

[[feature]]
id = "fillm-prclm"
title = "FILLM for open files and PRCLM for subprocesses"
status = "todo"
milestone = "m1"
note = "PRD-0003 Quotas"

[[feature]]
id = "enqlm"
title = "ENQLM for locks"
status = "todo"
milestone = "m1"
needs = ["/exec/lock-manager"]
note = "PRD-0003 Quotas and Lock manager"

[[feature]]
id = "getjpi"
title = "$GETJPI returns them and what is left; SHOW PROCESS/QUOTAS"
status = "wip"
milestone = "m1"
needs = ["/exec/information-services#jpi-quotas"]
note = "PRD-0003 tier 1 #4, step 5; the I/O ones done (ADR-0038)"

[[feature]]
id = "creprc"
title = "$CREPRC's quota list, a subprocess's taken from its creator's"
status = "todo"
note = "DESIGN-0002: $CREPRC ignores its quota argument, and gives the process its creator's I/O quotas"

[[feature]]
id = "job-pooled"
title = "Pooled quotas in a job information block, shared by a job's processes"
status = "todo"
note = "VMS keeps BYTLM, FILLM, PRCLM, TQELM, ENQLM in the JIB"

[[feature]]
id = "resource-wait"
title = "Resource wait mode, $SETRWM: wait for quota or fail with SS$_EXQUOTA"
status = "done"
note = "ADR-0038, for the I/O quotas"

[[feature]]
id = "memory-quotas"
title = "PGFLQUOTA, WSDEFAULT, WSQUOTA, WSEXTENT"
status = "todo"
note = "With t8y:/exec/paging, a PRD-0003 non-goal"

[[feature]]
id = "cpu-limit"
title = "A CPU time limit that ends the process"
status = "todo"
needs = ["/exec/accounting#cpu-time"]

[[needs]]
on = "/exec/processes"
why = "Limits and what is left of them live in the PCB and JIB"

[[needs]]
on = "/exec/pool"
why = "BYTLM is pool a process may take"

[[needs]]
on = "/exec/privileges"
why = "EXQUOTA lets a process exceed its quotas"
soft = true

[[needs]]
on = "/exec/paging"
why = "Working set and page file quotas"
soft = true
+++

VMS gives each process limits so one user can't take the system:
`BYTLM` bytes of pool for buffered I/O, `BIOLM` and `DIOLM` buffered
and direct I/O requests outstanding, `ASTLM` ASTs queued, `TQELM`
timer queue entries, `FILLM` open files, `PRCLM` subprocesses,
`ENQLM` locks, and for memory `PGFLQUOTA` and the working set sizes.
They come from the UAF at login, or from `$CREPRC`'s quota list, and
some are pooled in the job information block, shared by a process and
its subprocesses. A request past a limit waits, in resource wait mode,
or fails with `SS$_EXQUOTA`; `EXQUOTA` lets a process ignore them.
`SHOW PROCESS/QUOTAS` shows what is left. Disk quotas are a different
thing, per volume (`t8y:/files/disk-quotas`).

vaxpunk has the I/O ones in force
([ADR-0038](../../adr/0038-io-quotas-charged-with-the-irp.md)): each
IRP a `$QIO` makes is charged to `BIOLM` and `BYTLM`, or a disk's to
`DIOLM`, and a request past them waits in resource wait, or with
`$SETRWM` fails, so a process that queues reads past `BIOLM` waits
instead of taking the pool. `EXE$LOGIN` sets them from the UAF,
`$CREPRC` gives a process its creator's, and `$GETJPI` and `SHOW
PROCESS/QUOTAS` report them. The others are the UAF's, which `AUTHORIZE`
sets (PRD-0003 step 4), and nothing charges them yet: a mailbox's
`bufquo` is its own ([ADR-0019](../../adr/0019-mailboxes.md)), and
`$CREPRC` ignores its quota list.
