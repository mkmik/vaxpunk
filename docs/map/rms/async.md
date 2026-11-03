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
status = "done"
needs = ["/exec/asts"]
note = "rms.mar RMS$CMPL: err or suc as an AST in the caller's mode, the FAB or RAB its parameter, RMS$_CDA if it can't be queued; RMSTEST's AST.SEQ"

[[feature]]
id = "asy"
title = "RAB$V_ASY taken: the service ends before it returns, and $WAIT gives its status"
status = "done"
needs = ["#completion-routines"]
note = "rms.mar EXE$WAIT; what VMS's RMS does when the operation needn't wait"

[[feature]]
id = "asynchronous"
title = "RAB$V_ASY: a service returns RMS$_PENDING, $WAIT or an event flag ends it"
status = "todo"
needs = ["#asy", "/exec/event-flags", "/drivers/interrupt-io"]
note = "RMS must stall at a disk wait and go on from its completion, holding FIL$LOCK for the process meanwhile (PRD-0003 tier 3 #23)"

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

vaxpunk's RMS takes the completion routines and calls them as ASTs, as
VMS's does (`RMS$CMPL` in `rms.mar`), but each service still finishes
before it returns, `RAB$V_ASY` or not, which VMS's RMS also does when the
record is in its buffers: `$WAIT` gives the status the service left in
the RAB. The disks end requests by interrupt now
([ADR-0047](../../adr/0047-interrupt-driven-disk-io-and-locked-pages.md)),
so what is left is the overlap: RMS stalling at a disk wait, returning
`RMS$_PENDING`, and going on from the disk's completion, as VMS's
`RM$STALL` does (PRD-0003 tier 3 #23).
