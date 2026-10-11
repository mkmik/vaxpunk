+++
title = "Below the executive"
band = "platform"
summary = "Boot, seL4 and the PAL: everything that knows about the machine and seL4"
+++

On an Alpha, OpenVMS sat on PALcode and the SRM console firmware. PALcode
made the CPU look like the parts of a VAX that VMS relied on: four access
modes, `CHMx` and `REI`, IPL, AST registers, software interrupts and
context switching. The console loaded the primary bootstrap, `APB`, which
read the executive from the system disk. A VAX had VMB and its
microcode instead. Everything above that is the executive.

vaxpunk's version of that layer is in `pal/` and `boot/`. EDK2 and Limine
load a shim, which starts seL4 (MCS); seL4 starts the root task, which is
the PAL ([ADR-0001](../../adr/0001-pal-interface-vms-vocabulary.md),
[ADR-0002](../../adr/0002-root-task-is-the-pal.md)). The PAL reads
`EXEC.EXE` from the system disk, runs each process and each of its access
modes as a seL4 thread
([ADR-0003](../../adr/0003-one-cpu-many-threads.md),
[ADR-0005](../../adr/0005-access-modes-are-threads.md)), answers the
executive's privileged instructions as PAL calls
([DESIGN-0001](../../design/0001-pal-interface.md)), and drives the
virtio devices and the console UART. The TCP/IP component, lwIP, also
lives here, below the port. Only code in this area calls seL4. seL4 is
scaffolding: the long-term aim is a native kernel in MACRO-32 and BLISS,
with the executive unchanged above the same PAL interface.
