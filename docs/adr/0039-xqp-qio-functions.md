# ADR-0039 — A disk channel takes the XQP's IO$_ACCESS and IO$_MODIFY, which $QIO does in the process once IPL is back down

Oct 19, 2026 · @Marko Mikulicic

Proposed. A program reads a file's attributes with a `$QIO` of
`IO$_ACCESS` to the file's disk, and writes them with `IO$_MODIFY`, as on
VMS: p1 is a FIB that names the file, by its ID or by a name in a
directory, and p5 a list of attribute descriptors (`$FIBDEF`, `$ATRDEF`).
`DK$FDT` makes the IRP and leaves it in `XQP$GL_IRP`; `$QIO` takes it
before it lowers IPL, and once it has calls `XQP$PERFORM` (`xqp.mar`),
which holds `FIL$LOCK`, does the work, and completes the IRP. `SET
PROTECTION` and `SET FILE/OWNER_UIC` use it, so they no longer need read
access to the file, and so do the new `SET FILE/ATTRIBUTE`,
`/EXPIRATION_DATE` and `/VERSION_LIMIT` and `SET DIRECTORY/VERSION_LIMIT`.
Directory entries now keep a version limit, which `$CREATE` enforces.
`SET VOLUME` rewrites the home blocks through a kernel mode routine,
`FIL$SETVOLUME`, which it calls with `$CMKRNL`. The first part of
`t8y:/files/ods2#qio-interface`.

## Context

On VMS everything that changes a file without reading its records goes
to the XQP through the disk channel: `SET FILE`, `SET PROTECTION`,
`SET DIRECTORY`, `F$FILE_ATTRIBUTES`, `DIRECTORY`'s lookups, `BACKUP`.
vaxpunk has had no such interface. `SET PROTECTION` opened the file with
RMS and let `$CLOSE` write a protection XAB, which takes read access
VMS's doesn't; `$CREATE_DIR` is a system service of vaxpunk's own for the
same reason. `SET FILE/ATTRIBUTES`, `/EXPIRATION_DATE`,
`/VERSION_LIMIT` and `SET DIRECTORY` have no RMS way at all: `$CLOSE`
writes a protection XAB and a revision XAB, never record attributes or
an expiration date. [The research](../research/files-11-acp-qio.md) has
the real layouts and rules.

`$QIO` calls a driver's FDT routine at `IPL$_SYNCH`. The file system runs
below it, holding `FIL$LOCK`, and may wait for it
([ADR-0020](0020-file-system-lock-below-ipl-synch.md)), so the FDT
routine can't do the work. VMS's FDT routine queues the IRP to the XQP,
which runs in the process that asked, in kernel mode, as a kernel mode
AST.

## Decision

1. **`DK$FDT` makes the IRP for `IO$_ACCESS` and `IO$_MODIFY`**, charged
   as a disk's direct I/O like its others, and puts it in `XQP$GL_IRP`.
   No `LOG_IO`: the XQP checks the file's own protection.
2. **`$QIO` does it after the FDT routine**: still at `IPL$_SYNCH` it
   takes the IRP from `XQP$GL_IRP` and clears it, puts IPL back, and
   calls `XQP$PERFORM` in the process, in kernel mode, with `$QIO`'s
   arguments. `$QIO` returns `SS$_NORMAL`; the request's status goes in
   the IOSB, through `IOC$REQCOM` and `IOC$POST`, as any other's.
3. **`XQP$PERFORM` holds `FIL$LOCK`** while it looks the file up and
   reads or writes its header, as RMS does, and writes the FIB back: the
   file's ID and its name's version limit.
4. **The attributes are `$ATRDEF`'s**: the characteristics, the record
   attributes, the four dates, the owner and the protection; the header
   and `UIC_RO` read only. Reading takes read or control access; the
   owner, protection and dates control access, a new owner the system's,
   as RMS's `$CLOSE` asked; the characteristics and record attributes
   write access. A write marks the file revised, and the allocation and
   the characteristics the file system keeps stay as they were.
5. **A file open in any stream is `SS$_ACCONFLICT` for `IO$_MODIFY`**,
   by its access arbitration lock (`RMS$FILEBUSY`).
6. **Version limits are VMS's.** Each directory record keeps its name's
   limit, `DIR$W_VERLIMIT`; a new name gets the directory's default,
   `FAT$W_VERSIONS`, or 32767. `IO$_MODIFY` of a file named in a
   directory with `FIB$W_VERLIMIT` not 0 sets its name's, which takes
   write access to the directory (`FIL$SETLIMIT`). `$CREATE` then takes
   the lowest version out and deletes it when the name has more than
   its limit, one each time, as V4.3's XQP does. `$OPEN`'s XABFHC gives
   the limit in `XAB$W_VERLIMIT`, a directory's default for a directory,
   which `DIRECTORY/FULL` shows.
7. **`SET VOLUME` writes in kernel mode**: `FIL$SETVOLUME`, which it
   calls with `$CMKRNL`, checks control access to the volume, rewrites
   each copy of the home block, INDEXF.SYS's VBNs from 2 up to the
   backup index file header's, with both checksums, and changes the
   label, owner and protection in the VCB, as VMS's `SET VOLUME` does
   with `$QIO`s through INDEXF.SYS and a kernel mode routine of its own.

## Alternatives considered

| Option | Why not |
| --- | --- |
| `$CLOSE` writes an XABFHC's record attributes and an XABDAT's expiration date | VMS's `$CLOSE` doesn't, so a program that changed them would change the file here only; and `SET DIRECTORY` and the version limits would still have no way |
| A system service of vaxpunk's own, as `$CREATE_DIR` | Programs written for VMS use the `$QIO`; one more invented interface |
| Queue the IRP to the process as a kernel mode AST, as VMS's FDT routine does | The work would run at `IPL$_ASTDEL` inside an AST and wait there for `FIL$LOCK`; calling it from `$QIO`, in the same process and mode, does the same work with no AST and no wait inside one |
| Do it in the FDT routine | It runs at `IPL$_SYNCH`, where `FIL$LOCK` can't be waited for and the disk's I/O can't be either once a driver queues |
| Write a file that is open, as VMS's XQP does | RMS keeps its own copy of the header and writes it back on `$CLOSE` and each flush, over what `IO$_MODIFY` wrote; VMS's `SET FILE` opens the file for itself alone first and gets the same `SS$_ACCONFLICT` |
| No check for the record attributes, as V4.3's XQP | A world that may only read a file could change how its records read |
| Purge every version past the limit at once, as the `ods` tool does | V4.3's XQP takes one each time, and VSI's help says a lowered limit leaves the versions there are |
| `SET VOLUME` through `IO$_WRITELBLK`, by LBN | It takes `LOG_IO`, where VMS's takes control access to the volume, and the VCB would keep the old label until the next `MOUNT` |
| `SET VOLUME` through `IO$_WRITEVBLK` on INDEXF.SYS, as VMS's | It needs the file open on the channel, `IO$M_ACCESS`, not done yet; the VCB still needs kernel mode |

## Consequences

**What gets harder.**
- `$QIO` returns only once the file system's work is done, even without
  the W: the request is synchronous, as the disks' others are.
- vaxpunk: `IO$_MODIFY` of the characteristics or record attributes
  takes write access, and of a file open elsewhere fails, where VMS's
  writes it.
- vaxpunk: `SET.EXE` isn't installed with `CMKRNL`, as VMS's is, so a
  volume's owner without the privilege can't `SET VOLUME`.
- ponytail: no `IO$M_ACCESS`, so no file is ever open on the channel,
  and no `IO$_DEACCESS`, `IO$_CREATE`, `IO$_DELETE` or `IO$_ACPCONTROL`;
  a name with wildcards finds its first match and `FIB$L_WCC` isn't
  kept; no result string, p3 and p4; attributes outside `$ATRDEF`'s are
  `SS$_BADATTRIB`.

**What stays easy.**
- `SET FILE/ATTRIBUTES`, `/EXPIRATION_DATE` and `SET DIRECTORY` are a
  record attributes or a date in a list, and `/VERSION_LIMIT` a FIB
  field, on the same path.
- PROTTEST checks it at boot: PROTCHILD, which may read DATA.TXT, looks
  it up in `[000000]` by its name, reads its owner, and can't give it
  another. The boot test's *set file* phase sets a directory's and a
  file's version limit and sees `COPY` purge, changes a file's record
  attributes and expiration date, and relabels `DKB0:`'s volume, which
  `MOUNT` then finds.

**Follow-ups:** `IO$M_ACCESS`, `IO$_CREATE` and `IO$_DELETE`, then
`$CREATE_DIR` as `LIB$CREATE_DIR` on them, and `SET VOLUME` through
INDEXF.SYS; `SET VOLUME/[NO]ERASE_ON_DELETE` and
`/[NO]HIGHWATER_MARKING` once the file system does what they say.
