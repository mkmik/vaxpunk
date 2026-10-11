+++
title = "File system locking"
summary = "How the XQP serializes: one mutex, FIL$LOCK; locks per volume and file next"
aka = ["FIL$LOCK", "FIL$UNLOCK", "XQP locks", "access arbitration", "volume allocation lock", "file serialization lock"]
kind = "core"
intent = "goal"
research = "outline"
milestone = "m1"
refs = ["docs/adr/0020-file-system-lock-below-ipl-synch.md", "docs/adr/0030-lock-manager-in-the-executive.md", "docs/prd/0003-multi-user-vms.md", "docs/design/0002-executive-processes.md", "vms/exec/f11.mar", "vms/sysexe/fstest.mar"]

[[feature]]
id = "mutex"
title = "One lock for the file system, FIL$LOCK, held at IPL$_ASTDEL"
status = "done"
milestone = "m0"
refs = ["docs/adr/0020-file-system-lock-below-ipl-synch.md"]

[[feature]]
id = "per-volume-state"
title = "The file system's buffers, IRP and current VCB kept per volume"
status = "todo"
milestone = "m1"
note = "ADR-0030 follow-up: per-volume locks need them; F11$AB_IRP and F11$GL_VCB are global"

[[feature]]
id = "per-volume"
title = "A lock per volume and per file on the lock manager, in place of FIL$LOCK"
status = "todo"
milestone = "m1"
needs = ["//exec/lock-manager:enq-deq", ":per-volume-state"]
note = "PRD-0003 step 7 and tier 1 #5, in an ADR that supersedes ADR-0020"

[[feature]]
id = "arbitration"
title = "Access arbitration: a writer excludes other opens; an open file isn't deleted"
status = "todo"
milestone = "m1"
needs = [":per-volume"]
note = "PRD-0003 step 7 and tier 1 #5; rms.mar: $ERASE deletes an open file"

[[feature]]
id = "holder-checks"
title = "Checks that the holder neither takes the lock again nor waits"
status = "todo"
note = "ADR-0020 ponytail"

[[needs]]
on = "//exec/ipl-sync"
why = "The holder runs at IPL$_ASTDEL; the driver's call at IPL$_SYNCH"

[[needs]]
on = "//exec/scheduler"
why = "A process that finds the lock held waits in SCH$C_MWAIT"
+++

VMS's XQP runs in the process that asks, so several processes can be in
the file system at once, and it serializes them with the lock manager: a
lock on each volume while it allocates blocks and headers, a lock on each
file header it changes, an access arbitration lock per file that decides
which opens of it are compatible, and locks that keep its caches the same
in every process and, in a cluster, on every node. While it holds them it
runs at `IPL$_ASTDEL`, so no kernel mode AST, process deletion among
them, comes in the middle. Work on one volume never waits for another.

vaxpunk's file system has one lock for everything, `FIL$LOCK`, a mutex
with a wait queue that RMS, the volume services, `$GETDVI` and the image
activator take, held at `IPL$_ASTDEL`, so other processes run while one
is in the file system
([ADR-0020](../../docs/adr/0020-file-system-lock-below-ipl-synch.md));
`FSTEST` checks two processes waiting for each other's lock at boot. One
lock means one volume's work waits for another's, and a low-priority
holder can delay a high-priority waiter. PRD-0003's step 7 replaces it
with locks per volume and per file on the lock manager
([ADR-0030](../../docs/adr/0030-lock-manager-in-the-executive.md), not
built yet), which first needs the file system's buffers split per volume.
The same step adds access arbitration, which RMS's file sharing
(`//rms/sharing`) stands on.
