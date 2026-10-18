+++
title = "Scheduler"
summary = "Process states, 32 priorities, quantum end, waits and wakes"
aka = ["SCH$", "SCH$SCHED", "SCH$WAIT", "COM queue", "QUANTUM", "LEF", "HIB", "MWAIT", "SUSP", "priority boost", "class scheduler", "sched.mar"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["docs/design/0002-executive-processes.md", "docs/adr/0003-one-cpu-many-threads.md", "docs/adr/0004-interval-timer-is-a-pal-thread.md", "vms/exec/sched.mar", "vms/exec/timeschdl.mar", "docs/prd/0003-multi-user-vms.md"]

[[feature]]
id = "states"
title = "VMS's state queues; the highest computable process runs, the CPU idles in WTINT"
status = "done"
milestone = "m0"
refs = ["docs/adr/0003-one-cpu-many-threads.md"]

[[feature]]
id = "quantum"
title = "Quantum end from the 10 ms interval timer: round robin within a priority"
status = "done"
milestone = "m0"
refs = ["docs/adr/0004-interval-timer-is-a-pal-thread.md"]

[[feature]]
id = "realtime"
title = "Real-time priorities, 16 to 31, keep the CPU"
status = "done"
milestone = "m0"

[[feature]]
id = "cpu-time"
title = "CPU time charged to each process, for $GETJPI and accounting"
status = "todo"
milestone = "m1"
note = "PRD-0003 tier 1 #4, step 5"

[[feature]]
id = "boosts"
title = "Priority boosts on I/O completion and wakes, decaying each quantum"
status = "todo"
note = "PRD-0003 tier 3 #31; sched.mar"

[[feature]]
id = "resource-wait"
title = "Resource waits (RWAST, RWMBX, RWNPG) and $SETRWM"
status = "wip"
note = "ADR-0038: $SETRWM, and a $QIO past its I/O quotas waits in MWAIT for an AST, as RWAST; ADR-0019: a full mailbox still fails a write with SS$_MBFULL"

[[feature]]
id = "class-scheduler"
title = "The class scheduler: CPU limits per user, account or UIC (SYSMAN CLASS_SCHEDULE)"
status = "todo"

[[needs]]
on = "/exec/ipl-sync"
why = "Its database changes at IPL$_SYNCH; reschedules come at IPL$_RESCHED"

[[needs]]
on = "/platform/pal"
why = "SWPCTX gives the CPU to a process's thread; the PAL's clock ends quanta"
+++

VMS's scheduler picks the process that runs from 32 priorities. 0 to 15
are time-sharing: a process has a base priority, and the system raises
its current priority for a while after an I/O completes, a wait ends or a
resource frees, then lets it fall back by one each quantum. 16 to 31 are
real-time: no boosts and no quantum. A process is current, computable,
in one of the waits (local or common event flag, hibernation,
suspension, page fault, mutex or resource wait), or outswapped in one of
these. Rescheduling is the IPL 3 software interrupt. On a multiprocessor
each CPU has a current process, and affinity and capabilities bind
processes to CPUs. Later releases added a class scheduler that caps the
CPU share of groups of users.

vaxpunk's, in `vms/exec/sched.mar`, has VMS's states and queues,
`SCH$SCHED`, `SCH$WAIT` and the reschedule interrupt, on one CPU
([ADR-0003](../../adr/0003-one-cpu-many-threads.md)). The PAL's
clock thread gives it a 10 ms tick, and quantum end, 20 ticks, is round
robin within a priority
([ADR-0004](../../adr/0004-interval-timer-is-a-pal-thread.md)).
Missing: CPU time charged to processes (PRD-0003 tier 1 #4), priority
boosts and decay (tier 3 #31), and resource waits. A second CPU waits for
`t8y:/platform/smp` and spinlocks.
