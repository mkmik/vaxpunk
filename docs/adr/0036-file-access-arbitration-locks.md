# ADR-0036 — Each open of a file holds an access arbitration lock on the lock manager, and an open file that is deleted goes at its last close

Oct 14, 2026 · @Marko Mikulicic

Proposed. Each RMS `$OPEN` and `$CREATE` takes a lock on its file
through the lock manager, as VMS's XQP does: one kernel mode, system
resource for each file, in a mode that the access the FAB asks for and
the sharing it allows decide, never queued. When another open's lock
blocks it, the open fails with `RMS$_FLK`. `$CLOSE` gives the lock back.
`$ERASE` of a file that is open removes its directory entry and marks it
for delete, and the last stream to close it deletes it, as on VMS. The
file system still serializes on one mutex, `FIL$LOCK`
([ADR-0020](0020-file-system-lock-below-ipl-synch.md) stands): there is
no volume lock yet. Step 7 of [PRD-0003](../prd/0003-multi-user-vms.md)'s
work order.

## Context

Until now RMS had no sharing. Two processes could open the same file
for writing, and each kept its own copy of the header in its IFAB, so
the second to close wrote its end of file over the first's. `$ERASE`
deleted a file that another process was reading, so that process read
blocks the file no longer owned.

VMS settles both in the XQP, from the *Guide to OpenVMS File
Applications* and the RMS reference manual:

- **Access arbitration.** Each access to a file holds a lock on it. Its
  mode follows from `FAB$B_FAC` and `FAB$B_SHR`. The lock manager's
  compatibility table then decides which opens can coexist. An open that
  conflicts fails with `RMS$_FLK`, "file currently locked by another
  user". A read with `FAB$B_SHR` 0 implies `SHRGET`; a write with 0
  shares nothing; `FAB$M_NIL` shares nothing either way.
- **Delete of an open file.** The XQP removes the directory entry and
  marks the header for delete (`FH2$V_MARKDEL`). The file is deleted when
  the last accessor closes it. A crash before then leaves a lost file,
  still marked, which `ANALYZE/DISK_STRUCTURE/REPAIR` cleans up.
- **Serialization.** The XQP takes a lock on the volume while it
  allocates, and a lock on each header it changes, so work on two
  volumes never waits for each other.

[ADR-0030](0030-lock-manager-in-the-executive.md) built the lock manager
for this step and expected this ADR to replace `FIL$LOCK` with a lock per
volume and per file. That needs the file system's state split per
volume. Today it is global: `F11$GL_VCB`, the directory and header
buffers, `F11$AB_IRP`, and RMS's parse tables, which a search list
spreads across volumes.

## Decision

1. **One access arbitration lock per open file**, taken by `$OPEN` once
   it has checked the access, and by `$CREATE` once the file exists. It
   is a kernel mode lock in the system's group, so a program can't take
   or release it. Its resource is `F11B$a`, the device's name and the file
   ID. `IFB$L_LKID` holds its ID, and the IFAB gives it back when it goes:
   `$CLOSE`, image and process rundown, or an open that fails later.
2. **The mode, from `FAB$B_FAC` and `FAB$B_SHR`.**

   | Access | Others may | Mode |
   | --- | --- | --- |
   | any | nothing, `FAB$M_NIL` | EX |
   | write | nothing, `FAB$B_SHR` 0 | EX |
   | write | read, or write too | PW |
   | read | read only, or `FAB$B_SHR` 0 | PR |
   | read | write, `SHRPUT`, `SHRUPD` or `SHRDEL` | CR |

   This is VMS's mapping, with one exception: a write that allows
   other writers is PW, where VMS uses CW. Two writers would each flush
   their own copy of the header and their own buffers. Until RMS has
   bucket and record locks and one header per file (PRD-0008's step 9),
   a second writer gets `RMS$_FLK` instead.
3. **Never queued.** RMS asks with `LCK$M_NOQUEUE`, as the XQP does for
   arbitration, and turns `SS$_NOTQUEUED` into `RMS$_FLK`. RMS calls
   `LCK$ENQ` and `LCK$DEQ` directly, at `IPL$_SYNCH`, while it holds
   `FIL$LOCK`. It passes no ACB: a request that is never queued completes
   when `LCK$ENQ` returns, so there is no kernel mode AST to wait for.
   That matters because the holder of `FIL$LOCK` runs at `IPL$_ASTDEL`,
   where such an AST couldn't be delivered.
4. **`$ERASE` of an open file marks it for delete.** To find out whether
   anyone has the file open, `$ERASE` asks for the file's lock in EX mode,
   unqueued, and lets it go at once. If it was granted, nobody has the file
   open and it is deleted as before. If not, `$ERASE` removes the directory
   entry, sets `FCH$V_MARKDEL` in the header on the disk, and sets
   `IFB$V_MARKDEL` and the header bit in every IFAB that has the file open,
   in every process. Each IFAB keeps a copy of the header, and a writer
   that closes writes its copy back, so the mark has to be in the copies
   too. When a stream closes a marked file, it gives back its lock and asks
   for EX the same way. If that is granted, it was the last stream: it reads
   the header again, which the writers have brought up to date, and deletes
   the file.
5. **`FIL$LOCK` stays.** It serializes the file system's global state,
   which nothing else protects, and RMS holds it around everything here.
   A lock per volume comes when that state is per volume
   (`t8y:/files/fs-locking#per-volume-state`).
6. **Process rundown closes the files before the locks go.**
   `EXE$RUNDOWN` closes the command interpreter's files before
   `LCK$RUNDOWN` dequeues the kernel mode locks. Otherwise another process
   could open such a file for writing in between, while the dying
   process's copy of the header was still to be written.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Refuse `$ERASE` of an open file, as PRD-0003's step 7 first said | VMS deletes it at the last close; `DELETE` of a log file another process writes works there |
| Replace `FIL$LOCK` with `$ENQ` locks now, one for the volume and one for each file | The global state they would protect isn't per volume, so one lock over all of it would still be needed; and waiting for a granted lock's kernel mode AST at `IPL$_ASTDEL` is impossible |
| Find conflicting opens by walking every process's IFABs, without locks | Works on one node, but repeats what the lock manager is for. `$GETLKI` would show no file locks, and the cluster path VMS has would be gone |
| CW for a writer that shares writing, as VMS | Two writers each write their own copy of the header and buffers, and the file loses one's records |
| Keep the mark for delete only in memory | A crash would leave a lost file that nothing marks; the bit costs one header write |
| `$ENQ` from RMS through the system service | It qualifies the resource by the caller's previous mode, user, so image rundown would dequeue the lock of a file the command interpreter keeps open |

## Consequences

**What gets harder.**
- ponytail: a second writer of a file gets `RMS$_FLK` even when both
  allow writing, until PRD-0008's step 9.
- ponytail: `FAB$M_UPI` and `FAB$M_MSE` are ignored.
- ponytail: the image activator reads images without a lock. `DELETE` of
  a running image deletes it, which is harmless today because the
  activator reads an image's sections in whole.
- ponytail: an error while deleting a marked file at its last close isn't
  reported. The file is left lost, as after a crash on VMS.
- `SET PROTECTION` opens the file, so it now fails with `RMS$_FLK` while
  another process writes that file. VMS's asks the XQP, which doesn't
  arbitrate (PRD-0003 tier 3 #23).
- A program that used to open the same file twice for writing now gets
  `RMS$_FLK`, as it would on VMS.

**What stays easy.**
- `TYPE` of a file another process writes says `%RMS-E-FLK`, as on VMS.
  A batch log will be readable once its writer opens it with
  `SHRGET`.
- The locks are ordinary lock manager locks, so `$GETLKI` lists them.
- The boot test checks it: while DCL has a file open for writing, `TYPE`
  and `OPEN/READ` get `%RMS-E-FLK`; two readers share it; `DELETE` takes
  its entry while it is open, a reader still reads it, and after the last
  `CLOSE` the data disk's check finds no lost file.

**Follow-ups.** The file system's state per volume, then a lock per
volume in place of `FIL$LOCK`, in an ADR that supersedes ADR-0020; CW
for writers that share, with PRD-0008's step 9; `OPEN/SHARE` in DCL,
whose qualifier is taken and ignored.
