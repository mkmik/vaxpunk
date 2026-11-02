# ADR-0047 — A disk request ends by interrupt, the device reads and writes the requester's pages by PFN, and those pages stay locked until it is done

Nov 2, 2026 · @Marko Mikulicic

Proposed. The disks are interrupt driven, as VMS's are. The disk driver
hands the device one request at a time and queues the others on the
UCB (`EXE$INSIOQ`); the PAL starts the request with `DISKSTART` and
returns at once; the device's interrupt, through seL4's IRQ handler,
reaches the executive at IPL 21 through a vector of its own, whose
routine forks to `IPL$_SYNCH`, completes the IRP and starts the next.
The device reads and writes the requester's own pages, which the PAL
finds by PFN, so the request no longer depends on which process is
current when it ends. Those pages are locked for the I/O, as VMS's
`MMG$IOLOCK` locks them: a count in the PFN database keeps a page that
is deleted meanwhile off the free list until the last lock goes. The file
system waits for its IRP while other processes run, and rundown waits
for a request the device has. PRD-0003 tier 4 #37, tier 2 #18 and the
direct I/O half of #10 ([t8y:/drivers/interrupt-io](../map/drivers/interrupt-io.md),
[t8y:/drivers/disk#queued](../map/drivers/disk.md),
[t8y:/files/ods2#queued-io](../map/files/ods2.md)).

## Context

The PAL drove the disks polled
([ADR-0007](0007-system-disk-files-11-and-rms.md),
[ADR-0015](0015-file-system-io-through-the-disk-driver.md)): `READLBLK`
copied the data through a page of the PAL's and spun until the device
was done. With one VMS CPU ([ADR-0003](0003-one-cpu-many-threads.md)) a
PAL call is one instruction, so while a disk worked no process ran, and
ticks that came in the meantime were one tick. Asynchronous RMS
(`t8y:/rms/async`), paging, tapes and network drivers all need a device
to finish a request while the CPU does something else.

Most of what that takes was there. seL4 delivers a device's interrupt
to a notification, as it does virtio-net's to the TCP/IP component; the
PAL turns a notification into an interrupt request at an IPL and
delivers it through the SCB, as it does the port's completion
([DESIGN-0003](../design/0003-tcpip-port.md)); `NET$INTERRUPT` and
`NET$FORK` show the executive's half, an interrupt routine that forks
down to `IPL$_SYNCH` and completes IRPs there.

What wasn't there is how a device gets at the requester's memory once
that is no longer the current process's. `READLBLK` found a buffer
through the current process's PTEs; by the time an interrupt comes,
another process may be current, and the requester may have deleted the
pages, so their PFNs may be someone else's.

On VMS, a driver's FDT routine locks a direct I/O's buffer with
`EXE$READLOCK` or `EXE$WRITELOCK`, which call `MMG$IOLOCK`: each page is
faulted in, checked for the caller's access, and its PFN's reference
count raised, and the IRP keeps `IRP$L_SVAPTE`, the address of the
buffer's PTEs, and `IRP$L_BOFF`, its offset in the first page. The
driver loads the PFNs into the adapter's map registers and starts the
transfer. `IOC$IOPOST` unlocks the pages with `MMG$UNLOCK`. A page
deleted while its PFN's count is up isn't freed: the PFN is marked
delete contingent (`PFN$V_DELCON`) and goes to the free list when the
count reaches 0. Process rundown and `$DASSGN` cancel a channel's I/O and
wait, in `RWAST`, until the requests the device has are done.

## Decision

1. **A disk has a request at a time.** `EXE$INSIOQ` starts an IRP
   (`IOC$INITIATE`) if the device has none, `UCB$V_BSY` clear, and
   queues it on `UCB$L_IOQFL` if it has; `UCB$L_IRP`, in a disk's UCB,
   is the one it has. `IOC$REQCOM` starts the next, or clears
   `UCB$V_BSY`, when the IRP it completes is the device's: in a loop, so
   a queue of requests the start I/O routine completes at once, a bad LBN
   or no bytes, doesn't nest one completion in another.
2. **A direct I/O's pages are locked by PFN.** `DK$FDT` makes the IRP
   with room for 17 PFNs, a 64 KB buffer's pages, past the packet, and
   calls `EXE$READLOCK` or `EXE$WRITELOCK`. They call `MMG$IOLOCK`,
   which reads each page's PTE with a new PAL call, `RDPTE`, checks the
   caller's mode may write (read) it, raises a word count per PFN in a
   new table of the PFN database, `MMG$GL_PFNREFCNT`, or fails with
   `SS$_EXQUOTA` once it holds 32,767, and puts the PFN in the list; `IRP$L_BOFF` is the offset in the first page, and
   `IRP$V_LOCKED` says it holds the locks. `IOC$REQCOM` unlocks them
   (`MMG$UNLOCK`), and so does `IOC$CANCELQ` for a request it frees
   unstarted.
3. **A locked page's deletion is deferred.** Every page's deletion frees
   its PFN through `FREEPFN`: with a count up, it sets the count's top
   bit, `DELCON`, instead, and `MMG$UNLOCK` frees the PFN when the last
   lock goes. A global section's PFNs go the same way when the section
   is deleted.
4. **The PAL starts the request and returns.** `DISKSTART` takes the PFN
   list, the byte count, the LBN, the unit, the offset and read or
   write, gives the device a descriptor per page, the frame's physical
   address from seL4, and returns 0; or a status at once, for a bad
   argument or no bytes. A last block in part goes through a block of
   padding in the PAL's queue page. Each disk's queue has 32
   descriptors, the entropy device's still 4.
5. **The device's interrupt is the executive's at IPL 21.** The PAL
   takes each disk's IRQ, SPI 16 plus its transport, on its notification
   with a badge per unit. When the request is done it keeps the status
   and requests IPL 21, which now has two vectors: the port's, 0x100,
   and the disks', `SCB$L_DISK`, 0x104, delivered one after the other.
   `DK$INTERRUPT` requests the `IPL$_SYNCH` software interrupt,
   `DK$FORK`, which asks each disk with `DISKDONE` and completes its IRP.
6. **The file system waits for its IRP.** `FIL$READLBLK` and
   `FIL$WRITELBLK` lock the buffer, queue `F11$AB_IRP` and wait in
   resource wait, holding `FIL$LOCK`, while other processes run, on a
   queue of the driver's (`IOWAIT`), whose waits each disk request's end
   ends, each waiter looking again at what it waits for (`IOWAKE`).
   The wait delivers no AST, so no kernel mode AST deletes the process
   while it holds the lock ([ADR-0020](0020-file-system-lock-below-ipl-synch.md)).
   Above `IPL$_ASTDEL`, at boot before the scheduler runs, they use the
   PAL's polled `READLBLK` and `WRITELBLK`, which first wait for a request
   the device has.
7. **Rundown waits for the device.** `DK$CANCEL`, the disks' cancel
   routine, waits while the device has a request of the process's on
   the channel, for rundown and `$DASSGN`, as VMS's `RWAST` wait does:
   the IOSB and event flag are the process's until then. It waits in
   `IOWAIT` too, since a process `$DELPRC` deletes is no longer found by
   its PID when its IRP completes. `$CANCEL` lets
   such a request end as it would have.
8. **`$QIO` clears the IOSB**, as VMS's does: a disk request is no
   longer done when `$QIO` returns, so a status left from before must not
   end `$QIOW`'s wait.
9. **The idle loop looks for work after lowering IPL.** `SCH$SCHED`
   checks the computable queues again once `IPL$_RESCHED` has let the
   pending forks through, before `WTINT`, so a completion that came while
   it looked doesn't wait for the next tick.

## Alternatives considered

| Option | Why not |
| --- | --- |
| The PAL copies through its own page and back at the interrupt | It would still have to find the requester's pages after another process became current, and hold them: the same locking, plus a copy |
| Rundown waits for every I/O, and pages are never deleted with I/O pending | `$DELTVA`, `$EXPREG`'s contraction and image rundown may all delete a page a read still targets; VMS covers that with delete contingency, not by forbidding it |
| `DISKSTART` takes a virtual address and the PAL resolves it | The executive must count the locks to defer the PFNs' freeing, so it needs the PFNs itself; and the PAL would resolve through whatever process is current |
| Lock with `$LCKPAG` | `$LCKPAG` locks pages in the working set against paging; a device needs the frames kept from reuse, which is the PFN's count, as on VMS |
| A count per IRP of locked pages kept in the IRP | The count follows from `IRP$L_BOFF` and `IRP$L_BCNT`, as VMS's `MMG$UNLOCK` works it out |
| The disk interrupt at IPL 22 so that it has a level of its own | VAX and Alpha disks interrupted at 20 or 21, beside other devices; two vectors at one IPL is what the VAX's UNIBUS did |
| VMS's fork queue, `EXE$FORK`, with fork blocks | One driver forks; its routine looks at each disk. A queue when a second driver forks |
| Wait in `SCH$WAIT` | It delivers ASTs, kernel mode's among them, which may delete the process while it holds `FIL$LOCK`, or run a user mode AST in the middle of rundown |

## Consequences

**What gets harder.**
- The PFN database is 10 bytes a PFN, not 8: 128 MB of S0 indexes some
  51 GB of RAM, not 64 GB.
- A disk has 32 descriptors and an IRQ, and the PAL's `tick` handles a
  disk's badge.
- A request a disk has can't be cancelled; rundown waits for it.
- The file system's I/O switches processes; one process's file system
  I/O still waits for another's, as `FIL$LOCK` is one for every volume.

**What stays easy.**
- The executive's drivers look like VMS's: `EXE$INSIOQ`, `IOC$INITIATE`,
  `IOC$REQCOM`, `UCB$V_BSY`, `UCB$L_IRP`, `EXE$READLOCK`, `MMG$IOLOCK`,
  delete contingency.
- A buffer in P0 is as good as one in S0 to the device.
- The polled calls stay for the PAL's boot and the executive's.

**Follow-ups:** asynchronous RMS (`t8y:/rms/async#asynchronous`); more
than one request in a disk's queue at a time, with the device's own
ordering, if a measurement wants it; the console UART by interrupt
(PRD-0003 tier 4 #38); a fork queue when another driver forks;
`IOC$POST` probing a buffered read's buffer and the IOSB (the rest of
tier 2 #10).
