# ADR-0005 — Each access mode of a process is a thread with an address space of its own

Oct 1, 2026 · @Marko Mikulicic

Accepted. A process runs each outer access mode, executive, supervisor
and user, in a seL4 thread of its own, made the first time the process
enters that mode. Each of these threads has an address space holding the
pages that mode may read. Kernel mode stays the thread of
[ADR-0003](0003-one-cpu-many-threads.md), in the executive's address space,
where the PAL maps the current process's P0 and P1 pages on each `SWPCTX`.
`CHMx`, `REI`, interrupts and exceptions move the CPU between a process's
threads.

## Context

[ADR-0003](0003-one-cpu-many-threads.md) ran every process in kernel mode,
in one address space, and left supervisor and user mode for later: "each a
task with an address space of its own". Every process image had to be
linked at an address of its own, and any process could write any page,
the executive's included.

On a VAX, the four modes share one address space. Each page's protection
code says which modes may read and write it, and the CPU checks it on
every access. A process's P0 and P1 are its own; S0, the system space, is
the same in every process. `CHMx` moves inward and `REI` outward, and
each mode has its own stack pointer.

A seL4 thread runs at EL0, so its address space is the only protection it
has. Two facts about seL4 on ARM64 shape the design:

- A page table belongs to one address space, and unmapping a page table
  clears it (`performPageTableInvocationUnmap` calls `clearMemory_PT`). So
  the PAL can't keep a process's P0 in a page table and swap it in and out
  of the executive's address space, the way a VAX reloads P0BR.
- A frame is mapped once per capability, so a page mapped in N address
  spaces needs N capabilities, and the root CNode has 4096 slots.

## Decision

1. **The address space is VAX-shaped, below 2 GB**, where MACRO-32's
   sign-extended longwords reach: P0 from 0 to 1 GB, the process's own; S0
   from 1 GB to 1.5 GB, the system's; P1 from 1.5 GB to 2 GB, the
   process's own, its stacks at the top. Every image links at `0x10000`.
2. **The PAL keeps P0 and P1 PTEs per process context**, and a `WRPTE`
   there is the current process's, as on the VAX.
3. **Kernel mode stays one address space.** It is the executive's, with
   S0, and on `SWPCTX` the PAL unmaps the old process's P0 and P1 pages
   there and maps the new one's.
4. **Each outer mode is a thread of the process with an address space of
   its own.** That address space maps the process's pages the mode may
   read, writable if it may write them, and the S0 pages it may read: the
   system service vector, which the executive makes user readable.
5. **The PSL has the VAX's current and previous modes.** `CHMx` delivers
   through the SCB to mode x, or to the current mode if that is an inner
   one, with the old mode as the previous mode. `REI` goes only outward, at
   IPL 0. Interrupts go to kernel mode. Each move between modes saves the
   old mode's stack pointer in the HWPCB and takes the new mode's from it.
   The PAL copies the registers from one thread to the other and stops the
   thread it leaves.
6. **An outer mode's faults are exceptions.** An access violation or a
   reserved instruction, including a privileged PAL call, goes to the
   executive through the SCB, in kernel mode. A fault in kernel mode still
   stops the system.
7. **`PROBER` and `PROBEW` check a mode's access** against the PTEs, as the
   VAX does, so services can check the addresses callers pass them before
   using them.

## Alternatives considered

| Option | Why not |
| --- | --- |
| One address space per process, used by all four modes | No protection between the modes: a user-mode thread could write the executive's pages. |
| A kernel-mode address space per process, with S0 mapped in each | S0's pages would need a frame capability per process: hundreds of pages times 32 processes is more than the root CNode holds. Every S0 `WRPTE` would update every process. Worth it if switches get expensive, with a CNode of its own for frame caps. |
| Swap a page table holding P0 and P1 on `SWPCTX` | seL4 clears a page table when it is unmapped. |
| Keep processes in one P0, each image at its own address | That is what ADR-0003 did. It doesn't scale, since every image needs its own link address, and processes can't have the same addresses as on VMS. |
| Fault P0 and P1 pages into kernel mode lazily after a switch | Fewer calls on switches between processes that rarely enter kernel mode, but the PAL would have to tell a missing page from a real access violation in kernel mode. |
| Only user mode, without executive and supervisor | The PAL's code is the same for any mode, and `$CMEXEC` exercises executive mode. Supervisor mode waits for a command interpreter to run in it. |

## Consequences

**What gets harder.**
- A switch between processes costs a seL4 call per page of the two
  processes, for kernel mode's address space. ponytail: fine at today's
  process sizes.
- Every page a mode may read takes another frame capability. Frame caps
  still live in the root CNode.
- The executive can only reach another process's P0 and P1 from that
  process. So `$DELPRC` asks a process to delete itself, which it does the
  next time it has the CPU, standing in for VMS's kernel AST.
- Services must probe every address a caller passes before using it. A
  service that touches a bad one in kernel mode stops the system.
- S0 pages outer modes may read get their protection before any outer mode
  runs, and keep it.

**What stays easy.**
- Executive code is VAX code: `CHMK` into the dispatcher, `REI` out,
  `IFNORD` and `IFNOWRT` on the caller's addresses, the stack pointers in
  the HWPCB.
- Images link where VMS links them, and two processes can run the same
  image.
- A fault in a user program ends that process, not the system.

**Follow-ups:** ASTs, which bring proper deletion in a process's own
context; privileges, for `$CMKRNL` and `$CMEXEC`; a command interpreter in
supervisor mode; condition handlers in place of exiting on an exception.
