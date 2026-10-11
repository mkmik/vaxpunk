+++
title = "Several CPUs (SMP)"
summary = "More than one CPU running the executive, synchronized with spinlocks"
aka = ["SMP", "multiprocessing", "spinlocks", "IPIR", "WHAMI", "MULTIPROCESSING", "primary CPU", "secondary CPU", "START CPU"]
kind = "platform"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["docs/adr/0003-one-cpu-many-threads.md", "docs/prd/0003-multi-user-vms.md", "pal/kernel/config.cmake"]

[[needs]]
on = "/platform/sel4"
why = "A multicore seL4 build; KernelMaxNumNodes is 1"

[[needs]]
fuzzy = "Spinlocks in the executive in place of IPL alone, a current process per CPU"
why = "ADR-0003: one CPU makes IPL the only synchronization"

[[needs]]
fuzzy = "MTPR_IPIR and MFPR_WHAMI in the PAL, and interprocessor interrupts"
why = "DESIGN-0001 keeps them for more than one CPU"
+++

VMS has run on symmetric multiprocessors since VAX/VMS V5. Each CPU runs
the executive; code that raises IPL to protect data also takes a
spinlock (`SCHED`, `IOLOCK8`, `MMG` and others), which on a uniprocessor
reduces to raising IPL. CPUs signal each other with interprocessor
interrupts, `START CPU` and `STOP CPU` bring them in and out, and the
`MULTIPROCESSING` system parameter chooses the synchronization images.

vaxpunk has one CPU by design
([ADR-0003](../../adr/0003-one-cpu-many-threads.md)): seL4 is built
for one node, and only the current process's thread is runnable, so IPL
protects the executive's data as on a uniprocessor VAX. SMP is a
non-goal of PRD-0003, so an aspiration, not dropped. It would need a
multicore seL4 build, spinlocks in the executive, a current process per
CPU, and the PAL's `MTPR_IPIR` and `MFPR_WHAMI`.
