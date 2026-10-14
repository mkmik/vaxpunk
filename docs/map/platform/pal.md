+++
title = "PAL (root task)"
summary = "The root task: VMS's PALcode on seL4, answering the executive's PAL calls"
aka = ["PAL", "PALcode", "root task", "roottask.elf", "SWIS", "CALL_PAL", "WRPTE", "SWPCTX", "RPB", "restart parameter block", "READLBLK", "WRITELBLK", "pal/src/main.c"]
kind = "platform"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["pal/README.md", "docs/adr/0037-all-ram-and-address-spaces-filled-on-faults.md", "pal/src/main.c", "docs/design/0001-pal-interface.md", "docs/adr/0001-pal-interface-vms-vocabulary.md", "docs/adr/0002-root-task-is-the-pal.md", "docs/adr/0003-one-cpu-many-threads.md", "docs/adr/0004-interval-timer-is-a-pal-thread.md", "docs/adr/0005-access-modes-are-threads.md", "docs/adr/0031-entropy-from-virtio-rng.md", "docs/boot.md", "docs/prd/0003-multi-user-vms.md"]

[[feature]]
id = "pal-calls"
title = "PAL calls as svc #0 with Alpha's function codes in x7, served as seL4 faults"
status = "done"
milestone = "m0"
note = "ADR-0002; DESIGN-0001 Function codes: HALT, MTPR/MFPR, RD_PS, WTINT and vaxpunk's from 0x40"

[[feature]]
id = "boot-exec"
title = "Reads EXEC.EXE from Files-11 as VMB did, loads it, builds the RPB, starts it at IPL 31"
status = "done"
milestone = "m0"
note = "f11_boot_file, load_image, start_exec in main.c; DESIGN-0001 What the PAL sets up"

[[feature]]
id = "startup-params"
title = "STARTUP_P1 to STARTUP_P8 from fw_cfg into RPB$T_STARTUP"
status = "done"
note = "fwcfg_init; PR #140"

[[feature]]
id = "contexts"
title = "SWPCTX and DELCTX: a process context is a thread, one CPU, the executive schedules"
status = "done"
milestone = "m0"
note = "ADR-0003"

[[feature]]
id = "memory"
title = "WRPTE maps, unmaps and protects a page; PROBER and PROBEW"
status = "done"
milestone = "m0"
note = "DESIGN-0001 Memory; ADR-0005 point 7"

[[feature]]
id = "interrupts"
title = "IPL, software interrupts, AST delivery, the SCB and REI frames ($INTSTKDEF)"
status = "done"
milestone = "m0"
note = "DESIGN-0001 Interrupts and exceptions; ADR-0011 for the AST bytes in the HWPCB"

[[feature]]
id = "interval-timer"
title = "The interval timer: a PAL thread ticks every 10 ms, IPL 24, vector 0xC0"
status = "done"
milestone = "m0"
note = "ADR-0004; PRD-0003 tier 4 #40: the tick is a PAL thread, a timer driver on hardware that has one"

[[feature]]
id = "exceptions"
title = "Outer-mode faults as exceptions through the SCB; kernel-mode faults halt"
status = "done"
milestone = "m0"
note = "DESIGN-0001 Exceptions, Faults and HALT"

[[feature]]
id = "disk-calls"
title = "READLBLK and WRITELBLK on the virtio disks"
status = "done"
milestone = "m0"
needs = ["/platform/virtio#blk"]

[[feature]]
id = "getentropy"
title = "GETENTROPY, call 0x49, from virtio-rng"
status = "done"
needs = ["/platform/virtio#rng"]
note = "ADR-0031"

[[feature]]
id = "tcpip-component"
title = "Starts the TCP/IP component and rings its doorbell: MTPR_DOORBELL, the port interrupt"
status = "done"
milestone = "m0"
note = "start_tcpip in main.c; DESIGN-0003; the component is t8y:/platform/lwip"

[[feature]]
id = "image-load-relocation"
title = "EXEC.EXE relocated with its fixups, not only loaded at its link address"
status = "todo"
note = "main.c load_image ponytail: link address only; relocate with the EIAF when an image has to move"

[[feature]]
id = "frame-caps"
title = "Frame caps in a CNode of their own: more than 1,024 PFNs, and more than 256 extra mappings of them"
status = "done"
milestone = "m1"
note = "ADR-0037 decisions 2 and 3: a share of RAM for the PAL's objects, a CSpace of two levels whose CNode n + 1 holds PFNs from n * 4096; t8y:/exec/capacity"

[[feature]]
id = "tlb-fill"
title = "Address spaces filled on a fault from the PAL's PTEs, as Alpha PALcode filled the TLB"
status = "done"
milestone = "m1"
needs = ["#frame-caps"]
note = "ADR-0037 decisions 4 and 5: fill() on a fault the PTE allows; WRPTE unmaps a page wherever it is cached; other processes' caches go when the share runs low"

[[feature]]
id = "page-faults"
title = "A translation-not-valid fault sent to the executive so it can page, not an access violation"
status = "todo"
note = "Needed by paging; DESIGN-0001 Exceptions: a page fault is an access violation today; PRD-0003 tier 3 #21"

[[feature]]
id = "device-interrupts"
title = "Device interrupts through seL4's IRQ handlers, delivered at the device's IPL"
status = "todo"
note = "PRD-0003 tier 4 #37 and #38: the disks and the UART are polled"

[[feature]]
id = "dtb-devices"
title = "Device addresses from the DTB, not QEMU virt's fixed ones"
status = "todo"
note = "PRD-0003 tier 4 #38; main.c ponytail on UART_PADDR"

[[feature]]
id = "floating-point"
title = "MTPR_FEN and CLRFEN: floating point enabled per process"
status = "todo"
note = "DESIGN-0001 The Alpha calls: kept, with floating point"

[[feature]]
id = "interlocked-queues"
title = "INSQHIL ... REMQUEQ, the interlocked queue calls"
status = "todo"
note = "DESIGN-0001 The Alpha calls: kept; ADR-0002 says hot paths may want inline code"

[[feature]]
id = "machine-checks"
title = "Machine checks: MCES and the machine check vector"
status = "todo"
note = "DESIGN-0001 The Alpha calls: MFPR_MCES, MTPR_MCES kept"

[[feature]]
id = "smp-calls"
title = "MTPR_IPIR, MFPR_WHAMI and a current process per CPU"
status = "todo"
note = "ADR-0003 consequences; with t8y:/platform/smp"

[[needs]]
on = "/platform/sel4"
why = "Holds every capability and untyped; the only code that calls seL4"

[[needs]]
on = "/platform/virtio"
why = "Drives the disks and the entropy device itself"

[[needs]]
on = "/host/vmacro"
why = "Compiles the executive's privileged instructions into PAL calls, as AMACRO did"
soft = true
+++

On an Alpha, PALcode was firmware below the operating system: each OS
brought its own, and OpenVMS PAL made the CPU look like the parts of a
VAX that VMS relied on. The executive called it with `CALL_PAL` for
mode changes (`CHMx`, `REI`), processor registers (`MTPR`, `MFPR`: IPL,
the SCB base, AST enable and summary, software interrupt requests),
context switches (`SWPCTX`), probes, interlocked queues and halts, and
PALcode handled TLB misses by walking the page tables the executive
wrote. Itanium had no PALcode, so OpenVMS I64 added SWIS, a software
layer doing the same; VSI's x86-64 port kept that approach.

vaxpunk's PAL is seL4's root task (`pal/src/main.c`,
[ADR-0002](../../adr/0002-root-task-is-the-pal.md)). Its interface
speaks only VMS ([ADR-0001](../../adr/0001-pal-interface-vms-vocabulary.md)):
the executive writes `MTPR #8, #PR$_IPL`, vmacro turns it into `svc #0`
with the Alpha code in x7, and seL4 hands the PAL a fault.
[DESIGN-0001](../../design/0001-pal-interface.md) lists the calls.
It reads `EXEC.EXE` from the system disk, makes the RPB, runs each
process as a thread taking turns on one CPU
([ADR-0003](../../adr/0003-one-cpu-many-threads.md)), keeps IPL and
pending interrupts in software, ticks the interval timer from a thread
of its own ([ADR-0004](../../adr/0004-interval-timer-is-a-pal-thread.md)),
maps pages on `WRPTE` because the executive can't write seL4's page
tables, and drives the virtio disks, the entropy device and the UART.
It takes all the RAM seL4 gives it, keeps a share for its own objects and
gives the rest to the executive as PFNs, and fills each thread's address
space on faults from the executive's PTEs, as PALcode filled the TLB
([ADR-0037](../../adr/0037-all-ram-and-address-spaces-filled-on-faults.md)).
Its other limits are PRD-0003 tier 4: polled devices at fixed addresses.
A fault on a page whose PTE isn't valid is an access violation, so the
executive can't page yet.
