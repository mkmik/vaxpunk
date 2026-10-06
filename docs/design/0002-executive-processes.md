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
mode AST, and
[ADR-0019](../adr/0019-mailboxes.md): a mailbox is a unit of its own,
`MBnn`, and process deletion writes the termination message to it.

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
| `process.mar` | `$CREPRC`, process start, image activation, `$IMGACT`, `$EXIT`, image rundown, deletion, `$FORCEX`, `$HIBER`, `$WAKE`, `$SUSPND`, `$RESUME`, `$SETPRI`, `$SETPRN`, `$CMKRNL`, `$SETPRV` |
| `lnm.mar` | logical name tables, `$CRELNM`, `$DELLNM`, `$TRNLNM` |
| `qio.mar` | the devices' UCBs, `$ASSIGN`, `$DASSGN`, `$CANCEL`, `$QIO`, `$QIOW`; IRPs, their completion and cancelling |
| `mbdriver.mar` | mailboxes: `$CREMBX`, `$DELMBX`, their driver, and `MB$SEND`, which writes the termination message |
| `ttdriver.mar` | the console's terminal driver: writes, queued reads and their editing, the console receive interrupt and the type-ahead buffer, CTRL/C and CTRL/Y ASTs |
| `getdvi.mar` | `$GETDVI`, `$GETDVIW`, `$DEVICE_SCAN`: what the devices are |
| `syssrv.mar` | the system service vector, the `CHMK` and `CHME` dispatchers, `$CMEXEC`, where processes enter user and supervisor mode, and the stubs |
| `f11.mar` | Files-11 ODS-2 volumes: the disks' VCBs, reading and writing their blocks, `FIL$MOUNT` and `$MOUNT`, headers, maps, directories, `FIL$OPENFILE` for the image activator |
| `f11wrt.mar` | Files-11 writes: headers, blocks, directory entries, `FIL$INIT` and `$INIT_VOL` |
| `mddriver.mar` | `MDA0:`, the ramdisk |
| `rms.mar` | RMS: file specifications, `$PARSE`, `$SEARCH`, `$OPEN`, `$CREATE`, `$CONNECT`, `$GET`, `$PUT`, `$DISCONNECT`, `$CLOSE`, `$ERASE` |
| `sysunwind.mar` | conditions: the exception handlers, `EXE$SIGNAL`, which calls the condition handlers, `$UNWIND`, the catch-all and `$PUTMSG` |

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
   executive halts with `%EXEC-F-NOMOUNT` and the status. The data disk,
   `DKB0:`, is left to `SYSTARTUP_VMS.COM` (*The command interpreter*).
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
| `0x7FFEF800` | `VA$C_CLI_RESULT`, at the user stack's top: the parse of the image's command, up to 2 KB, which `$IMGACT` puts there (*The command interpreter*) |

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

A process is a PCB (`$PCBDEF`, 592 bytes, from pool), a 16 KB kernel stack
from pool, an image and stacks in its P0 and P1, and the threads the PAL
makes for its HWPCB, which is inside the PCB: one for kernel mode and one
for each outer mode it enters. `SCH$GL_PCBVEC` holds the PCBs by index; a PID is a
sequence number in the high word and the index in the low.

### Creation

`$CREPRC` (in the creator's context):

1. Copies the image and process names into a new PCB, the priority, the
   UIC and the privileges (*Privileges*), and makes its logical name table, with `SYS$INPUT`, `SYS$OUTPUT` and
   `SYS$ERROR` for its `input`, `output` and `error` arguments, those it
   was given (*Logical names*).
2. Allocates the kernel stack and builds at its top the frame the PAL pops
   when the process first runs: PC `EXE$PROCSTRT`, PSL kernel mode at IPL 0,
   the stack top as SP. The HWPCB's KSP points at it.
3. At `IPL$_SYNCH`: checks that the name is unique in its UIC group, takes a slot and a PID,
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
  and P1, common event flag clusters, logical names, channels, open files, slot),
  writes its termination message to the mailbox `$CREPRC`'s `mbxunt`
  named, if any: `MSG$_DELPROC`, the status of its last `$EXIT`, its PID,
  the time and its creator's PID (`$ACCDEF`), goes on the
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
  copies the name into the PCB, and `cmdlin`, the result block of the
  command's parse, if there is one, up to `VA$C_CLI_RESMAX` bytes, to
  `VA$C_CLI_RESULT`. There the image's `CLI$PRESENT` and `CLI$GET_VALUE`
  find it (*Commands*).
  It remembers the channels and files the process has in `PCB$L_CLICHANS`
  and `PCB$L_CLIFILES`, runs the
  old image down, activates the new one, which must be in P0, and calls it
  in user mode at `EXE$USRENTRY`, on an empty user stack below the command
  line. It returns only if the activation fails, with its status.
  ponytail: a copy at a fixed address. VMS's `CLI$` routines read the
  parse in DCL's own pages, which user mode may read.
- **Exit handlers.** `$DCLEXH desblk` puts an exit handler first on the
  caller's mode's list, `PCB$AL_EXH`, and `$CANEXH` takes one off, or all.
  `$EXIT` calls them, last declared first, in the mode that called it:
  while the list has one, `EXE$EXIT` takes it off and returns it to
  `SYS$EXIT`, in the vector, which writes the status where the block says,
  calls the handler with the block's argument list, and calls `$EXIT`
  again. Image rundown forgets user mode's. A condition no handler takes
  exits through them too (*Exceptions and condition handlers*).
- **`$EXIT`**, the image's or an exception's, in a process with a command
  interpreter: `EXE$IMGRUNDOWN` gives back the image's timer queue
  entries, P0 pages, common event flag clusters and user mode exit
  handlers, the channels not in
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
  mode, and ends a wait. `$EXIT` then does what it does for any image,
  exit handlers first.

The command interpreter keeps nothing on its stack across an image; what
it remembers is in its P1 data. `EXEC$START` creates the console's
process, `SYSTEM`, with `DCL.EXE`, whose first commands, before it
prompts, are `@SYS$MANAGER:SYSTARTUP_VMS`, the site's startup, which
mounts `DKB0:`, and `@SYS$MANAGER:SYLOGIN`. ponytail: VMS runs
`SYSTARTUP_VMS.COM` in a `STARTUP` process of its own, before anyone
logs in. Its verbs are those of `DCL$TABLES` (*Commands*); a verb with
an image runs it with `$IMGACT` and the parse:

| Command | Does |
| --- | --- |
| `RUN image` | `$IMGACT`, with `.EXE` if the name has no type, and no parse |
| `DIRECTORY [spec]` | `DIRECTORY.EXE` |
| `TYPE spec` | `TYPE.EXE` |
| `DEFINE name equivalence` | `$CRELNM` in `LNM$PROCESS`, and `%DCL-I-SUPERSEDE` if it replaced one, unless `/NOLOG`; `/SYSTEM`, `/PROCESS` or `/TABLE=table` names the table, one of those `lnm.mar` knows, else `SS$_NOLOGTAB` |
| `DEASSIGN name` | `$DELLNM` from `LNM$PROCESS`, or the table the same qualifiers name |
| `SHOW LOGICAL name` | `$TRNLNM` in `LNM$FILE_DEV`, or the table the same qualifiers name: `"name" = "equivalence" (table)`, or `%SHOW-S-NOTRAN` |
| `SHOW LOGICAL [*]` | lists every name, the process's table's and then the system's, or only that table's, in the order they were made, under each table's name; it copies them one at a time with `$CMKRNL`, so it takes CMKRNL. ponytail: VMS's DCL asks the executive's logical name routines, and sorts them |
| `SET DEFAULT [dev:][dir]` | `$PARSE`s it, which must name no file, `$SETDDIR` with the directory it expands to, and `$CRELNM` of `SYS$DISK` in `LNM$PROCESS` with its device; one that doesn't exist is still set, after `%DCL-I-INVDEF` |
| `SHOW DEFAULT` | the device and directory `$PARSE` expands an empty specification to |
| `EDIT spec` | `EDIT.EXE` |
| `COPY[/LOG] from to` | `COPY.EXE` |
| `DELETE[/LOG] spec` | `DELETE.EXE` |
| `INITIALIZE device label` | `INIT.EXE` |
| `MOUNT device [label]` | `MOUNT.EXE` |
| `DISMOUNT device` | `DISMOUNT.EXE` |
| `CREATE/DIRECTORY spec` | `CREATE.EXE`; without `/DIRECTORY`, `%CREATE-E-NOTDIR` |
| `SHOW PROCESS[/PRIVILEGES]`, `SHOW SYSTEM` | `SHOW.EXE`, whose `OPTION` says which |
| `SET PROCESS/PRIVILEGES=(priv[,...])` | `SET.EXE` |
| `TCPIP [command]` | `TCPIP.EXE`, which parses `command`, or each one after its `TCPIP>` prompt, with its own tables, `sysexe/tcpip.cld` ([ADR-0022](../adr/0022-tcpip-utility-and-dhcp.md)) |
| `SET [NO]CONTROL[=Y]` | enables DCL's CTRL/Y AST again, or disables it with `IO$_SETMODE`, so that CTRL/Y, and CTRL/C no image has an AST for, do nothing, until `SET CONTROL`. ponytail: no `T`, there is no CTRL/T |
| `SET HOST address` | `RTPAD.EXE`, whose `NODE` is the address |
| `CONTINUE` | returns from the CTRL/Y AST, which goes back to the image |
| `STOP [process-name]`, `STOP/IDENTIFICATION=pid` | `$DELPRC`, by name or by the PID in hex; with no name, ends the procedures, and the image CTRL/Y stopped with `$EXIT` from DCL, which skips its exit handlers, user mode's, with `SS$_ABORT` |
| `HELP [verb]` | `HELP.EXE`, which describes the verbs from `DCL$TABLES` (*The system disk's programs*) |
| `LOGOUT` | `$DELPRC` |
| `@file [p1 ... p8]` | reads `file.COM` with RMS and takes its `$` lines as commands |
| `SET COMMAND file` | compiles `file.CLD`'s verbs into the process's tables, which DCL parses with first |
| `name := $image`, then `name args` | runs `image`, a foreign command, which reads `args` with `LIB$GET_FOREIGN` |
| `name = expression`, `name := string` | sets a symbol, `==` and `:==` a global one |
| `IF`, `GOTO`, `EXIT`, `WRITE SYS$OUTPUT` | as in VMS's procedures; `EXIT [status]` outside a procedure `$FORCEX`es the image CTRL/Y stopped and continues it, so that it exits through its exit handlers |
| `SHOW SYMBOL name`, `DELETE/SYMBOL name` | shows a symbol, the nearest, deletes a local one; `/LOCAL` or `/GLOBAL` says which; `/ALL` instead of a name shows or deletes all of those |

DCL reads a line
from `SYS$INPUT` with `IO$_READPROMPT` and the prompt `$ `, and reports a
failure status with its message, `%RMS-E-DNF, directory not found`, from
`$GETMSG`, whose table in the executive has VMS's texts of the file
system's, RMS's and the volume services' statuses, or as VMS does one it
has no text for, `%NONAME-F-NOMSG, Message number 0000000C`, after
`%DCL-W-ACTIMAGE` if `$IMGACT` returned it, unless the status has
`STS$M_INHIB_MSG`, bit 28, set: the image, or the parse, reported it.

DCL enables its CTRL/Y AST, `CTRLY`, when it starts, before each image it
runs and when it continues one, and keeps whether an image runs. With
one, the AST ends the procedures running, keeps its frame pointer and
goes to the command loop, where `CONTINUE` enables the AST again and
returns from it; a command that runs another image runs the stopped one
down. With none, the AST enables itself again, and DCL ends its
procedures at the next command; the read CTRL/Y ended gives it an empty
line. A command that runs an image ends the one CTRL/Y stopped, as
`STOP` does. ponytail: one message table in the executive rather than
message files.

### Commands

[ADR-0017](../adr/0017-command-tables-from-cld-with-vcdu.md): commands are
defined in CLD, which vcdu compiles into command tables at build time
(`vtools/docs/command-tables.md`). `CLI`, `roottask/sysexe/lib/cli.mar`,
parses commands with them, and is linked into DCL and into every image:

- **`CLI$DCL_PARSE line, table [,prompt]`** parses a command into the
  result block, `CLI$$RESULT`, of the image it runs in. It works as
  VMS's DCL does:
  - A verb, or a qualifier, matches on its first 4 characters; one of
    fewer must be the only one it abbreviates. A keyword matches if it
    abbreviates only one.
  - Parameters are separated by blanks. Qualifiers, `/name`,
    `/NOname`, `/name=value` or `/name=(value,...)`, may come after the
    verb or any parameter. One with `PLACEMENT=LOCAL` is given after a
    parameter's value, and is that value's; one with
    `PLACEMENT=POSITIONAL` is too, or the command's after the verb.
  - A `LIST` parameter's values are separated by commas, and a
    concatenating one's by plus signs.
  - A word is taken in capitals. Text in quotes is kept as it is, `""`
    a quote.
  - A keyword or qualifier with `SYNTAX=` makes the parse start again
    with that syntax, unless the parse switched to it once already: a
    `SYNTAX=` back to an earlier syntax stays in the later one, so
    `DELETE/SYMBOL/ALL` stays `DELETE_SYMBOL_ALL`. ponytail: again from
    the verb, so a qualifier the syntax doesn't have is an error, not
    ignored as VMS's `IGNQUAL`.
  - At the end, a required parameter missing is prompted for, `_From: `,
    if there is a prompt routine. The answer goes on the line, and the
    parse goes on with it. Then the defaults are set, and `DISALLOW`
    is checked.

  An error is written as `%CLI-W-IVQUAL, unrecognized qualifier - check
  validity, spelling, and placement`, then ` \FOO\`, and returned with
  `STS$M_INHIB_MSG`; DCL's facility, for its own parses, is `DCL`.
  ponytail: the parse is limited to 128 entities and 128 values of up to
  255 characters, 1 KB of value text, a 512-byte line and a 2 KB result
  block, each `CLI$_BUFOVF` past that.
- **The result block** holds no addresses. It is a word, its length (0
  for no command), a spare word, then an entry for `$VERB`, `$LINE`
  and each parameter, qualifier and keyword of the command, given or
  not, then one for each qualifier given after a parameter's value. An
  entry is:
  - a word, its length;
  - a byte, its state: absent, present, negated or defaulted;
  - a byte of flags: 1 for a parameter's;
  - a word, the cursor of `CLI$GET_VALUE`;
  - a word, a local qualifier's: where in the block the parameter
    value it was given after is;
  - its path, `.ASCIC`: its label, after its parent's and a dot for a
    keyword (`MODE.SLOW`);
  - its values, each `.ASCIC` and a byte, the comma, plus or 0 that
    followed it.

  A defaulted parameter, and an absent or defaulted entity with a
  default value, have that value.
- **`CLI$PRESENT name`** answers from the block: `CLI$_PRESENT`,
  `CLI$_DEFAULTED`, `CLI$_NEGATED` or `CLI$_ABSENT`; for a qualifier
  given after the parameter value `CLI$GET_VALUE` returned last,
  `CLI$_LOCPRES` or `CLI$_LOCNEG`.
- **`CLI$GET_VALUE name, retdesc [,retlen]`** returns the next value
  each call, `CLI$_COMMA`, `CLI$_CONCAT` or `SS$_NORMAL` for the last,
  then `CLI$_ABSENT`, and starts again. A qualifier given after the
  parameter value it returned last has its values there.
- **`CLI$DISPATCH [userarg]`** calls the `ROUTINE` of the verb or syntax
  parsed last, with userarg, and returns its status, or `CLI$_INVROUT`.
- **`LIB$GET_FOREIGN get_str [,prompt [,outlen [,force_prompt]]]`**, in
  `lib/getforeign.mar`, returns `$LINE` past its first word, the verb:
  a foreign command's arguments. With none, it reads a line with
  `LIB$GET_INPUT` (`lib/getinput.mar`), in capitals, if given a prompt.
- Both read the block of the last parse in the image or, if there was
  none, the one at `VA$C_CLI_RESULT`; with no command, every name is
  absent. A name the command doesn't have is `%CLI-F-SYNTAX, error
  parsing 'NAME'`, `-CLI-E-ENTNF`, and the image exits with
  `CLI$_ENTNF`.

DCL parses each command, after symbol substitution, labels and
assignments, with `DCL$TABLES`, after the tables `SET COMMAND` made, the
last first: `CLI$$DCL_PARSE` looks for the verb in each, and one of a
name a table before has doesn't count ([ADR-0018](../adr/0018-set-command-and-foreign-commands.md)).
It passes a prompt routine at the console, which reads with
`IO$_READPROMPT`, and none in procedures. Then:

- A verb or syntax with an `IMAGE` (`CLI$$IMAGE`) runs it, with
  `CLI$$RESULT` as `$IMGACT`'s `cmdlin`.
- One with a `CLIROUTINE` (`CLI$$ROUTINE`) is DCL's own, which DCL
  finds by name.
  - Each takes its values with `CLI$GET_VALUE`. `IF`, `EXIT` and
    `WRITE`'s expressions are a `$REST_OF_LINE`, which DCL's expression
    code reads.
- `SET COMMAND file` reads `file.CLD` with RMS and compiles it with
  `CDU$COMPILE`, `roottask/sysexe/dcl/cdu.mar`, into DCL's P1 data: 16
  KB of tables, 8 files at most.
- A verb that is a symbol whose value starts with `$`, `name :=
  $image`, is a foreign command: DCL runs `image` with a block that
  `CLI$$FOREIGN` made, `$VERB` and `$LINE` only, the command in
  capitals outside quotes.
- `DELETE/SYMBOL` is the `DELETE_SYMBOL` syntax of `DELETE`, and
  `DELETE/SYMBOL/ALL` its `DELETE_SYMBOL_ALL`, which has no
  parameters. `SHOW DEVICES`, `SHOW LOGICAL` and `SHOW SYMBOL` are
  syntaxes too, each with its parameter and its qualifiers.
  `SHOW PROCESS` and `SHOW SYSTEM` are `SHOW_IMAGE`, a syntax with
  `IMAGE SHOW`.

## Scheduling

States and queues, as VMS's `$STATEDEF`:

| State | Queue |
| --- | --- |
| `CUR` | none: `SCH$GL_CURPCB` |
| `COM` | `SCH$AQ_COMH`, one per priority 0-31, highest first; bit n of `SCH$GL_COMQS` set if queue n isn't empty |
| `HIB` | `SCH$GQ_HIBWQ` |
| `LEF` | `SCH$GQ_LEFWQ` |
| `CEF` | the common event block's own queue |
| `MWAIT` | `f11.mar`'s, for the file system's lock (*Files*) |
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
macros of `starlet.mlb`, which push the arguments: by the calling standard
([DESIGN-0004](0004-calling-standard.md)), in x0-x7 and on the stack, their
count in x9. `SYS$name` is a frameless routine in `syssrv.mar` that moves
x7 to x10 and does `CHMK #code`, which puts the code in x7, and returns;
`SYS$EXIT` has a frame, since it calls the exit handlers.
The PAL delivers the `CHMK` to `EXE$CMODKRNL`, in kernel mode, with every
register as it was and the code below its frame. `EXE$CMODKRNL` checks the
code and the count (`SS$_ILLSER`, `SS$_INSFARG`), copies the arguments past
the eighth from the caller's stack, which its mode must be able to read
(`SS$_ACCVIO`), checks that each is a sign-extended longword
(`SS$_ARG_GTR_32_BITS`, as 64-bit VMS: no service here takes 64-bit
addresses), calls `EXE$name` with them, and `REI`s with its status in R0,
back to the caller's mode. A `CALLG` list the caller can't read faults in
the caller, which loads it. `$CMEXEC` does `CHME #0` instead, to
`EXE$CMODEXEC` in executive mode.

Programs run in user mode, so the `SYS$name` routines, `EXE$CMODEXEC`,
`EXE$USRSTART` and the condition handling code, which runs in the mode
that signals, are in a psect of their own, `EXEC$VECTOR`, page aligned,
between `EXE$VECTOR` and `EXE$VECTOREND`: the vector, which `EXEC$START`
makes user readable and executable, as VMS's system service vector is.
`$UNWIND` and `$PUTMSG` are there too, as `SYS$UNWIND` and
`SYS$PUTMSG`, which run in the caller's mode, without `CHMK`. The rest
of S0 is the kernel's.

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
| Process control | `$CREPRC`, `$DELPRC`, `$EXIT`, `$FORCEX`, `$HIBER`, `$WAKE`, `$SUSPND`, `$RESUME`, `$SETPRI`, `$SETPRN`, `$GETJPI`, `$GETJPIW`, `$SETPRV`, `$CMKRNL`, `$CMEXEC`, `$DCLEXH`, `$CANEXH` | |
| Event flags | `$ASCEFC`, `$DACEFC`, `$SETEF`, `$CLREF`, `$READEF`, `$WAITFR`, `$WFLOR`, `$WFLAND` | `$DLCEFC` |
| Memory | `$CRETVA`, `$DELTVA`, `$EXPREG` | `$CNTREG`, `$SETPRT`, `$LKWSET`, `$ULWSET`, `$LCKPAG`, `$ULKPAG`, `$CRMPSC`, `$MGBLSC` |
| Time | `$GETTIM`, `$SETIMR`, `$CANTIM` | |
| I/O | `$ASSIGN`, `$DASSGN`, `$CANCEL`, `$QIO`, `$QIOW`, `$CREMBX`, `$DELMBX` | |
| Logical names | `$CRELNM`, `$DELLNM`, `$TRNLNM` | |
| Images | `$IMGACT` | |
| Conditions, in the caller's mode | `$UNWIND`, `$PUTMSG` | |
| RMS | `$PARSE`, `$SEARCH`, `$OPEN`, `$CREATE`, `$CONNECT`, `$GET`, `$PUT`, `$DISCONNECT`, `$CLOSE`, `$ERASE`, `$SETDDIR`, `$CREATE_DIR` | |
| Volumes | `$MOUNT`, `$DISMOU`, `$INIT_VOL` | |
| ASTs | `$DCLAST`, `$SETAST`, `$ASTEXIT` | |
| Other | `$GETSYI`, `$GETSYIW`, `$GETMSG` | |

Arguments the implemented services take but ignore: `$CREPRC`'s
quotas and status flags, `$ASSIGN`'s mailbox,
`$CREMBX`'s protection, access mode and flags, the logical name
services' attributes, `$ASCEFC`'s protection
and permanence (every cluster is temporary), the access modes.

### Privileges

A process has a UIC, `PCB$L_UIC`, and three masks of privileges,
VMS's bits (`$PRVDEF`): the current ones, `PCB$Q_PRIV`, which the
checks look at; the permanent ones, which the current ones go back to
when an image exits; and the authorized ones, which `$SETPRV` may enable
without `SETPRV`. The swapper has every privilege and the UIC `[1,4]`,
and `SYSTEM` inherits them: `$CREPRC` gives a process its creator's
UIC, or another one for `DETACH`, its creator's privileges or those of
its `prvadr` the creator has, and its creator's authorized ones. A
service checks with `IFPRIV` and `IFNPRIV` from `lib.mlb`, and says
`SS$_NOPRIV` without the privilege:

| Privilege | What it takes it |
| --- | --- |
| `CMKRNL`, `CMEXEC` | `$CMKRNL`; `$CMEXEC`, with either. Executive mode can't read the PCB, so `EXE$CMODEXEC` asks the kernel with a `CHMK` of a code of its own |
| `SETPRV` | `$SETPRV` enabling what isn't authorized: without, it enables what is and says `SS$_NOTALLPRIV` |
| `GROUP`, `WORLD` | another process, in `EXE$NAMPID`, for `$DELPRC`, `$FORCEX`, `$GETJPI`, `$SUSPND`, `$RESUME`, `$WAKE`, `$SETPRI`, `$SCHDWK` and `$CANWAK`: none for one with the caller's UIC, `GROUP` for another in its group, `WORLD` for any. A wildcard `$GETJPI` skips the others. A name is looked for in the caller's group only, as names are a group's |
| `DETACH` | `$CREPRC` of a process with another UIC |
| `SYSNAM` | `$CRELNM` and `$DELLNM` in `LNM$SYSTEM_TABLE`, a `$CREMBX` logical name, and `$MOUNT` and `$DISMOU`, whose volumes every process sees |
| `PRMMBX`, `TMPMBX` | `$CREMBX` of a permanent mailbox, or a temporary one; `$DELMBX` |
| `LOG_IO`, `PHY_IO` | the disks' `IO$_READLBLK` and `IO$_WRITELBLK`, with either; `IO$_READPBLK` and `IO$_WRITEPBLK`, with `PHY_IO` |
| `VOLPRO` | `$INIT_VOL` of a volume someone else owns, and `$MOUNT`'s `MNT$_OWNER` and `MNT$_VPROT`. ponytail: VMS lets a volume's owner give those too |
| `SYSPRV` | a file's or a volume's system access, as if the process's UIC group were up to `MAXSYSGROUP`, 8 (*Files*) |
| `GRPPRV` | the system access to the files and volumes of its UIC group |
| `READALL` | reading any file |
| `BYPASS` | any access to any file or volume |
| `OPER` | `IO$_SETCHAR` on `BGA0:`, the network's interface |

`SET PROCESS/PRIVILEGES` changes the permanent ones, and `SHOW
PROCESS/PRIVILEGES` lists the authorized and current ones, by the names
in `sysexe/lib/prvnam.mar`. Files and volumes are checked by their
owner and protection (*Files*).

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
  IRPs waiting for the device, its driver's FDT routine, a disk's VCB, a
  disk driver's start I/O routine, which the file system calls too
  ([ADR-0015](../adr/0015-file-system-io-through-the-disk-driver.md)),
  and the count of errors the device reported, `UCB$L_ERRCNT`.
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
  type-ahead buffer, and requests the `IPL$_IOPOST` software interrupt.
  When the buffer is full it leaves the rest in the UART and disables
  its interrupt until `GETCHAR` takes a character, so the host waits, as
  for a terminal's XOFF, and no line loses its end. `TTY$IOPOST` gives the buffer's
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
made. `SS$_DRVERR`, the PAL's word that the device itself failed, adds
one to the UCB's error count. The file system's own reads and writes go
to `DK$STARTIO` too (*Files*).

ponytail: the console's line being read is kept in `ttdriver.mar`, not
its UCB, since there is one terminal; output waits for the console at
`IPL$_SYNCH`, and a write in the middle of a line being read doesn't
redisplay it. The recall buffer is the console's, shared by every
reader, where VMS has DCL's own, with `RECALL`. No quotas. The
terminal's characteristics, which `IO$_SETMODE` sets and
`IO$_SENSEMODE` and `$GETDVI` return, are `ttdriver.mar`'s too,
`TTY$AB_CHAR`; of them only `NOECHO` changes what the driver does.

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
  `DEVCLASS`, `UNIT` (0), `ERRCNT` (a disk's UCB's, 0 for the
  console), `DEVNAM`, `VOLNAM`, `FREEBLOCKS`
  (`FIL$FREEBLOCKS`), `CLUSTER`, `MOUNTCNT`, the `AVL`, `MNT` and `SWL`
  bits, and `STS`, `UCB$M_ONLINE` if the device is there: the console, a
  mounted disk, the ramdisk once made, a PAL disk whose first block
  reads. Others are `SS$_BADPARAM`. It clears and sets the event flag and
  fills the IOSB, done at once; `$GETDVIW` is `$GETDVI`.

DCL's `SHOW DEVICES` scans the disks, then the terminals, and asks
`$GETDVIW` about each; with `/MOUNTED` it lists only the devices with a
volume mounted. ponytail: a VCB, or none for the console, stands
for the device rather than its UCB, which has no operation or
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
disk's driver is done when it returns.

The file system, RMS, `$MOUNT`, `$DISMOU`, `$INIT_VOL`, `$GETDVI` and
the image activator use it holding the file system's lock
([ADR-0020](../adr/0020-file-system-lock-below-ipl-synch.md)), as VMS's
XQP holds its volume's. `FIL$LOCK` waits, in `MWAIT`, while another
process holds it, then raises IPL to `IPL$_ASTDEL`, where kernel mode
ASTs, a `$DELPRC` among them, wait for `FIL$UNLOCK`, but reschedules and
interrupts don't: other processes run while one is in the file system.
The lock keeps the file system's buffers, `F11$AB_IRP` and `F11$GL_VCB`
to one process. `FIL$READLBLK` raises IPL to `IPL$_SYNCH` for the
driver's call, `RMS$PARSE` while it holds pointers into logical names'
blocks, and `RMS$VOLIDLE` while it looks through the PCBs. At boot,
`FIL$MOUNT` mounts `DKA0:` without it, before there is another process.
ponytail: one lock for every volume, where the XQP `$ENQ`s one per
volume and one per file; no priority boost for the holder.

`FIL$SELECT` picks the VCB by device name. `f11.mar` reads Files-11 (`ods/docs/`) as VMS's XQP
does:

- **`FIL$MOUNT`**, at boot for `DKA0:` and from `$MOUNT itmlst`, which
  takes `MNT$_DEVNAM` and `MNT$_VOLNAM`, reads the home block at LBN 1,
  checks its format, `DECFILE11B`, and its label against the one asked
  for, keeps where file headers start in the index file, and reads the
  index file's header, through whose map it finds every other header.
  On a disk it can write, it finds the storage bitmap, `BITMAP.SYS`'s
  second block. It reports the volume on the console:
  `%MOUNT-I-MOUNTED, RAM mounted on _MDA0:`. Without `MNT$_VOLNAM`, any
  label will do. The VCB keeps the volume's owner and protection, from
  the home block, or `MNT$_OWNER`'s and `MNT$_VPROT`'s while it is
  mounted.
- **`$DISMOU devnam, flags`** clears the VCB's `VCB$V_MOUNTED`, so the
  volume can be mounted again, or another written there:
  `SS$_DEVNOTMOUNT` if none is, `SS$_DEVACTIVE` for `DKA0:`, or while
  a process has a file on it open (`RMS$VOLIDLE` looks through every
  PCB's IFABs). ponytail: no flags.
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
  image activator, which frees it once the sections are copied:
  `RMS$_PRV` without execute access to it.
- **`FIL$CHKPRO`** says whether the current process may read, write,
  execute or delete what an owner's UIC and a protection mask guard,
  or control it, by VMS's rules (below), and **`FIL$CHKHDR`** checks a
  file header's, after its volume's.

`f11wrt.mar` writes, as the XQP does, on a disk that can be written:

- **`FIL$CREHDR`** takes the first free slot of the index file bitmap and
  makes an empty header for it, with the slot's next sequence number,
  owned by the current process's UIC, with VMS's default protection,
  `(S:RWED,O:RWED,G:RE,W)`; **`FIL$WRITEHDR`** writes a header, with its
  checksum. `$CREATE` gives a new version its predecessor's protection
  instead, and a protection XAB's owner and protection win over both.
  ponytail: no process default protection.
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
- **`FIL$MKDIR`** makes an empty directory, `NAME.DIR;1`, in another, as
  `INITIALIZE` makes the MFD: a header marked a directory and contiguous,
  VAR records in blocks of 512, the one above's protection without delete
  access, as VMS's `CREATE/DIRECTORY` does, and one block holding only
  the end of block's -1, entered last.
- **`FIL$INIT`**, for `$INIT_VOL devnam, volnam, itmlst`, writes an empty volume
  on `DKB0:`, 4,096 blocks, or the ramdisk, 1,024, as `INITIALIZE` lays one out (`ods/docs/initialize.md`):
  the boot block, the home block, the index file bitmap, 64 header
  slots, `BITMAP.SYS`'s SCB and bitmap and the MFD's first block, and the
  nine reserved files, (1,1,0) to (9,9,0), in the MFD. The volume and its
  files are `INIT$_OWNER`'s, the caller's UIC by default, and the volume
  has `INIT$_VOLPRO`'s protection, none denied by default; the MFD has
  `(S:RWE,O:RWE,G:RE,W:RE)`, so that all may look in it. Another's volume
  takes `VOLPRO` to write over, a blank disk none.

Each file has an owner, a UIC, and a protection mask in its header
(`FH2$L_FILEOWNER`, `FH2$W_FILEPROT`), and each volume in its home block:
4 bits for each of four categories, system, owner, group and world, a
set bit denying read, write, execute or delete. `FIL$CHKPRO` gives a
process what the world may do; what the group may too if its UIC group
is the owner's; what the owner may, and control, if its UIC is the
owner's; and what the system may, and control, if its UIC group is up to
8, or with `SYSPRV`, or with `GRPPRV` in the owner's group. `READALL`
reads anything and `BYPASS` does anything. A volume's mask applies to
every access to its files, as VMS's does. Without, RMS says `RMS$_PRV`:
a lookup takes execute access to each directory it looks in, a wildcard
read access; `$OPEN` read access to the file, `$CREATE` and
`$CREATE_DIR` write access to the directory, `$ERASE` delete access to
the file and write access to its directory, and the image activator
execute access to the image. ponytail: no ACLs; a volume's bits mean
what a file's do, where VMS's third is create.

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
| `$OPEN fab` | opens one file, the highest version unless the specification gives one, for reading, and with `FAB$M_PUT` in `FAB$B_FAC` for `$PUT` too: `RMS$_WLK` on a volume that can't be written, `DKA0:`, `RMS$_PRV` without write access or for the volume's own files and directories. Its IFI, record format, attributes, maximum record size and allocation into the FAB, its resultant string into the NAM block if there is one, its owner and protection into the protection XAB (`$XABPRODEF`) in `FAB$L_XAB`'s chain if there is one |
| `$CREATE fab` | makes a new file, one version above the highest unless the specification gives one (`RMS$_FEX` if it is there), with the FAB's organization, record format and attributes, maximum record size and `FAB$L_ALQ` blocks, and opens it for `$PUT`; `RMS$_WLK` on `DKA0:`, `RMS$_FUL` if the volume is full. It has the protection of the highest version there was, as VMS's does, else the default, unless a protection XAB gives one, and an owner, which takes the system's access if it isn't the caller's UIC |
| `$CONNECT rab` | connects the RAB to the file its FAB opened, at its start, or at its end with `RAB$M_EOF` in `RAB$L_ROP` |
| `$GET rab` | the next record into the RAB's user buffer: `RAB$W_RSZ`, `RAB$L_RBF`; `RMS$_RTB` if it didn't fit, `RMS$_EOF` past the end; VAR and FIX records only |
| `$PUT rab` | appends the record at `RAB$L_RBF`, `RAB$W_RSZ` bytes, to a file `$CREATE` made or `$OPEN` opened for it, at its end, `RMS$_NEF` if the stream is elsewhere: VAR records with their size first, FIX ones of the file's size (`RMS$_RSZ`), each on a word; a block at a time, extending the file by 8 blocks as it fills |
| `$DISCONNECT rab`, `$CLOSE fab` | undo `$CONNECT` and `$OPEN`; `$CLOSE` writes a written file's last block and its end of file, and gives the file the protection and owner of its FAB's protection XAB, as VMS's does: a new protection takes control access, a new owner the system's. ponytail: VMS's `SET PROTECTION` asks the XQP with `IO$_MODIFY`, and needs no read access to the file |
| `$ERASE fab` | deletes a file, the highest version unless the specification gives one, or, with `FAB$M_NAM` in `FAB$L_FOP`, the one the NAM block's resultant string names, and the next `$SEARCH` finds the one after it; `RMS$_PRV` for the volume's own files, 1 to 9, `RMS$_MKD` with `SS$_DIRNOTEMPTY` in `FAB$L_STV` for a directory with files in it |
| `$CREATE_DIR devdirspec` | makes the directory `[dev:][dir.dir]` names, and those above it that aren't there, with `FIL$MKDIR`, as the directory walk finds each missing: `SS$_CREATED`, or `SS$_NORMAL` if they were all there. ponytail: VMS's `LIB$CREATE_DIR` is a library routine that asks the XQP with `$QIO`; here it is a service |
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
and process deletion, writing those opened for `$PUT` as `$CLOSE` does.
`NAM$L_WCC` counts the matches `$SEARCH` skips, and its top bit says it
found one, so `RMS$_FNF` and `RMS$_NMF` stay apart when `$ERASE` takes a
match away.

The services, and `FIL$OPENFILE`, run in kernel mode at `IPL$_SYNCH`, one
at a time, which keeps the file system's buffers theirs. They return
VMS's `RMS$_` statuses (`$RMSDEF`) and put them in `FAB$L_STS` or
`RAB$L_STS`. ponytail: VMS's RMS runs in executive mode; no ASTs,
completion routines, logical names, wildcard directories, block I/O,
file sharing or locking, and no `$UPDATE` or `$TRUNCATE`, so `$PUT` writes only at the end.

### Exceptions and condition handlers

Conditions are signaled and handled as on VMS
([ADR-0021](../adr/0021-condition-handlers-run-in-the-mode-that-signals.md)).
A routine establishes a condition handler by writing its address at `0(FP)`, in
its frame, which `.ENTRY` leaves 0 (`vtools/docs/macro32.md`), or with
`LIB$ESTABLISH`. A condition is signaled by an exception in an outer
mode, or by `LIB$SIGNAL` or `LIB$STOP`, which build the signal array, the
condition and its arguments, then the PC and PSL, and the mechanism
array, the frame, its depth, R0 and R1 (`$CHFDEF`).

An access violation or a reserved instruction in an outer mode reaches
the executive through the SCB (DESIGN-0001, *Exceptions*), in kernel
mode. `EXE$ACVIOLAT` and `EXE$OPCDEC` copy the PAL's frame, the
registers and the two arrays below the stack of the mode that took it,
and `REI` to `EXE$SRCHANDLER` there. If that stack can't take them, the
image exits with the condition.

`EXE$SIGNAL`, in the vector, looks for a handler in the mode that
signaled: from the frame that signaled, depth 0, out along the saved
FPs, until one is 0, as `EXE$USRSTART`, `EXE$CLISTART` and
`EXE$ASTDISP` leave it, or can't be read. It calls each handler with the
two arrays. A handler returns `SS$_CONTINUE` to go on where the
condition was signaled, with R0 and R1 from the mechanism array, or at
the PC it wrote in the signal array; `SS$_RESIGNAL` to let the next one
have it; or calls `$UNWIND`. `$UNWIND` asks for the frames from the one
that signaled out to the handler's establisher, or as many as it says,
to go once the handler returns: `EXE$SIGNAL` calls each one's handler
with `SS$_UNWIND`, then loads the registers each of them saved, as the
frame descriptor at its `24(FP)` says, and returns from the last, to its
caller or the PC `$UNWIND` was given.
`LIB$SIG_TO_RET` is the handler that does it with the condition in R0.

A condition no handler takes goes to `EXE$CATCHALL`, which writes its
message with `$PUTMSG`, `$GETMSG`'s text with the FAO arguments filled
in, on `SYS$OUTPUT`. A severe one then ends the image with `$EXIT`,
through its exit handlers, with `STS$M_INHIB_MSG` set, so DCL doesn't
write it again; any other goes on. That ends the process, or, under
DCL, the image:

```
%SYSTEM-F-ACCVIO, access violation, reason mask=00, virtual address=40010000, PC=00010004, PSL=03C00000
%EXEC-W-EXITED, process SNOOP exited with status 1000000C
```

ponytail: no exception vectors, and a fault in a handler is looked for
from there out again, through the frames already searched.

## The system disk's programs

`roottask/sysexe/` holds the programs in `DKA0:[SYSEXE]`: DCL, DIRECTORY,
TYPE, EDIT, COPY, DELETE, INIT, MOUNT, DISMOUNT and CREATE, and those which show the services at
work, which `cargo test -p boot` runs from DCL's prompt (`RUN STARTUP`,
`RUN SNOOP`, a bad verb, `DIR [SYSEXE]P%NG`, `TYPE WELCOME.TXT` and an `EDIT WELCOME.TXT`
session, then
`INIT` and `MOUNT MDA0: RAM`, a `COPY/LOG` to it, which prompts for its
parameters, an `EDIT` in keypad mode that writes a second version, `DIR`,
`DELETE`s and `DIR` again, `RUN CLITEST`) and to the end. `roottask/sysmgr/` holds the text files in
`DKA0:[SYSMGR]`, among them `SYSTARTUP_VMS.COM`, which mounts `DKB0:` at
boot, and `SYLOGIN.COM`. They run in user mode, DCL in supervisor mode, and write
on the console with `PRINT` and `PRINTHEX` from `sysexe.mlb`, which call
`PUT_LINE` in `sysexe/lib/print.mar`: a line at a time on `OPA0:`, with
`$QIOW`. They take their parameters and qualifiers with `GETVALUE` and
`PRESENT`, also in `sysexe.mlb`, which call `CLI$GET_VALUE` and
`CLI$PRESENT` in `sysexe/lib/cli.mar` (*Commands*). `build.rs` links
`sysexe/lib/` into each, and `sysexe/NAME.cld`'s table into `NAME.EXE`.

| Program | Does |
| --- | --- |
| `DCL` | the command interpreter (*The command interpreter*) |
| `DIRECTORY` | `$PARSE`s its parameter, with `*.*;*` for what it leaves out, and lists the files `$SEARCH` finds: the directory, the names four to a line, how many; with `/OWNER` and `/PROTECTION`, a line each, with what `$OPEN` puts in a protection XAB: `[g,m]` and `(RWED,RWED,RE,)` |
| `TYPE` | `$OPEN`s the file its parameter names and writes each record `$GET` reads on the console, a line each |
| `EDIT` | EDT: `$GET`s the file its parameter names into a buffer, a line a record, and at its `*` prompt, read with `IO$_READPROMPT`, types the lines a range names (numbers, `.`, `BEGIN`, `END`, `WHOLE`, `REST`, `"text"` searches), `INSERT`s lines typed up to a CTRL/Z before it, `DELETE`s or `REPLACE`s them; `CHANGE` goes to keypad mode, which paints a VT100 screen, reads a key at a time with `IO$M_NOECHO` and `IO$M_NOFILTR` and changes the buffer, until CTRL/Z; `EXIT` `$CREATE`s the next version and `$PUT`s the buffer to it, `QUIT` doesn't |
| `COPY` | `$OPEN`s its first parameter, `$CREATE`s its second, with the first's attributes and its name and type for what the second leaves out, and copies each record with `$GET` and `$PUT`; with `/LOG`, `%COPY-S-COPIED, from copied to to (n records)`. `APPEND` runs it too, and it `$OPEN`s the second for `$PUT` instead, `%APPEND-S-APPENDED` |
| `DELETE` | `$PARSE`s its parameter, which must give a version or `;*` (`%DELETE-E-DELVER`), and `$ERASE`s each file `$SEARCH` finds; with `/LOG`, `%DELETE-I-FILDEL, name deleted` for each |
| `INIT` | `$INIT_VOL` with its two parameters, the device and the label, and `/OWNER_UIC` and `/PROTECTION`'s items, which `sysexe/lib/protect.mar` reads |
| `MOUNT` | `$MOUNT` with its parameters, the device and the label, if there is one, and `/OWNER_UIC` and `/PROTECTION`'s items |
| `DISMOUNT` | `$DISMOU` with its parameter, the device |
| `SET` | `$SETPRV`s each privilege of `SET PROCESS/PRIVILEGES`, `NO` before one to disable it, `ALL` for every one, permanently; `%DCL-W-IVKEYW` for a name it doesn't know. `SET PROTECTION=(code[,...]) file` and `SET FILE/OWNER_UIC=uic file` `$OPEN` the file with a protection XAB, change it and `$CLOSE` it |
| `SHOW` | `SHOW PROCESS`: what `$GETJPI` says of the process, its UIC, and with `/PRIVILEGES` its authorized and current privileges; `SHOW SYSTEM`: a line per process |
| `CREATE` | `$CREATE_DIR` with its parameter, for `CREATE/DIRECTORY` |
| `STARTUP` | makes 4 pages with `$EXPREG`, checks and deletes them; creates `SLEEPER` at a higher priority, which runs at once, and `PING` and `PONG`; waits until `PONG` sets flag 66 of their cluster; deletes `SLEEPER`; creates `SVCTEST`, `HOG`, `TIMETEST`, `ASTTEST`, `MBXTEST`, `FSTEST1` and `FSTEST2` and `CHFTEST` |
| `SLEEPER` | hibernates until it is deleted |
| `PING`, `PONG` | take three turns through common event flags 64 and 65 of the cluster `PINGPONG`; `PONG` then sets flag 66, which `STARTUP` waits for |
| `SVCTEST` | checks the statuses of the services the others don't use, and of errors: local event flags, the dispatcher's checks and a stub, `$CRETVA` and `$DELTVA`, `$CMKRNL` and `$CMEXEC`, with privileges and without; a process in another UIC group, which takes `DETACH` and, to touch it, `WORLD`; what user mode may `PROBE`, and that services refuse it the executive's data; the console's channels; logical names in both tables, `$ASSIGN` through two of them, and the errors; `$SETPRI`, and `$SUSPND`, `$WAKE`, `$RESUME` and `$DELPRC` on a process of its own, and `$FORCEX` on another, which exits with `SS$_FORCEDEXIT` before its image runs; `$DCLEXH` and `$CANEXH`, and a `$FORCEX` of itself, whose `$EXIT` calls its exit handler, which says it is ok; then creates one whose image doesn't exist, which exits with `RMS$_FNF`, and `SNOOP` and `USURP` |
| `SNOOP` | reads S0 from user mode, which no handler takes: exits with `SS$_ACCVIO`, its message written |
| `USURP` | raises IPL from user mode, and exits the same way with `SS$_OPCDEC` |
| `HOG` | associates a common event flag cluster, creates `NUDGE` at its own priority and loops reading flag 64 until `NUDGE` sets it, with no wait: only quantum end lets `NUDGE` run |
| `NUDGE` | sets `HOG`'s flag |
| `TIMETEST` | checks that `$GETTIM` reads a time after 2026; waits for `$SETIMR`s, a delta and a time, 50 ms on, and that a cancelled one, due 200 ms on, never sets its flag; hibernates through three repeating `$SCHDWK` wakeups, cancels them, and checks that the next wakeup is a new one's |
| `PROTTEST` | creates `PROTCHILD`, the same image as a process of UIC `[200,1]` with no privileges, and reads its termination message: it reads `DKB0:[000000]DATA.TXT`, then may not give it another owner, delete it or make a file in `[000000]`. `PROTTEST` says so, or returns the status that stopped it, `RMS$_PRV` |
| `CTRLC` | enables a CTRL/C AST, starts a console read and waits for it; the AST, once CTRL/C is typed, `$CANCEL`s the read, which ends with `SS$_ABORT` |
| `HELP` | describes DCL's verbs, from `DCL$TABLES`, which `build.rs` links into it as into DCL: with no topic, each verb and its parameters, then what DCL does without a verb; with one, each verb whose name starts with it, its parameters, the keywords a parameter may be and the qualifiers, then each syntax a qualifier or keyword leads to that has parameters or qualifiers of its own. ponytail: no text, which VMS's HELP reads from a help library |
| `CLITEST` | parses commands with its own tables, `CLITEST.CLD`, and `CLI$DCL_PARSE`, and checks what `CLI$PRESENT` and `CLI$GET_VALUE` say of them: lists, concatenation, quoted strings, default values, negation, keywords and their values, a syntax switched to, abbreviations, each error, qualifiers given after a parameter's value, a `ROUTINE` `CLI$DISPATCH` calls, tables looked in first, and `LIB$GET_FOREIGN`'s line; run as a foreign command, or as a verb with an image, it writes the words after the verb |
| `ASTTEST` | checks that `$DCLAST`'s AST is delivered as the service returns, or when `$SETAST` enables ASTs again; that one declared in an AST routine waits until it returns; that a `$SETIMR` AST's `$WAKE` ends a `$HIBER`; and that one delivered while it computes in user mode leaves every register as it was |
| `CHFTEST` | checks condition handlers: a `LIB$SIGNAL` its handler continues with its own R0, at depth 0; one an inner routine's resignals, at depth 1; an access violation `LIB$SIG_TO_RET` makes its routine's status, its caller's registers as they were; a `BPT` its handler continues at another PC, its registers as they were; `$UNWIND` outside a handler; a warning no handler takes, written; and `LIB$STOP`, which exits through its exit handler, which says it is ok |

When every process but the swapper waits, the CPU idles in `WTINT`,
taking the clock's interrupts.

## Next

- Priority boosts on wake and decay at quantum end.
- Writing the system disk, the disk's interrupt, `$QIO` on disk
  channels, logical names in file specifications (`SYS$SYSTEM:DCL.EXE`)
  and `SYS$DISK`.
- Access modes and search lists for logical names.
- `INITIALIZE/SIZE`, and the index file extended past the headers
  `INITIALIZE` made room for.
