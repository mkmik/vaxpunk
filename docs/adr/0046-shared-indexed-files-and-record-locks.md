# ADR-0046 — Writers share an indexed file in CW mode, each open keeps the file's header and prologue in step under FIL$LOCK, and record locks are sublocks of the access arbitration lock

Oct 30, 2026 · @Marko Mikulicic

Proposed. An indexed file opened for record I/O by writers that let
others write is arbitrated in CW mode, as VMS's XQP does, so several
processes, or several streams of one, write it at once. RMS services
still run one at a time, holding `FIL$LOCK`, so they need no bucket
locks: a service that changes the file gives its other opens, in any
process, its copy of the header, the key and area descriptors, and they
forget the buckets they kept (`RMS$SYNC`). A stream that reads a record
of a file shared with a writer locks it, as VMS's RMS does, with a
kernel mode sublock of its open's access arbitration lock named by the
record's RFA (`RMS$LOCKREC`), and a stream that has to wait lets go of
`FIL$LOCK` while it does. This replaces the exception in decision 2 of
[ADR-0036](0036-file-access-arbitration-locks.md), PW for a writer that
shares writing, for indexed files. PRD-0008's step 9
([t8y:/rms/sharing#record-locks](../map/rms/sharing.md),
[t8y:/rms/indexed#shared](../map/rms/indexed.md)).

## Context

[ADR-0036](0036-file-access-arbitration-locks.md) arbitrated opens as the
XQP does but gave a second writer `RMS$_FLK`: each IFAB kept its own copy
of the file's header, and a writer's buffers and descriptors were its
own, so two writers would lose each other's changes. A program that
expects VMS's sharing, two sessions updating different records of one
file, couldn't run.

On VMS (the *Guide to OpenVMS File Applications*, chapter 7, the RMS
reference manual's `$GET`, `$FREE` and `$RELEASE`, and the V4.3 sources,
read for the design only):

- A writer that lets others write holds the arbitration lock in CW, a
  reader that lets others write in CR, a writer that lets others only
  read in PW; all of those lock records.
- RMS keeps its own file lock, `RMS$` and the file ID, with a sublock
  for each bucket it works in and each record a stream holds. Bucket
  locks keep the buckets in each process's buffers current, through the
  lock value block's sequence number.
- A writer's `$GET` and `$FIND` lock the record EX, PW with `RAB$M_RLK`,
  PR with `RAB$M_REA`; a reader, or `RAB$M_NLK`, only asks whether it
  may read it. Another stream's lock gives `RMS$_RLK`, or with
  `RAB$M_RRL` the record anyway, `RMS$_OK_RRL`, or with `RAB$M_WAT` a
  wait, `RMS$_OK_WAT`. The stream's next operation drops the lock, unless
  `RAB$M_ULK` took it: then `$FREE` and `$RELEASE` do. `$UPDATE` and
  `$DELETE` need the stream's own EX or PW lock, else `RMS$_RNL`. Locks
  belong to the stream, so two streams of one process conflict.

OpenVMS Alpha V8.4 in AXPbox confirmed the DCL side, two and three
streams in one process (`crosstools/ods/fixtures/rms/make/lock.com`, its
output `lock.log`), and a SPAWNed process that waits for a record.

## Decision

1. **CW for an indexed file's writers that share writing.** `ARBLOCK`
   asks for CW where ADR-0036 asked for PW, for an indexed file opened
   for record I/O; CW, PW and CR opens of it set `IFB$V_RLK`, record
   locking. Relative and sequential files, and block I/O, keep PW.
2. **One copy of the file's state, kept in step.** Every service runs
   holding `FIL$LOCK`, so the only thing two opens can disagree on is
   what each keeps between services: the header, the key and area
   descriptors, and the buckets in its slots. A service that wrote gives
   the others its copies (`RMS$SYNC`, `IDX$PEER`), and `$OPEN` takes the
   newest header from another open (`RMS$TAKEHDR`). No bucket locks.
3. **Record locks are sublocks of the open's arbitration lock.** Kernel
   mode, the system's, named by the RFA's ID and VBN, 8 bytes, as VMS's
   record locks are named; the arbitration locks of all opens of a file
   are on one resource, so their sublocks meet on one too. Never queued,
   but with `RAB$M_WAT`. Each stream keeps its locks in a list of RLBs
   from its IFAB, and gives them back at `$FREE`, `$REWIND`,
   `$DISCONNECT`, `$CLOSE` and when its IFAB goes, before the
   arbitration lock: kernel mode locks outlive the image.
4. **A wait lets go of `FIL$LOCK`.** Its holder runs at `IPL$_ASTDEL`,
   where the grant's kernel mode AST can't come. `RECENQ` gives the lock
   back, queues the request and waits on an event flag VMS keeps for
   itself (`LCK$ENQWK`), takes `FIL$LOCK` again and finds the record
   again by its RFA. The lock manager's deadlock search ends a wait that
   closes a cycle, a stream's wait for another of its own process's
   too: `RMS$_DEADLOCK`.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Bucket locks, as VMS | Under `FIL$LOCK` no two services meet in a bucket; they would buy nothing until RMS runs in executive mode without it |
| A shared block per file, VMS's SIFAB, holding the header and descriptors | One more structure to find, count and free, where copying into the few other opens, under the lock that already orders them, is enough |
| The header written at each extension and read again by the other opens | A disk write and read for each, and every open still needs telling its copy is stale |
| An `RMS$` file lock for record locks, as VMS | A second lock per open, whose only use would be a parent the arbitration lock already is |
| Polling for a locked record with `RAB$M_WAT` | No order among waiters, and no deadlock search |
| CW for relative files too | `rmsrel.mar` keeps a bucket in the IFAB and writes it late, with its own end of file: two writers would lose records until it keeps them in step as indexed files do |

## Consequences

**What gets harder.**
- ponytail: `RAB$M_TMO` isn't there: a wait lasts until the lock is
  granted or the deadlock search ends it; nor DCL's `READ/TIME_OUT`.
- ponytail: relative files still have one writer at a time, and no
  record locks (`t8y:/rms/relative#shared`).
- ponytail: one stream per open, as before: `FAB$M_MSE` and `FAB$M_UPI`
  are ignored (`t8y:/rms/sharing#streams`).
- A service that writes walks every process's open files to find the
  others; few processes have many files open.

**What stays easy.**
- Moving RMS to executive mode in P1 (PRD-0003 backlog item 23) is when
  bucket locks come, and the sync copies become their value blocks.
