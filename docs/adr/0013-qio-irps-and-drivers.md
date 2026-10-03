# ADR-0013 — $QIO queues an I/O request packet to the device's driver, and its completion is a kernel mode AST

Oct 3, 2026 · @Marko Mikulicic

Proposed. Each device has a unit control block (UCB) naming its driver,
and a channel holds its device's UCB. `$QIO` hands the request to the
driver, which makes an I/O request packet (IRP) and either does the I/O
at once or queues the IRP on the UCB and returns, so the caller goes on.
The driver ends a request with `IOC$REQCOM`, which queues the IRP to the
process as a kernel mode AST. In the process, that AST copies a read's
data to the caller's buffer, writes the I/O status block, sets the event
flag and becomes the caller's completion AST, if it gave one. `$QIOW` is
`$QIO` and a wait for the event flag. The console's terminal driver queues
reads and edits the line as characters come in; the disk driver reads and
writes the caller's buffer by LBN at once.

## Context

DESIGN-0002 had `$QIO` do the I/O in the caller's context before it
returned: a console read waited inside the service, editing the line, and
`$QIOW` was `$QIO`. It refused an AST address, and only the console could
be assigned a channel. Its ponytail said a terminal driver with I/O
request packets would replace it.

Much of VMS needs the request to outlive the service call. A program that
reads the terminal and goes on computing, or waits for a read or a timer,
whichever comes first, needs `$QIO` to return before the read is done and
an AST when it is. `$CANCEL`, CTRL/C and a CTRL/Y AST all act on a read
the driver holds, not on a process that waits inside one. On VMS the
request is an IRP: `$QIO` checks the arguments, the driver's FDT routines
check the function's, and the driver's start I/O routine works on the
queue. `IOC$REQCOM` puts a finished IRP on the I/O post queue, and the
IPL 4 interrupt hands each to its process as a special kernel mode AST,
which copies a buffered read's data from the system buffer to the
caller's and becomes the user's AST. The IRP's head is an ACB's.

[ADR-0011](0011-asts-on-the-kernel-stack.md) gave the executive kernel
mode ASTs that run an executive routine, `ACB$L_KAST`, in the process,
and listed `$QIO` completion ASTs as a follow-up. P0 and P1 are the
process's own ([ADR-0005](0005-access-modes-are-threads.md)), so only code
in that process can write the caller's buffer and I/O status block.

The console's output is synchronous: `MTPR` to `TXDB` writes a character
before it returns. So are the disks: the PAL's `READLBLK` and `WRITELBLK`
wait for the device, and the ramdisk is a copy.

## Decision

1. **A UCB per device**, `$UCBDEF` in `lib.mlb`: a queue of IRPs, the
   driver's FDT routine, and a disk's VCB. `qio.mar` has one each for
   `OPA0:`, `DKA0:`, `DKB0:` and `MDA0:`. `$ASSIGN` finds the device by
   name among them and keeps its UCB in the channel's slot of the PCB,
   `PCB$AL_CCB`.
2. **`$QIO` checks the channel and the I/O status block, clears the event
   flag, and calls the driver's FDT routine at `IPL$_SYNCH`**, with the
   service's arguments. The FDT routine checks the function and its
   parameters, which may fail the service, makes the IRP with
   `IOC$ALLOCIRP`, and either finishes the request or queues it.
3. **An IRP is an ACB with the request after it** (`$IRPDEF`): the
   channel, function, event flag, IOSB, the caller's buffer and its mode,
   and the I/O status once done. A buffered read's prompt and data follow
   it in the same block of pool, so freeing the IRP frees them, wherever
   it is: on the UCB, on an AST queue, or in `SCH$QAST` for a process
   that is gone.
4. **`IOC$REQCOM` completes a request at `IPL$_SYNCH`**: it puts the I/O
   status in the IRP and queues it to its process as a kernel mode AST
   whose routine is `IOC$POST`. That copies a buffered read's data to the
   caller's buffer, writes the IOSB and sets the event flag. With an AST
   address, the IRP is then queued again as an ACB of the caller's mode;
   else it is freed.
5. **`$QIOW` is `$QIO`, then a wait for the event flag** until the IOSB,
   if it gave one, is written, as VMS's `$SYNCH`.
6. **The terminal driver** (`ttdriver.mar`) writes at once, from the
   caller's buffer, in its FDT routine; a write doesn't wait behind a read
   that waits for input. A read is queued on `OPA0:`'s UCB. The one at the
   head writes its prompt and takes what is typed: the receive interrupt
   puts characters in the type-ahead buffer and requests `IPL$_IOPOST`,
   whose handler feeds them to the read at `IPL$_SYNCH`, which edits and
   echoes as before. A read that ends is completed, and the next starts.
7. **The disk driver** (`DK$FDT`, `f11.mar`) does `IO$_READLBLK` and
   `IO$_WRITELBLK`, and the `PBLK` ones, which are the same, on the
   caller's buffer, at once, as `FIL$READLBLK` and `FIL$WRITELBLK` do for
   the file system: direct I/O, a status in the IOSB.
8. **Image rundown, process rundown and `$DASSGN` cancel** the requests of
   the channels they give back, with `IOC$CANCEL`: the IRPs come off the
   UCBs and the process's AST queue and are freed, unfinished. This is
   before P0 goes, so no `IOC$POST` writes into it afterwards.
9. **CTRL/Y completes the stopped process's console reads with
   `SS$_CONTROLY`**, so the command interpreter's reads aren't queued
   behind the image's. After `CONTINUE`, the image finds that status in
   its IOSB, as on VMS.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Keep the I/O in the caller's context and add only the completion AST | `$QIO` would still return when the I/O is done, so a read can't be left pending while the program computes or waits for something else, and CTRL/C or `$CANCEL` would have to unwind a process waiting inside the service |
| VMS's I/O post queue and its `IPL$_IOPOST` interrupt between `IOC$REQCOM` and the AST | The drivers here complete at `IPL$_SYNCH`, where `SCH$QAST` can be called; the queue would only defer that call |
| Write the read's data into the caller's buffer from the driver | P0 and P1 belong to the process; the driver runs in whichever one was interrupted |
| A separate ACB for the completion AST | VMS reuses the IRP: no second allocation, and nothing to fail when the I/O is done |
| A system buffer apart from the IRP, as VMS's `IRP$L_SVAPTE` | Two blocks to free in every path that frees an IRP; one block bounds a request to 64 KB of pool, which `IRP$W_SIZE` can hold |
| DDBs, CRBs, IDBs and a DDT, as VMS | One UCB per device and controller, and one entry a driver needs, its FDT routine; the terminal's start I/O is local to it |
| Queue writes behind reads | A program with a read pending couldn't write; VMS's terminal driver writes while a read waits for input |
| Complete cancelled requests with `SS$_ABORT`, as VMS's `$CANCEL` | Rundown has no one left to tell; `$DASSGN` gives the channel up. A `$CANCEL` service would complete them |
| Keep the image's read and let CTRL/Y's command interpreter read anyway, as before | The command interpreter's read would queue behind the image's, which would take its command |

## Consequences

**What gets harder.**
- Every `$QIO` takes pool: `SS$_INSFMEM` when there is none, and
  `SS$_EXQUOTA` for a request over 64 KB. ponytail: no `BIOLM`, `DIOLM`
  or `BYTLM` quotas, so one process can take the pool with reads.
- A buffered read is copied twice, into the IRP and then to the caller.
- `IOC$POST` writes the caller's buffer and IOSB without probing them
  again: a program that deletes their pages while a read is pending takes
  the system down. ponytail: VMS probes or locks them.
- Echo and editing run at `IPL$_SYNCH` in whichever process the receive
  interrupt finds, and a write in the middle of a line being edited goes
  out as it is, without redisplaying the line.
- The disk driver writes any LBN of a disk that can be written, mounted or
  not, for any process: there are no privileges.

**What stays easy.**
- Every caller used `$QIOW`, which behaves as before.
- CTRL/C, a CTRL/Y AST with `IO$M_CTRLYAST`, `$CANCEL` and `$FORCEX`
  act on IRPs the terminal's UCB holds.
- A new device is a UCB and an FDT routine.

**Follow-ups:** `$CANCEL`; `IO$_SETMODE` with CTRL/C and CTRL/Y ASTs,
which replace `EXE$CTRLY` and `$CONTINUE`; `$FORCEX`; the file system
through `$QIO` to the disk, as the XQP does; quotas; mailboxes; a transmit
interrupt, so long writes don't hold `IPL$_SYNCH`.
