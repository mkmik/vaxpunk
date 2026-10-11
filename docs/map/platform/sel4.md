+++
title = "seL4 (MCS)"
summary = "The microkernel under the PAL, MCS API: bootstrap scaffolding for now"
aka = ["seL4", "MCS", "mixed-criticality scheduling", "kernel.elf", "libsel4", "microkernel"]
kind = "platform"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["pal/README.md", "pal/kernel/config.cmake", "pal/kernel/qemu.env", "docs/adr/0001-pal-interface-vms-vocabulary.md", "docs/adr/0002-root-task-is-the-pal.md", "docs/adr/0004-interval-timer-is-a-pal-thread.md"]

[[feature]]
id = "kernel"
title = "seL4 16.0.0 for qemu-arm-virt, aarch64, one CPU, built from the submodule"
status = "done"
milestone = "m0"
note = "pal/kernel/config.cmake: KernelMaxNumNodes 1, no hypervisor support (Limine hands off at EL1)"

[[feature]]
id = "mcs"
title = "MCS: scheduling contexts with budgets and periods, replies through reply objects"
status = "done"
milestone = "m0"
note = "The clock thread sleeps by seL4_Yield on a budget below its period (ADR-0004)"

[[feature]]
id = "gicv3"
title = "GICv3, so the same kernel boots under TCG and HVF"
status = "done"
note = "pal/kernel/qemu.env; HVF does not emulate GICv2"
+++

seL4 is a formally verified capability microkernel. Its kernel does
little: threads (TCBs), address spaces built from page-table objects,
capability spaces (CNodes), IPC endpoints, notifications and IRQ
handlers, all carved from untyped memory by whoever holds it. With the
MCS (mixed-criticality scheduling) API, a thread runs only on a
scheduling context, which gives it a budget of CPU time per period.
VMS never ran on anything like it; it is vaxpunk's stand-in for the
hardware help an Alpha gave its PALcode.

vaxpunk builds seL4 16.0.0 (`pal/kernel/`, a submodule) with its own
CMake for the QEMU machine in `qemu.env`: Cortex-A57, 1 GB, GICv3, one
CPU, MCS on, a debug kernel that prints its own messages. Only the PAL
calls it
([ADR-0001](../../adr/0001-pal-interface-vms-vocabulary.md),
[ADR-0002](../../adr/0002-root-task-is-the-pal.md)): the executive
has an empty CSpace and reaches the PAL through `svc` faults. seL4 is
scaffolding. Marko's long-term aim is a native kernel in MACRO-32 and
BLISS (`t8y:/platform/native-kernel`), so nothing above the PAL may depend
on it.
