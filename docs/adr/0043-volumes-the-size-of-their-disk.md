# ADR-0043 — A volume is the size of its disk, in VMS's clusters, with bitmaps of as many blocks as it needs, and the PAL tells the executive the disk's size

Oct 23, 2026 · @Marko Mikulicic

Proposed. `INITIALIZE DKB0:` writes a volume as big as the disk, which a
new PAL call, `GETMAXBLOCK`, gives. It picks the cluster size and the
maximum number of files as VMS's `INITIALIZE` does by default, and lays
the index file out as VMS does: a backup home block and a backup index
file header, then the index file bitmap and the headers. The file system
reads and writes bitmaps of any number of blocks, so it can write any
ODS-2 volume, including one `ods` made on the host. This replaces
decision 4 of [ADR-0012](0012-data-disk-writable-files-11.md), which made
every volume on `DKB0:` 4,096 blocks. The system disk stays read only.

## Context

[t8y:/files/ods2#big-volumes](../map/files/ods2.md): `f11wrt.mar` used
one bitmap block of each kind. A volume could have at most 4,096 blocks,
the clusters one storage bitmap block maps at one block each, and 4,096
files, the bits of one index file bitmap block. ADR-0012 gave `DKB0:` a
4,096-block volume whatever the disk's size, and noted that
`f11wrt.mar` couldn't write the volumes `ods-core` lays out for bigger
disks. The ramdisk's volume was laid out the same way.

The storage bitmap side already went past one block: `ALLOCBLK` and
`FREEBLK` walk `VCB$L_SBMAPBLKS` blocks and count in clusters. The index
file bitmap side did not: `FIL$CREHDR` looked at one block, and the
index file grew 16 blocks at a time, so a file with many headers
fragmented the index file until its map filled.

On VMS, `INITIALIZE` takes the disk's size from its UCB, which the
driver filled from the device (`DVI$_MAXBLOCK`). Its defaults are a
cluster of 1 up to 50,000 blocks, else 3, and more if the storage bitmap
would need more than 255 blocks; and a maximum of size / ((cluster + 1)
× 2) files, the index file bitmap having a bit for each. `ods-core`'s
`initialize` does the same and was checked against a volume VMS 7.1
made.

## Decision

1. **`GETMAXBLOCK` (0x4A)**: a0 = a disk's unit, v0 = its blocks, from
   the virtio-blk device's capacity, or 0 if there is no such disk. It
   stands in for what a VMS driver learns from the device when the unit
   is set up, as SCSI's READ CAPACITY. `$INIT_VOL` calls it for
   `DKB0:`. With no disk there it returns `SS$_MEDOFL`.
2. **`FIL$INIT` uses VMS's defaults** for the cluster and the maximum
   number of files, and makes room for 16 headers, VMS's `/HEADERS`
   default. A last cluster that is only part there is left out of the
   volume: the SCB's size is whole clusters.
3. **The layout is VMS's, from LBN 0.** The index file is one run from
   LBN 0: the boot and home blocks' two clusters, the backup home
   block's cluster at LBN 2v, the backup index file header's at LBN 3v,
   then the index file bitmap at LBN 4v and the headers. `BITMAP.SYS`
   (the SCB, then the storage bitmap) and the MFD's first cluster
   follow. The rest of the volume is free. `FIL$INIT` zeroes everything
   up to there first, so no header from an older volume is left in a
   slot. VMS's `/INDEX=MIDDLE` puts these structures in the middle of
   the volume.
4. **The index file bitmap is as many blocks as the home block says.**
   The VCB gets `VCB$L_IBMAPBLKS` and `VCB$L_MAXFILES` when the volume
   is mounted. `FIL$CREHDR` searches each block up to the maximum, and
   `FIL$DELHDR` clears the bit in the right block.
5. **The index file doubles when it fills**, but not past the slot of
   the volume's last file. If the disk hasn't that many blocks free,
   it takes those it has. A slot that the index file maps but is past
   its end of file moves the end of file, as VMS's XQP does, and every
   change to the index file's header is written to its backup too.

## Alternatives considered

| Option | Why not |
| --- | --- |
| The disk's size in the UCB, set at boot | It means a disk UCB layout and an init routine per unit, for one caller. `UCB$L_MAXBLOCK` and `$GETDVI` belong with [t8y:/drivers/disk#ucb-items](../map/drivers/disk.md), which can call the same PAL call. |
| Find the size by probing `READLBLK` until `SS$_ILLBLKNUM` | About 32 reads instead of one call, and a guess, where the device already knows. |
| The size in the RPB, as STARTUP_P1-P8 are | The RPB carries boot parameters; a disk's size is the device's to say, and disks may come and go one day. |
| `INITIALIZE/SIZE` | VMS's `/SIZE` is for DECram's ramdisks, not real disks, and needs DCL to pass qualifiers to `$INIT_VOL` (PRD-0003 tier 3 #36). |
| A cluster of 1 always | A 1 GB disk would need a 512-block storage bitmap, past what VMS's `INITIALIZE` makes, and volumes made here wouldn't match VMS's. |
| Grow the index file 16 blocks at a time, with extension headers when its map fills | Extension headers are a feature of their own ([t8y:/files/ods2#extension-headers](../map/files/ods2.md)). Doubling keeps the index file's map short with none. |

## Consequences

**What gets harder.**
- `FIL$INIT` writes more: every block up to the first free one, and the
  whole storage bitmap, a bit at a time. For a 64 MB disk that's under
  60 blocks.
- `FIL$FREEBLOCKS` still counts free clusters a bit at a time on each
  call, and `ALLOCBLK` still searches from the bitmap's first block on
  each allocation. On a big, full volume both get slow. ponytail: a free
  count in the VCB ([t8y:/files/ods2#free-count](../map/files/ods2.md))
  and a place to start searching from, when that shows.
- Doubling can ask for a big run on a nearly full volume. It takes what
  is free, but a header that doesn't fit in the index file's map is
  still `SS$_HEADERFULL`.
- Up to `MAXVOL`, 0x7F000000 blocks (about 1 TB), since the arithmetic
  is signed longwords.

**What stays easy.**
- The boot test's data disk is 64 MB, three blocks a cluster, and `ods
  verify` checks what the executive wrote, as before. A test in
  `boot/tests/rms.rs` mounts a 100,000-block volume that `ods` made with
  4,100 files, creates a file numbered past 4,096 there, deletes another,
  and verifies the result.
- Volumes made by the old `FIL$INIT` still mount and are written: their
  home block says one bitmap block and 4,096 files.

**Follow-ups:** `UCB$L_MAXBLOCK` and `DVI$_MAXBLOCK`; the free count in
the VCB; extension headers; `INITIALIZE`'s `/CLUSTER_SIZE`, `/HEADERS`,
`/MAXIMUM_FILES` and `/INDEX`; more disks (`DKC0:` and up); writing the
system disk ([t8y:/files/ods2#system-disk-writes](../map/files/ods2.md)).
