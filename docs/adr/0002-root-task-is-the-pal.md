# ADR-0002 — The root task is the PAL; the executive is a task that calls it with privileged instructions

Oct 1, 2026 · @Marko Mikulicic

Accepted. The root task is the PAL. The executive runs as a separate seL4 task
with no capabilities and reaches the PAL only through VAX privileged
instructions. vmacro compiles each of them into an `svc` that seL4 hands the
PAL as a fault.

## Context

[ADR-0001](0001-pal-interface-vms-vocabulary.md) put a PAL below the
executive and kept seL4 out of the executive, but left open where the PAL
runs and how the executive calls it. Until now the root task loaded
`exec.exe` into its own address space and called it as a subroutine. There
was no boundary between them, and nothing to trap into.

The executive's MACRO-32, and later BLISS, should read like VMS kernel
code: the source a VAX or Alpha executive would have, within reason. On a
VAX the executive talks to the processor through privileged instructions:
`MTPR` and `MFPR` for processor registers, `CHMx` and `REI` for mode
changes, `HALT`. Alpha had no such instructions. The MACRO-32 compiler
(AMACRO) turned each one into a `CALL_PAL` to the OpenVMS PALcode, so the
same source kept working.

## Decision

1. **The root task is the PAL.** It already holds every capability and
   untyped, and it stays the only code that calls seL4.
2. **The executive is a task of its own.** It has its own address space, an
   empty CSpace, its own TCB and scheduling context, and a priority below the
   PAL's. The PAL's endpoint is its fault endpoint.
3. **The executive calls the PAL with VAX privileged instructions.** vmacro
   compiles each into a PAL call, as AMACRO compiled them into `CALL_PAL`,
   so executive source stays VAX source.
4. **A PAL call is `svc #0` with a PAL function code in x7.**
   - seL4 reads its syscall number from x7, and its own numbers are
     negative. So a non-negative x7 is an unknown syscall, which seL4 sends
     to the fault endpoint as a fault message carrying x0-x7 and the PC.
   - The PAL replies with the registers to resume with.
   - The codes are Alpha OpenVMS PALcode's. Where Alpha has no call for
     something the executive needs, vaxpunk adds its own, from 0x40.
5. **Every other fault of the executive goes to the PAL too:** page faults
   and undefined instructions.
6. **vrun doesn't implement PAL calls.** vrun is a user-mode runner for
   quick turns on compiler output. A privileged instruction fails there as
   it would in user mode.

[DESIGN-0001](../design/0001-pal-interface.md) specifies the calls.

## Alternatives considered

| Option | Why not |
| --- | --- |
| A bootstrapper root task that starts a PAL task and the executive | The bootstrapper would hand every capability to the PAL and then idle: one more hop, and nothing gained until the PAL must be restartable or hold fewer rights than the boot code. [ADR-0001](0001-pal-interface-vms-vocabulary.md) keeps the PAL's internals free, so the split can come later without touching executive code. |
| The executive as a second thread in the root task's address space | Cheaper to set up, but the executive could write the PAL's memory, and there would be no address space for the PAL to manage on the executive's behalf. |
| The executive calls the PAL with `seL4_Call` on an endpoint | The executive would need libsel4, an IPC buffer and a capability: seL4 concepts in the executive's runtime, which ADR-0001 rules out. |
| The executive holds every capability and handles its own page faults | seL4 delivers a thread's faults to another thread, so it can't handle its own. And MACRO-32 code can't make seL4 calls without the libsel4 runtime it isn't meant to have. |
| The function code in the `svc` immediate | seL4 ignores the immediate and doesn't report it in the fault message. x7 is what it reads. |
| One generic `MTPR` call that takes the register number as an argument | Fewer codes, but Alpha's per-register calls already record which VAX registers an OpenVMS executive needs. Following Alpha's table gives the interface a published reference. |
| New function codes throughout | Nothing gained over a published table. |

## Consequences

**What gets harder.**
- Every PAL call is a fault round trip through seL4: two context switches.
  That is fine for `MTPR` and `MFPR`. Hot paths such as the interlocked
  queue instructions may want inline code later.
- Only x0-x7 reach the PAL, so a call takes at most six arguments, x0-x5.
  Alpha's calls had six too, a0-a5. The call overwrites the VAX's R0 and R7,
  so vmacro saves them around it.
- With an empty CSpace, a raw `svc` with a negative x7 is a real seL4
  syscall that fails. The exception is a debug kernel's debug syscalls,
  which need no capability. Release kernels don't have them.
- Once there are access modes other than kernel, the PAL must check the
  mode on privileged calls.
- The console routines in `crosstools/vtools/lib/consolio.mar` still write through
  vrun's `svc #2`. They run under vrun until the executive links them;
  `CON$PUTCHAR` then becomes `MTPR R0, #PR$_TXDB`.

**What stays easy.**
- Executive source is VAX source. `MTPR #8, #PR$_IPL` is written as on a
  VAX, and vmacro and the PAL decide what it costs.
- The PAL's internals, including a later split into more tasks, can change
  without touching executive code.

**Follow-ups:** `CHMx` and `REI`, the AST and software interrupt registers,
interrupts as notifications held pending by IPL, and the memory calls, as
listed in ADR-0001.
