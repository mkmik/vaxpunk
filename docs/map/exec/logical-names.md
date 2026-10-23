+++
title = "Logical names"
summary = "Names that stand for strings: tables, directories, search lists, $CRELNM and $TRNLNM"
aka = ["$CRELNM", "$DELLNM", "$TRNLNM", "$CRELNT", "LNM$PROCESS_DIRECTORY", "LNM$SYSTEM_DIRECTORY", "CREATE/NAME_TABLE", "LNM$PROCESS", "LNM$JOB", "LNM$GROUP", "LNM$SYSTEM", "LNM$FILE_DEV", "LNM$CLUSTER", "search list", "concealed device", "rooted directory", "SYS$SYSROOT", "lnm.mar"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["docs/design/0002-executive-processes.md", "vms/exec/lnm.mar", "docs/prd/0003-multi-user-vms.md"]

[[feature]]
id = "tables"
title = "The process's table and the system's; $CRELNM, $DELLNM, $TRNLNM; LNM$FILE_DEV"
status = "done"
milestone = "m0"

[[feature]]
id = "process-permanent"
title = "SYS$INPUT, SYS$OUTPUT and SYS$ERROR from $CREPRC"
status = "done"
milestone = "m0"

[[feature]]
id = "search-lists"
title = "Search lists, up to 128 strings; SYS$SYSROOT a list of the ramdisk and the system disk"
status = "done"
note = "PRD-0003 tier 3 #25, checked against OpenVMS V8.4-2L1"

[[feature]]
id = "sysnam"
title = "SYSNAM to change the system table"
status = "done"
milestone = "m1"
note = "PRD-0003 step 1"

[[feature]]
id = "job-group"
title = "LNM$JOB and LNM$GROUP: each job's table and each UIC group's; GRPNAM"
status = "todo"
milestone = "m1"
needs = ["/exec/privileges"]
note = "PRD-0003 tier 1 #1 and *Privileges, UICs and protection*"

[[feature]]
id = "access-modes"
title = "Access modes: user mode names image rundown deletes, names an outer mode can't delete"
status = "todo"
note = "PRD-0003 tier 3 #25: first, since LOGINOUT's user mode names need it; DCL's data lines want it (tier 1 #6)"

[[feature]]
id = "attributes"
title = "CONCEALED, TERMINAL, CONFINE, NO_ALIAS; rooted directories, SYS$SYSROOT as [SYS0.]"
status = "todo"
note = "PRD-0003 tier 3 #25"

[[feature]]
id = "directories"
title = "Logical name directories, tables of one's own ($CRELNT, CREATE/NAME_TABLE), table search lists; $DELLNM without a name"
status = "done"
note = "PRD-0003 tier 3 #25, checked against OpenVMS V8.4-2L1: names in directories, protection of shareable tables, SS$_ status values and DCL's messages"

[[feature]]
id = "cluster-table"
title = "LNM$CLUSTER, the same names on every node"
status = "todo"

[[needs]]
on = "/exec/pool"
why = "Each name is a block of pool"

[[needs]]
on = "/exec/ipl-sync"
why = "The tables change at IPL$_SYNCH"

[[needs]]
on = "/exec/privileges"
why = "SYSNAM for the system table"

[[needs]]
on = "/exec/cluster"
why = "The cluster table"
soft = true
+++

A VMS logical name stands for one or more strings, its equivalence: a
program opens `SYS$OUTPUT`, and the process says what that is. Names live
in tables: each process's (`LNM$PROCESS`), each job's (`LNM$JOB`), each
UIC group's (`LNM$GROUP`), the system's (`LNM$SYSTEM`), and from V7.2 the
cluster's, found through logical name directories and searched in the
order `LNM$FILE_DEV` lists. A name has an access mode, so user mode
names go at image rundown and outer modes can't delete inner ones, and
attributes such as `CONCEALED` and `TERMINAL`. A name with several
strings is a search list, which RMS tries in turn; a concealed rooted
device name such as `SYS$SYSROOT`, `DKA0:[SYS0.]`, gives each node its
own system directories. `$ASSIGN`, RMS, `DEFINE`, `ASSIGN` and the C
library's `getenv` all translate them.

vaxpunk's are in `vms/exec/lnm.mar`: queues of blocks from pool,
`$CRELNM`, `$DELLNM` and `$TRNLNM` with VMS's item lists, and search
lists, which behave as on OpenVMS V8.4-2L1. The tables are names in two
directories, the process's and the system's, which `$CRELNT` and DCL's
`CREATE/NAME_TABLE` add to; a table's name is translated there into the
tables it stands for, so a process may have an `LNM$FILE_DEV` of its
own. Shareable tables have VMS's owner and protection; the system table
takes `SYSNAM` too (PRD-0003 step 1). `SYS$SYSROOT` is a search list of
`MDA0:` and `DKA0:`, whole volumes, not rooted directories. Missing:
`LNM$JOB` and `LNM$GROUP` (tier 1 #1), and access modes, attributes and
rooted names (tier 3 #25).
