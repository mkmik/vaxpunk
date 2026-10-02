# ADR-0009 — MDA0:, a ramdisk the executive drives, holds a Files-11 volume the file system writes

Oct 2, 2026 · @Marko Mikulicic

Proposed. A second disk, `MDA0:`, is a ramdisk, as DECram's was on VMS: a
driver in the executive keeps its blocks in pages of S0, which the first
`INITIALIZE` makes. The executive writes Files-11 on it: `$INIT_VOL`
writes an empty ODS-2 volume, `$MOUNT` mounts it, and RMS's `$CREATE`,
`$PUT` and `$ERASE` make, write and delete files there. DCL gets
`INITIALIZE`, `MOUNT`, `COPY` and `DELETE`.

## Context

[ADR-0007](0007-system-disk-files-11-and-rms.md) made the system disk,
`DKA0:`, a Files-11 volume the PAL reads by LBN, and left the file system
read only, with writing as a follow-up. Nothing could make a file, so
`COPY` and `DELETE` had nowhere to go.

On VMS, DECram's `MDDRIVER` is a disk driver whose blocks are memory:
`INITIALIZE/SIZE=n MDA0: label` gives it its size and writes a volume,
`MOUNT MDA0: label` mounts it, and from then on it is a disk like any
other, lost at shutdown. Writing a Files-11 volume is the XQP's work: a
header from the index file bitmap, blocks from the storage bitmap, an
entry in a directory. `ods-core` does the same on the host, checked
against real VMS.

## Decision

1. **`MDA0:` is a ramdisk in the executive** (`exec/mddriver.mar`):
   1,024 blocks in pages of S0 from `0x50000000`, which `MD$CREATE` takes
   from the PFN list, zeroed, at each `INITIALIZE`. Until the first one,
   the device is offline (`SS$_MEDOFL`). The file system copies blocks in
   and out of it where it would call the PAL for `DKA0:`.
2. **Each disk has a VCB** (`$VCBDEF`): its name, whether a volume is
   mounted and whether the disk can be written, and the volume's index
   file header and bitmaps. `F11$GL_VCB` is the one the file system works
   on; RMS selects it from a specification's device, `DKA0:` or `MDA0:`,
   and refuses one with nothing mounted (`RMS$_DNR`).
3. **`$INIT_VOL devnam, volnam` writes an empty ODS-2 volume** on the
   ramdisk, laid out as `INITIALIZE` does (`ods/docs/initialize.md`): the
   home block, the index file bitmap, 64 header slots, `BITMAP.SYS` with
   its SCB, the MFD, and the nine reserved files' headers, entered in the
   MFD. **`$MOUNT itmlst`** mounts a volume, with `MNT$_DEVNAM` and
   `MNT$_VOLNAM`, if its label matches (`SS$_INCVOLLABEL`).
4. **The file system writes** (`exec/f11wrt.mar`): a new file takes the
   first free header slot, its blocks the first free runs of the storage
   bitmap, one map pointer each, merged with the last one when they
   follow it; deleting it gives them back and marks the header deleted,
   keeping its sequence number. A directory is rewritten whole on each
   change, its records copied with the entry entered or removed, and
   extended if it grew.
5. **RMS writes files**: `$CREATE` makes a new version, one above the
   highest unless the specification gives one, with the FAB's attributes
   and allocation; `$PUT` appends VAR and FIX records, extending the file
   by 8 blocks at a time as it fills; `$CLOSE`, and image rundown, write
   its last block and its end of file. `$ERASE` deletes a file, by its
   NAM block's resultant string after `$SEARCH`, which then goes on from
   the file after it.
6. **DCL runs `INIT.EXE`, `MOUNT.EXE`, `COPY.EXE` and `DELETE.EXE`**,
   images in `[SYSEXE]` as on VMS, with the command's parameters as the
   command line; `GET_PARAM` (`sysexe/lib/param.mar`) picks them out.

## Alternatives considered

| Option | Why not |
| --- | --- |
| The ramdisk in the PAL, with a unit number on `READLBLK` and a `WRITELBLK` | Grows the PAL interface for memory the executive already owns: the PFNs are the executive's, and DECram was a driver, not console firmware. |
| A volume `build.rs` makes, loaded at boot | Not a ramdisk one initializes, and it brings back a Limine module and its S0 mapping, which ADR-0007 removed. |
| Writing `DKA0:` first | Needs a write call in the PAL and the disk read-write, and a bug would corrupt the system disk; the ramdisk exercises the same code, and nothing on it survives a reboot. |
| `$QIO IO$_WRITELBLK` on a disk channel for `INIT.EXE`, as VMS's `INITIALIZE` writes | There are no disk channels yet (ADR-0007's follow-up), and `$INIT_VOL`, which VMS also has, is one service. |
| Inserting directory records in place, splitting blocks, as the XQP does | More code for directories of a few blocks; rewriting the whole directory is the same result, its cost proportional to its size. |
| `INITIALIZE` and `MOUNT` built into DCL | VMS runs them as images, `INIT.EXE` and `MOUNT.EXE`; an image needs only `GET_PARAM`, and DCL stays a dispatcher. |

## Consequences

**What gets harder.**
- The ramdisk's size is fixed, 1,024 blocks, since DCL has no qualifiers
  for `INITIALIZE/SIZE`. ponytail: a cluster is a block and each bitmap is
  one block, so a volume has at most 4,096 blocks and files; the index
  file isn't extended, so it has the 64 headers `INITIALIZE` made room
  for, 55 files besides the reserved ones.
- No file sharing or locking: `$ERASE` deletes an open file all the same,
  and `$OPEN` only reads. A file is written from its `$CREATE` to its
  `$CLOSE`, by appending.
- Two kinds of disk behind one file system: the VCB says which, and every
  RMS service selects it first. `$INIT_VOL` and `$MOUNT` take no other
  items, and any process may call them, until there are privileges.
- The volume is in memory only: `just check` sees it through DCL. Its
  structure was checked by saving QEMU's RAM from the monitor, putting
  the ramdisk's pages back in order and running `ods verify`, which found
  no errors, leaks or warnings, but that isn't automated.

**What stays easy.**
- Programs write files as on VMS, with FABs and RABs, and `COPY`ed images
  run from the ramdisk (`RUN MDA0:[000000]TIMETEST`).
- The volume is real ODS-2: what `ods` reads and checks on the host.

**Follow-ups:** `DISMOUNT`; `INITIALIZE/SIZE` and DCL qualifiers;
subdirectories (`CREATE/DIRECTORY`); extending the index file, extension
headers, version limits; `$OPEN` for writing; writing `DKA0:`, with a PAL
call; message texts, so `COPY` and `DELETE` report errors as VMS does.
