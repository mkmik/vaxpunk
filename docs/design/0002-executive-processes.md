# DESIGN-0002 — Processes, memory and system services in the executive

Oct 1, 2026 · @Marko Mikulicic

How the MACRO-32 executive (`roottask/exec/`) manages memory, creates,
schedules and deletes processes, synchronizes, and serves system calls,
on the PAL interface of [DESIGN-0001](0001-pal-interface.md). It follows
[ADR-0003](../adr/0003-one-cpu-many-threads.md): processes are threads that
take turns on one CPU, and IPL synchronizes them, and
[ADR-0004](../adr/0004-interval-timer-is-a-pal-thread.md): the interval
timer ends their quanta. Every process runs in
kernel mode, in the address space they all share; supervisor and user mode
come later as tasks of their own.

The executive borrows VMS's structure and names (PCB, `SCH$`, `MMG$`,
`EXE$` routines, `SS$_` codes, the system service interfaces) but none of
its code.

## Modules

| File | What |
| --- | --- |
| `exec.mar` | `EXEC$START`, the swapper, `CON$PUTCHAR` |
| `memory.mar` | the PFN list, pages, nonpaged pool, `$CRETVA`, `$DELTVA`, `$EXPREG` |
| `sched.mar` | state queues, `SCH$SCHED`, waits and wakes, the reschedule interrupt, quantum end |
| `timeschdl.mar` | the interval timer and software timer interrupts |
| `event.mar` | event flags, local and common |
| `process.mar` | `$CREPRC`, process start, image activation, `$EXIT`, deletion, `$HIBER`, `$WAKE`, `$SUSPND`, `$RESUME`, `$SETPRI`, `$SETPRN`, `$CMKRNL` |
| `syssrv.mar` | the system service vector, the `CHMK` dispatcher and the stubs |

`roottask/build.rs` links them, with `vtools/lib/consolio.mar`, into
`EXEC.EXE`. The structures are in `vtools/lib/lib.mlb` (`$PCBDEF`,
`$CEBDEF`, `$PTEDEF`, `$RPBDEF`...), what programs need in
`vtools/lib/starlet.mlb` (`$SSDEF`, `$PRTDEF`, the `$name_S` macros).

## Start

`EXEC$START` runs in the boot context at IPL 31:

1. Saves the RPB address from R11.
2. Fills the SCB: `CHMK` to `EXE$CMODKRNL`, software interrupt level 3 to
   `SCH$RESCHED`, level 7 to `EXE$SWTIMINT`, the interval timer to
   `EXE$HWCLKINT`. `MTPR #PR$_SCBB`.
3. `MMG$INIT`: the PFN list and the pool.
4. `SCH$INIT`: empty queues, and the boot context becomes the swapper,
   process 1, current, at priority 16.
5. Lowers IPL to 0 and creates `STARTUP` from `STARTUP.EXE`.
6. Becomes the swapper: it deletes what deleted processes left behind,
   and hibernates in between.

## Memory

The address space every process shares:

| Address | What |
| --- | --- |
| `0x00010000` | `EXEC.EXE` |
| `0x01000000` | process images, each linked at its own 1 MB (`build.rs`) |
| `0x1FFF0000` | the RPB; `0x20000000` the boot volume, read-only |
| `0x40000000` | nonpaged pool, 512 KB: PCBs, kernel stacks, common event blocks |
| `0x50000000` | the pages processes make with `$CRETVA` and `$EXPREG` |
| `0x7FFF0000` | the boot stack's top |

- **PFNs.** `MMG$INIT` puts the PFNs from `RPB$L_FREEPFN` up on a free list,
  a stack. `MMG$ALLOCPFN` and `MMG$DEALLOCPFN` take and give back one.
- **Pages.** `MMG$CREPAG` makes pages at an address: for each, a PFN, a
  `WRPTE` with kernel write to zero it, and another with the protection
  asked for. A page that was there goes back to the free list.
  `MMG$DELPAG` unmaps pages and frees their PFNs; `MMG$SETPRT` changes the
  protection of pages that exist. `WRPTE` returns the old PTE, so the
  executive keeps no page tables of its own.
- **Pool.** `EXE$ALONONPAGED` allocates first fit from a list of free
  blocks sorted by address, in 16-byte units; `EXE$DEANONPGDSIZ` frees,
  merging neighbours.
- **Services.** `$CRETVA` and `$DELTVA` make and delete the pages of a range
  between `0x50000000` and `0x7F000000`; anything else is
  `SS$_PAGOWNVIO`. `$EXPREG` makes pages above the last it made.
  ponytail: one region for every process, never given back.

The PFN list and the pool are synchronized at `IPL$_SYNCH`.

## Processes

A process is a PCB (`$PCBDEF`, 256 bytes, from pool), a 16 KB kernel stack
from pool, an image, and the thread the PAL makes for its HWPCB, which is
inside the PCB. `SCH$GL_PCBVEC` holds the PCBs by index; a PID is a
sequence number in the high word and the index in the low.

### Creation

`$CREPRC` (in the creator's context):

1. Copies the image and process names into a new PCB, and the priority.
2. Allocates the kernel stack and builds at its top the frame the PAL pops
   when the process first runs: PC `EXE$PROCSTRT`, PSL kernel mode at IPL 0,
   the stack top as SP. The HWPCB's KSP points at it.
3. At `IPL$_SYNCH`: checks that the name is unique, takes a slot and a PID,
   and puts the PCB on the COM queue of its priority. If it outranks the
   creator, it requests a reschedule, and runs as soon as IPL drops.

### Start and exit

The scheduler's first `SWPCTX` to the new HWPCB starts its thread at
`EXE$PROCSTRT`, in kernel mode at IPL 0, as a VMS process starts in
`EXE$PROCSTRT` after `SHELL` built its kernel stack:

1. `EXE$IMGACT`, the image activator, finds the image on the boot volume
   (`FIL$OPENFILE`), checks its header, makes sure no other process's image
   uses its addresses, and maps each section: zeroed pages, the contents
   copied in, then the protection: code read and execute, read-only data
   read, the rest kernel write.
2. `CALLS #0` to the image's transfer address.
3. `$EXIT` with the status it returns.

A process has no command interpreter, so, as on VMS without one, the end of
its image is its own end: `$EXIT` deletes it. A failure status is reported
on the console first, as a command interpreter would:

```
%EXEC-W-EXITED, process NOSUCH exited with status 00000910
```

### Deletion

- **Itself** (`$EXIT`, or `$DELPRC` naming itself): it runs itself down
  (image pages, common event flag clusters, slot), goes on the swapper's
  queue, wakes it, and gives up the CPU for good. The swapper deletes its
  context with `DELCTX`, then frees its kernel stack and PCB, since a
  process can't free the stack it runs on.
- **Another** (`$DELPRC`): it isn't running, so it is in a state queue. It
  leaves the queue and its slot at `IPL$_SYNCH`, and is run down and
  deleted at once. ponytail: from outside rather than by a kernel AST in
  its context, which is fine while a process holds nothing but its queue
  entry.

The swapper can't be deleted. ponytail: pages from `$CRETVA` and `$EXPREG`
outlive the process that made them, until processes have address spaces of
their own.

## Scheduling

States and queues, as VMS's `$STATEDEF`:

| State | Queue |
| --- | --- |
| `CUR` | none: `SCH$GL_CURPCB` |
| `COM` | `SCH$AQ_COMH`, one per priority 0-31, highest first; bit n of `SCH$GL_COMQS` set if queue n isn't empty |
| `HIB` | `SCH$GQ_HIBWQ` |
| `LEF` | `SCH$GQ_LEFWQ` |
| `CEF` | the common event block's own queue |
| `SUSP` | `SCH$GQ_SUSPWQ` |

- **`SCH$SCHED`** takes the first process from the highest non-empty COM
  queue, makes it current and `SWPCTX`es to its HWPCB, unless it is the
  process already there. The process leaving waits inside its `SWPCTX`
  until the CPU comes back, so the switch saves no registers. With no
  process computable, the CPU waits for an interrupt (`WTINT`).
- **`SCH$WAIT`** puts the current process on a wait queue and calls
  `SCH$SCHED`. It returns when the wait is over and the CPU is back.
- **`SCH$WAKEPCB`** takes a process off its wait queue and makes it
  computable, or suspended if a `$SUSPND` came while it waited.
- **`SCH$MAKECOM`** puts a process on its COM queue and, if it outranks the
  current one, requests the reschedule interrupt:
  `SOFTINT #IPL$_RESCHED`.
- **`SCH$RESCHED`**, the level 3 software interrupt, puts the current
  process back on its COM queue's tail and calls `SCH$SCHED`.

### Quantum end

A process keeps the CPU until it waits, a higher priority one becomes
computable, or its quantum is up, as on VMS:

1. **`EXE$HWCLKINT`**, the interval timer interrupt, every 10 ms at
   `IPL$_HWCLK` (24), counts the current process's `PCB$W_QUANT` up from
   minus `SGN$GW_QUANTUM`, 20 ticks, which `$CREPRC` sets. At 0 it requests
   the software timer interrupt, `SOFTINT #IPL$_TIMER` (7). A tick while
   the CPU is idle charges no one.
2. **`EXE$SWTIMINT`**, at `IPL$_TIMER`, raises IPL to `IPL$_SYNCH` and calls
   `SCH$QEND` if the quantum is still up.
3. **`SCH$QEND`** gives the process a new quantum and, if a COM queue at its
   priority or above isn't empty, requests the reschedule interrupt, which
   puts it on its queue's tail: round robin within a priority. Real-time
   processes, priority 16 and up, keep the CPU. ponytail: no priority decay
   toward the base, since nothing boosts a priority yet.

Code at `IPL$_TIMER` or above, the scheduler included, defers quantum end
until IPL drops.

## Synchronization

Inside the executive, IPL, as on a uniprocessor VMS. `DSBINT`, `ENBINT`,
`SETIPL` and `SOFTINT` in `lib.mlb` are VMS's macros:

| IPL | Protects |
| --- | --- |
| `IPL$_HWCLK` (24) | the interval timer interrupt runs here |
| `IPL$_SYNCH` (8) | the scheduler's queues and PCBs, the PFN list, pool, common event blocks, the console |
| `IPL$_TIMER` (7) | the software timer interrupt runs here |
| `IPL$_RESCHED` (3) | the reschedule interrupt runs here; below it, the CPU may move |
| 0 | process code |

Code at or above `IPL$_SYNCH` keeps the CPU: only the interval timer
interrupt reaches it, which takes nothing away, and it only gives the CPU
away by calling `SCH$SCHED`. The console is shared, so a line is written at
`IPL$_SYNCH`, or another process's could come in the middle of it. The
queue instructions, `INSQUE` and `REMQUE`, are plain code, since only one
thread runs. A second CPU will need spinlocks, which on VMS also raise IPL
(ADR-0003).

Between processes, VMS's event flags and hibernation:

- **Local event flags**, 0-63, are the process's own.
- **Common event flags**, 64-127: `$ASCEFC` associates cluster 2 (64-95) or
  3 (96-127) with a named common event block (`$CEBDEF`), made on first
  use. Every process associated with it shares its 32 flags. It goes away
  when the last process dissociates.
- `$SETEF`, `$CLREF` and `$READEF` set, clear and read a flag; `$WAITFR`,
  `$WFLOR` and `$WFLAND` wait for one, any or all of a mask. A waiting
  process is in `LEF`, or in `CEF` on the block's queue, and `$SETEF` on a
  common flag wakes those whose wait it satisfies. A woken process checks
  its flags again.
- `$HIBER` sleeps until `$WAKE`; a `$WAKE` that comes first makes the next
  `$HIBER` return at once.

## System services

A program calls `SYS$name` with `CALLS` or `CALLG`, or with the `$name_S`
macros of `starlet.mlb`, which push the arguments. `SYS$name` is a routine
in `syssrv.mar` that does `CHMK #code` and returns. The PAL delivers the
`CHMK` to `EXE$CMODKRNL`, which checks the code and the argument count
(`SS$_ILLSER`, `SS$_INSFARG`), calls `EXE$name` with `CALLG` on the caller's
argument list, and `REI`s with its status in R0. Every process is in
kernel mode, so `CHMK` changes no mode yet; it is where supervisor and
user mode will come in.

Programs link against `SYS.STB`, which `build.rs` makes from `EXEC.EXE`'s
map: every global symbol of the executive as a constant, as kernel-mode
code on VMS linked against `SYS.STB`. So they reach `SYS$name` and executive
routines such as `EXE$OUTZSTRING` directly.

| Group | Implemented | Stubs: `SS$_ILLSER` |
| --- | --- | --- |
| Process control | `$CREPRC`, `$DELPRC`, `$EXIT`, `$HIBER`, `$WAKE`, `$SUSPND`, `$RESUME`, `$SETPRI`, `$SETPRN`, `$CMKRNL` | `$FORCEX`, `$SCHDWK`, `$CANWAK`, `$GETJPI`, `$GETJPIW`, `$DCLEXH`, `$CANEXH`, `$SETPRV`, `$CMEXEC` |
| Event flags | `$ASCEFC`, `$DACEFC`, `$SETEF`, `$CLREF`, `$READEF`, `$WAITFR`, `$WFLOR`, `$WFLAND` | `$DLCEFC` |
| Memory | `$CRETVA`, `$DELTVA`, `$EXPREG` | `$CNTREG`, `$SETPRT`, `$LKWSET`, `$ULWSET`, `$LCKPAG`, `$ULKPAG`, `$CRMPSC`, `$MGBLSC` |
| Other | | `$DCLAST`, `$SETAST`, `$GETTIM`, `$SETIMR`, `$CANTIM`, `$ASSIGN`, `$DASSGN`, `$QIO`, `$QIOW`, `$CRELNM`, `$DELLNM`, `$TRNLNM`, `$GETSYI` |

Arguments the implemented services take but ignore: `$CREPRC`'s I/O,
privileges, quotas, UIC, mailbox and status flags, `$ASCEFC`'s protection
and permanence (every cluster is temporary), the access modes.

## The boot volume's programs

`roottask/sysexe/` holds the programs on the boot volume, which show the
services at work and which `just check` boots to the end:

| Program | Does |
| --- | --- |
| `STARTUP` | makes 4 pages with `$EXPREG`, checks and deletes them; creates `SLEEPER` at a higher priority, which runs at once, and `PING` and `PONG`; hibernates until `PONG` wakes it; deletes `SLEEPER`; creates `SVCTEST` and `HOG` |
| `SLEEPER` | hibernates until it is deleted |
| `PING`, `PONG` | take three turns through common event flags 64 and 65 of the cluster `PINGPONG`; `PONG` then wakes `STARTUP` |
| `SVCTEST` | checks the statuses of the services the others don't use, and of errors: local event flags, the dispatcher's checks and a stub, `$CRETVA` and `$DELTVA`, `$CMKRNL`, `$SETPRI`, and `$SUSPND`, `$WAKE`, `$RESUME` and `$DELPRC` on a process of its own; then creates one whose image doesn't exist, which exits with `SS$_NOSUCHFILE` |

| `HOG` | makes a page, creates `NUDGE` at its own priority and loops until `NUDGE` writes the page, with no system service: only quantum end lets `NUDGE` run |
| `NUDGE` | writes `HOG`'s page |

When every process but the swapper is gone, the CPU idles in `WTINT`,
taking the clock's interrupts.

## Next

- The system time, `$GETTIM`, and the timer queue: `$SETIMR`, `$SCHDWK`,
  served by `EXE$SWTIMINT`. Priority boosts on wake and decay at quantum
  end.
- ASTs: `$DCLAST`, AST delivery at `IPL$_ASTDEL`, and with them process
  deletion in the process's own context.
- Supervisor and user mode, each a task per process, with `CHMx` and `REI`
  between them, `PROBE`, and per-process address spaces for P0 and P1.
- A disk driver and Files-11, in place of the boot volume.
