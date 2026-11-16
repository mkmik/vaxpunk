# ADR-0049 — A file opened on a disk channel is a WCB in the PCB, read and written by VBN through the XQP in the process, and the XQP creates, deletes and looks up by wildcard

Nov 16, 2026 · @Marko Mikulicic

Proposed. A program opens a file on a channel to its disk with
`IO$_ACCESS!IO$M_ACCESS`, or makes one with `IO$_CREATE`, then reads and
writes its blocks with `IO$_READVBLK` and `IO$_WRITEVBLK` and closes it
with `IO$_DEACCESS`, as on VMS. `IO$_DELETE` removes a directory entry
and deletes a file, `IO$_ACPCONTROL` takes the volume's other functions,
and a lookup by a name with wildcards goes on from `FIB$L_WCC`. The file
open on channel n is a block of pool, a WCB, in the process's PCB at
`PCB$AL_WCB`, and every one of these functions runs in `XQP$PERFORM`,
as `IO$_ACCESS` and `IO$_MODIFY` do since
[ADR-0039](0039-xqp-qio-functions.md). The rest of
`t8y:/files/ods2#qio-interface`.

## Context

ADR-0039 gave disks the XQP's `IO$_ACCESS` and `IO$_MODIFY`, which read
and write a file's attributes, and left out the rest of the interface:
no file could be open on a channel, so there was no `IO$_DEACCESS` and no
virtual I/O; no `IO$_CREATE`, `IO$_DELETE` or `IO$_ACPCONTROL`; and a
name with wildcards found only its first match. Those are what VMS's own
programs use when they don't go through RMS: `LIB$CREATE_DIR` makes a
directory with `IO$_CREATE`, `SET VOLUME` rewrites the home block through
`INDEXF.SYS` open on a channel, `BACKUP` restores a file's header and
blocks, `DUMP/HEADER` reads one, and file-backed global sections map
the file open on a channel. [The research](../research/files-11-acp-qio.md)
and the VAX/VMS V4.3 XQP (`ACCESS`, `CREATE`, `DEACCS`, `DELETE`,
`FIND`, `ACPCNTRL`, `LOCKERS`, `SYSACPFDT`) give the rules.

VMS keeps the file open on a channel as a window control block, a WCB,
which the channel's CCB points at (`CCB$L_WIND`): the file's map, and a
link to its FCB, the file's state every open shares. vaxpunk has no CCB:
`PCB$AL_CCB` holds the channel's UCB. RMS keeps each open file in an
IFAB with a copy of its header, and each open holds the file's access
arbitration lock ([ADR-0036](0036-file-access-arbitration-locks.md)).

## Decision

1. **The file open on channel n is a WCB at `PCB$AL_WCB` + 4n**: a block
   of pool with the file's volume, its access arbitration lock and the
   lock's mode, whether it is open for writing and was written, and a
   copy of its header, whose map turns a VBN into an LBN (`$WCBDEF`).
2. **`DK$FDT` makes VMS's checks and leaves the IRP for `$QIO`**, as for
   `IO$_ACCESS` and `IO$_MODIFY`: `IO$_DEACCESS` and the virtual
   functions need a file open on the channel, else `SS$_FILNOTACC`,
   `IO$M_ACCESS` none, else `SS$_FILALRACC`, and the caller must be able
   to write (read) the buffer. `XQP$PERFORM` then does every function in
   the process, holding `FIL$LOCK`.
3. **The virtual functions go through a buffer of the XQP's**, eight
   blocks at a time, as `FIL$READIMG` reads an image: `FIL$READVBN` and
   `FIL$WRITEVBN` with the WCB's header, up to the blocks it has
   allocated, then `SS$_ENDOFFILE` with the bytes done in the IOSB. A
   VBN past them reads the header again once, for another open may have
   extended the file: an RMS stream's copy, newer than the disk's
   (`RMS$PEERHDR`), else the disk's. A write's last block is filled with
   zeros, as a disk writes a partial block.
4. **An open takes the access arbitration lock in the mode VMS's XQP
   takes** (`LOCK_MODE`): EX for `FIB$M_NOREAD`, else CW for writing, PW
   with `FIB$M_NOWRITE`, CR for reading, PR with `FIB$M_NOWRITE`, through
   RMS's `RMS$LOCKFILE`, so RMS's opens and the channel's arbitrate with
   each other. It takes write access to the file for `FIB$M_WRITE` or
   `FIB$M_NOREAD`, execute for `FIB$M_EXECUTE`, else read; a file marked
   for delete is there only for `BYPASS`.
5. **A close works on the header as the disk has it**: `IO$_DEACCESS`
   reads it, writes p5's attributes into it if the file was open for
   writing and no other open has it, whose header would win, else
   `SS$_ACCONFLICT`, or marks it revised if it was written, unless
   `FIB$M_NORECORD`, then gives back the lock, and the last open of a
   file marked for delete deletes it, whether RMS or a channel had it
   last. `$DASSGN`, an image's exit and a process's deletion close the
   files open on the channels they take (`XQP$RUNDOWN`), and `DISMOUNT`
   refuses a volume with one (`RMS$VOLIDLE`).
6. **`IO$_MODIFY` of the file open on the channel itself** gives back the
   channel's lock for the look at other opens and takes it again, so
   `SET FILE`'s way, open the file for itself alone and change it,
   works.
7. **`IO$_CREATE` and `IO$_DELETE` are RMS's `$CREATE` and `$ERASE` on
   the same file system routines**: `FIL$CREHDR`, `FIL$ENTER`,
   `FIL$EXTEND` for `FIB$M_EXTEND`; `FIL$REMOVE`, `FIL$DELHDR`, or
   `RMS$MARKDEL` for a file open anywhere. `IO$_CREATE` without
   `IO$M_CREATE` enters an existing file, `IO$_DELETE` without
   `IO$M_DELETE` only removes the entry, and `IO$_ACCESS!IO$M_CREATE`
   makes a file that isn't there, `SS$_CREATED`, as VMS's. A new file
   takes write access to the volume, and p5's characteristics may make
   it a directory, as `LIB$CREATE_DIR` asks; `IO$_MODIFY` keeps that bit.
   A channel's write makes RMS's indexed streams of the file forget
   their buckets (`RMS$FORGET`), as another stream's block I/O does.
8. **A wildcard lookup goes on after the result string**, the
   `NAME.TYPE;VER` it gave in p4, which the caller passes back with
   `FIB$L_WCC` not 0, as VMS's `FIND` does; with none, after as many
   matches as `FIB$L_WCC` counts, which the XQP adds one to each time.
   Past the last, `SS$_NOMOREFILES` and `FIB$L_WCC` is 0.
9. **`IO$_ACPCONTROL` has nothing to do but say so**: `FIB$C_REMAP` and
   `FIB$C_FLUSH_CACHE` succeed, as there is no window to map again and
   no cache, and the disk quota functions are `SS$_QFNOTACT`.

## Alternatives considered

| Option | Why not |
| --- | --- |
| The WCB in the UCB, or a list the XQP keeps | A channel is a process's; the PCB already holds its channels by number |
| The virtual functions queued to the driver, a VBN turned into an LBN in `DK$FDT`, as VMS's window turn | A transfer can span extents, which the driver would have to split, and the map may need a header read, which the FDT routine at `IPL$_SYNCH` can't wait for |
| Reading into the caller's buffer directly | `FIL$READLBLK` locks the pages it is given as kernel mode's; through the XQP's own buffer nothing of the caller's is locked, as `FIL$READIMG` does |
| A file control block every open shares, as VMS's FCB | RMS keeps its own copy in each IFAB and makes them agree under `FIL$LOCK` (ADR-0046); a close reading the header from the disk is enough for a channel's |
| `IO$_MODIFY` of the channel's own file refused while it is open | `SET FILE` and `BACKUP` open a file and then change it, as on VMS |
| Resume a wildcard by `FIB$L_WCC` alone, a count | A caller that deletes what it finds, as `DELETE` does, would skip a file each time |

## Consequences

**What gets harder.**
- Every virtual transfer is synchronous, as the XQP's other functions
  are, and copies through the XQP's buffer holding `FIL$LOCK`.
- vaxpunk: `PCB$AL_WCB` is a slot per channel of the PCB, not VMS's
  `CCB$L_WIND`.
- ponytail: no extending or truncating but `IO$_CREATE`'s, no
  `FIB$M_SUPERSEDE`, `FIB$M_NEWVER` or placement; `IO$_CREATE`'s new
  name gets the directory's version limit, not `FIB$W_VERLIMIT`, purges
  no version past it and takes no protection or ACL from the version
  before; `IO$_DEACCESS` ignores its FIB; `FIB$C_LOCK_VOL` and
  `FIB$C_UNLK_VOL` are `SS$_ILLIOFUNC`.

**What stays easy.**
- The XQP's routines are RMS's and the file system's, so a file has one
  set of rules whichever way it is opened.
- XQPTEST checks it in the boot test's *set file* phase on `DKB0:`: it
  makes, writes and reads a file, is refused a second open, a write
  opened for reading and virtual I/O once closed, looks up and deletes
  by wildcard, deletes a file still open and one made marked for delete
  at `$DASSGN`, and the volume `ods verify` checks after has no leak.

**Follow-ups:** `LIB$CREATE_DIR` on `IO$_CREATE` in place of
`$CREATE_DIR`; `SET VOLUME` through `INDEXF.SYS` open on a channel;
`IO$_MODIFY`'s extend and truncate; file-backed global sections on the
file open on a channel (`t8y:/exec/global-sections#file-sections`).
