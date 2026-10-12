+++
title = "Lock manager"
summary = "$ENQ, $DEQ and $GETLKI: named resources, six lock modes, value blocks"
aka = ["$ENQ", "$ENQW", "$DEQ", "$GETLKI", "LKB", "RSB", "LCK$", "LKI$", "ENQLM", "SYSLCK", "DEADLOCK_WAIT", "LOCKING", "DLM", "distributed lock manager", "locking.b64"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m1"
refs = ["docs/adr/0030-lock-manager-in-the-executive.md", "docs/prd/0003-multi-user-vms.md", "docs/adr/0020-file-system-lock-below-ipl-synch.md"]

[[feature]]
id = "enq-deq"
title = "$ENQ, $ENQW and $DEQ: a resource tree, six modes, conversions, the three queues"
status = "todo"
milestone = "m1"
note = "PRD-0003 step 6, tier 1 #5; ADR-0030 designs it, nothing built"

[[feature]]
id = "value-blocks"
title = "16-byte lock value blocks, LCK$M_VALBLK"
status = "todo"
milestone = "m1"
needs = ["#enq-deq"]
note = "ADR-0030"

[[feature]]
id = "asts"
title = "Completion and blocking ASTs; NOQUEUE, SYNCSTS"
status = "todo"
milestone = "m1"
needs = ["#enq-deq"]
note = "ADR-0030 decision 4"

[[feature]]
id = "qualified-names"
title = "Names qualified by access mode and UIC group; LCK$M_SYSTEM takes SYSLCK"
status = "todo"
milestone = "m1"
needs = ["#enq-deq", "/exec/privileges"]
note = "PRD-0003 *Lock manager*"

[[feature]]
id = "deadlock"
title = "Deadlock detection when a request waits: SS$_DEADLOCK to the one that closed the cycle"
status = "todo"
milestone = "m1"
needs = ["#enq-deq"]
note = "ADR-0030 decision 5; VMS searches after DEADLOCK_WAIT"

[[feature]]
id = "getlki"
title = "$GETLKI with the items LOCKTEST and a lock display need"
status = "todo"
milestone = "m1"
needs = ["#enq-deq"]
note = "ADR-0030 decision 3"

[[feature]]
id = "rundown"
title = "Image rundown dequeues user mode locks; process rundown all of them"
status = "todo"
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
title = "The other flags: EXPEDITE, QUECVT, NODLCKWT, CVTSYS, XVALBLK's 64-byte value blocks"
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

vaxpunk has none yet.
[ADR-0030](../../adr/0030-lock-manager-in-the-executive.md) designs
it as executive code for one node, PRD-0003's step 6: VMS's services,
status block, modes and order of granting, a subset of the flags, the
lock database in pool at `IPL$_SYNCH`, a deadlock search when a request
waits, and the logic in BLISS-64, `vms/exec/locking.b64`, named after
VMS's `LOCKING` execlet. `LOCKTEST` is to check it. Step 7 then puts the
file system (`t8y:/files/fs-locking`, replacing `FIL$LOCK`) and RMS sharing
(`t8y:/rms/sharing`) on it. The remote side waits for a cluster.
