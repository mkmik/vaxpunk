+++
title = "Driver loading and autoconfiguration"
summary = "Loadable drivers, the I/O database, and SYSMAN IO AUTOCONFIGURE"
aka = ["I/O database", "DDB", "CRB", "IDB", "DDT", "SYSMAN IO AUTOCONFIGURE", "IO CONNECT", "SYSGEN CONNECT", "SYS$LOADABLE_IMAGES", "IOGEN", "SYS$USER_CONFIG.DAT", "device tree"]
kind = "core"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["docs/adr/0013-qio-irps-and-drivers.md", "docs/design/0001-pal-interface.md", "docs/prd/0003-multi-user-vms.md"]

[[needs]]
on = "//exec/qio"
why = "Drivers plug into $QIO through their UCBs and dispatch tables"

[[needs]]
on = "//drivers/interrupt-io"
why = "A loaded driver connects its interrupt service routine to its device's vector"

[[needs]]
fuzzy = "Executive images loaded at run time, with their own relocation and symbol vector"
why = "A driver is a loadable image in SYS$LOADABLE_IMAGES"

[[needs]]
fuzzy = "Device discovery from the device tree or ACPI, in the PAL or the executive"
why = "What AUTOCONFIGURE finds; QEMU virt's addresses are fixed today"
+++

On VMS a device driver is an image in `SYS$LOADABLE_IMAGES`, loaded
into the executive at boot or later. The I/O database records what
exists: a device data block (DDB) per controller type, a unit control
block (UCB) per unit, channel request and interrupt dispatch blocks
(CRB, IDB) for the controller and its interrupt, and the driver's
dispatch table (DDT). At boot, `SYSMAN IO AUTOCONFIGURE` (`SYSGEN
AUTOCONFIGURE` on VAX) walks the buses, names each device by its
controller letter and unit (`DKA100:`, `EWB0:`), and loads its driver;
`IO CONNECT` adds one by hand, and `SYS$USER_CONFIG.DAT` teaches it
devices it doesn't know.

vaxpunk's drivers are linked into `EXEC.EXE`, each a UCB with an FDT
routine and a start I/O routine; [ADR-0013](../../docs/adr/0013-qio-irps-and-drivers.md)
chose that over DDBs, CRBs, IDBs and a DDT. The PAL finds the virtio
devices at QEMU virt's fixed addresses (PRD-0003 tier 4 #38). Real
hardware (//platform/real-hardware) or many kinds of device would need
this.
