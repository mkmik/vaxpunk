+++
title = "Access modes as threads"
summary = "Kernel, executive, supervisor, user: each mode a thread with its own address space"
aka = ["access modes", "CHMK", "CHME", "CHMS", "CHMU", "REI", "PSL", "PROBER", "PROBEW", "four modes", "P0", "P1", "S0"]
kind = "platform"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["docs/adr/0005-access-modes-are-threads.md", "docs/adr/0003-one-cpu-many-threads.md", "docs/design/0001-pal-interface.md", "pal/src/main.c"]

[[feature]]
id = "kernel-threads"
title = "Kernel mode: a thread per process in the executive's address space"
status = "done"
milestone = "m0"
note = "ADR-0003"

[[feature]]
id = "vax-layout"
title = "VAX-shaped address space below 2 GB: P0, S0, P1; every image links at 0x10000"
status = "done"
milestone = "m0"
note = "ADR-0005 point 1; P0 and P1 PTEs kept per process context"

[[feature]]
id = "outer-modes"
title = "Executive, supervisor and user mode threads, each mapping the pages its mode may read"
status = "done"
milestone = "m0"
note = "ADR-0005 point 4; DCL runs in supervisor mode, images in user mode"

[[feature]]
id = "mode-changes"
title = "CHMx inward, REI outward, stack pointers saved in the HWPCB"
status = "done"
milestone = "m0"
note = "ADR-0005 point 5"

[[feature]]
id = "probes"
title = "PROBER and PROBEW check a mode's access against the PTEs"
status = "done"
milestone = "m0"
note = "ADR-0005 point 7"

[[feature]]
id = "s0-protection-changes"
title = "An S0 page's outer-mode access changed after outer modes run"
status = "todo"
note = "main.c ponytail: their protection is set before any outer mode runs, and stays"

[[feature]]
id = "per-process-kernel-space"
title = "A kernel-mode address space per process, with S0 in each, so switches don't remap"
status = "todo"
needs = ["//platform/pal:frame-caps"]
note = "PRD-0003 tier 4 #40; ADR-0005 alternatives"

[[needs]]
on = "//platform/pal"
why = "The PAL makes the threads and moves the CPU between them"

[[needs]]
on = "//platform/sel4"
why = "Threads, address spaces and the faults that move the CPU between them"
+++

A VAX or Alpha has four access modes: kernel, executive, supervisor and
user. They share one address space, and each page's protection code says
which modes may read and write it; the CPU checks it on every access.
`CHMK`, `CHME`, `CHMS` and `CHMU` move inward through the SCB, `REI`
moves outward, and each mode has its own stack pointer. The executive
runs in kernel mode, RMS in executive mode, DCL in supervisor mode, and
programs in user mode.

ARM64 gives an OS only EL0 and EL1, and seL4 runs every thread at EL0,
where its address space is its only protection. So vaxpunk gives each
outer mode of a process a seL4 thread with an address space of its own,
holding exactly the pages that mode may read
([ADR-0005](../../docs/adr/0005-access-modes-are-threads.md)). Kernel
mode is the process's thread in the executive's address space
([ADR-0003](../../docs/adr/0003-one-cpu-many-threads.md)); on `SWPCTX` the
PAL unmaps the old process's P0 and P1 pages there and maps the new
one's, since seL4 clears a page table when it is unmapped. The PAL copies
registers from one mode's thread to the other on `CHMx`, `REI`,
interrupts and exceptions. A switch costs a seL4 call per page, and S0's
outer-mode protection is fixed once outer modes run.
