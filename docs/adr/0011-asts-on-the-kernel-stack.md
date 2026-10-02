# ADR-0011 — The PAL requests AST delivery from bits in the HWPCB, and the executive calls AST routines on top of the kernel stack

Oct 2, 2026 · @Marko Mikulicic

Proposed. Each process has an AST queue of ACBs, by access mode. The
executive keeps two bytes in the HWPCB, Alpha's ASTSR and ASTEN: a bit per
mode with an AST queued, and a bit per mode whose ASTs may be delivered.
The PAL requests the IPL 2 software interrupt when IPL is below 2 and a
mode with both bits set is the current mode or an inner one. The executive
delivers there, and when it ends a wait, by calling the AST routine in its
mode on top of the kernel stack, as ADR-0010 calls DCL for CTRL/Y.
`$ASTEXIT` puts the stack back. `$DELPRC` deletes another process with a
kernel mode AST.

## Context

VMS's asynchronous system traps (ASTs) are calls a process gets on top of
whatever it was doing. Timers, I/O completion, CTRL/Y and process deletion
all reach a process through them. DESIGN-0002 left them for later.
`$DCLAST` and `$SETAST` were stubs, `$SETIMR` refused an AST address, and
`$DELPRC` had the scheduler check a flag, `PCB$V_DELPEN`.

An AST is delivered when the process runs at IPL 0 or 1, in the AST's
mode or an outer one, with ASTs of that mode enabled and none of its AST
routines running. That moment can come on any `REI`, from an interrupt as
well as from a service. On the VAX, `REI` compared the new mode with the
process's `ASTLVL` and requested the IPL 2 interrupt. On Alpha, the PAL did
the same with `ASTSR` and `ASTEN`, and DESIGN-0001 already lists those calls
as kept.

A process waiting in a service, in `$HIBER` or `$WAITFR`, gets its ASTs
too: a `$SETIMR` AST whose routine calls `$WAKE` is how a VMS program
sleeps with a timeout. VMS's waits return to the caller's mode and run the
service again. Here the waits loop in kernel mode, which ADR-0010 already
made them do for CTRL/Y.

## Decision

1. **ASTSR and ASTEN are bytes of the HWPCB** (`HWPCB$B_ASTSR`,
   `HWPCB$B_ASTEN`), which the executive writes at `IPL$_SYNCH`, for any
   process. The PAL reads them each time it might deliver an interrupt,
   when IPL drops, on `REI` and on a context's first start, and requests
   software interrupt level 2 if IPL is below 2 and
   `ASTSR & ASTEN` has a bit for the current mode or an inner one. There
   are no `MTPR_ASTSR` or `MTPR_ASTEN` calls.
2. **The executive keeps the ACBs**, on `PCB$Q_ASTQFL`, kernel mode's
   first. ASTSR has a bit for each mode with one queued. ASTEN is
   `PCB$B_ASTEN`, which `$SETAST` sets, less `PCB$B_ASTACT`, the modes
   whose AST routine runs. `SCH$QAST` queues an ACB and ends the process's
   wait, if it waits.
3. **`SCH$ASTDEL`, the level 2 interrupt, delivers** in the mode it
   interrupted. So does `SCH$WAIT` when a wait ends, in the mode that called
   the service waiting, which `RD_PS`, Alpha's call, now implemented, reads
   from the PSL.
4. **An AST routine is called on top of the kernel stack.** `EXE$ASTDEL`
   saves the registers and IPL there, links the save area to the one
   before it in `PCB$L_ASTSP`, and `REI`s below it, to `EXE$ASTDISP` in the
   vector, in the AST's mode on that mode's stack. `EXE$ASTDISP` calls the
   routine with its parameter, then `$ASTEXIT`. That service puts the
   kernel stack pointer back from `PCB$L_ASTSP`, and `EXE$ASTDEL` delivers
   the next AST or returns to what it interrupted.
5. **A kernel mode ACB may name an executive routine**, `ACB$L_KAST`,
   which is `JSB`'d instead, at `IPL$_SYNCH`. `$DELPRC` of another process
   queues one that runs the process down, replacing `PCB$V_DELPEN`.
6. **A `$SETIMR` entry with an AST becomes its ACB** when it is due,
   as on VMS: the TQE has the ACB's PID, AST and parameter at the same
   offsets.
7. **Image rundown flushes user mode ACBs** and forgets every AST routine
   that runs, since the kernel stack holding their state is emptied next.
   Process rundown flushes the rest.

## Alternatives considered

| Option | Why not |
| --- | --- |
| The executive checks for ASTs before each `REI` to an outer mode, as `EXE$CTRLYCHK` does for CTRL/Y | Every interrupt handler's `REI` would need the check, and missing one leaves an AST waiting until the next service; the PAL sees every `REI` already |
| The VAX's `ASTLVL`, one number per process | Doesn't say which modes have ASTs disabled or one running; the executive would recompute it from the same bits |
| `MTPR_ASTSR` and `MTPR_ASTEN`, as Alpha | The PAL would cache bits the executive must also set for processes that aren't current, through their HWPCBs; reading the HWPCB on each check is one lookup |
| Save the interrupted registers in the ACB or the PCB, and `REI` back from `$ASTEXIT` | A wait inside a service would have to be unwound; the kernel stack already holds that state, as ADR-0010 found for CTRL/Y |
| Rerun the waiting service from the caller's mode after the AST, as VMS does | The services here wait in kernel mode loops; they already look again after `SCH$WAIT` returns |
| Allocate the TQE's ACB when the timer is due | Pool may be gone by then; the TQE is free once it is due |
| Keep `PCB$V_DELPEN` | It is a kernel AST already, checked in two places; the AST is delivered wherever kernel ASTs are |

## Consequences

**What gets harder.**
- Kernel code that runs at IPL 0 can be interrupted by a kernel mode AST,
  including deletion, and may never get the CPU back. Code that holds pool
  there loses it. ponytail: as before, when `SCH$SCHED` deleted on its
  way back; VMS raises IPL to `IPL$_ASTDEL` around such code.
- An AST routine runs on top of the service that was waiting, so the
  kernel stack holds both.
- The PAL reads the current HWPCB each time it might deliver.
- `$IMGACT` while an AST routine runs throws it away with the image.

**What stays easy.**
- The image doesn't know: every register comes back as it was.
- CTRL/Y can become DCL's supervisor mode AST, and `$CONTINUE` the
  return from it.
- I/O completion ASTs, `$QIO`'s `astadr`, are a `SCH$QAST` away.

**Follow-ups:** `$QIO` completion ASTs; a CTRL/Y AST in place of
`EXE$CTRLY` and `$CONTINUE`, then CTRL/C and `$FORCEX`; the full VMS AST
argument list (R0, R1, PC, PSL after the parameter); AST quotas.
