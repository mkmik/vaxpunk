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
status = "done"
needs = ["#asy", "/exec/event-flags", "/drivers/interrupt-io"]
note = "ADR-0048: rms.mar RECSVC runs it on a stack of its own, RMS$STALL at FIL$READLBLK's wait returns RMS$_PENDING holding FIL$LOCK, RMS$STALLAST goes on; $WAIT ends it in place (RMS$FINISH); ASYTEST"

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
VMS's does (`RMS$CMPL` in `rms.mar`). A record or block I/O service with
`RAB$V_ASY` runs on a stack of its own; when it has to wait for the disk
it stalls, as VMS's `RM$STALL` does, and returns `RMS$_PENDING`, its
process still holding `FIL$LOCK`; the disk's completion queues a kernel
mode AST that goes on with it, as `RM$STALLAST`
([ADR-0048](../../adr/0048-asynchronous-rms-stalls-on-a-stack-of-its-own.md)).
`$WAIT`, or any file system work of the same process, ends it in place
first. A service that needn't wait ends before it returns, as on VMS.
Left: letting go of `FIL$LOCK` at the stall, which wants the file
system's buffers per volume, and stalls for record locks.
