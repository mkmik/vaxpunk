+++
title = "File sharing"
summary = "Several processes and streams on one file: FAB$B_SHR, bucket and record locks"
aka = ["FAB$B_SHR", "SHRGET", "SHRPUT", "SHRUPD", "SHRDEL", "UPI", "MSE", "RMS$_FLK", "RMS$_RLK", "$FREE", "$RELEASE", "record locking", "query locking"]
kind = "core"
intent = "goal"
research = "outline"
milestone = "m1"
refs = ["docs/prd/0003-multi-user-vms.md", "docs/prd/0008-rms-record-and-indexed-files.md", "docs/adr/0030-lock-manager-in-the-executive.md", "docs/adr/0036-file-access-arbitration-locks.md", "docs/adr/0046-shared-indexed-files-and-record-locks.md", "vms/exec/rms.mar", "crosstools/ods/fixtures/rms/make/lock.com", "https://wasd.vsm.com.au/htbin/helpgate/HELP/SET/RMS_DEFAULT/QUALIFIERS"]

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
status = "done"
milestone = "m1"
needs = ["/exec/lock-manager#enq-deq", "#file-sharing"]
refs = ["docs/adr/0046-shared-indexed-files-and-record-locks.md"]
note = "ADR-0046: record locks for indexed files, writers in CW; no bucket locks under FIL$LOCK; ponytail: no RAB$M_TMO"

[[feature]]
id = "wait-timeout"
title = "RAB$M_TMO: a wait for a record lock that ends with RMS$_TMO"
status = "todo"
needs = ["#record-locks"]
note = "ADR-0046 ponytail; RECENQ's wait has no timer"

[[feature]]
id = "buffer-coherence"
title = "Each process's buffers kept current when another writes the bucket"
status = "done"
milestone = "m1"
needs = ["#record-locks"]
note = "RMS$SYNC: a service that wrote gives the other opens its header and descriptors, and they empty their slots"

[[feature]]
id = "query-locking"
title = "Query locking and the contention policy, SET RMS_DEFAULT/QUERY_LOCK"
status = "todo"
needs = ["#record-locks"]
note = "Readers and RAB$M_NLK query (RMS$LOCKREC); no NQL, FAB$M_NQL or SET RMS_DEFAULT/QUERY_LOCK"

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
`$ERASE` of an open file deletes it at its last close. Writers of an
indexed file that let others write share it, in CW, and its records are
locked as VMS's RMS locks them, with `RMS$_RLK`, `RAB$M_WAT`, `NLK`,
`RRL`, `RLK`, `REA` and `ULK`, `$FREE` and `$RELEASE`
([ADR-0046](../../adr/0046-shared-indexed-files-and-record-locks.md)):
the boot test runs OpenVMS's `LOCK.COM` and gets what it said, and a
SPAWNed process's `READ/WAIT` waits for its parent's lock. Services
still run one at a time under `FIL$LOCK`, so there are no bucket locks;
a service that writes gives the file's other opens what changed. A
relative file still has one writer at a time, and a wait no timeout.
