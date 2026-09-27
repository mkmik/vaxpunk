# ADR-0001 — PAL interface speaks only VMS vocabulary

Sep 27, 2026 · @Marko Mikulicic

Accepted. The vaxpunk executive runs as a native seL4 component on top of a PAL layer whose interface uses only VMS concepts; all seL4 knowledge lives inside the PAL.

## Context

The bulk of the vaxpunk executive will be written in BLISS and MACRO, the languages VMS itself used, and seL4 should be as invisible to that code as possible. The executive needs something below it that looks like the machine VMS expects, not like a capability microkernel.

VMS has solved this twice before:

- **Alpha PALcode.** Each OS had its own PAL. OpenVMS PAL made Alpha look like the parts of a VAX VMS relied on: four access modes, change-mode calls (`CHMK`, `CHME`, `CHMS`, `CHMU`) and `REI`, interrupt priority levels (IPL), AST registers, software interrupts, probing, interlocked queues and context switching. The executive never touched raw Alpha privileged state.
- **Itanium SWIS.** Itanium had no PALcode, so OpenVMS I64 added software interrupt services, a kernel layer emulating Alpha PAL behaviour. It ended up doing more than PAL did, because it had no hardware help. VSI's x86-64 port followed the same approach.

seL4 is a third case with even less help: the executive runs at user level, cannot mask interrupts, and cannot write page tables.

## Decision

1. **The executive is a native seL4 component**, not a guest OS at EL1 in a virtual machine.
2. **Below it sits a PAL layer, inspired by OpenVMS Alpha PALcode but not copied literally.** Like SWIS, it does more than Alpha PAL did, wherever seL4 gives less than Alpha hardware.
3. **The PAL interface speaks only VMS vocabulary:** PCBs, PTEs, IPL, ASTs, access modes, the system control block. Never capabilities, endpoints or scheduling contexts. Whatever seL4 needs is translated inside the PAL.
4. **The PAL is the only code that calls seL4.** It is written in C or Rust. Everything above it is BLISS and MACRO.

The test for any new PAL call: could its signature have appeared in a DEC internals manual? If it names a seL4 concept, it belongs inside the PAL, not in its interface.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Executive as an EL1 guest, seL4 as hypervisor | Real page tables would be easy, but seL4 would isolate nothing inside the VM, the four modes would still need emulating on EL0/EL1, and it needs `KernelArmHypervisorSupport`. Rejected: it makes seL4 a VM host rather than the base we build on. |
| Literal Alpha PAL interface | Alpha PAL assumed hardware help seL4 doesn't give: masking interrupts, editing page tables in memory, a TLB-miss handler reading them. A literal copy would be a lie at exactly the hard spots. |
| Executive calls seL4 directly | Spreads capability and object management through BLISS and MACRO code, ties every subsystem to seL4, and loses the epoch-appropriate look of the executive. |

## Consequences

**What the PAL must cover**, starting from the Alpha OpenVMS PAL call list:

| Area | Alpha had | vaxpunk PAL does |
| --- | --- | --- |
| Mode changes | `CHMx` and `REI` as PAL calls | IPC to the right task for that process |
| IPL and interrupts | Hardware interrupt masking by IPL | Software IPL: interrupts arrive as notifications and are held pending until IPL drops, as SWIS did |
| ASTs, software interrupts | `ASTEN`, `ASTSR`, `SIRR` registers | Same bits and delivery rules; notifications carry the wakeups |
| Memory | Executive writes PTEs, PAL reads them on TLB miss | Explicit PTE-shaped map, unmap and protect calls; frames and page-table objects stay hidden |
| Process contexts | `SWPCTX` | Create and delete a process context, hiding TCBs, address spaces, capability spaces and scheduling contexts |
| Time | Interval clock interrupt | Clock, timers and quantum on seL4 timeouts and budgets |
| Devices | Direct register access | Interrupt lines, register mappings and DMA memory for drivers |
| Console and boot | Console callbacks | Minimum early output and startup |

**What gets harder.** Memory management is the one executive subsystem that departs from Alpha: it calls the PAL instead of writing page tables. The PAL is larger than Alpha PAL and must be kept honest to the vocabulary rule as it grows.

**What stays easy.** The executive's code is the same whichever way the PAL implements a call, so PAL internals, including how modes map to seL4 tasks, can change without touching BLISS or MACRO code.

**Follow-up.** Write the PAL interface document in the repo, with this rule as its opening section, and mark each Alpha PAL call as kept, changed or new.
