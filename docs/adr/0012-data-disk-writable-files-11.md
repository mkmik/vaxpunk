# ADR-0012 — DKB0:, a second virtio disk the PAL writes, holds a Files-11 volume that outlives the system

Oct 2, 2026 · @Marko Mikulicic

Proposed. A second disk, `DKB0:`, is a virtio block device on a disk
image of the host's, `out/datadisk.img`, which QEMU attaches read-write.
The PAL reads and writes it by LBN: `READLBLK` takes a unit, and a new
call, `WRITELBLK`, writes. The executive writes Files-11 on it as it does
on the ramdisk, so what `INITIALIZE`, `COPY` and `DELETE` do there is
still there at the next boot. The system disk, `DKA0:`, stays read only.

## Context

[ADR-0009](0009-ramdisk-writable-files-11.md) gave the executive a
Files-11 writer, on `MDA0:`, a ramdisk, and left writing a real disk as a
follow-up, for fear of corrupting the system disk. A ramdisk is gone at
shutdown, and its volume can only be checked on the host by saving QEMU's
RAM and putting the pages back in order.

On VMS the system disk is written like any other, but a site's files
usually live on other disks, mounted by `SYSTARTUP`. Alpha's console
callbacks had `READ` and `WRITE`, both by unit and LBN.

## Decision

1. **`DKB0:` is a second virtio-blk device**, unit 1, after the system
   disk, unit 0, on QEMU virt's virtio-mmio transports.
   `scripts/run-qemu.sh` attaches `out/datadisk.img`, read-write, and
   makes it, 4,096 blocks of zeros, if it isn't there. The PAL sets up
   each device it finds, a queue each, in the order of QEMU's `-device`
   options, which take the transports from the highest down.
2. **`READLBLK` takes a unit in a3, and `WRITELBLK` (0x47) writes**, with
   the same arguments: a0 = a buffer, a1 = a byte count, a2 = an LBN,
   a3 = a unit, v0 = an `SS$` status. Kernel mode must be able to read
   the buffer to write it. A write to `DKA0:` fails in QEMU, which
   attaches it read only, with `SS$_DRVERR`; the executive never sends
   one (`SS$_WRITLCK`).
3. **`DKB0:` has a VCB** that may be written, with its unit in
   `VCB$B_UNIT`. `FIL$READLBLK` and `FIL$WRITELBLK` call the PAL with it.
   RMS, `$INIT_VOL` and `$MOUNT` take `DKB0:` as they take `MDA0:`.
4. **`$INIT_VOL` writes a 4,096-block volume on `DKB0:`**, whatever the
   disk held, as `FIL$INIT` lays out the ramdisk's. 4,096 blocks is what
   one bitmap block maps.
5. **The executive mounts `DKB0:` at boot** if it holds a volume, after
   `DKA0:`, as `SYSTARTUP` would, whatever its label. A blank disk, or
   none, is left unmounted.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Write the system disk, `DKA0:` | A bug in a new writer would corrupt the disk the system boots from, and `build.rs` rewrites it at each build anyway, so nothing written there would last. |
| A volume `ods init` makes on the host | `ods-core` lays out a volume as VMS does for its size, with more headers and room to extend, which `f11wrt.mar`'s one-block bitmaps and fixed index file don't handle. `INITIALIZE DKB0:` makes one the executive can write, and `ods` reads and checks it all the same. |
| A unit in the LBN's high bits, or a call per disk | Alpha's callbacks took a unit; a register is clearer and leaves the LBN 32 bits. |
| The size from the device, a PAL call or the SCB | Nothing but `INITIALIZE` needs it, and it can't use more than 4,096 blocks yet. |
| `DKA100:`, as VMS names SCSI ID 1 | Every device name the executive knows is four characters and a colon, compared as a longword. `DKB0:` is the first disk on a second controller, which a second virtio device is. |

## Consequences

**What gets harder.**
- A disk image smaller than 4,096 blocks gets a volume that says it has
  4,096: writes past its end fail with `SS$_ILLBLKNUM`. A bigger one has
  blocks the volume doesn't use. ponytail: the device's capacity, and
  more bitmap blocks, when volumes grow.
- The CPU waits for each write, as for each read (ADR-0007).
- No write ordering or flush: QEMU writes the image as requests come, so
  a host crash mid-`COPY` can leave a header without its directory entry.
  `ods verify` finds those.

**What stays easy.**
- `ods dir out/datadisk.img '[000000]'`, `ods type` and `ods verify` read
  what the executive wrote, on the host: the boot test (`cargo test -p
  boot`) initializes a disk of its own, copies a file to it, and checks
  the volume and the file with `ods-image` once QEMU is gone.
- The ramdisk is unchanged, and the same writer serves both.

**Follow-ups:** `DISMOUNT`; mounting from a `SYSTARTUP` command procedure;
the disk's size from the device; flushing with `VIRTIO_BLK_T_FLUSH`;
volumes bigger than 4,096 blocks; more disks.
