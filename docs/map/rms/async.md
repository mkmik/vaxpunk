+++
title = "Asynchronous RMS"
summary = "RAB$V_ASY, completion routines and $WAIT: record I/O that returns at once"
aka = ["RAB$V_ASY", "$WAIT", "completion routine", "SUC", "ERR", "RMS$_PENDING", "AST"]
kind = "core"
intent = "goal"
research = "outline"
refs = ["docs/prd/0003-multi-user-vms.md", "docs/prd/0008-rms-record-and-indexed-files.md", "docs/adr/0007-system-disk-files-11-and-rms.md", "vms/exec/rms.mar", "vms/exec/syssrv.mar"]

[[feature]]
id = "sync"
title = "Every service synchronous; $WAIT returns at once"
status = "done"
milestone = "m0"
note = "rms.mar: $WAIT is RMS$_NORMAL"

[[feature]]
id = "completion-routines"
title = "Success and error completion routines, called as ASTs"
status = "todo"
needs = ["/exec/asts"]
note = "PRD-0003 tier 3 #23; PRD-0008 non-goal; the services take one argument"

[[feature]]
id = "asynchronous"
title = "RAB$V_ASY: a service returns RMS$_PENDING, $WAIT or an event flag ends it"
status = "todo"
needs = ["#completion-routines", "/exec/event-flags", "/drivers/interrupt-io"]
note = "PRD-0003 tier 3 #23"

[[needs]]
on = "/rms/core"
why = "The FAB and RAB services it makes asynchronous"

[[needs]]
on = "/exec/asts"
why = "Completion routines are delivered as ASTs in the caller's mode"
+++

Every RMS service can take a success and an error routine, which RMS
calls as ASTs when the operation ends. With `RAB$V_ASY` set, a record
service starts the I/O and returns `RMS$_PENDING` at once; the program
goes on, and `$WAIT` on the RAB, the routines or the RAB's event flag
tell it when the record is there. Programs use it to overlap reading one
file with computing on the last record, and servers to keep many streams
going in one process.

vaxpunk's RMS is synchronous: each service finishes before it returns,
takes only its FAB or RAB, and `$WAIT` returns at once. The disks are
synchronous too, so there is nothing to overlap until the disk driver
finishes requests by interrupt (PRD-0003 tier 3 #23 and tier 4 #37).
