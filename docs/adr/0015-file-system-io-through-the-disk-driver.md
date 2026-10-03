# ADR-0015 — The file system hands its block I/O to the disk's start I/O routine in an IRP of its own

Oct 3, 2026 · @Marko Mikulicic

Proposed. The disks' driver is split in two, as VMS's are: `DK$FDT`
checks a `$QIO`'s function and buffer and makes the IRP, and
`DK$STARTIO`, which the disk's UCB names in `UCB$L_START`, does the I/O
and completes the IRP. `FIL$READLBLK` and `FIL$WRITELBLK` no longer call
the PAL or the ramdisk themselves: they fill in `F11$AB_IRP`, an IRP of
the file system's with no process, and call the start I/O routine of the
volume's UCB, `VCB$L_UCB`. `IOC$REQCOM` leaves such an IRP's status in it
instead of queuing it as an AST, and the file system reads it from there.

## Context

[ADR-0013](0013-qio-irps-and-drivers.md) gave `$QIO` a disk driver, but
`f11.mar` kept its own path to the disks: `FIL$READLBLK` and
`FIL$WRITELBLK` called the PAL's `READLBLK` and `WRITELBLK`, or `MD$IO`
for the ramdisk, beside the driver rather than through it. It listed "the
file system through `$QIO` to the disk, as the XQP does" as a follow-up.
Two paths to one device means a new kind of disk, or a driver that queues
its requests, has to be taught to both.

On VMS the XQP runs in the process that asked, in kernel mode, and reads
and writes the volume with IRPs that go to the disk's driver like any
other, whose completion comes back to it as a kernel mode AST.

Here the file system runs at `IPL$_SYNCH`, which keeps its buffers and
`F11$GL_VCB` to one caller at a time, and is called at boot, by
`FIL$MOUNT`, before there is a process to deliver an AST to. A kernel
mode AST can't be delivered at `IPL$_SYNCH`. The disks' I/O is
synchronous: the PAL waits for the device, and the ramdisk is a copy.

## Decision

1. **`DK$FDT` checks and makes the IRP; `DK$STARTIO` does it.** The
   LBN goes in `IRP$L_MEDIA`, as on VMS. The UCB names the start I/O
   routine in `UCB$L_START`; the console's is 0, its start I/O being
   local to `ttdriver.mar`.
2. **Each VCB names its disk's UCB**, `VCB$L_UCB`, as VMS's
   `VCB$L_RVT` does for a volume of one disk.
3. **The file system has one IRP of its own, `F11$AB_IRP`**, in
   `EXEC$DATA`, whose `IRP$L_PID` is 0. `FIL$READLBLK` and
   `FIL$WRITELBLK` put the function, buffer, byte count, LBN and UCB in
   it and call `UCB$L_START`. `IPL$_SYNCH` keeps it to one request at a
   time, as it does the file system's buffers.
4. **`IOC$REQCOM` doesn't queue an IRP with no process**: it writes the
   status in it and returns, and `FIL$READLBLK` takes it from
   `IRP$L_IOST1`.

## Alternatives considered

| Option | Why not |
| --- | --- |
| The file system calls `$QIOW` on a channel of its own, as the XQP does | At `IPL$_SYNCH` the completion AST can't be delivered, so `$QIOW` would never return; at boot there is no process to own the channel |
| Go through `DK$FDT` with an argument list made up for it | The FDT routine's work is probing a caller's buffer and making an IRP from pool; the file system's buffers are the executive's, and its IRP needs no pool |
| An IRP from pool for each read | One allocation and free per block for a request that is done when the call returns; and `FIL$READLBLK` could then fail for want of pool |
| Find the UCB by scanning `IOC$AB_UCB` for the VCB | A loop on every block where a longword in the VCB does it |

## Consequences

**What gets harder.**
- The file system assumes the driver is done when its start I/O routine
  returns. ponytail: a driver that queues the IRP, an interrupt-driven
  virtio disk, needs the file system to wait for it, which means running
  it below `IPL$_SYNCH`, holding a lock of its own, as the XQP does.

**What stays easy.**
- A new disk is a UCB, a VCB that names it, and a start I/O routine; the
  file system and `$QIO` both use it.
- `FIL$READLBLK` and `FIL$WRITELBLK` take and return what they did, so
  none of their callers changed.

**Follow-ups:** the file system below `IPL$_SYNCH`, with the XQP's
lock, once a driver queues; error and operation counts in the UCB.
