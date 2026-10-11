+++
title = "Kernel threads"
summary = "Several threads of one process scheduled on their own, on several CPUs"
aka = ["KTB", "kernel thread block", "MULTITHREAD", "upcalls", "FRED", "KPS", "kernel processes", "/THREADS_ENABLE"]
kind = "core"
intent = "aspiration"
research = "outline"
refs = ["docs/adr/0003-one-cpu-many-threads.md", "docs/adr/0005-access-modes-are-threads.md", "https://docs.vmssoftware.com/vsi-openvms-programming-concepts-manual-volume-i"]

[[feature]]
id = "threads"
title = "Several kernel threads per process, each a KTB with its own stacks and priority"
status = "todo"

[[feature]]
id = "upcalls"
title = "Upcalls: the scheduler tells the threads library when a thread blocks or can run"
status = "todo"
needs = ["#threads"]

[[feature]]
id = "thread-asts"
title = "ASTs delivered to the thread that started the request"
status = "todo"
needs = ["#threads"]

[[feature]]
id = "kps"
title = "KP services: more stacks for a thread in one access mode (V8.2)"
status = "todo"
needs = ["#threads"]

[[needs]]
on = "/exec/processes"
why = "The PCB's scheduling state moves into a block per thread"

[[needs]]
on = "/exec/scheduler"
why = "The thread, not the process, is what it schedules"

[[needs]]
on = "/exec/asts"
why = "Per-thread AST queues"

[[needs]]
on = "/platform/access-mode-threads"
why = "A process's modes are seL4 threads; more of them, one set per kernel thread"

[[needs]]
on = "/platform/smp"
why = "Threads of one process on several CPUs at once"
soft = true

[[needs]]
fuzzy = "The PAL running several HWPCBs of one process, and the executive's synchronization with more than one of its threads runnable"
why = "ADR-0003 runs one executive thread at a time"
+++

OpenVMS Alpha V7.0 split a process into kernel threads, up to 16 and
from V7.2 up to 256 per process (`MULTITHREAD`). Each has a kernel
thread block (KTB) with its own scheduling state, priority, AST queue and
stacks for each access mode, while the address space, quotas,
privileges, event flags and locks stay the process's. The POSIX threads
library maps many user threads onto them, so threads of one program run
on several CPUs, and with upcalls the scheduler tells the library when a
thread blocks, so the others keep running. Images ask for this with
`LINK/THREADS_ENABLE`.

vaxpunk has one thread of control per process at a time, on one CPU
([ADR-0003](../../adr/0003-one-cpu-many-threads.md)), though each of
its access modes is a seL4 thread
([ADR-0005](../../adr/0005-access-modes-are-threads.md)). Nothing
plans kernel threads; `t8y:/rtl/pthreads` and the Linux binaries escape
hatch would use them.
