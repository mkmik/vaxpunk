# DESIGN-0002 — Processes, memory and system services in the executive

Oct 1, 2026 · @Marko Mikulicic

How the MACRO-32 executive (`roottask/exec/`) manages memory, creates,
schedules and deletes processes, synchronizes, and serves system calls,
on the PAL interface of [DESIGN-0001](0001-pal-interface.md). It follows
[ADR-0003](../adr/0003-one-cpu-many-threads.md): processes are threads that
take turns on one CPU, and IPL synchronizes them,
[ADR-0004](../adr/0004-interval-timer-is-a-pal-thread.md): the interval
timer keeps the system time, serves the timer queue and ends their quanta,
[ADR-0005](../adr/0005-access-modes-are-threads.md): each process has
P0 and P1 of its own and runs its image in user mode, entering the inner
modes through system services, and
[ADR-0006](../adr/0006-cli-in-p1-runs-images-in-its-process.md): a command
interpreter lives in P1 in supervisor mode and runs images in its own
process, and
[ADR-0007](../adr/0007-system-disk-files-11-and-rms.md): the files are on a
Files-11 volume, which the executive reads by LBN, with RMS on top.

The executive borrows VMS's structure and names (PCB, `SCH$`, `MMG$`,
`EXE$` routines, `SS$_` codes, the system service interfaces) but none of
its code.

## Modules

| File | What |
| --- | --- |
| `exec.mar` | `EXEC$START`, the swapper, `CON$PUTCHAR` |
| `memory.mar` | the PFN list, pages, nonpaged pool, `$CRETVA`, `$DELTVA`, `$EXPREG` |
| `sched.mar` | state queues, `SCH$SCHED`, waits and wakes, the reschedule interrupt, quantum end |
| `timeschdl.mar` | the interval timer and software timer interrupts, the system time, the timer queue, `$GETTIM`, `$SETIMR`, `$CANTIM`, `$SCHDWK`, `$CANWAK` |
| `event.mar` | event flags, local and common |
| `process.mar` | `$CREPRC`, process start, image activation, `$IMGACT`, `$EXIT`, image rundown, deletion, `$HIBER`, `$WAKE`, `$SUSPND`, `$RESUME`, `$SETPRI`, `$SETPRN`, `$CMKRNL` |
| `lnm.mar` | logical name tables, `$CRELNM`, `$DELLNM`, `$TRNLNM` |
| `qio.mar` | `$ASSIGN`, `$DASSGN`, `$QIO`, `$QIOW`: writes on the console and reads from it; the console receive interrupt and the type-ahead buffer |
| `syssrv.mar` | the system service vector, the `CHMK` and `CHME` dispatchers, `$CMEXEC`, where processes enter user and supervisor mode, the exception handlers and the stubs |
| `f11.mar` | the system disk, Files-11 ODS-2, read only: `FIL$MOUNT`, headers, maps, directories, `FIL$OPENFILE` for the image activator |
| `rms.mar` | RMS: file specifications, `$PARSE`, `$SEARCH`, `$OPEN`, `$CONNECT`, `$GET`, `$DISCONNECT`, `$CLOSE` |

`roottask/build.rs` links them, with `vtools/lib/consolio.mar`, into
`EXEC.EXE`, in S0 at `0x40010000`. The structures are in `vtools/lib/lib.mlb` (`$PCBDEF`,
`$CEBDEF`, `$PTEDEF`, `$RPBDEF`...), what programs need in
`vtools/lib/starlet.mlb` (`$SSDEF`, `$PRTDEF`, the `$name_S` macros, RMS's
`$FABDEF`, `$RABDEF`, `$NAMDEF`, `$RMSDEF` and the `$FAB`, `$RAB`, `$NAM`
blocks and `$OPEN`... calls).

## Start

`EXEC$START` runs in the boot context at IPL 31:

1. Saves the RPB address from R11.
2. Fills the SCB: reserved instructions to `EXE$OPCDEC`, access violations
   to `EXE$ACVIOLAT`, `CHMK` to `EXE$CMODKRNL`, `CHME` to `EXE$CMODEXEC`,
   software interrupt level 3 to `SCH$RESCHED`, level 4 to `TTY$IOPOST`,
   level 7 to `EXE$SWTIMINT`, the interval timer to `EXE$HWCLKINT`, the
   console receiver to `TTY$RCVINT`. `MTPR #PR$_SCBB`.
3. `MMG$INIT`: the PFN list and the pool. Then makes the vector's pages
   user readable (*System services*), before any outer mode runs.
4. `SCH$INIT`: empty queues, and the boot context becomes the swapper,
   process 1, current, at priority 16.
5. `EXE$INITTIM`: an empty timer queue, and the system time from
   `RPB$L_BOOTTIME`. `TTY$INIT`: an empty type-ahead buffer, and the
   console receive interrupt enabled.
6. `FIL$MOUNT` mounts the system disk (*Files*) and prints
   `%MOUNT-I-MOUNTED, VAXPUNK mounted on _DKA0:`. If it can't, the
   executive halts with `%EXEC-F-NOMOUNT` and the status.
7. Lowers IPL to 0 and creates the console's process, `SYSTEM`, from
   `DCL.EXE` (*The command interpreter*).
8. Becomes the swapper: it deletes what deleted processes left behind,
   and hibernates in between.

## Memory

The address space, VAX-shaped (DESIGN-0001, *Memory*; `$VADEF`). P0 and
P1 are each process's own, S0 every process's:

| Address | What |
| --- | --- |
| `0x00010000` | P0: the process's image, linked there (`build.rs`), then its `$EXPREG` pages |
| `0x40010000` | S0: `EXEC.EXE` |
| `0x48000000` | nonpaged pool, 512 KB: PCBs, kernel stacks, common event blocks, timer queue entries, logical names, open files, images being activated |
| `0x4FFF0000` | the RPB |
| `0x5FFF0000` | the boot stack's top |
| `0x60000000` | P1: the process's `$CRETVA` pages |
| `0x7FF00000` | `VA$C_CLI`: its command interpreter, if it has one, `DCL.EXE` |
| `0x7FFE4000` | its executive stack, 16 KB, executive write; above it its supervisor stack, supervisor write, and its user stack, each 16 KB, to `0x7FFF0000` |
| `0x7FFEFF00` | `VA$C_FOREIGN`, at the user stack's top: the image's command line, `.ASCIC`, which `$IMGACT` puts there |

- **PFNs.** `MMG$INIT` puts the PFNs from `RPB$L_FREEPFN` up on a free list,
  a stack. `MMG$ALLOCPFN` and `MMG$DEALLOCPFN` take and give back one.
- **Pages.** `MMG$CREPAG` makes pages at an address: for each, a PFN, a
  `WRPTE` with kernel write to zero it, and another with the protection
  asked for. A page that was there goes back to the free list.
  `MMG$DELPAG` unmaps pages and frees their PFNs; `MMG$SETPRT` changes the
  protection of pages that exist. `WRPTE` returns the old PTE, so the
  executive keeps no page tables of its own. A page in P0 or P1 is the
  current process's.
- **Pool.** `EXE$ALONONPAGED` allocates first fit from a list of free
  blocks sorted by address, in 16-byte units; `EXE$DEANONPGDSIZ` frees,
  merging neighbours.
- **Services.** `$CRETVA` and `$DELTVA` make and delete the pages of a range
  of the process's own: all in P0, or all in P1 below `VA$C_CLI`;
  anything else is `SS$_PAGOWNVIO`. `$EXPREG` makes pages past the end of
  P0. Pages they make are user writable. The PCB keeps how far P0 and P1
  reach (`PCB$L_FREP0VA`, `PCB$L_FREP1VA`), and rundown deletes every page
  in between.

The PFN list and the pool are synchronized at `IPL$_SYNCH`.

## Processes

A process is a PCB (`$PCBDEF`, 352 bytes, from pool), a 16 KB kernel stack
from pool, an image and stacks in its P0 and P1, and the threads the PAL
makes for its HWPCB, which is inside the PCB: one for kernel mode and one
for each outer mode it enters. `SCH$GL_PCBVEC` holds the PCBs by index; a PID is a
sequence number in the high word and the index in the low.

### Creation

`$CREPRC` (in the creator's context):

1. Copies the image and process names into a new PCB, and the priority,
   and makes its logical name table, with `SYS$INPUT`, `SYS$OUTPUT` and
   `SYS$ERROR` for its `input`, `output` and `error` arguments, those it
   was given (*Logical names*).
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

1. Makes the executive, supervisor and user stacks at the top of P1 and
   puts their tops in the HWPCB.
2. `IMG$ACTIVATE`, the image activator, reads the image from the system
   disk, in `DKA0:[SYSEXE]` unless its name says where, into pool
   (`FIL$OPENFILE`, *Files*), checks its header and that it is all in P0, or all
   from `VA$C_CLI` to the stacks in P1, and maps each section: zeroed
   pages, the contents copied in, then the protection: code read and
   execute, read-only data read, the rest write, for user mode in P0 and
   supervisor mode in P1. Then it frees the pool.
3. An image in P1 is a command interpreter: `EXE$CLIENTRY` (*The command
   interpreter*). Any other, `EXE$USRENTRY`: `REI` to `EXE$USRSTART`, in
   user mode, on an empty user stack, with the image's transfer address in
   R1. There it calls the image with `CALLS #0`, and `$EXIT`s with the
   status it returns.

A process without a command interpreter, as on VMS, ends with its image:
`$EXIT` deletes it. A failure status is reported on the console first, as
a command interpreter would:

```
%EXEC-W-EXITED, process NOSUCH exited with status 00018292
```

### Deletion

- **Itself** (`$EXIT` without a command interpreter, or `$DELPRC` naming
  itself): it runs itself down (timer queue entries, every page of its P0
  and P1, common event flag clusters, logical names, channels, open files, slot), goes on the
  swapper's queue, wakes it, and
  gives up the CPU for good. The swapper deletes its context with
  `DELCTX`, then frees its kernel stack and PCB, since a process can't free
  the stack it runs on.
- **Another** (`$DELPRC`): its P0 and P1 can only be reached from itself,
  so it deletes itself. At `IPL$_SYNCH` it leaves its slot, so that no one
  finds it any more, gets `PCB$V_DELPEN`, and its wait ends. The next time
  it has the CPU, when `SCH$SCHED` returns to it or it starts at
  `EXE$PROCSTRT`, it deletes itself as above. ponytail: a flag the
  scheduler checks, standing in for VMS's kernel AST.

The swapper can't be deleted.

### The command interpreter

[ADR-0006](../adr/0006-cli-in-p1-runs-images-in-its-process.md): an image
linked in P1, at `VA$C_CLI`, is a command interpreter, `DCL.EXE` the one
there is. The image activator puts its transfer address in
`PCB$L_CLI`, and the process keeps it, in supervisor mode, for its life:

- **`EXE$CLIENTRY`** empties the kernel stack and `REI`s to `EXE$CLISTART`
  in supervisor mode, on an empty supervisor stack, which calls the
  command interpreter with one argument, a status: `SS$_NORMAL` when the
  process starts, and after that each image's. If it returns, the process
  deletes itself with `$DELPRC`.
- **`$IMGACT image, cmdlin`** runs an image in the current process. It
  copies the name into the PCB, and the command line, if there is one, up
  to 255 characters, to `VA$C_FOREIGN`, where the image finds it.
  It remembers the channels and files the process has in `PCB$L_CLICHANS`
  and `PCB$L_CLIFILES`, runs the
  old image down, activates the new one, which must be in P0, and calls it
  in user mode at `EXE$USRENTRY`, on an empty user stack below the command
  line. It returns only if the activation fails, with its status.
  ponytail: a fixed address, standing in for `LIB$GET_FOREIGN`, which asks
  the command interpreter.
- **`$EXIT`**, the image's or an exception's, in a process with a command
  interpreter: `EXE$IMGRUNDOWN` gives back the image's timer queue
  entries, P0 pages and common event flag clusters, the channels not in
  `PCB$L_CLICHANS` and the files not in `PCB$L_CLIFILES`; then
  `EXE$CLIENTRY` with the status.

The command interpreter keeps nothing on its stack across an image; what
it remembers is in its P1 data. `EXEC$START` creates the console's
process, `SYSTEM`, with `DCL.EXE`:

| Command | Does |
| --- | --- |
| `RUN image` | `$IMGACT`, with `.EXE` if the name has no type |
| `DIRECTORY [spec]` | `$IMGACT` of `DIRECTORY.EXE`, with the spec, in capitals, as its command line |
| `TYPE spec` | `$IMGACT` of `TYPE.EXE`, the same way |
| `DEFINE name equivalence` | `$CRELNM` in `LNM$PROCESS`, and `%DCL-I-SUPERSEDE` if it replaced one |
| `DEASSIGN name` | `$DELLNM` from `LNM$PROCESS` |
| `SHOW LOGICAL name` | `$TRNLNM` in `LNM$FILE_DEV`: `"name" = "equivalence" (table)`, or `%SHOW-S-NOTRAN` |
| `SHOW LOGICAL [*]` | lists every name, the process's table's and then the system's, in the order they were made, under each table's name; it copies them one at a time with `$CMKRNL`. ponytail: VMS's DCL asks the executive's logical name routines, and sorts them |
| `HELP` | lists the commands |
| `LOGOUT` | returns, which deletes the process |

Verbs may be abbreviated, the first that matches winning. DCL reads a line
from `SYS$INPUT` with `IO$_READPROMPT` and the prompt `$ `, and reports a failure status as VMS does one it has no text
for, `%NONAME-F-NOMSG, Message number 0000000C`, after
`%DCL-W-ACTIMAGE` if `$IMGACT` returned it. ponytail: no CTRL/Y, so an
image that never exits keeps the console; no message texts, symbols,
qualifiers, quoted strings or command procedures.

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
| `MWAIT` | the resource's own queue: `TTY$GQ_READQ` for a console read |

- **`SCH$SCHED`** takes the first process from the highest non-empty COM
  queue, makes it current and `SWPCTX`es to its HWPCB, unless it is the
  process already there. The process leaving waits inside its `SWPCTX`
  until the CPU comes back, so the switch saves no registers. A process
  that `$DELPRC` deleted meanwhile deletes itself when its `SWPCTX`
  returns. With no
  process computable, the CPU waits for an interrupt (`WTINT`) at
  `IPL$_RESCHED`, so that the software timer interrupt can make one
  computable.
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
2. **`EXE$SWTIMINT`**, at `IPL$_TIMER`, raises IPL to `IPL$_SYNCH`, calls
   `SCH$QEND` if the quantum is still up, and serves the timer queue
   ([Time](#time)).
3. **`SCH$QEND`** gives the process a new quantum and, if a COM queue at its
   priority or above isn't empty, requests the reschedule interrupt, which
   puts it on its queue's tail: round robin within a priority. Real-time
   processes, priority 16 and up, keep the CPU. ponytail: no priority decay
   toward the base, since nothing boosts a priority yet.

Code at `IPL$_TIMER` or above, the scheduler included, defers quantum end
until IPL drops.

## Time

The system time, `EXE$GQ_SYSTIME`, is VMS's: a quadword of 100 ns units
since 17-Nov-1858. `EXE$INITTIM` sets it from the RTC's seconds, which
the PAL puts in `RPB$L_BOOTTIME`, and each tick of `EXE$HWCLKINT` adds
10 ms. ponytail: `EMUL` is signed, so the RTC's seconds fit until 2038.
A time a service takes is a time, or, negative, a delta from now.

The timer queue, `EXE$GQ_TQFL`, holds timer queue entries (`$TQEDEF`,
48 bytes of pool) in the order they are due, at `IPL$_SYNCH`:

| Type | Made by | When due |
| --- | --- | --- |
| `TQE$C_TMSNGL` | `$SETIMR` | sets the event flag, waking the process if its wait is over |
| `TQE$C_WKSNGL` | `$SCHDWK` | `$WAKE`s the process |
| `TQE$C_WKREPT` | `$SCHDWK` with a repeat time | the same, and goes back on the queue, due one repeat time later |

- `EXE$GQ_1ST_TIME` holds when the first entry is due, or never.
  `EXE$HWCLKINT` compares it with the system time on each tick and, once
  it is due, requests the software timer interrupt. Both quadwords change
  at `IPL$_HWCLK`, so the handler never reads half of one.
- `EXE$SWTIMINT` takes off the queue each entry that is due and serves it
  for its process, found by PID, or frees it if the process is gone.
- `EXE$RMVTIMQ` removes a process's entries: `$CANTIM` its `$SETIMR`s,
  by request identifier or all; `$CANWAK` its wakeups, and a pending
  `$WAKE`; process rundown all of them.
- `$SETIMR` clears its flag first. An AST address is `SS$_ILLSER`, until
  there are ASTs. A `$SCHDWK` repeat time must be a delta of a tick or
  more, or it is `SS$_BADPARAM`.

## Synchronization

Inside the executive, IPL, as on a uniprocessor VMS. `DSBINT`, `ENBINT`,
`SETIPL` and `SOFTINT` in `lib.mlb` are VMS's macros:

| IPL | Protects |
| --- | --- |
| `IPL$_HWCLK` (24) | the interval timer interrupt runs here; the system time, `EXE$GQ_1ST_TIME` |
| `IPL$_CONSOLE` (20) | the console receive interrupt runs here; the type-ahead buffer |
| `IPL$_SYNCH` (8) | the scheduler's queues and PCBs, the PFN list, pool, common event blocks, the timer queue, the logical name tables, the console |
| `IPL$_TIMER` (7) | the software timer interrupt runs here |
| `IPL$_IOPOST` (4) | the software interrupt that wakes the console's readers runs here |
| `IPL$_RESCHED` (3) | the reschedule interrupt runs here; below it, the CPU may move |
| 0 | process code |

Code at or above `IPL$_SYNCH` keeps the CPU: only the interval timer
and console interrupts reach it, which take nothing away, and it only gives the CPU
away by calling `SCH$SCHED`. The console is shared, so a line, or a
`$QIO`'s buffer, is written at `IPL$_SYNCH`, or another process's could
come in the middle of it. The
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
  process is in `LEF`, or in `CEF` on the block's queue. `$SETEF` on a
  common flag, or a `$SETIMR` that is due, wakes those whose wait it
  satisfies. A woken process checks
  its flags again.
- `$HIBER` sleeps until `$WAKE`; a `$WAKE` that comes first makes the next
  `$HIBER` return at once.

## System services

A program calls `SYS$name` with `CALLS` or `CALLG`, or with the `$name_S`
macros of `starlet.mlb`, which push the arguments. `SYS$name` is a routine
in `syssrv.mar` that does `CHMK #code` and returns. The PAL delivers the
`CHMK` to `EXE$CMODKRNL`, in kernel mode, which checks the code and the
argument list (`SS$_ILLSER`, `SS$_INSFARG`, `SS$_ACCVIO`), calls `EXE$name`
with `CALLG` on the caller's argument list, and `REI`s with its status in
R0, back to the caller's mode. `$CMEXEC` does `CHME #0` instead, to
`EXE$CMODEXEC` in executive mode.

Programs run in user mode, so the `SYS$name` routines, `EXE$CMODEXEC` and
`EXE$USRSTART` are in a psect of their own, `EXEC$VECTOR`, page aligned,
between `EXE$VECTOR` and `EXE$VECTOREND`: the vector, which `EXEC$START`
makes user readable and executable, as VMS's system service vector is.
The rest of S0 is the kernel's.

A service checks every address its caller passes before using it, with
VMS's `IFNORD` and `IFNOWRT` macros, which `PROBE` the caller's mode, and
`EXE$PROBER_DSC` for string descriptors: one its mode can't read, or write
if the service writes it, is `SS$_ACCVIO`.

Programs link against `SYS.STB`, which `build.rs` makes from `EXEC.EXE`'s
map: every global symbol of the executive as a constant, as code on VMS
linked against `SYS.STB`. So they reach `SYS$name`, with `G^`, from P0 to
S0; the executive's other routines and data are there too, but user mode
can't reach them.

| Group | Implemented | Stubs: `SS$_ILLSER` |
| --- | --- | --- |
| Process control | `$CREPRC`, `$DELPRC`, `$EXIT`, `$HIBER`, `$WAKE`, `$SUSPND`, `$RESUME`, `$SETPRI`, `$SETPRN`, `$CMKRNL`, `$CMEXEC` | `$FORCEX`, `$GETJPI`, `$GETJPIW`, `$DCLEXH`, `$CANEXH`, `$SETPRV` |
| Event flags | `$ASCEFC`, `$DACEFC`, `$SETEF`, `$CLREF`, `$READEF`, `$WAITFR`, `$WFLOR`, `$WFLAND` | `$DLCEFC` |
| Memory | `$CRETVA`, `$DELTVA`, `$EXPREG` | `$CNTREG`, `$SETPRT`, `$LKWSET`, `$ULWSET`, `$LCKPAG`, `$ULKPAG`, `$CRMPSC`, `$MGBLSC` |
| Time | `$GETTIM`, `$SETIMR`, `$CANTIM` | |
| I/O | `$ASSIGN`, `$DASSGN`, `$QIO`, `$QIOW` | |
| Logical names | `$CRELNM`, `$DELLNM`, `$TRNLNM` | |
| Images | `$IMGACT` | |
| RMS | `$PARSE`, `$SEARCH`, `$OPEN`, `$CONNECT`, `$GET`, `$DISCONNECT`, `$CLOSE` | |
| Other | | `$DCLAST`, `$SETAST`, `$GETSYI` |

Arguments the implemented services take but ignore: `$CREPRC`'s
privileges, quotas, UIC, mailbox and status flags, the logical name
services' attributes, `$ASCEFC`'s protection
and permanence (every cluster is temporary), the access modes. Any process
may call `$CMKRNL` and `$CMEXEC`. ponytail: until there are privileges.

### Logical names

A logical name stands for a string, its equivalence, as on VMS: a
program opens `SYS$OUTPUT`, and the process says what that is. `lnm.mar`
keeps two logical name tables, queues of logical name blocks (`$LNMBDEF`,
from pool) holding a name and its equivalence, each up to 255 characters,
case sensitive:

| Table | Queue | Whose |
| --- | --- | --- |
| `LNM$PROCESS_TABLE` | `PCB$Q_LNMHD` | the process's; process rundown deletes its names |
| `LNM$SYSTEM_TABLE` | `LNM$GQ_SYSTEM` | every process's |

A service's `tabnam` is one of those, `LNM$PROCESS` or `LNM$SYSTEM` for
them, or `LNM$FILE_DEV`, the process's table and then the system's; any
other is `SS$_NOLOGTAB`. An item list is VMS's: 12-byte items, each a
buffer length and item code, words, a buffer address and the address of a
word for the length returned, and a longword 0 at the end (`$LNMDEF`).

- **`$CRELNM`** makes a name, in the process's table for `LNM$FILE_DEV`,
  with the equivalence of the `LNM$_STRING` item, the one item it takes.
  A name already there by that name goes: `SS$_SUPERSEDE`.
- **`$DELLNM`** deletes a name, from the process's table for
  `LNM$FILE_DEV`; `SS$_NOLOGNAM` if it isn't there.
- **`$TRNLNM`** looks a name up and fills in its items, as much of each
  as fits: `LNM$_STRING`, the equivalence; `LNM$_TABLE`, the table it was
  found in; `LNM$_LENGTH`, the equivalence's length. `SS$_NOLOGNAM` if no
  table has it. One level, as on VMS: the equivalence may be a logical
  name too.
- **Process-permanent names.** `$CREPRC`'s `input`, `output` and `error`
  are the equivalences of the new process's `SYS$INPUT`, `SYS$OUTPUT` and
  `SYS$ERROR`, those it was given. `EXEC$START` gives `SYSTEM` `_OPA0:` for
  all three, as `LOGINOUT` gives a terminal's process its terminal.

ponytail: one equivalence string per name, so no search lists; no access
modes, so no user-mode names that image rundown deletes and no names an
outer mode can't delete; no `LNM$JOB` or `LNM$GROUP`, no tables of one's
own, no directory tables, and `$DELLNM` without a name doesn't empty the
table. VMS's logical name directories, hash table and mutex replace this.

### I/O

The console is the one device. `$ASSIGN` gives a channel to `OPA0:`, one of
31, a bit in `PCB$L_CHANS`. It translates the device name it is given
first, as VMS does: without a colon at its end, in `LNM$FILE_DEV`, and
what that translates to, up to `LNM$C_MAXDEPTH`, 10, times, until a name
doesn't translate, or starts with an underscore, which says it is a
device's own name and is dropped. So `SYS$INPUT` is `_OPA0:`, the console. `$QIO` writes a buffer on it, for
`IO$_WRITEVBLK`, `IO$_WRITELBLK` and `IO$_WRITEPBLK`, or reads a line into
one, for `IO$_READVBLK`, `IO$_READLBLK`, `IO$_READPBLK`, and
`IO$_READPROMPT`, which writes the prompt in p5 and p6 first. Then it sets
the event flag and the I/O status block: the status, the byte count and,
for a read, the terminator, a carriage return. So the I/O is done when
`$QIO` returns, and `$QIOW` is `$QIO`.

- **Receiving.** `TTY$RCVINT`, the console receive interrupt
  (DESIGN-0001), puts each character in `TTY$AB_RING`, the 256-byte
  type-ahead buffer, or drops it if the buffer is full, and requests the
  `IPL$_IOPOST` software interrupt. `TTY$IOPOST` ends the wait of each
  process in `TTY$GQ_READQ`.
- **Reading**, at `IPL$_SYNCH`: takes characters from the buffer, at
  `IPL$_CONSOLE`, and echoes them, up to a carriage return, echoed as
  CR LF. DEL and BS erase a character, CTRL/U the line; other control
  characters, and those past the buffer's size, are dropped. While the
  buffer is empty the process waits in `MWAIT` on `TTY$GQ_READQ`. Echo is
  the reader's, so what is typed ahead shows when it is read.

ponytail: a buffer at a time, at `IPL$_SYNCH`, so lines don't mix; a
terminal driver with I/O request packets, CTRL/Y and escape sequences
replaces it.

### Files

The system disk, `DKA0:`, is a Files-11 ODS-2 volume, which the PAL reads
a block at a time with `READLBLK` (DESIGN-0001, *The system disk*).
`f11.mar` reads Files-11 (`ods/docs/`) as VMS's XQP does, read only:

- **`FIL$MOUNT`**, at boot, reads the home block at LBN 1, checks its
  format, `DECFILE11B`, keeps where file headers start in the index file
  and its label, and reads the index file's header, through whose map it
  finds every other header.
- **`FIL$READHDR`** reads file number n's header, VBN
  `IBMAPVBN + IBMAPSIZE + n - 1` of the index file, and checks its
  checksum and number. **`FIL$MAPVBN`** finds a VBN's LBN in a header's
  map, retrieval pointers of formats 1 to 3, and **`FIL$READVBN`** reads a
  file's blocks, an extent at a time. ponytail: no extension headers.
- **`FIL$SEARCHDIR`** reads a directory's blocks up to its end of file and
  finds the next record whose name, `NAME.TYPE`, matches a pattern with
  `*` and `%`, and the version asked for: any, the highest (the first of
  the name's record) or one. It can skip matches, which is how `$SEARCH`
  goes on from where it was.
- **`FIL$OPENFILE`** finds an image and reads it whole into pool, for the
  image activator, which frees it once the sections are copied.

RMS (`rms.mar`) is a set of system services on VMS's FAB, RAB and NAM
blocks. A file specification is `[DKA0:][[dir.dir]]name.type;version`.
`RMS$PARSE` splits it, and the FAB's default specification, and
`DKA0:[SYSMGR]`, the console process's default directory, into device,
directory, name, type and version, takes each part from the first that
has it, in capitals, into the expanded specification, checks it, and walks
the directory from the MFD, each name `NAME.DIR;1` in the one before
(`[000000]` is the MFD). The images' default is `DKA0:[SYSEXE]` instead.

| Service | Does |
| --- | --- |
| `$PARSE fab` | the expanded string, its parts and the directory's ID into the FAB's NAM block |
| `$SEARCH fab` | the next file the NAM block's expanded string names: its resultant string, parts and file ID; `RMS$_FNF` if there is none, then `RMS$_NMF` |
| `$OPEN fab` | opens one file, the highest version unless the specification gives one, for reading; `RMS$_WLK` for writing. Its IFI, record format, attributes, maximum record size and allocation into the FAB, its resultant string into the NAM block if there is one |
| `$CONNECT rab` | connects the RAB to the file its FAB opened, at its start |
| `$GET rab` | the next record into the RAB's user buffer: `RAB$W_RSZ`, `RAB$L_RBF`; `RMS$_RTB` if it didn't fit, `RMS$_EOF` past the end; VAR and FIX records only |
| `$DISCONNECT rab`, `$CLOSE fab` | undo `$CONNECT` and `$OPEN` |

An open file is an IFAB, 1,040 bytes of pool: the file's header, the block
`$GET` reads in, and where it is. The PCB holds up to 15, by IFI, in
`PCB$A_IFAB`, a bit each in `PCB$L_FILES`; `$CONNECT` puts the IFI in
`RAB$W_ISI` too. `RMS$RUNDOWN` closes a process's files at image exit
and process deletion.

The services, and `FIL$OPENFILE`, run in kernel mode at `IPL$_SYNCH`, one
at a time, which keeps the file system's buffers theirs. They return
VMS's `RMS$_` statuses (`$RMSDEF`) and put them in `FAB$L_STS` or
`RAB$L_STS`. ponytail: VMS's RMS runs in executive mode; no ASTs,
completion routines, logical names, wildcard directories or block I/O.

### Exceptions

An access violation or a reserved instruction in an outer mode reaches
the executive through the SCB (DESIGN-0001, *Exceptions*).
`EXE$ACVIOLAT` and `EXE$OPCDEC` report it on the console and `$EXIT` with
`SS$_ACCVIO` or `SS$_OPCDEC`, as VMS does for an image with no condition
handler. That ends the process, or, under DCL, the image:

```
%SYSTEM-F-ACCVIO, access violation, virtual address 40010000, PC 00010020, process SNOOP
%EXEC-W-EXITED, process SNOOP exited with status 0000000C
```

```
$ RUN SNOOP
%SYSTEM-F-ACCVIO, access violation, virtual address 40010000, PC 00010020, process SYSTEM
%NONAME-F-NOMSG, Message number 0000000C
```

## The system disk's programs

`roottask/sysexe/` holds the programs in `DKA0:[SYSEXE]`: DCL, DIRECTORY,
TYPE, and those which show the services at work, which `just check` runs
from DCL's prompt (`RUN STARTUP`, `RUN SNOOP`, a bad verb,
`DIR [SYSEXE]P%NG` and `TYPE WELCOME.TXT`) and to the end.
`roottask/sysmgr/` holds the text files in `DKA0:[SYSMGR]`. They
run in user mode, DCL in supervisor mode, and write on the console with
`PRINT` and `PRINTHEX` from `sysexe.mlb`, which call `PUT_LINE` in
`sysexe/lib/print.mar`, linked into each: a line at a time on `OPA0:`,
with `$QIOW`.

| Program | Does |
| --- | --- |
| `DCL` | the command interpreter (*The command interpreter*) |
| `DIRECTORY` | `$PARSE`s its command line, with `*.*;*` for what it leaves out, and lists the files `$SEARCH` finds: the directory, the names four to a line, how many |
| `TYPE` | `$OPEN`s the file its command line names and writes each record `$GET` reads on the console, a line each |
| `STARTUP` | makes 4 pages with `$EXPREG`, checks and deletes them; creates `SLEEPER` at a higher priority, which runs at once, and `PING` and `PONG`; waits until `PONG` sets flag 66 of their cluster; deletes `SLEEPER`; creates `SVCTEST`, `HOG` and `TIMETEST` |
| `SLEEPER` | hibernates until it is deleted |
| `PING`, `PONG` | take three turns through common event flags 64 and 65 of the cluster `PINGPONG`; `PONG` then sets flag 66, which `STARTUP` waits for |
| `SVCTEST` | checks the statuses of the services the others don't use, and of errors: local event flags, the dispatcher's checks and a stub, `$CRETVA` and `$DELTVA`, `$CMKRNL` and `$CMEXEC`; what user mode may `PROBE`, and that services refuse it the executive's data; the console's channels; logical names in both tables, `$ASSIGN` through two of them, and the errors; `$SETPRI`, and `$SUSPND`, `$WAKE`, `$RESUME` and `$DELPRC` on a process of its own; then creates one whose image doesn't exist, which exits with `RMS$_FNF`, and `SNOOP` and `USURP` |
| `SNOOP` | reads S0 from user mode, and exits with `SS$_ACCVIO` |
| `USURP` | raises IPL from user mode, and exits with `SS$_OPCDEC` |
| `HOG` | associates a common event flag cluster, creates `NUDGE` at its own priority and loops reading flag 64 until `NUDGE` sets it, with no wait: only quantum end lets `NUDGE` run |
| `NUDGE` | sets `HOG`'s flag |
| `TIMETEST` | checks that `$GETTIM` reads a time after 2026; waits for `$SETIMR`s, a delta and a time, 50 ms on, and that a cancelled one never sets its flag; hibernates through three repeating `$SCHDWK` wakeups, cancels them, and checks that the next wakeup is a new one's |

When every process but the swapper waits, the CPU idles in `WTINT`,
taking the clock's interrupts.

## Next

- Priority boosts on wake and decay at quantum end.
- ASTs: `$DCLAST`, AST delivery at `IPL$_ASTDEL`, and with them process
  deletion by a kernel AST.
- Privileges, for `$CMKRNL` and `$CMEXEC`, and condition handlers in place
  of exiting on an exception.
- CTRL/Y, to take the console back from an image, and `$FORCEX`.
- Writing the system disk, the disk's interrupt, `$QIO` on disk
  channels, logical names in file specifications (`SYS$SYSTEM:DCL.EXE`)
  and `SET DEFAULT`.
- Access modes and search lists for logical names.
