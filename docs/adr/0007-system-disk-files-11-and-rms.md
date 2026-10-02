# ADR-0007 — The system disk is a Files-11 volume the PAL reads by LBN, with the file system and RMS in the executive

Oct 2, 2026 · @Marko Mikulicic

Accepted. The system disk, `DKA0:`, is a real Files-11 ODS-2 volume on a
virtio block device. The PAL drives the device and gives the executive one
call, `READLBLK`, which reads blocks by LBN. The executive mounts the
volume and reads it itself: file headers, maps and directories. RMS's
services read files on it, and the image activator loads images from it.
The PAL boots `EXEC.EXE` from the same volume. Read only, for now.

## Context

Until now the files were `sys.vol`, a Limine module the shim appended to
the root task and the PAL mapped into S0: a 512-byte directory of names,
first blocks and sizes, then the files (`$BVDDEF`). DIRECTORY copied that
directory with `$CMKRNL`, and there was no way to read a file but to be
an image. The `ods` crates have read and written Files-11 volumes on the
host for a while, checked against real VMS.

On VMS the system disk is Files-11. The primary bootstrap, VMB, reads just
enough of it to find the executive. The XQP, in kernel mode in the
process's context, reads headers and directories for the `$QIO` functions
`IO$_ACCESS` and `IO$_READVBLK`, and RMS, in executive mode, turns those
into `$OPEN`, `$GET` and `$SEARCH` on FABs, RABs and NAM blocks. The disk
driver does the I/O, with interrupts.

## Decision

1. **The system disk is an ODS-2 volume** labelled `VAXPUNK`, which
   `roottask/build.rs` makes with `ods-image`: the images in `[SYSEXE]`,
   SYSTEM's files in `[SYSMGR]`. QEMU attaches it read only as a
   virtio-blk device on a virtio-mmio transport. It replaces `sys.vol`,
   `$BVDDEF`, the shim's `volume` module and the S0 mapping of the volume.
2. **The PAL drives the disk**, polled, a request at a time, and offers
   one PAL call, `READLBLK` (0x46): a0 = a buffer kernel mode can write,
   a1 = a byte count, a2 = an LBN, v0 = an `SS$` status. Its arguments are
   `IO$_READLBLK`'s P1-P3, and the call is the shape of Alpha's console
   `READ` callback, which VMS's bootstrap read the boot disk with: blocks
   and statuses, no queues or descriptors (ADR-0001).
3. **The PAL boots `[SYSEXE]EXEC.EXE` from the volume**, as VMB found
   `SYS.EXE`: home block, index file header, MFD, `SYSEXE.DIR`, the file.
4. **The executive reads Files-11 itself** (`exec/f11.mar`): `FIL$MOUNT`
   at boot, then headers by file number, virtual blocks through the
   header's map, and directory records with wildcards. `FIL$OPENFILE`
   reads an image into pool for the image activator.
5. **RMS is a set of system services** (`exec/rms.mar`): `$PARSE`,
   `$SEARCH`, `$OPEN`, `$CONNECT`, `$GET`, `$DISCONNECT`, `$CLOSE`, on
   VMS's FAB, RAB and NAM layouts, with VMS's `RMS$_` statuses. An open
   file is an IFAB in pool, which the PCB holds by IFI. Image rundown
   closes the image's files, as it deassigns its channels.
6. **DIRECTORY and TYPE use RMS**, from user mode, like any program.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Keep the volume in memory, as an ODS-2 image instead of `sys.vol` | A ramdisk the boot loads whole: no device, its size bounded by boot memory, and nothing to grow into writes or a second disk. |
| The disk driver in the executive, with the virtio registers mapped into S0 | The device needs physical addresses, which the executive never sees (PFNs are the PAL's), and its interrupt would need seL4's IRQ handling, which the PAL holds. The PAL already drives the UART the same way. |
| `$QIO` `IO$_ACCESS` with a FIB on a disk channel, and RMS on top of `$QIO`, as on VMS | Twice the interface for the same reads, with nothing but RMS to call it. RMS calls the file system's routines directly instead. |
| RMS in executive mode, as on VMS | Executive mode may only run the system service vector's page (ADR-0005), and RMS would need its own data in P1. Kernel mode at `IPL$_SYNCH` serializes the file system's buffers for free. |
| ODS-5 | Nothing needs long or mixed-case names yet, and ODS-2 is what VMS boots from. The on-disk structures are the same but for the ident area and directory name types. |
| Boot `EXEC.EXE` from a Limine module, the disk for the rest | Two places for system files, and the shim's volume code stays. The PAL's Files-11 reader is about a hundred lines. |

## Consequences

**What gets harder.**
- The CPU waits while the device reads: the PAL polls, and the file
  system runs at `IPL$_SYNCH`, so no other process runs meanwhile.
  ponytail: QEMU reads in microseconds; the device's interrupt and
  asynchronous I/O when that matters.
- There are two Files-11 readers, the PAL's in C for booting and the
  executive's in MACRO-32, as VMB and the XQP were.
- Read only: `$OPEN` for writing returns `RMS$_WLK`. No extension headers,
  wildcard directories, logical names, `SET DEFAULT`, ASTs or completion
  routines; `$SEARCH` finds its place again by counting matches.

**What stays easy.**
- `cargo run -p boot` builds the volume, and `ods dir`, `ods type` and
  `ods verify` read the same file on the host.
- A program opens a file as on VMS, with the same blocks and statuses, so
  VMS sources that read files sequentially need little change.

**Follow-ups:** writing (the XQP's allocation, from `ods-core`'s model),
`$QIO` on disk channels, the disk's interrupt, logical names
(`SYS$SYSTEM`, `SYS$LOGIN`) and `SET DEFAULT`, message texts for `RMS$_`
statuses, `$READ` for block I/O.
