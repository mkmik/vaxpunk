+++
title = "Event flags"
summary = "128 flags a process waits on: local ones and named common clusters"
aka = ["EFN", "$SETEF", "$CLREF", "$READEF", "$WAITFR", "$WFLOR", "$WFLAND", "$ASCEFC", "$DACEFC", "$DLCEFC", "CEB", "common event flag cluster", "EFN$C_ENF", "event.mar"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["docs/design/0002-executive-processes.md", "vms/exec/event.mar", "docs/prd/0003-multi-user-vms.md"]

[[feature]]
id = "local"
title = "Local flags 0-63: set, clear, read; wait for one, any or all"
status = "done"
milestone = "m0"

[[feature]]
id = "common"
title = "Common clusters 64-127 by name, shared by every process associated"
status = "done"
milestone = "m0"

[[feature]]
id = "dlcefc"
title = "$DLCEFC, and permanent clusters (PRMCEB)"
status = "todo"
note = "$DLCEFC is a stub; $ASCEFC ignores perm (event.mar)"

[[feature]]
id = "group-protection"
title = "Cluster names qualified by UIC group, and $ASCEFC's protection"
status = "todo"
note = "event.mar matches names across groups and ignores prot"

[[feature]]
id = "enf"
title = "EFN$C_ENF, flag 128: no event flag at all"
status = "todo"
note = "event.mar takes 0-127; the C RTL waits on flag 0 for want of it (t8y:/rtl/crtl#efn-enf)"

[[needs]]
on = "/exec/scheduler"
why = "Waits are the LEF and CEF states"

[[needs]]
on = "/exec/pool"
why = "Common event blocks come from pool"

[[needs]]
on = "/exec/ipl-sync"
why = "Flags and CEBs change at IPL$_SYNCH"
+++

Each VMS process has 128 event flags in four clusters of 32. Clusters 0
and 1 are its own; `$QIO`, `$SETIMR`, `$ENQ` and the other services that
complete later set one when they do, and `$WAITFR`, `$WFLOR` and
`$WFLAND` wait for one, any or all of a mask. Clusters 2 and 3 can be
associated with a named common event flag cluster (`$ASCEFC`), shared by
every process in the UIC group associated with it, which is how
processes synchronize without a mailbox or a lock. A cluster is
temporary, gone with its last process, or permanent until `$DLCEFC`.
Flag 128, `EFN$C_ENF`, means no flag at all, for callers that wait on the
I/O status block instead.

vaxpunk's are in `vms/exec/event.mar`: local flags, common clusters made
on first use and gone with the last process, and the six services. `PING`
and `PONG` on the system disk take turns through a common cluster.
Missing: `$DLCEFC` and permanent clusters, names qualified by UIC group,
the protection mask, and flag 128. A cluster is kept while processes wait
on it (PRD-0003 tier 2 #19, fine as is).
