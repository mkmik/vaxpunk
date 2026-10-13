+++
title = "IPL and synchronization"
summary = "Interrupt priority levels, software interrupts, forks, mutexes, spinlocks"
aka = ["IPL", "IPL$_SYNCH", "IPL$_ASTDEL", "IPL$_RESCHED", "IPL$_IOPOST", "DSBINT", "ENBINT", "SETIPL", "SOFTINT", "fork processing", "fork block", "mutex", "MWAIT", "spinlock", "SPL"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["docs/adr/0003-one-cpu-many-threads.md", "docs/adr/0004-interval-timer-is-a-pal-thread.md", "docs/adr/0020-file-system-lock-below-ipl-synch.md", "docs/design/0002-executive-processes.md", "docs/design/0001-pal-interface.md", "crosstools/vtools/lib/lib.mlb"]

[[feature]]
id = "ipl"
title = "IPL on one CPU: DSBINT, ENBINT, SETIPL; IPL$_SYNCH guards the executive's data"
status = "done"
milestone = "m0"
refs = ["docs/adr/0003-one-cpu-many-threads.md"]

[[feature]]
id = "software-interrupts"
title = "Software interrupts 1-15: AST delivery, reschedule, the software timer"
status = "done"
milestone = "m0"

[[feature]]
id = "async-interrupts"
title = "Interrupts that stop a running process: the clock, the console, the network port"
status = "done"
milestone = "m0"
refs = ["docs/adr/0004-interval-timer-is-a-pal-thread.md"]

[[feature]]
id = "fork-levels"
title = "Fork levels for drivers: TTY$IOPOST at IPL 4, NET$FORK at IPL 6"
status = "done"
milestone = "m0"
note = "One routine per level, not VMS's queues of fork blocks"

[[feature]]
id = "mutex"
title = "A mutex with a wait state, MWAIT: the file system's FIL$LOCK"
status = "done"
milestone = "m0"
refs = ["docs/adr/0020-file-system-lock-below-ipl-synch.md"]

[[feature]]
id = "fork-queues"
title = "Fork queues: fork blocks queued per IPL, a driver's work deferred from its interrupt"
status = "todo"
note = "Wanted once disk I/O is interrupt driven (PRD-0003 tier 4 #37)"

[[feature]]
id = "spinlocks"
title = "Spinlocks that raise IPL, for several CPUs"
status = "todo"
note = "ADR-0003: a second CPU needs them; t8y:/platform/smp"

[[needs]]
on = "/platform/pal"
why = "Delivers interrupts and software interrupts by IPL; MTPR #PR$_IPL"
+++

VMS synchronizes inside the executive with interrupt priority levels:
code raises IPL to keep out interrupts at or below it. Device interrupts
are IPL 20-23, the clock 22 or 24, and software interrupts 1-15: AST
delivery at 2, rescheduling at 3, I/O post at 4, AST queueing forks at
6, the software timer at 7, driver forks at 8 to 11, and `IPL$_SYNCH`,
8, which guards the scheduler's database, pool and the I/O database. A driver's interrupt routine does little and forks the
rest to a lower IPL. Mutexes, with the `MWAIT` state, guard structures
used at IPL 0, such as the logical name tables. On a multiprocessor
(VAX V5 and later) each guarded structure also has a spinlock, which
raises IPL and spins.

vaxpunk runs the executive on one CPU, one thread at a time, and IPL is
the whole of its synchronization, as on a uniprocessor VAX
([ADR-0003](../../adr/0003-one-cpu-many-threads.md)). The PAL keeps
a pending bit per IPL and delivers interrupts through the SCB when IPL
allows, the clock's even to a running process
([ADR-0004](../../adr/0004-interval-timer-is-a-pal-thread.md)).
The levels are VMS's (`lib.mlb`), with `IPL$_NETPOST`, 6, the TCP/IP
port's fork. The file system runs below `IPL$_SYNCH` holding a mutex,
`FIL$LOCK` ([ADR-0020](../../adr/0020-file-system-lock-below-ipl-synch.md)).
Spinlocks wait for a second CPU.
