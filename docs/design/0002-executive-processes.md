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
Files-11 volume, which the executive reads by LBN, with RMS on top, and
[ADR-0009](../adr/0009-ramdisk-writable-files-11.md): a ramdisk, `MDA0:`,
holds a Files-11 volume the executive writes, and
[ADR-0012](../adr/0012-data-disk-writable-files-11.md): so does a second
virtio disk, `DKB0:`, which the PAL writes, and
[ADR-0011](../adr/0011-asts-on-the-kernel-stack.md): ASTs are delivered
when the PAL asks, on top of the kernel stack, and
[ADR-0013](../adr/0013-qio-irps-and-drivers.md): `$QIO` queues an I/O
request packet to the device's driver, and its completion is a kernel
mode AST.

The executive borrows VMS's structure and names (PCB, `SCH$`, `MMG$`,
`EXE$` routines, `SS$_` codes, the system service interfaces) but none of
its code.

## Modules

| File | What |
| --- | --- |
| `exec.mar` | `EXEC$START`, the swapper, `CON$PUTCHAR` |
| `memory.mar` | the PFN list, pages, nonpaged pool, `$CRETVA`, `$DELTVA`, `$EXPREG` |
| `sched.mar` | state queues, `SCH$SCHED`, waits and wakes, the reschedule interrupt, quantum end |
| `astdel.mar` | AST queues, `SCH$QAST`, the AST delivery interrupt, `$DCLAST`, `$SETAST`, `$ASTEXIT` |
| `timeschdl.mar` | the interval timer and software timer interrupts, the system time, the timer queue, `$GETTIM`, `$SETIMR`, `$CANTIM`, `$SCHDWK`, `$CANWAK` |
| `event.mar` | event flags, local and common |
| `process.mar` | `$CREPRC`, process start, image activation, `$IMGACT`, `$EXIT`, image rundown, deletion, `$FORCEX`, `$HIBER`, `$WAKE`, `$SUSPND`, `$RESUME`, `$SETPRI`, `$SETPRN`, `$CMKRNL` |
| `lnm.mar` | logical name tables, `$CRELNM`, `$DELLNM`, `$TRNLNM` |
| `qio.mar` | the devices' UCBs, `$ASSIGN`, `$DASSGN`, `$CANCEL`, `$QIO`, `$QIOW`; IRPs, their completion and cancelling |
| `ttdriver.mar` | the console's terminal driver: writes, queued reads and their editing, the console receive interrupt and the type-ahead buffer, CTRL/C and CTRL/Y ASTs |
| `getdvi.mar` | `$GETDVI`, `$GETDVIW`, `$DEVICE_SCAN`: what the devices are |
| `syssrv.mar` | the system service vector, the `CHMK` and `CHME` dispatchers, `$CMEXEC`, where processes enter user and supervisor mode, the exception handlers and the stubs |
| `f11.mar` | Files-11 ODS-2 volumes: the disks' VCBs, reading and writing their blocks, `FIL$MOUNT` and `$MOUNT`, headers, maps, directories, `FIL$OPENFILE` for the image activator |
| `f11wrt.mar` | Files-11 writes: headers, blocks, directory entries, `FIL$INIT` and `$INIT_VOL` |
| `mddriver.mar` | `MDA0:`, the ramdisk |
| `rms.mar` | RMS: file specifications, `$PARSE`, `$SEARCH`, `$OPEN`, `$CREATE`, `$CONNECT`, `$GET`, `$PUT`, `$DISCONNECT`, `$CLOSE`, `$ERASE` |

`roottask/build.rs` links them, with `vtools/lib/consolio.mar`, into
`EXEC.EXE`, in S0 at `0x40010000`. The structures are in `vtools/lib/lib.mlb` (`$PCBDEF`,
`$CEBDEF`, `$PTEDEF`, `$RPBDEF`, `$VCBDEF`...), what programs need in
`vtools/lib/starlet.mlb` (`$SSDEF`, `$PRTDEF`, the `$name_S` macros, RMS's
`$FABDEF`, `$RABDEF`, `$NAMDEF`, `$RMSDEF` and the `$FAB`, `$RAB`, `$NAM`
blocks and `$OPEN`... calls, `$MNTDEF`).

## Start

`EXEC$START` runs in the boot context at IPL 31:

1. Saves the RPB address from R11.
2. Fills the SCB: reserved instructions to `EXE$OPCDEC`, access violations
   to `EXE$ACVIOLAT`, `CHMK` to `EXE$CMODKRNL`, `CHME` to `EXE$CMODEXEC`,
   software interrupt level 2 to `SCH$ASTDEL`, level 3 to `SCH$RESCHED`, level 4 to `TTY$IOPOST`,
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
   executive halts with `%EXEC-F-NOMOUNT` and the status. Then it mounts
   the data disk, `DKB0:`, if it holds a volume, and goes on if it doesn't.
   Then
   `LNM$CREATE` puts the system's logical names in `LNM$SYSTEM_TABLE`
   (*Logical names*).
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
| `0x50000000` | `MDA0:`'s blocks, 512 KB, once `INITIALIZE` makes them |
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

A process is a PCB (`$PCBDEF`, 440 bytes, from pool), a 16 KB kernel stack
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
   disk, in `SYS$SYSTEM:` unless its name says where, into pool
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
  so it deletes itself, in a kernel mode AST (*ASTs*) that runs it down as
  above. At `IPL$_SYNCH` `$DELPRC` queues the AST, which ends its wait or
  suspension, and the process leaves its slot, so that no one finds it
  any more. It gets the AST the next time it runs at an IPL below
  `IPL$_ASTDEL`, or ends a wait: at once if it never ran.

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
- **CTRL/Y** ([ADR-0014](../adr/0014-ctrlc-ctrly-asts.md)) is the
  command interpreter's supervisor mode AST, which it enables on the
  console with `IO$_SETMODE` (*I/O*). Delivered on top of the image, in
  user mode or in a wait, as any AST (*ASTs*), it leaves the image's
  registers and the service it was in on the kernel stack, and the command
  interpreter prompts inside it. Returning from the AST routine goes back
  to the image. `$IMGACT` while one is stopped runs it down, which forgets
  the AST routine, and keeps `PCB$L_CLICHANS` and `PCB$L_CLIFILES` as they
  were when it ran; it empties the supervisor stack too, by setting the
  HWPCB's `SSP` to its top.
- **`$FORCEX pidadr, prcnam, code`** makes a process's image exit: it
  queues a user mode AST whose routine is `SYS$EXIT`, with code, or
  `SS$_FORCEDEXIT` for 0, which comes when the image next runs in user
  mode, and ends a wait. `$EXIT` then does what it does for any image.

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
| `SET DEFAULT [dev:][dir]` | `$PARSE`s it, which must name no file, `$SETDDIR` with the directory it expands to, and `$CRELNM` of `SYS$DISK` in `LNM$PROCESS` with its device; one that doesn't exist is still set, after `%DCL-I-INVDEF` |
| `SHOW DEFAULT` | the device and directory `$PARSE` expands an empty specification to |
| `EDIT spec` | `$IMGACT` of `EDIT.EXE`, the same way |
| `COPY from to` | `$IMGACT` of `COPY.EXE`, with the two, a blank between |
| `DELETE spec` | `$IMGACT` of `DELETE.EXE` |
| `INITIALIZE device label` | `$IMGACT` of `INIT.EXE` |
| `MOUNT device label` | `$IMGACT` of `MOUNT.EXE` |
| `CONTINUE` | returns from the CTRL/Y AST, which goes back to the image |
| `HELP` | lists the commands |
| `LOGOUT` | `$DELPRC` |
| `@file [p1 ... p8]` | reads `file.COM` with RMS and takes its `$` lines as commands |
| `name = expression`, `name := string` | sets a symbol, `==` and `:==` a global one |
| `IF`, `GOTO`, `EXIT`, `WRITE SYS$OUTPUT` | as in VMS's procedures |
| `SHOW SYMBOL name`, `DELETE/SYMBOL name` | shows a symbol, deletes a local one |

Verbs may be abbreviated, the first that matches winning. A command
with fewer parameters than it needs is `%DCL-W-INSFPRM`. DCL reads a line
from `SYS$INPUT` with `IO$_READPROMPT` and the prompt `$ `, and reports a
failure status with its message, `%RMS-E-DNF, directory not found`, VMS's
text from a table in DCL of the file system's, RMS's and the volume
services' statuses, or as VMS does one it has no text for,
`%NONAME-F-NOMSG, Message number 0000000C`, after
`%DCL-W-ACTIMAGE` if `$IMGACT` returned it, unless the status has
`STS$M_INHIB_MSG`, bit 28, set: the image reported it.

DCL enables its CTRL/Y AST, `CTRLY`, when it starts, before each image it
runs and when it continues one, and keeps whether an image runs. With
one, the AST ends the procedures running, keeps its frame pointer and
goes to the command loop, where `CONTINUE` enables the AST again and
returns from it; a command that runs another image runs the stopped one
down. With none, the AST enables itself again, and DCL ends its
procedures at the next command; the read CTRL/Y ended gives it an empty
line. ponytail:
message texts in DCL rather than message files and `$GETMSG`; no
qualifiers, and no `STOP`: another command that runs an image ends the one CTRL/Y
stopped.

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
  until the CPU comes back, so the switch saves no registers. A process
  that `$DELPRC` deleted meanwhile deletes itself when its `SWPCTX`
  returns. With no
  process computable, the CPU waits for an interrupt (`WTINT`) at
  `IPL$_RESCHED`, so that the software timer interrupt can make one
  computable.
- **`SCH$WAIT`** puts the current process on a wait queue and calls
  `SCH$SCHED`. It returns when the wait is over and the CPU is back. An
  AST ends the wait early, so its callers look again at what they wait
  for. It delivers the ASTs the mode that called the service
  waiting lets through (*ASTs*).
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
| `TQE$C_TMSNGL` | `$SETIMR` | sets the event flag, waking the process if its wait is over, and, with an AST, becomes its ACB |
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
- `$SETIMR` clears its flag first. With an AST address, the entry, when
  it is due, goes on the process's AST queue as its ACB, for the mode
  that called `$SETIMR`, with `reqidt` as its parameter. A `$SCHDWK` repeat time must be a delta of a tick or
  more, or it is `SS$_BADPARAM`.

## Synchronization

Inside the executive, IPL, as on a uniprocessor VMS. `DSBINT`, `ENBINT`,
`SETIPL` and `SOFTINT` in `lib.mlb` are VMS's macros:

| IPL | Protects |
| --- | --- |
| `IPL$_HWCLK` (24) | the interval timer interrupt runs here; the system time, `EXE$GQ_1ST_TIME` |
| `IPL$_CONSOLE` (20) | the console receive interrupt runs here; the type-ahead buffer |
| `IPL$_SYNCH` (8) | the scheduler's queues and PCBs, the PFN list, pool, common event blocks, the timer queue, the logical name tables, the UCBs and their IRPs, the console |
| `IPL$_TIMER` (7) | the software timer interrupt runs here |
| `IPL$_IOPOST` (4) | the software interrupt that gives typed characters to the console's reads runs here |
| `IPL$_RESCHED` (3) | the reschedule interrupt runs here; below it, the CPU may move |
| `IPL$_ASTDEL` (2) | the AST delivery interrupt runs here; below it, kernel mode ASTs come |
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
  `$HIBER` return at once. `$WAKE` sets `PCB$V_WAKEPEN` and `$HIBER` takes
  it, so a hibernation that ends without one goes on.

### ASTs

An AST is a call a process gets in one of its access modes, on top of
whatever it was doing ([ADR-0011](../adr/0011-asts-on-the-kernel-stack.md)).
An AST control block (`$ACBDEF`, 32 bytes of pool) names the process, the
mode, and the AST routine and its parameter, or, for a kernel mode AST,
an executive routine, `ACB$L_KAST`.

- **`SCH$QAST`** puts an ACB on the process's queue, `PCB$Q_ASTQFL`,
  after those of its mode and the inner ones, and ends its wait, if it
  waits. It keeps the HWPCB's two bytes the PAL reads (DESIGN-0001,
  *Interrupts and exceptions*): `ASTSR`, the modes with an ACB queued, and
  `ASTEN`, those whose ASTs `$SETAST` enabled (`PCB$B_ASTEN`) less those
  whose AST routine runs (`PCB$B_ASTACT`).
- **Delivery.** When IPL is below `IPL$_ASTDEL` and a mode set in both is
  the current mode or an inner one, the PAL requests the level 2 software
  interrupt, `SCH$ASTDEL`, which delivers in the mode it interrupted.
  `SCH$WAIT` delivers too, when a wait ends, in the mode that called the
  service waiting, which `RD_PS` reads from the PSL. `EXE$ASTDEL` takes
  the first ACB that mode lets through, kernel mode's first. It `JSB`s an
  executive routine at `IPL$_SYNCH`, which frees the ACB. For an AST
  routine, it frees the ACB, marks the mode's AST active, pushes the
  registers and IPL on the kernel stack, links them to the AST routine
  before it in `PCB$L_ASTSP`, and `REI`s below them to `EXE$ASTDISP`, in
  the vector, in the AST's mode at IPL 0. That mode's stack is the one in
  the HWPCB, which the PAL left there when it entered kernel mode.
  `EXE$ASTDISP` calls the routine, `CALLS #1`, with the parameter.
- **`$ASTEXIT`**, which `EXE$ASTDISP` calls when the routine returns,
  puts the kernel stack pointer back from `PCB$L_ASTSP`, and `EXE$ASTDEL`
  goes on: the next AST, or a return to what the AST interrupted, every
  register as it was.
- **Rundown.** Image rundown frees the user mode ACBs, enables user mode
  ASTs again and forgets the AST routines that run, whose state goes with
  the kernel stack. Process rundown frees the rest.

| Service | Does |
| --- | --- |
| `$DCLAST astadr, astprm, acmode` | queues an AST to the current process, in `acmode` or the caller's mode, whichever is the outer one |
| `$SETAST enbflg` | enables or disables the caller's mode's ASTs: `SS$_WASSET` or `SS$_WASCLR` |
| `$ASTEXIT` | ends the AST routine that runs; with none, does nothing |

`$SETIMR`'s AST comes from its timer queue entry (*Time*), `$DELPRC`'s
kernel mode AST deletes the process (*Deletion*), `$FORCEX`'s user mode
one calls `$EXIT` (*The command interpreter*), and CTRL/C and CTRL/Y are
the ASTs a terminal channel enabled (*I/O*). ponytail: the routine
gets its parameter only, not VMS's R0, R1, PC and PSL after it; no AST
quotas.

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
| Process control | `$CREPRC`, `$DELPRC`, `$EXIT`, `$FORCEX`, `$HIBER`, `$WAKE`, `$SUSPND`, `$RESUME`, `$SETPRI`, `$SETPRN`, `$GETJPI`, `$GETJPIW`, `$CMKRNL`, `$CMEXEC` | `$DCLEXH`, `$CANEXH`, `$SETPRV` |
| Event flags | `$ASCEFC`, `$DACEFC`, `$SETEF`, `$CLREF`, `$READEF`, `$WAITFR`, `$WFLOR`, `$WFLAND` | `$DLCEFC` |
| Memory | `$CRETVA`, `$DELTVA`, `$EXPREG` | `$CNTREG`, `$SETPRT`, `$LKWSET`, `$ULWSET`, `$LCKPAG`, `$ULKPAG`, `$CRMPSC`, `$MGBLSC` |
| Time | `$GETTIM`, `$SETIMR`, `$CANTIM` | |
| I/O | `$ASSIGN`, `$DASSGN`, `$CANCEL`, `$QIO`, `$QIOW` | |
| Logical names | `$CRELNM`, `$DELLNM`, `$TRNLNM` | |
| Images | `$IMGACT` | |
| RMS | `$PARSE`, `$SEARCH`, `$OPEN`, `$CREATE`, `$CONNECT`, `$GET`, `$PUT`, `$DISCONNECT`, `$CLOSE`, `$ERASE`, `$SETDDIR` | |
| Volumes | `$MOUNT`, `$INIT_VOL` | |
| ASTs | `$DCLAST`, `$SETAST`, `$ASTEXIT` | |
| Other | `$GETSYI`, `$GETSYIW` | |

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
- **System names.** `EXEC$START` makes `SYS$SYSDEVICE`, `DKA0:`;
  `SYS$DISK`, `SYS$SYSDEVICE:`, the default device, which `SET DEFAULT`
  gives a process one of its own of; and `SYS$SYSTEM`,
  `SYS$SYSDEVICE:[SYSEXE]`, where the images are. ponytail: VMS's
  `SYS$SYSTEM` is `SYS$SYSROOT:[SYSEXE]`, a rooted directory in
  `[SYS0.]`, and `LOGINOUT` defines each process's `SYS$DISK`.

ponytail: one equivalence string per name, so no search lists; no access
modes, so no user-mode names that image rundown deletes and no names an
outer mode can't delete; no `LNM$JOB` or `LNM$GROUP`, no tables of one's
own, no directory tables, and `$DELLNM` without a name doesn't empty the
table. VMS's logical name directories, hash table and mutex replace this.

### I/O

[ADR-0013](../adr/0013-qio-irps-and-drivers.md): a request goes to the
device's driver in an I/O request packet, and comes back to the process
as a kernel mode AST.

- **Devices.** Each has a unit control block (`$UCBDEF`) in `qio.mar`,
  `IOC$AB_UCB`: `OPA0:`, `DKA0:`, `DKB0:`, `MDA0:`. A UCB holds a queue of
  IRPs waiting for the device, its driver's FDT routine, a disk's VCB, and
  a disk driver's start I/O routine, which the file system calls too
  ([ADR-0015](../adr/0015-file-system-io-through-the-disk-driver.md)).
- **Channels.** `$ASSIGN` gives one of 31, a bit in `PCB$L_CHANS`, with
  the device's UCB in `PCB$AL_CCB`. It translates the device name it is
  given first, as VMS does: without a colon at its end, in
  `LNM$FILE_DEV`, and what that translates to, up to `LNM$C_MAXDEPTH`,
  10, times, until a name doesn't translate, or starts with an
  underscore, which says it is a device's own name and is dropped. So
  `SYS$INPUT` is `_OPA0:`, the console. `$DASSGN` cancels the channel's
  requests and gives it back.
- **`$QIO`** checks the channel and that the caller can write the IOSB,
  clears the event flag, and calls the FDT routine at `IPL$_SYNCH` with
  its arguments. That checks the function and its parameters, a failure
  being `$QIO`'s status, makes the IRP (`$IRPDEF`, `IOC$ALLOCIRP`, from
  pool, up to 64 KB with a buffered read's prompt and data), and does
  the I/O or queues it.
- **Completion.** A driver ends a request with `IOC$REQCOM`, which puts
  the IOSB's two longwords in the IRP and queues it, whose head is an
  ACB's, to the process as a kernel mode AST, `IOC$POST`. In the process,
  that copies a buffered read's data to the caller's buffer, writes the
  IOSB and sets the event flag. If the caller gave an AST, the IRP goes
  on the AST queue again as an ACB for it, in the caller's mode, with
  `astprm`; else it is freed.
- **`$QIOW`** is `$QIO`, then `$WAITFR` on the event flag, until the
  IOSB, if there is one, has a status: a flag set before that, for
  something else, is cleared and waited for again, as VMS's `$SYNCH`
  does.
- **Cancelling.** `IOC$CANCEL` takes a process's IRPs for some of its
  channels off the UCBs, and its CTRL/C and CTRL/Y ASTs off the console's
  queues. `$CANCEL` completes the requests with `SS$_ABORT`, and leaves
  those done already. Rundown and `$DASSGN` free them unfinished, from
  its AST queue too: image rundown cancels the image's channels before
  its P0 goes, process rundown all of them, `$DASSGN` the one.

The console's driver, `ttdriver.mar`, `TT$FDT`:

- **Writing**, for `IO$_WRITEVBLK`, `IO$_WRITELBLK` and `IO$_WRITEPBLK`,
  is done in the FDT routine, from the caller's buffer, at `IPL$_SYNCH`,
  so lines don't mix; then the request is completed with the byte count.
  A write doesn't wait behind a read.
- **Reading**, for `IO$_READVBLK`, `IO$_READLBLK`, `IO$_READPBLK`, and
  `IO$_READPROMPT`, whose prompt is in p5 and p6, queues the IRP, with
  the prompt copied in, on `OPA0:`'s UCB. The read at the head writes its
  prompt and takes the characters typed, one at a time, editing the line
  in the IRP; when it ends it is completed, with the line's length and,
  in the IOSB's second longword, its terminator, a carriage return or
  CTRL/Z, or none if the buffer filled, and the next read starts.
- **Receiving.** `TTY$RCVINT`, the console receive interrupt
  (DESIGN-0001), puts each character in `TTY$AB_RING`, the 256-byte
  type-ahead buffer, or drops it if the buffer is full, and requests the
  `IPL$_IOPOST` software interrupt. `TTY$IOPOST` gives the buffer's
  characters to the read at the head, at `IPL$_SYNCH`. CTRL/C and CTRL/Y
  empty the buffer instead, and `TTY$IOPOST` delivers their ASTs.
- **CTRL/C and CTRL/Y ASTs.** `IO$_SETMODE` with `IO$M_CTRLCAST` or
  `IO$M_CTRLYAST` puts the AST p1 names, with p2 its parameter, in p3's
  mode or the caller's, the outer one, in an IRP on `TTY$Q_CTRLC` or
  `TTY$Q_CTRLY`, in place of the one the channel had; p1 = 0 only takes
  that off. `TTY$IOPOST` echoes `*CANCEL*` or `*INTERRUPT*` and queues
  every AST on the key's queue, which takes it off: one enable, one AST.
  CTRL/Y first completes the console reads of each process it goes to
  with `SS$_CONTROLY`, so that DCL's own read isn't queued behind the
  image's. CTRL/C with no AST is CTRL/Y; CTRL/Y with none does nothing.
- **Editing**: each character is echoed, up to a carriage return, echoed
  as CR LF, or a CTRL/Z, echoed as `*EXIT*`. The line is edited as on
  VMS, in insert mode: the left and right arrows move the cursor, CTRL/H
  to the start and CTRL/E to the end, DEL erases the character before the
  cursor and CTRL/U all of them. The up arrow or CTRL/B recalls the line
  read before, and again the one before that, up to 16, from
  `TTY$AB_RECALL`, and the down arrow goes back. Other control characters
  and escape sequences are dropped, and the buffer's last character ends
  the read too. With `IO$M_NOECHO` nothing is echoed or recalled, and with
  `IO$M_NOFILTR` every character but a carriage return goes in the buffer
  as it is, so a read of one byte reads a key, as EDT's keypad mode does,
  escape sequences a character at a time. Echo is the read's, so what is
  typed ahead shows when a read takes it.

The disks' driver, in `f11.mar`, reads the blocks from LBN p3 into the
p2 bytes at p1, for `IO$_READLBLK` and `IO$_READPBLK`, or writes them
there, for `IO$_WRITELBLK` and `IO$_WRITEPBLK`. Its FDT routine,
`DK$FDT`, checks the function and the buffer and makes the IRP, with the
LBN in `IRP$L_MEDIA`; its start I/O routine, `DK$STARTIO`, does the I/O
at once, calling the PAL with the VCB's unit, or `MD$IO` for the ramdisk,
and completes the request with the status, and the byte count if it is a
success: `SS$_WRITLCK` for `DKA0:`, `SS$_MEDOFL` for `MDA0:` until it is
made. The file system's own reads and writes go to `DK$STARTIO` too
(*Files*).

ponytail: the console's line being read is kept in `ttdriver.mar`, not
its UCB, since there is one terminal; output waits for the console at
`IPL$_SYNCH`, and a write in the middle of a line being read doesn't
redisplay it. The recall buffer is the console's, shared by every
reader, where VMS has DCL's own, with `RECALL`. No quotas, and
`IO$_SETMODE` sets no terminal characteristics.

### Devices

The devices are the disks, each a VCB (*Files*), and the console,
`OPA0:`, numbered in that order: `DKA0:`, `DKB0:`, `MDA0:`, `OPA0:`.
`getdvi.mar` answers what they are, with VMS's services, item codes and
bits (`$DVIDEF`, `$DVSDEF`, `$DCDEF`, `$DEVDEF` in `starlet.mlb`):

- **`$DEVICE_SCAN return_devnam, retlen, search_devnam, itmlst, contxt`**
  returns the next device's name, `_DDCU:`, whose name matches
  `search_devnam`, with `*` and `%` (`FIL$MATCH`), and whose class is
  `DVS$_DEVCLASS`'s, `DC$_DISK` or `DC$_TERM`, if the item list has it.
  `contxt`, a quadword, 0 at first, keeps the next device's number.
  `SS$_NOMOREDEV` after the last.
- **`$GETDVI efn, chan, devnam, itmlst, iosb, astadr, astprm, nullarg`**
  finds the device `devnam` names, translated as `$ASSIGN` translates it
  (`IOC$TRNDEVNAM`), or else `chan`'s, the console, and returns the items
  asked for: `DVI$_DEVCHAR` (`DEV$M_FOD`, `DIR`, `SHR`, `AVL`, `IDV`,
  `ODV`, `RND` for a disk, with `MNT` once mounted and `SWL` too if read
  only; `REC`, `CCL`, `TRM`, `AVL`, `IDV`, `ODV` for the console),
  `DEVCLASS`, `UNIT` and `ERRCNT` (0), `DEVNAM`, `VOLNAM`, `FREEBLOCKS`
  (`FIL$FREEBLOCKS`), `CLUSTER`, `MOUNTCNT`, the `AVL`, `MNT` and `SWL`
  bits, and `STS`, `UCB$M_ONLINE` if the device is there: the console, a
  mounted disk, the ramdisk once made, a PAL disk whose first block
  reads. Others are `SS$_BADPARAM`. It clears and sets the event flag and
  fills the IOSB, done at once; `$GETDVIW` is `$GETDVI`.

DCL's `SHOW DEVICES` scans the disks, then the terminals, and asks
`$GETDVIW` about each. ponytail: a VCB, or none for the console, stands
for the device rather than its UCB, which has no error, operation or
reference counts, device type or `MAXBLOCK` yet; no ASTs.

### Files

There are three disks, each with a VCB (`$VCBDEF`) in `f11.mar`: the
system disk, `DKA0:`, a Files-11 ODS-2 volume which the PAL reads a block
at a time with `READLBLK` (DESIGN-0001, *The disks*); the data disk,
`DKB0:`, a disk image of the host's, which the PAL also writes, with
`WRITELBLK`, and which keeps its volume from boot to boot; and `MDA0:`, a
ramdisk (`mddriver.mar`), as DECram's: 1,024 blocks in pages of S0, which
`$INIT_VOL` makes, zeroed, and which last until the system stops.
`FIL$READLBLK` and `FIL$WRITELBLK` read and write `F11$GL_VCB`'s disk
through its driver, as VMS's XQP does
([ADR-0015](../adr/0015-file-system-io-through-the-disk-driver.md)):
they fill in `F11$AB_IRP`, the file system's own IRP, with no process,
and call the start I/O routine of the VCB's UCB, `VCB$L_UCB`. The driver
completes it with `IOC$REQCOM`, which, for an IRP with no process, only
leaves the status in it. ponytail: the file system doesn't wait; the
disk's driver is done when it returns. `FIL$SELECT` picks the
VCB by device name. `f11.mar` reads Files-11 (`ods/docs/`) as VMS's XQP
does:

- **`FIL$MOUNT`**, at boot for `DKA0:` and `DKB0:` and from `$MOUNT itmlst`, which
  takes `MNT$_DEVNAM` and `MNT$_VOLNAM`, reads the home block at LBN 1,
  checks its format, `DECFILE11B`, and its label against the one asked
  for, keeps where file headers start in the index file, and reads the
  index file's header, through whose map it finds every other header.
  On a disk it can write, it finds the storage bitmap, `BITMAP.SYS`'s
  second block. It reports the volume on the console:
  `%MOUNT-I-MOUNTED, RAM mounted on _MDA0:`.
- **`FIL$READHDR`** reads file number n's header, VBN
  `IBMAPVBN + IBMAPSIZE + n - 1` of the index file, and checks its
  checksum and number. **`FIL$MAPVBN`** finds a VBN's LBN in a header's
  map, retrieval pointers of formats 1 to 3, and **`FIL$READVBN`** reads a
  file's blocks, an extent at a time. ponytail: no extension headers.
- **`FIL$FREEBLOCKS`** counts a mounted volume's free blocks, the set
  bits of its storage bitmap times its cluster, which `FIL$MOUNT` keeps
  in the VCB, for `$GETDVI`'s `DVI$_FREEBLOCKS` (*Devices*). ponytail:
  counted each time; VMS keeps the count in the VCB.
- **`FIL$SEARCHDIR`** reads a directory's blocks up to its end of file and
  finds the next record whose name, `NAME.TYPE`, matches a pattern with
  `*` and `%`, and the version asked for: any, the highest (the first of
  the name's record) or one. It can skip matches, which is how `$SEARCH`
  goes on from where it was.
- **`FIL$OPENFILE`** finds an image and reads it whole into pool, for the
  image activator, which frees it once the sections are copied.

`f11wrt.mar` writes, as the XQP does, on a disk that can be written:

- **`FIL$CREHDR`** takes the first free slot of the index file bitmap and
  makes an empty header for it, with the slot's next sequence number;
  **`FIL$WRITEHDR`** writes a header, with its checksum.
- **`FIL$EXTEND`** gives a file more blocks, the first free runs of the
  storage bitmap (a set bit is a free block), a format 2 map pointer each,
  or added to the last one when they follow it. **`FIL$DELHDR`** gives a
  file's blocks and header slot back and marks the header deleted.
- **`FIL$ENTER`** and **`FIL$REMOVE`** add and take away a directory
  entry: the directory's records are copied into pool, sorted by name,
  versions from the highest, with the entry entered (a new version one
  above the highest unless one is given) or removed, packed into blocks
  again, each ended by a record size of all ones, and written back, the
  directory extended if it grew.
- **`FIL$INIT`**, for `$INIT_VOL devnam, volnam`, writes an empty volume
  on `DKB0:`, 4,096 blocks, or the ramdisk, 1,024, as `INITIALIZE` lays one out (`ods/docs/initialize.md`):
  the boot block, the home block, the index file bitmap, 64 header
  slots, `BITMAP.SYS`'s SCB and bitmap and the MFD's first block, and the
  nine reserved files, (1,1,0) to (9,9,0), in the MFD.

ponytail: a cluster is a block, each bitmap one block, the index file
never extended, so a volume has at most 4,096 blocks and the 64 files
`FIL$INIT` made room for; no backup home block or index file header.
RMS (`rms.mar`) is a set of system services on VMS's FAB, RAB and NAM
blocks. A file specification is `[dev:][[dir.dir]]name.type;version`,
the device `DKA0:`, `DKB0:` or `MDA0:` with a volume mounted (`RMS$_DNR`).
`RMS$PARSE` splits it, and the FAB's default specification, and
`SYS$DISK:` with the process's default directory, into device,
directory, name, type and version. A device that is a logical name in
`LNM$FILE_DEV`, and has no underscore before it, is translated (`XLATE`),
up to `LNM$C_MAXDEPTH` times: the equivalence is split the same way, its
device takes the logical name's place and its other parts fill in those
the specification leaves out, so `SYS$SYSTEM:DCL.EXE` is
`DKA0:[SYSEXE]DCL.EXE` and `SYS$SYSTEM:[SYSMGR]` is `DKA0:[SYSMGR]`. It
takes each part from the first that has it, in capitals, into the
expanded specification, checks it, and walks
the directory from the MFD, each name `NAME.DIR;1` in the one before
(`[000000]` is the MFD). The images' default is `SYS$SYSTEM:` instead.
ponytail: no search lists, rooted directories or concealed devices; the
expanded string has the device a name translates to, as VMS's does
without them.
A relative directory is made absolute first, against the default
specification's directory if it has one, else the process's: `[]` is
that one, `[-]` its parent, `[--]` the one above, `[.SUB]` and `[-.SUB]`
a directory in one of those. The MFD has no parent: `RMS$_DIR`.

| Service | Does |
| --- | --- |
| `$PARSE fab` | the expanded string, its parts and the directory's ID into the FAB's NAM block |
| `$SEARCH fab` | the next file the NAM block's expanded string names: its resultant string, parts and file ID; `RMS$_FNF` if there is none, then `RMS$_NMF` |
| `$OPEN fab` | opens one file, the highest version unless the specification gives one, for reading; `RMS$_WLK` for writing. Its IFI, record format, attributes, maximum record size and allocation into the FAB, its resultant string into the NAM block if there is one |
| `$CREATE fab` | makes a new file, one version above the highest unless the specification gives one (`RMS$_FEX` if it is there), with the FAB's organization, record format and attributes, maximum record size and `FAB$L_ALQ` blocks, and opens it for `$PUT`; `RMS$_WLK` on `DKA0:`, `RMS$_FUL` if the volume is full |
| `$CONNECT rab` | connects the RAB to the file its FAB opened, at its start |
| `$GET rab` | the next record into the RAB's user buffer: `RAB$W_RSZ`, `RAB$L_RBF`; `RMS$_RTB` if it didn't fit, `RMS$_EOF` past the end; VAR and FIX records only |
| `$PUT rab` | appends the record at `RAB$L_RBF`, `RAB$W_RSZ` bytes, to a file `$CREATE` made: VAR records with their size first, FIX ones of the file's size (`RMS$_RSZ`), each on a word; a block at a time, extending the file by 8 blocks as it fills |
| `$DISCONNECT rab`, `$CLOSE fab` | undo `$CONNECT` and `$OPEN`; `$CLOSE` writes a new file's last block and its end of file |
| `$ERASE fab` | deletes a file, the highest version unless the specification gives one, or, with `FAB$M_NAM` in `FAB$L_FOP`, the one the NAM block's resultant string names, and the next `$SEARCH` finds the one after it; `RMS$_PRV` for the volume's own files, 1 to 9 |
| `$SETDDIR newdir, oldlen, olddir` | the old default directory into `olddir`, then `newdir`, `[dir.dir]` up to 63 characters, the new one, unchecked against the disk; `RMS$_DIR` if it isn't one |

A process's default directory is `PCB$T_DEFDIR`. It starts as its
creator's: the swapper's is `[SYSMGR]`, which `SYSTEM` and the processes
it creates inherit. `$PARSE` gives the expanded string even when it
returns `RMS$_DNF`, which is how `SET DEFAULT` names a directory that isn't
there. The default device is `SYS$DISK`. ponytail: VMS keeps the
directory in P1.

An open file is an IFAB, 1,056 bytes of pool: the file's header, its
volume, the block `$GET` reads in or `$PUT` fills, and where it is. The PCB holds up to 15, by IFI, in
`PCB$A_IFAB`, a bit each in `PCB$L_FILES`; `$CONNECT` puts the IFI in
`RAB$W_ISI` too. `RMS$RUNDOWN` closes a process's files at image exit
and process deletion, writing those `$CREATE` made as `$CLOSE` does.
`NAM$L_WCC` counts the matches `$SEARCH` skips, and its top bit says it
found one, so `RMS$_FNF` and `RMS$_NMF` stay apart when `$ERASE` takes a
match away.

The services, and `FIL$OPENFILE`, run in kernel mode at `IPL$_SYNCH`, one
at a time, which keeps the file system's buffers theirs. They return
VMS's `RMS$_` statuses (`$RMSDEF`) and put them in `FAB$L_STS` or
`RAB$L_STS`. ponytail: VMS's RMS runs in executive mode; no ASTs,
completion routines, logical names, wildcard directories, block I/O,
file sharing or locking, and `$PUT` only appends to a file `$CREATE`
made.

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
TYPE, EDIT, COPY, DELETE, INIT and MOUNT, and those which show the services at
work, which `cargo test -p boot` runs from DCL's prompt (`RUN STARTUP`,
`RUN SNOOP`, a bad verb, `DIR [SYSEXE]P%NG`, `TYPE WELCOME.TXT` and an `EDIT WELCOME.TXT`
session, then
`INIT` and `MOUNT MDA0: RAM`, a `COPY` to it, an `EDIT` in keypad mode
that writes a second version, `DIR`, `DELETE`s and `DIR` again) and to the end. `roottask/sysmgr/` holds the text files in
`DKA0:[SYSMGR]`. They run in user mode, DCL in supervisor mode, and write
on the console with `PRINT` and `PRINTHEX` from `sysexe.mlb`, which call
`PUT_LINE` in `sysexe/lib/print.mar`: a line at a time on `OPA0:`, with
`$QIOW`. `GET_PARAM n, desc`, in `sysexe/lib/param.mar`, points a
descriptor at the nth parameter of the command line. `build.rs` links
`sysexe/lib/` into each.

| Program | Does |
| --- | --- |
| `DCL` | the command interpreter (*The command interpreter*) |
| `DIRECTORY` | `$PARSE`s its command line, with `*.*;*` for what it leaves out, and lists the files `$SEARCH` finds: the directory, the names four to a line, how many |
| `TYPE` | `$OPEN`s the file its command line names and writes each record `$GET` reads on the console, a line each |
| `EDIT` | EDT: `$GET`s the file its command line names into a buffer, a line a record, and at its `*` prompt, read with `IO$_READPROMPT`, types the lines a range names (numbers, `.`, `BEGIN`, `END`, `WHOLE`, `REST`, `"text"` searches), `INSERT`s lines typed up to a CTRL/Z before it, `DELETE`s or `REPLACE`s them; `CHANGE` goes to keypad mode, which paints a VT100 screen, reads a key at a time with `IO$M_NOECHO` and `IO$M_NOFILTR` and changes the buffer, until CTRL/Z; `EXIT` `$CREATE`s the next version and `$PUT`s the buffer to it, `QUIT` doesn't |
| `COPY` | `$OPEN`s its first parameter, `$CREATE`s its second, with the first's attributes and its name and type for what the second leaves out, and copies each record with `$GET` and `$PUT` |
| `DELETE` | `$PARSE`s its parameter, which must give a version or `;*` (`%DELETE-E-DELVER`), and `$ERASE`s each file `$SEARCH` finds |
| `INIT` | `$INIT_VOL` with its two parameters, the device and the label |
| `MOUNT` | `$MOUNT` with its two parameters, the device and the label |
| `STARTUP` | makes 4 pages with `$EXPREG`, checks and deletes them; creates `SLEEPER` at a higher priority, which runs at once, and `PING` and `PONG`; waits until `PONG` sets flag 66 of their cluster; deletes `SLEEPER`; creates `SVCTEST`, `HOG`, `TIMETEST` and `ASTTEST` |
| `SLEEPER` | hibernates until it is deleted |
| `PING`, `PONG` | take three turns through common event flags 64 and 65 of the cluster `PINGPONG`; `PONG` then sets flag 66, which `STARTUP` waits for |
| `SVCTEST` | checks the statuses of the services the others don't use, and of errors: local event flags, the dispatcher's checks and a stub, `$CRETVA` and `$DELTVA`, `$CMKRNL` and `$CMEXEC`; what user mode may `PROBE`, and that services refuse it the executive's data; the console's channels; logical names in both tables, `$ASSIGN` through two of them, and the errors; `$SETPRI`, and `$SUSPND`, `$WAKE`, `$RESUME` and `$DELPRC` on a process of its own, and `$FORCEX` on another, which exits with `SS$_FORCEDEXIT` before its image runs; then creates one whose image doesn't exist, which exits with `RMS$_FNF`, and `SNOOP` and `USURP` |
| `SNOOP` | reads S0 from user mode, and exits with `SS$_ACCVIO` |
| `USURP` | raises IPL from user mode, and exits with `SS$_OPCDEC` |
| `HOG` | associates a common event flag cluster, creates `NUDGE` at its own priority and loops reading flag 64 until `NUDGE` sets it, with no wait: only quantum end lets `NUDGE` run |
| `NUDGE` | sets `HOG`'s flag |
| `TIMETEST` | checks that `$GETTIM` reads a time after 2026; waits for `$SETIMR`s, a delta and a time, 50 ms on, and that a cancelled one never sets its flag; hibernates through three repeating `$SCHDWK` wakeups, cancels them, and checks that the next wakeup is a new one's |
| `CTRLC` | enables a CTRL/C AST, starts a console read and waits for it; the AST, once CTRL/C is typed, `$CANCEL`s the read, which ends with `SS$_ABORT` |
| `ASTTEST` | checks that `$DCLAST`'s AST is delivered as the service returns, or when `$SETAST` enables ASTs again; that one declared in an AST routine waits until it returns; that a `$SETIMR` AST's `$WAKE` ends a `$HIBER`; and that one delivered while it computes in user mode leaves every register as it was |

When every process but the swapper waits, the CPU idles in `WTINT`,
taking the clock's interrupts.

## Next

- Priority boosts on wake and decay at quantum end.
- Privileges, for `$CMKRNL` and `$CMEXEC`, and condition handlers in place
  of exiting on an exception.
- DCL's `STOP` and `SET NOCONTROL`, and exit handlers for `$FORCEX` to
  run.
- Writing the system disk, the disk's interrupt, `$QIO` on disk
  channels, logical names in file specifications (`SYS$SYSTEM:DCL.EXE`)
  and `SYS$DISK`.
- Access modes and search lists for logical names.
- `DISMOUNT`, `INITIALIZE/SIZE`, subdirectories, and the index file
  extended past the headers `INITIALIZE` made room for.
