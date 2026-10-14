+++
title = "Lock manager"
summary = "$ENQ, $DEQ and $GETLKI: named resources, six lock modes, value blocks"
aka = ["$ENQ", "$ENQW", "$DEQ", "$GETLKI", "LKB", "RSB", "LCK$", "LKI$", "ENQLM", "SYSLCK", "DEADLOCK_WAIT", "LOCKING", "DLM", "distributed lock manager", "locking.b64"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m1"
refs = ["docs/adr/0030-lock-manager-in-the-executive.md", "docs/adr/0035-deadlock-search-after-deadlock-wait.md", "docs/prd/0003-multi-user-vms.md", "docs/adr/0020-file-system-lock-below-ipl-synch.md", "vms/exec/sysenqdeq.mar", "vms/exec/locking.b64", "vms/sysexe/locktest.mar"]

[[feature]]
id = "enq-deq"
title = "$ENQ, $ENQW and $DEQ: six modes, conversions, the three queues, CONVERT and DEQALL"
status = "done"
milestone = "m1"
note = "PRD-0003 step 6, tier 1 #5; sysenqdeq.mar and locking.b64, LOCKTEST at boot"

[[feature]]
id = "sublocks"
title = "Parent locks: a tree of resources; SS$_SUBLOCKS, SS$_PARNOTGRANT, $DEQ's DEQALL of a parent"
status = "done"
milestone = "m1"
needs = ["#enq-deq"]
note = "locking.b64; no resource tree depth limit, SS$_EXDEPTH"

[[feature]]
id = "value-blocks"
title = "16-byte lock value blocks, LCK$M_VALBLK, $DEQ's valblk and LCK$M_INVVALBLK"
status = "done"
milestone = "m1"
needs = ["#enq-deq"]
note = "Written and read as VMS's SYSENQDEQ does; rundown invalidates a PW or EX holder's"

[[feature]]
id = "asts"
title = "Completion ASTs and event flags; NOQUEUE, SYNCSTS"
status = "done"
milestone = "m1"
needs = ["#enq-deq"]
note = "ADR-0030 decision 4: LCK$POST, a kernel mode AST in the lock's process"

[[feature]]
id = "blocking-asts"
title = "Blocking ASTs to the holder of a lock that blocks another request"
status = "done"
milestone = "m1"
needs = ["#enq-deq"]
note = "ADR-0030 decision 4: once each time a conversion arms it; an ACB of its own, made by $ENQ"

[[feature]]
id = "qualified-names"
title = "Names qualified by access mode and UIC group; LCK$M_SYSTEM takes SYSLCK"
status = "done"
milestone = "m1"
needs = ["#enq-deq", "/exec/privileges"]
note = "PRD-0003 *Lock manager*"

[[feature]]
id = "deadlock"
title = "Deadlock detection after DEADLOCK_WAIT seconds: SS$_DEADLOCK to the request searched from"
status = "done"
milestone = "m1"
needs = ["#enq-deq", "/exec/timers#timer-queue"]
note = "ADR-0035, which replaces ADR-0030 decision 5; LCK$SEARCH once a second; DEADLOCK_WAIT is a cell until SYSGEN"

[[feature]]
id = "getlki"
title = "$GETLKI with the items LOCKTEST and a lock display need"
status = "done"
milestone = "m1"
needs = ["#enq-deq"]
note = "ADR-0030 decision 3; wildcard by lock ID, WORLD and SYSLCK checks; done at once, no AST, as $GETJPI"

[[feature]]
id = "rundown"
title = "Image rundown dequeues user mode locks; process rundown all of them"
status = "done"
milestone = "m1"
needs = ["#enq-deq"]
note = "ADR-0030 decision 6: in EXE$IMGRUNDOWN and EXE$RUNDOWN"

[[feature]]
id = "enqlm"
title = "ENQLM charged: SS$_EXENQLM past it"
status = "todo"
milestone = "m1"
needs = ["#enq-deq"]
note = "PRD-0003 step 5; ADR-0030 ponytail; with t8y:/exec/quotas#enqlm, which needs this component, so no need back"

[[feature]]
id = "more-flags"
title = "The other flags: EXPEDITE, QUECVT, NODLCKWT, CVTSYS, $DEQ's CANCEL, XVALBLK's 64-byte value blocks"
status = "todo"
note = "ADR-0030: SS$_BADPARAM until a program needs one"

[[feature]]
id = "distributed"
title = "The remote side: resource masters, the directory, remastering across a cluster"
status = "todo"
note = "A non-goal of PRD-0003; ADR-0030 leaves room under $ENQ"

[[needs]]
on = "/exec/pool"
why = "Resource and lock blocks come from nonpaged pool"

[[needs]]
on = "/exec/asts"
why = "Completion and blocking ASTs"

[[needs]]
on = "/exec/event-flags"
why = "$ENQ sets an event flag; $ENQW waits for it"

[[needs]]
on = "/exec/scheduler"
why = "A request that can't be granted waits"

[[needs]]
on = "/exec/ipl-sync"
why = "The lock database changes at IPL$_SYNCH"

[[needs]]
on = "/exec/system-services"
why = "$ENQ, $DEQ and $GETLKI go in the vector"

[[needs]]
on = "/host/vbliss"
why = "The logic is a BLISS-64 module, locking.b64, the first in kernel mode"

[[needs]]
on = "/exec/cluster"
why = "Locks held across nodes"
soft = true
+++

The VMS lock manager coordinates access to named resources. A process
asks for a lock on a resource name with `$ENQ`, in one of six modes from
null to exclusive, and gets it at once or waits in the resource's queue;
it converts it to another mode or releases it with `$DEQ`. A lock can
carry a value block, which holders of the resource read and write, and a
blocking AST that tells the holder someone is waiting. Resources form
trees, and names are qualified by access mode and UIC group. The XQP
locks volumes and files with it, RMS buckets and records, and programs
their own resources. In a cluster the same services reach locks
mastered on other nodes over SCS, which makes it the distributed lock
manager; deadlocks are searched for after `DEADLOCK_WAIT` seconds.
SDA's `SHOW LOCKS` and `MONITOR LOCK` show them.

vaxpunk's is executive code for one node,
[ADR-0030](../../adr/0030-lock-manager-in-the-executive.md), PRD-0003's
step 6. `$ENQ`, `$ENQW` and `$DEQ` (`vms/exec/sysenqdeq.mar`) check their
arguments and call the lock database's logic at `IPL$_SYNCH`, in BLISS-64,
`vms/exec/locking.b64`, named after VMS's `LOCKING` execlet: VMS's six
modes, three queues and order of granting, its status block, completion
ASTs and event flags, names qualified by access mode and UIC group or the
system's, and the flags `CONVERT`, `NOQUEUE`, `SYNCSTS`, `SYSTEM` and
`DEQALL`. Image rundown dequeues the user mode locks, process rundown the
rest. Value blocks, blocking ASTs, sublocks and `$GETLKI` are VMS's, and
a request that has waited `DEADLOCK_WAIT` seconds is searched for a
deadlock once, from the timer queue
([ADR-0035](../../adr/0035-deadlock-search-after-deadlock-wait.md)).
`LOCKTEST` checks it at boot, in two processes. Not yet: `ENQLM`, with
PRD-0003's step 5. That is what PRD-0003's step 7 needs to put the file system
(`t8y:/files/fs-locking`, replacing `FIL$LOCK`) and RMS sharing
(`t8y:/rms/sharing`) on it. The remote side waits for a cluster.
