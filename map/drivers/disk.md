+++
title = "Disk driver"
summary = "DKA0: and DKB0:: logical and physical block I/O on virtio disks"
aka = ["DK", "DKA0:", "DKB0:", "DK$FDT", "DK$STARTIO", "DKDRIVER", "SYS$DKDRIVER", "IO$_READLBLK", "IO$_WRITELBLK", "IO$_READPBLK", "virtio-blk", "LOG_IO", "PHY_IO"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["docs/adr/0007-system-disk-files-11-and-rms.md", "docs/adr/0012-data-disk-writable-files-11.md", "docs/adr/0013-qio-irps-and-drivers.md", "docs/adr/0015-file-system-io-through-the-disk-driver.md", "docs/design/0001-pal-interface.md", "docs/design/0002-executive-processes.md", "vms/exec/f11.mar", "vms/exec/qio.mar", "pal/src/main.c"]

[[feature]]
id = "system-disk"
title = "DKA0:, the system disk, read by LBN through the PAL's READLBLK"
status = "done"
milestone = "m0"
note = "ADR-0007"

[[feature]]
id = "data-disk"
title = "DKB0:, a second disk, written with WRITELBLK"
status = "done"
milestone = "m0"
note = "ADR-0012"

[[feature]]
id = "qio"
title = "$QIO IO$_READLBLK, WRITELBLK, READPBLK, WRITEPBLK, with LOG_IO and PHY_IO"
status = "done"
milestone = "m0"
note = "ADR-0013; privileges with PRD-0003 step 1"

[[feature]]
id = "start-io"
title = "FDT routine and start I/O routine split; the file system's I/O goes through it"
status = "done"
milestone = "m0"
note = "ADR-0015"

[[feature]]
id = "error-count"
title = "The device's failures counted in UCB$L_ERRCNT, shown by SHOW DEVICES"
status = "done"

[[feature]]
id = "ucb-items"
title = "$GETDVI from the disk's UCB, not its VCB: operation counts, MAXBLOCK, device type"
status = "todo"
milestone = "m1"
note = "PRD-0003 tier 1 #9; DESIGN-0002 Devices"

[[feature]]
id = "virtual-io"
title = "IO$_READVBLK and IO$_WRITEVBLK on a file's channel, mapped through the XQP"
status = "todo"
note = "ADR-0007 follow-up; the XQP maps the file's VBNs to LBNs (//files/ods2:qio-interface). Not a need: the XQP sits on this driver, so the link runs from //files/ods2 down"

[[feature]]
id = "flush"
title = "A virtio-blk flush request, so writes reach the disk before the next ones"
status = "todo"
milestone = "m1"
note = "PRD-0003 tier 2 #11: what //files/write-ordering builds on for its ordering and the flush on DISMOUNT"

[[feature]]
id = "queued"
title = "Requests queued, completed by the device's interrupt"
status = "todo"
needs = [":start-io", "//drivers/interrupt-io"]
note = "PRD-0003 tier 4 #37"

[[feature]]
id = "more-disks"
title = "Any number of disks, found at boot from the device tree"
status = "todo"
note = "pal/src/main.c: DISK_UNITS 2, units from QEMU's -device order; PRD-0003 tier 4 #38"

[[feature]]
id = "multipath"
title = "Multipath: one disk reached by several paths, SET DEVICE/SWITCH"
status = "todo"

[[feature]]
id = "logical-disks"
title = "Logical disks, LDDRIVER: a container file mounted as a disk"
status = "todo"
note = "Freeware, in OpenVMS since V7.3-1"

[[needs]]
on = "//exec/qio"
why = "$QIO's FDT routine and IRPs"

[[needs]]
on = "//platform/pal"
why = "READLBLK and WRITELBLK: the PAL does the block I/O"

[[needs]]
on = "//platform/virtio"
why = "The disks are virtio-blk devices the PAL drives"

[[needs]]
on = "//exec/privileges"
why = "LOG_IO and PHY_IO"
+++

VMS's disk drivers (`DKDRIVER` for SCSI, `DUDRIVER` for MSCP, `DQDRIVER`
for IDE, and others) do block I/O for the file system and for programs.
Logical functions read and write by logical block number, physical ones
by the device's geometry, both needing `LOG_IO` or `PHY_IO`; virtual
functions on a file's channel are mapped to logical blocks by the XQP.
Drivers retry, replace bad blocks, count errors for `SHOW DEVICES` and
the error log, and queue requests, which the controller completes by
interrupt. `LDDRIVER`, in OpenVMS since V7.3-1, makes a container file
a disk.

vaxpunk's disk driver is `DK$FDT` and `DK$STARTIO` in
`vms/exec/f11.mar`, with UCBs for `DKA0:`, `DKB0:` and `MDA0:` in
`qio.mar`. The PAL drives the virtio-blk devices itself, polled and a
request at a time (`pal/src/main.c`): `READLBLK` reads either disk,
`WRITELBLK` writes `DKB0:`; QEMU attaches `DKA0:` read only
([ADR-0007](../../docs/adr/0007-system-disk-files-11-and-rms.md),
[ADR-0012](../../docs/adr/0012-data-disk-writable-files-11.md)). The file
system hands its own I/O to the start I/O routine in an IRP of its own
([ADR-0015](../../docs/adr/0015-file-system-io-through-the-disk-driver.md)).
Each request finishes before `DK$STARTIO` returns (PRD-0003 tier 4 #37).
