# ADR-0048 — An asynchronous RMS service runs on a stack of its own, stalls at its disk wait holding FIL$LOCK, and a kernel mode AST goes on with it

Nov 3, 2026 · @Marko Mikulicic

Proposed. A record service with `RAB$M_ASY` returns `RMS$_PENDING` when
it has to wait for the disk, and goes on once the disk's request has
ended, as VMS's RMS does (`RM$STALL`, `RM$STALLAST`). It runs on a stack
of its own, from pool, from the start; at the disk wait it switches back
to the service, which returns to the program, its process still holding
`FIL$LOCK`. The disk's completion queues the process a kernel mode AST
that switches to that stack again. PRD-0003 tier 3 #23
([t8y:/rms/async#asynchronous](../map/rms/async.md)).

## Context

Every RMS service ran to its end before it returned, `RAB$M_ASY` or not:
the file system waited for each disk request in `FIL$READLBLK`, holding
`FIL$LOCK` ([ADR-0047](0047-interrupt-driven-disk-io-and-locked-pages.md)
decision 6). VMS's RMS returns `RMS$_PENDING` from a service that must
wait, so a program can compute, or keep other streams going, while the
disk works; `$WAIT`, the completion routines or `RAB$L_STS` tell it when
the service has ended.

VMS's RMS runs in executive mode in its caller's process. `RM$STALL`
copies the stack between the service's entry and itself, and the
registers, into the stream's asynchronous save block (ASB), and returns
`RMS$_PENDING` through the service. The `$QIO`'s AST, an executive mode
AST, `RM$STALLAST`, copies the stack back onto the AST's stack and
returns into the code that stalled. RMS is written for that: nothing on
its stack points into its stack, and it probes with the caller's mode
saved in the IRAB.

vaxpunk's RMS runs in kernel mode, on the process's kernel stack. Its
routines are JSB routines on a CALL frame whose argument list is on that
stack: `AP` is `32(FP)`, and the frames point at each other. A stack copied
to another address doesn't work. And its probes take the previous mode
from the PSL ([DESIGN-0001](../design/0001-pal-interface.md)), which is
kernel in an AST.

The file system's buffers, `F11$AB_IRP` and `F11$GL_VCB` are one for all
volumes, and `FIL$LOCK` keeps them to the process that holds it
([ADR-0020](0020-file-system-lock-below-ipl-synch.md)). A stalled service
is in the middle of using them.

## Decision

1. **An asynchronous service runs on a stack of its own.** `RECSVC`
   takes `FIL$LOCK` and, for a record or block I/O service with
   `RAB$M_ASY` set, allocates an ASB from pool, 16 KB of stack, a
   kernel stack's, past its header, and calls the service's routine,
   `REC$name`, there (`THREAD`) with a copy of its argument list.
   `SWITCH` goes from one stack to the other, keeping the registers the
   standard keeps on each. A stream whose open locks records runs
   synchronously, as its record lock waits let go of `FIL$LOCK`; so does a
   service there is no pool for.
2. **It stalls at its disk wait.** In `FIL$READLBLK` and `FIL$WRITELBLK`,
   while the service runs on its stack (`RMS$GL_THREAD`), a request that
   didn't end at once stalls it (`RMS$STALL`) instead of waiting
   (`FIL$IOWAIT`): it switches back, and the service returns
   `RMS$_PENDING`, `RAB$L_STS` 0, at the IPL its caller had. Its process
   keeps `FIL$LOCK`. Its request has `IRP$M_NOLOOK`, which `DK$STARTIO`
   passes to `DISKSTART` as a5's 2: the PAL hands the device the request
   without looking a moment whether it is done (ADR-0047 decision 4), so
   the program, not a spin in the PAL, has the CPU meanwhile. A second
   service on the stream gets `RMS$_RSA`, as on VMS.
3. **A kernel mode AST goes on with it.** When `F11$AB_IRP` ends,
   `RMS$IODONE` queues the process the ASB's own ACB, a kernel mode AST,
   `RMS$STALLAST`, with a boost, `PRI$_IOCOM`; the AST switches to the
   service's stack (`RUNTHREAD`), which goes on until it ends, setting
   `RAB$L_STS` and queuing its completion routine, or stalls again. The
   ASB goes once it has ended.
4. **The previous mode is the caller's.** Before it goes on, the service
   makes its caller's mode the previous one (`SETPRV`, a `REI` from a
   frame of every register to the next instruction, as a VAX's `REI`
   sets it), so that RMS's probes check that mode's access; the AST gets
   its own back.
5. **The process ends its own stalled service before it takes the lock
   again.** `FIL$LOCK`, called by the process that holds the lock for its
   stalled service, `$WAIT` among them, waits for the request and goes on
   with the service itself until it ends (`RMS$FINISH`), then takes the
   lock in turn. Image rundown does the same first, before the service's
   RAB and buffers go, and so ends it for `$DELPRC` too, whose process is
   out of its slot and gets no AST for it.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Copy the stack into the ASB, and back at a new address, as VMS's RMS | vaxpunk's frames point into the stack: FP's chain and `AP` at `32(FP)` |
| Copy it back to the same address | The AST runs near the top of the same kernel stack, where the service's frames were |
| Leave the frames on the kernel stack and run the rest of the process below them | Every path that empties the kernel stack (`REIOUT`, `EXE$CLIENTRY`) would need to know |
| Let go of `FIL$LOCK` at the stall, as VMS's RMS lowers its file lock | The file system's buffers and its IRP are one for every volume and in use; that wants per-volume locks and buffers first |
| The AST in the caller's mode, which calls a service to go on | It waits on the program: ASTs it disables, or an AST routine that runs, would hold `FIL$LOCK` for every process |
| A probe mode saved in the ASB, as VMS's IRAB keeps it | Every probe in RMS would have to take it; the PSL's previous mode is what they read |
| `$WAIT` waits on an event flag, as VMS's | The AST can't run while the process waits for `FIL$LOCK` with ASTs blocked, nor in `$DELPRC`'s kernel mode AST; going on with it in place covers all three |
| One ASB per process, kept | Its stack would be held for the life of a process that used `RAB$M_ASY` once |

## Consequences

**What gets harder.**
- While a service is stalled its process holds `FIL$LOCK`, in user mode,
  and every other process's file system work waits until the disk's
  request ends and the AST runs; a process `$SUSPND` suspended then holds
  it until `$RESUME`.
- An asynchronous service takes 16 KB of pool while it runs.
- Going on after a stall costs two `REI`s, for the previous mode, and an
  interrupt and a kernel mode AST even when QEMU finished at once, as it
  usually does: an asynchronous read of a few blocks is slower than a
  synchronous one.
- The RAB and the buffers aren't probed again when the service goes on;
  VMS's RMS probes them.
- A stream that locks records runs synchronously.

**What stays easy.**
- RMS's code doesn't change: it runs on whichever stack, with the
  registers, IPL and previous mode it had.
- The names are VMS's: `RMS$_PENDING`, `RMS$_RSA`, the ASB, `RM$STALL`'s
  and `RM$STALLAST`'s jobs.
- Synchronous services are as they were.

**Follow-ups:** letting go of `FIL$LOCK` at the stall once the file system
has a lock and buffers per volume; read-ahead and write-behind
([t8y:/rms/sequential#read-ahead](../map/rms/sequential.md)); stalls for
record locks.
