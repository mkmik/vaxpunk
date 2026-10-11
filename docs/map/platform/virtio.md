+++
title = "virtio devices (blk, net, rng)"
summary = "QEMU's virtio-mmio disks, network and entropy device, driven by the PAL"
aka = ["virtio", "virtio-mmio", "virtio-blk", "virtio-net", "virtio-rng", "sysdisk.img", "datadisk.img"]
kind = "platform"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["docs/design/0001-pal-interface.md", "docs/adr/0007-system-disk-files-11-and-rms.md", "docs/adr/0012-data-disk-writable-files-11.md", "docs/adr/0015-file-system-io-through-the-disk-driver.md", "docs/adr/0031-entropy-from-virtio-rng.md", "pal/src/main.c", "scripts/run-qemu.sh"]

[[feature]]
id = "blk"
title = "virtio-blk: the system disk, read only, and the data disk, read-write"
status = "done"
milestone = "m0"
note = "disk_init in main.c; modern (virtio 1) mmio transports; ADR-0007, ADR-0012"

[[feature]]
id = "net"
title = "virtio-net, found by the PAL and driven by the TCP/IP component"
status = "done"
milestone = "m0"
note = "ADR-0016; its interrupt goes to the component"

[[feature]]
id = "rng"
title = "virtio-rng for GETENTROPY"
status = "done"
note = "ADR-0031"

[[feature]]
id = "blk-interrupts"
title = "Disk requests queued and completed by interrupt, not polled one at a time"
status = "todo"
note = "PRD-0003 tier 4 #37; main.c ponytail; the file system then waits on a queued IRP"

[[feature]]
id = "more-units"
title = "More disks than the two fixed units"
status = "todo"
note = "DISK_UNITS 2 in main.c"

[[feature]]
id = "pci"
title = "virtio over PCI, and transports found from the DTB"
status = "todo"
note = "PRD-0003 tier 4 #38: VIRTIO_PADDR is QEMU virt's fixed address"

[[needs]]
on = "/platform/sel4"
why = "Device untypeds for the registers, frames with physical addresses for the queues"
+++

VMS on a VAX or an Alpha drove real controllers: MSCP disks through
UDA50 and HSC ports, SCSI through `PKDRIVER`, DEC's Ethernet chips
through `EWDRIVER` and friends. VSI's x86-64 port, which runs mostly in
virtual machines, added drivers for virtio and other virtual hardware.
VAX and Alpha had no random number device at all.

vaxpunk runs on QEMU's `virt` machine and uses its virtio-mmio
transports, in virtio 1 mode. The PAL drives the disks and the entropy
device itself, a request at a time, polling the used ring
([DESIGN-0001](../../design/0001-pal-interface.md)): unit 0 is the
system disk `sysdisk.img`, read only, unit 1 the data disk
`datadisk.img`, and the last transport is virtio-rng
([ADR-0031](../../adr/0031-entropy-from-virtio-rng.md)). The network
device is found by the PAL and handed to the TCP/IP component, which
drives it with its own interrupt. The executive sees the disks only
through `READLBLK` and `WRITELBLK`, called by `DK$STARTIO`
([ADR-0015](../../adr/0015-file-system-io-through-the-disk-driver.md)).
