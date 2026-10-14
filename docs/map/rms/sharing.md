+++
title = "File sharing"
summary = "Several processes and streams on one file: FAB$B_SHR, bucket and record locks"
aka = ["FAB$B_SHR", "SHRGET", "SHRPUT", "SHRUPD", "SHRDEL", "UPI", "MSE", "RMS$_FLK", "RMS$_RLK", "$FREE", "$RELEASE", "record locking", "query locking"]
kind = "core"
intent = "goal"
research = "outline"
milestone = "m1"
refs = ["docs/prd/0003-multi-user-vms.md", "docs/prd/0008-rms-record-and-indexed-files.md", "docs/adr/0030-lock-manager-in-the-executive.md", "docs/adr/0036-file-access-arbitration-locks.md", "vms/exec/rms.mar", "https://wasd.vsm.com.au/htbin/helpgate/HELP/SET/RMS_DEFAULT/QUALIFIERS"]

[[feature]]
id = "file-sharing"
title = "FAB$B_SHR at $OPEN and $CREATE: RMS$_FLK when the opens conflict, one writer at a time"
status = "done"
milestone = "m1"
needs = ["/files/fs-locking#arbitration"]
refs = ["docs/adr/0036-file-access-arbitration-locks.md"]
note = "ponytail: UPI and MSE ignored"

[[feature]]
id = "erase-open"
title = "$ERASE of an open file: its entry goes, and the last to close it deletes it"
status = "done"
milestone = "m1"
needs = ["/files/fs-locking#arbitration"]
refs = ["docs/adr/0036-file-access-arbitration-locks.md"]

[[feature]]
id = "streams"
title = "Several RABs connected to one FAB, FAB$M_MSE"
status = "todo"
milestone = "m1"
note = "PRD-0008 step 9; PRD-0003 tier 3 #23; rms.mar ponytail: one stream per file"

[[feature]]
id = "record-locks"
title = "Bucket and record locks; RMS$_RLK; RAB$V_WAT, NLK, RRL, ULK; $FREE and $RELEASE"
status = "todo"
milestone = "m1"
needs = ["/exec/lock-manager#enq-deq", "#file-sharing"]
note = "PRD-0008 step 9, with two writers of one file, CW, which ADR-0036 makes PW; $FREE and $RELEASE return RMS$_RNL now"

[[feature]]
id = "buffer-coherence"
title = "Each process's buffers kept current when another writes the bucket"
status = "todo"
milestone = "m1"
needs = ["#record-locks"]
note = "Matters once buckets are kept between services (t8y:/rms/indexed's bucket-cache)"

[[feature]]
id = "query-locking"
title = "Query locking and the contention policy, SET RMS_DEFAULT/QUERY_LOCK"
status = "todo"
needs = ["#record-locks"]

[[needs]]
on = "/rms/core"
why = "Sharing is asked for in the FAB and RAB"

[[needs]]
on = "/exec/lock-manager#enq-deq"
why = "Bucket and record locks are $ENQ locks"

[[needs]]
on = "/files/fs-locking"
why = "The XQP's access arbitration decides which opens of a file are compatible"
+++

On VMS several processes can have one file open at once, each saying in
`FAB$B_SHR` what it lets the others do: read (`SHRGET`), add (`SHRPUT`),
update and delete records. RMS asks the XQP whether the opens are
compatible, `RMS$_FLK` if not, then locks each bucket while a service
works in it and each record a stream holds, through the lock manager.
Another stream that wants a held record gets `RMS$_RLK`, or waits with
`RAB$V_WAT`, or reads past the lock with `RAB$V_NLK` or `RAB$V_RRL`; a
program that keeps records locked by choice releases them with `$FREE`
and `$RELEASE`. A process can connect several RABs to one file with
`FAB$M_MSE`. With `UPI` the program does its own locking. In a cluster
the same locks work across nodes.

vaxpunk arbitrates opens as VMS does, through the lock manager
([ADR-0036](../../adr/0036-file-access-arbitration-locks.md)): readers
share a file, a writer excludes the opens that don't allow it, and
`$ERASE` of an open file deletes it at its last close. But one writer at
a time: with one stream per file and no record locks, a writer that
allows others to write still gets the file to itself. `$FREE` and
`$RELEASE` exist and say there is no lock (`RMS$_RNL`). Record locks are
PRD-0008's step 9: its test is two sessions updating different records
of one file, and `%RMS-E-RLK` on the same record
([PRD-0003](../../prd/0003-multi-user-vms.md),
[PRD-0008](../../prd/0008-rms-record-and-indexed-files.md)).
