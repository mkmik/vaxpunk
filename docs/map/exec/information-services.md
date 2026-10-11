+++
title = "$GETJPI, $GETSYI, $GETDVI"
summary = "Item-list services that describe processes, the system and devices"
aka = ["$GETJPI", "$GETJPIW", "$GETSYI", "$GETSYIW", "$GETDVI", "$GETDVIW", "$DEVICE_SCAN", "$PROCESS_SCAN", "$GETRMI", "$GETQUI", "$GETLKI", "$GETUAI", "$SETUAI", "$GETTIM", "JPI$_", "SYI$_", "DVI$_", "F$GETJPI", "F$GETSYI", "F$GETDVI", "item list", "SHOW SYSTEM", "SHOW PROCESS", "SHOW DEVICES"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["docs/prd/0003-multi-user-vms.md", "docs/design/0002-executive-processes.md", "vms/exec/getjpi.mar", "vms/exec/getdvi.mar", "vms/sysexe/show.mar", "vms/sysexe/dcl/lexical.mar", "crosstools/vtools/lib/starlet.mlb"]

[[feature]]
id = "getjpi"
title = "$GETJPI on one process or a wildcard scan: PID, names, state, priorities, UIC, privileges"
status = "done"
milestone = "m0"
note = "getjpi.mar; SHOW SYSTEM lists processes with it. Privileges and UIC from PRD-0003 step 1"

[[feature]]
id = "getsyi"
title = "$GETSYI: version, process slots, quantum, boot time, STARTUP_P1 to P8"
status = "done"
milestone = "m0"
note = "getjpi.mar; STARTUP_Pn from commit 6e5df3f"

[[feature]]
id = "getdvi"
title = "$GETDVI and $DEVICE_SCAN on disks, terminals, mailboxes and sockets"
status = "done"
milestone = "m0"
note = "getdvi.mar; terminals from their UCBs since PRD-0003 step 3"

[[feature]]
id = "devices-from-ucbs"
title = "Every device from its UCB: BGA0: and its units scanned, a disk's UCB rather than its VCB"
status = "todo"
milestone = "m1"
note = "PRD-0003 tier 1 #9; netdriver.mar, getdvi.mar"

[[feature]]
id = "jpi-quotas"
title = "$GETJPI's CPU time, I/O counts, quotas and what is left of them"
status = "todo"
milestone = "m1"
note = "PRD-0003 tier 1 #4, step 5; WASMRUN's clocks want JPI$_CPUTIM (PRD-0007 M4)"

[[feature]]
id = "async"
title = "The non-W forms complete later, with an AST and the IOSB"
status = "todo"
note = "PRD-0003 tier 3 #33: fine on one node, done at once now"

[[feature]]
id = "process-scan"
title = "$PROCESS_SCAN: wildcard $GETJPI by username, mode, node and other criteria"
status = "todo"

[[feature]]
id = "getrmi"
title = "$GETRMI: the system-wide performance counters MONITOR reads"
status = "todo"

[[feature]]
id = "getuai"
title = "$GETUAI and $SETUAI: a user's UAF record through a service"
status = "todo"
note = "LOGINOUT and AUTHORIZE read SYSUAF.DAT through RMS now"

[[feature]]
id = "all-items"
title = "The rest of each service's item codes"
status = "todo"
note = "JPITAB, SYITAB and getdvi.mar's 28 items are a small part of VMS's"

[[feature]]
id = "cluster-items"
title = "$GETSYI of another cluster node by csidadr or nodename"
status = "todo"
note = "getjpi.mar ignores them; with t8y:/exec/cluster"

[[needs]]
on = "/exec/processes"
why = "$GETJPI reads the PCB"

[[needs]]
on = "/exec/qio"
why = "$GETDVI reads UCBs"

[[needs]]
on = "/exec/privileges"
why = "GROUP and WORLD for another process's information"

[[needs]]
on = "/exec/event-flags"
why = "Each sets its event flag when done"
+++

VMS answers questions through item-list services: a program passes a
list of item codes, each with a buffer, and the service fills them in.
`$GETJPI` asks about a process (its name, state, priority, UIC,
privileges, quotas, CPU time, I/O counts, image), and with a wildcard
PID walks every process it may see; `$PROCESS_SCAN` narrows the walk.
`$GETSYI` asks about the system, and in a cluster about another node:
version, node name, memory, the system parameters. `$GETDVI` asks
about a device, and `$DEVICE_SCAN` finds devices by name or class. Each
has a `W` form that waits and a plain form that completes with an event
flag, an IOSB and an AST. DCL's `F$GETJPI`, `F$GETSYI` and `F$GETDVI`
call them, and so do `SHOW SYSTEM`, `SHOW PROCESS` and `SHOW DEVICES`.
Other services follow the same pattern for their own objects:
`$GETQUI` for queues, `$GETLKI` for locks, `$GETUAI` for UAF records,
`$GETRMI` for performance counters.

vaxpunk's are `vms/exec/getjpi.mar` (`$GETJPI` and `$GETSYI`, each a
table of item codes and where their values are) and `getdvi.mar`
(`$GETDVI` and `$DEVICE_SCAN`). They take VMS's item lists and codes
from `starlet.mlb`, but each answers only the items the system keeps,
and completes at once. PRD-0003 adds CPU time, I/O counts and quotas
to `$GETJPI` (step 5) and every device from its UCB to `$GETDVI`
(tier 1 #9). `$GETQUI` and `$GETLKI` come with the job controller and
the lock manager.
