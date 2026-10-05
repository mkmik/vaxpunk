# ADR-0020 — The file system runs below IPL$_SYNCH, holding a lock of its own, as the XQP does

Oct 5, 2026 · @Marko Mikulicic

Proposed. RMS, the volume services, `$GETDVI` and the image activator
no longer raise IPL to `IPL$_SYNCH` to use the file system. They take its
lock, `FIL$LOCK`, which a process holds at `IPL$_ASTDEL` and gives back
with `FIL$UNLOCK`; a process that wants it while another holds it waits,
in `SCH$C_MWAIT`. Only what `IPL$_SYNCH` protects is still done there:
the driver's start I/O routine, logical names and the PCBs. The disks'
driver also counts the errors the device reports in the UCB,
`UCB$L_ERRCNT`, which `$GETDVI` and `SHOW DEVICES` return.

## Context

[ADR-0007](0007-system-disk-files-11-and-rms.md) ran the file system at
`IPL$_SYNCH`, which kept its buffers to one caller for free, and
[ADR-0015](0015-file-system-io-through-the-disk-driver.md) kept it there
when its I/O went through the disk's driver. Its follow-ups were the
file system below `IPL$_SYNCH`, with the XQP's lock, and error counts in
the UCB.

At `IPL$_SYNCH` nothing else runs: no reschedule, no timer, no console
input or network fork. A `COPY` or a `DIRECTORY` holds the CPU for every
block it reads, and the PAL's disk reads wait for the device. A driver
that queues its IRP and finishes it at interrupt level can't be waited
for there either, since the wait needs IPL below the interrupt's.

VMS's XQP runs in the process that asked, in kernel mode, and serializes
with the lock manager: a lock on the volume, and one on each file it
works on. Holding them it runs at `IPL$_ASTDEL`, as VMS's code that holds
a mutex does, so no kernel mode AST, process deletion among them, comes
while it holds one.

`SHOW DEVICES` printed 0 for every error count: nothing kept one.

## Decision

1. **One lock, `FIL$LOCK`, for the file system.** `OWNER` holds its
   holder's PCB. A process that finds it held waits on its own queue, in
   `SCH$C_MWAIT`, and tries again when `FIL$UNLOCK` wakes every waiter.
2. **The holder runs at `IPL$_ASTDEL`**, or the IPL it came with if that
   is higher, which `FIL$UNLOCK` puts back. Reschedules, the timer and
   I/O interrupts come; kernel mode ASTs wait.
3. **`IPL$_SYNCH` only where it is needed**: `FIL$READLBLK` raises it
   around the start I/O routine; `RMS$PARSE` from the first logical name
   translation until `MERGE` has copied the parts it found in logical
   names' blocks, which another process may delete; `RMS$VOLIDLE` while
   it looks at every PCB; `$GETDVI` while it translates the device name.
4. **Every entry to the file system takes the lock**: each RMS service,
   `RMS$RUNDOWN`, `$MOUNT`, `$DISMOU`, `$INIT_VOL`, `$GETDVI` and
   `FIL$OPENFILE`. `FIL$MOUNT` at boot doesn't: no other process is there.
5. **`UCB$L_ERRCNT`**, a longword after `UCB$L_START`, counts
   `SS$_DRVERR`, the PAL's word that the device failed. `DK$STARTIO`
   counts it; `DVI$_ERRCNT` returns a disk's, 0 for the console.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Stay at `IPL$_SYNCH` | Nothing else runs while a process reads a file, and a driver that queues can't be waited for |
| A lock per volume, as the XQP's | The file system's buffers, `F11$GL_VCB` and RMS's are global, not per volume; splitting them is the work that would make per-volume locks worth having |
| `$ENQ` and the lock manager | There is no lock manager; a mutex with a wait queue is what VMS's own executive mutexes are |
| Hold the lock at IPL 0 | A `$DELPRC`'s kernel mode AST could then run the holder down while it holds it, and its rundown would wait for its own lock |
| Wake only the first waiter | It might be deleted before it runs again, and the others would wait for nothing; there are few waiters |
| Count every failed request as an error | `SS$_ILLBLKNUM`, `SS$_WRITLCK` and the `SS$_NOSUCHDEV` of a disk that isn't there are the request's or the configuration's, and `$GETDVI`'s `STS` reads a missing disk on every `SHOW DEVICES` |

## Consequences

**What gets harder.**
- Code in the file system must not take the lock again, and must not
  wait: it holds the lock at `IPL$_ASTDEL`. ponytail: no check for
  either.
- A process waiting for the lock gets its ASTs, as in any wait, and may
  be deleted there, which is fine: it holds nothing.
- ponytail: no priority boost for the holder, so a lower priority holder
  delays a higher one that waits, as long as something between them
  computes.

**What stays easy.**
- Code that ran at `IPL$_SYNCH` in the file system runs unchanged under
  the lock: the routines between `FIL$LOCK` and `FIL$UNLOCK` are the
  ones that were between `DSBINT` and `ENBINT`.
- A driver that queues needs `FIL$READLBLK` to wait for its IRP, which it
  now may.
- FSTEST checks it at boot: two processes count the files in
  `SYS$SYSTEM:` at once, and wait for each other's lock.

**Follow-ups:** `FIL$READLBLK` waiting for a queued IRP, with an
interrupt-driven virtio disk; operation counts in the UCB; per-volume
locks once the file system's buffers are per volume.
