# ADR-0030 — The lock manager is part of the executive, for one node, with VMS's $ENQ, $DEQ and $GETLKI, and its logic is BLISS-64

Oct 9, 2026 · @Marko Mikulicic

Proposed. `$ENQ`, `$ENQW`, `$DEQ` and `$GETLKI` are system services like
the others: `SYS$name` changes mode to kernel, and the executive keeps the
lock database, a resource block per resource and a lock block per lock,
in nonpaged pool, and changes it at `IPL$_SYNCH`. Names, modes, flags,
queues, the lock status block and the condition values are VMS's. There
is one node, so every resource is mastered here and there is no remote
side. The lock manager's logic is a BLISS-64 module, `LOCKING`, called
from MACRO-32 where IPL and the scheduler are involved. Step 6 of
[PRD-0003](../prd/0003-multi-user-vms.md)'s work order; its step 7 puts
the file system and RMS on it.

## Context

Milestone 1 has several users at once, and they share files. Today the
file system serializes with one mutex, `FIL$LOCK`
([ADR-0020](0020-file-system-lock-below-ipl-synch.md)), and RMS has no
sharing at all: `$ERASE` deletes an open file, and a second writer
overwrites the first. VMS solves both with its lock manager: the XQP
takes a lock per volume and per file, and RMS takes bucket and record
locks ([PRD-0008](../prd/0008-rms-record-and-indexed-files.md) step 9).
Programs use the same services to coordinate with each other, and the
job controller and `LOGINOUT` will too.

On VMS the lock manager is executive code, not a process. In a cluster
the same code sends a request for a resource another node masters as a
message over SCS and that node's executive answers; a single node never
sends one. Clusters are a non-goal of Milestone 1 (ADR-0003 holds: one
CPU), so the single-node part is all of it.

What VMS does, from the VSI *System Services Reference Manual* (`$ENQ`,
`$DEQ`, `$GETLKI`) and *Programming Concepts Manual*, volume I, chapter 7:

- **Six modes**, `LCK$K_NLMODE` to `LCK$K_EXMODE`, with the usual
  compatibility table.
- **A resource is its name** (1 to 31 bytes), **its access mode** (the
  less privileged of the caller's and `acmode`, or the parent lock's),
  **its UIC group** unless `LCK$M_SYSTEM`, which needs `SYSLCK` outside
  executive and kernel mode, **and its parent lock**, for a tree of
  sublocks. A sublock needs its parent granted.
- **Three queues per resource**: granted, conversion, waiting. A new
  request waits behind anything already waiting; a conversion keeps its
  old mode while it waits, and conversions are granted before new
  requests. Each queue is granted in order, up to the first that can't be.
- **The lock status block** is a status word, a reserved word and the
  lock ID, then the 16-byte value block with `LCK$M_VALBLK`. The
  completion AST and event flag come when the status is written;
  `$ENQW` waits for them.
- **Blocking ASTs** go to the holder of a granted lock that blocks
  another request.
- **Deadlock**: after `DEADLOCK_WAIT` seconds (10 by default), the system
  searches for a cycle from a waiting request and fails one request in it
  with `SS$_DEADLOCK` in its status block: a new lock isn't granted, a
  conversion goes back to its old mode. VMS documents no rule for which.
- **`ENQLM`** is a quota of locks; past it `$ENQ` fails with
  `SS$_EXENQLM`.
- **Image rundown** dequeues the user mode locks with `LCK$M_DEQALL` and
  `LCK$M_INVVALBLK`; process deletion dequeues the rest.
- **No DCL command lists locks.** `SHOW PROCESS/QUOTAS` prints the
  enqueue quota; SDA's `SHOW LOCKS`, `SHOW RESOURCES` and `SHOW
  PROCESS/LOCKS` and `MONITOR LOCK` are where VMS shows them.

vaxpunk's STARLET has no `$LCKDEF` or `$LKIDEF` and none of the lock
manager's `SS$_` codes; only `RMS$_DEADLOCK` and `RMS$_EXENQLM`.

## Decision

1. **In the executive, in kernel mode.** `$ENQ`, `$ENQW`, `$DEQ` and
   `$GETLKI` (and `$GETLKIW`, which is the same, as `$GETJPIW` is) go in
   `syssrv.mar`'s vector with `SERVICE`. They change the lock database at
   `IPL$_SYNCH`, as the rest of the executive changes PCBs; the file
   system calls the same routines from kernel mode, without the change
   of mode.
2. **The database.** A resource block (`RSB`) holds the name, its
   length, access mode and group, the parent `RSB`, the three queues, the
   value block and a count of locks; a lock block (`LKB`) holds the
   granted and requested modes, which queue it is on, the owner's PID,
   the status block's address, the event flag, both ASTs and their
   parameter, the parent `LKB`, and its place in its owner's list of
   locks in the PCB. Both come from nonpaged pool. `RSB`s are found
   through a hash table of the qualified name. A lock ID is an index in a
   lock ID table with a sequence number above it, so a stale ID fails
   with `SS$_IVLOCKID` instead of naming the next lock in that slot.
3. **VMS's interface, a subset of it.** The argument lists, `$LCKDEF`,
   `$LKIDEF`, the status block, the qualification of names and the order
   of granting are VMS's. The `$ENQ` flags are `VALBLK`, `CONVERT`,
   `NOQUEUE`, `SYNCSTS` and `SYSTEM`; the `$DEQ` flags `DEQALL` and
   `INVVALBLK`; any other fails with `SS$_BADPARAM` until a program needs
   it. The `$GETLKI` items are those `LOCKTEST` and a lock display need:
   `LOCKID`, `PID`, `RESNAM`, `STATE`, `PARENT`, `NAMSPACE`, `VALBLK`,
   `LOCKS`, `BLOCKING` and `BLOCKEDBY`. Symbol values come from real
   OpenVMS (the AXPbox oracle's STARLET), not from guesses: VSI's manuals
   print the names but not the numbers.
4. **Completion.** Granting a lock writes the status block, sets the event
   flag and queues the completion AST with `SCH$QAST`, in the owner's
   access mode. `$ENQW` waits for the event flag and the status, as
   `$QIOW` does. A granted lock with a blocking AST gets it once for each
   time it starts blocking a request; a conversion arms it again.
5. **Deadlock search when a request has to wait**, not after
   `DEADLOCK_WAIT`. A cycle can only close when a request is queued, so
   searching from that request, along "waits for the holder of", finds
   every cycle, and the request that closed it is the one that fails.
   VMS's choice of victim is undocumented, so no program may depend on
   either.
6. **Rundown as VMS's.** `EXE$IMGRUNDOWN` dequeues the user mode locks
   with `DEQALL` and `INVVALBLK`; `EXE$RUNDOWN` dequeues all of them,
   before the PCB goes.
7. **BLISS-64 for the logic, MACRO-32 at the edges.** Queues, granting,
   conversions, deadlock search and `$GETLKI` are a BLISS-64 module,
   `vms/exec/locking.b64`, named after VMS's `LOCKING` execlet. Raising
   and lowering IPL, waiting, and queueing ASTs stay in MACRO-32, which
   the module calls through `LINKAGE`s for their JSB register contracts
   ([ADR-0023](0023-calling-standard.md)).

## Alternatives considered

| Option | Why not |
| --- | --- |
| A lock manager process, asked through a mailbox or a `$QIO` | Every file open and every RMS record lock would be two context switches; VMS's isn't one; the file system would wait on another process while it holds a file |
| A cluster-ready design now: masters, directory node, messages | Clusters are a non-goal; the interface is the same, so the remote side can be added where VMS added it, under `$ENQ` |
| Search for deadlocks after `DEADLOCK_WAIT` seconds, as VMS | A timer scan of every waiting lock, and `LOCKTEST` would wait 10 s to see one; with one node and few locks the search at queueing time is cheap |
| A lock database of fixed tables | Pool is what the rest of the executive allocates from; fixed tables cap locks below `ENQLM` or waste pool |
| Every `$LCKDEF` flag from the start | `EXPEDITE`, `QUECVT`, `NODLCKWT`, `CVTSYS` and the rest have no caller yet; failing them is visible, ignoring them would not be |
| MACRO-32 throughout | The lock manager is long logic with little hardware in it, which is where MACRO-32's register contracts cost most ([PRD-0004](../prd/0004-bliss64-compiler.md)'s context); BLISS-64 is there now |

## Consequences

**What gets harder.**
- This is the first BLISS-64 in kernel mode: `numtim.b64` runs in the
  caller's mode, from the vector. A routine vbliss can't yet compile
  well is written in MACRO-32 instead, and the gap goes to PRD-0004.
- STARLET grows `$LCKDEF`, `$LKIDEF` and the codes `SS$_NOTQUEUED`,
  `SS$_DEADLOCK`, `SS$_CVTUNGRANT`, `SS$_EXDEPTH`, `SS$_EXENQLM`,
  `SS$_IVLOCKID`, `SS$_NOLOCKID`, `SS$_NOSYSLCK`, `SS$_PARNOTGRANT`,
  `SS$_SUBLOCKS`, `SS$_VALNOTVALID`, `SS$_ABORT`, `SS$_CANCELGRANT` and
  `SS$_SYNCH`, with their messages in `getmsg.mar`, and `vdefs`
  regenerates the BLISS require files.
- ponytail: `ENQLM` isn't charged until PRD-0003's step 5 charges the
  quotas; until then a process can fill pool with locks.
- ponytail: the deadlock search runs at `IPL$_SYNCH` on every wait; with
  many locks it becomes a timer scan, as VMS's.

**What stays easy.**
- Programs written for VMS's `$ENQ` run unchanged within the subset.
- `LOCKTEST`, like `ASTTEST`, checks it at boot in two processes:
  compatibility, conversions, value blocks, blocking ASTs, `NOQUEUE`, a
  sublock, and a deadlock.

**Follow-ups.**
- PRD-0003 step 7: the file system on a lock per volume and per file
  needs the file system's buffers per volume, which is why ADR-0020 kept
  one lock; that step's ADR supersedes ADR-0020.
- PRD-0008 step 9: RMS's bucket and record locks, `$FREE`, `$RELEASE`.
- A lock display, if one is wanted: SDA's `SHOW LOCKS` is VMS's, and
  there is no SDA yet.
