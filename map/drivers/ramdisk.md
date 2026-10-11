+++
title = "Ramdisk"
summary = "MDA0:, a disk in memory, as DECram's MDDRIVER"
aka = ["MDA0:", "MDDRIVER", "mddriver.mar", "DECram", "MD$IO", "INITIALIZE/SIZE"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["docs/adr/0009-ramdisk-writable-files-11.md", "vms/exec/mddriver.mar", "vms/sysmgr/systartup_vms.com", "docs/prd/0003-multi-user-vms.md"]

[[feature]]
id = "ramdisk"
title = "MDA0:, 1,024 blocks of S0 pages, made afresh by each INITIALIZE"
status = "done"
milestone = "m0"
note = "ADR-0009"

[[feature]]
id = "sys-specific"
title = "Made and mounted at boot; SYS$SPECIFIC, the writable root of SYS$SYSROOT"
status = "done"
milestone = "m0"
note = "SYSTARTUP_VMS.COM"

[[feature]]
id = "size"
title = "INITIALIZE/SIZE gives its size"
status = "todo"
note = "PRD-0003 tier 3 #36; mddriver.mar ponytail"

[[feature]]
id = "units"
title = "More than one unit, MDA1:, ..."
status = "todo"

[[needs]]
on = "//drivers/disk"
why = "DK$STARTIO calls MD$IO where it would call the PAL"

[[needs]]
on = "//exec/memory"
why = "Its blocks are pages of S0 that MMG$CREPAG maps"
+++

DECram made a disk of memory: `MDDRIVER`'s units, `MDA0:` and on, take
their size from `INITIALIZE/SIZE`, then are mounted and used like any
disk, and are lost at shutdown. It was a layered product until OpenVMS
V8.2 made it part of the system. Sites used it for scratch files and for
files read often.

vaxpunk's is `vms/exec/mddriver.mar`: 1,024 blocks in pages of S0 from
`0x50000000`, which the first `INITIALIZE MDA0:` makes, zeroed; until
then the device is offline
([ADR-0009](../../docs/adr/0009-ramdisk-writable-files-11.md)). The
disks' start I/O routine copies blocks in and out. It was the first disk
the file system could write. `SYSTARTUP_VMS.COM` makes and mounts it at
each boot as `SYS$SPECIFIC`, where the UAF, TCP/IP's files and anything
written to `SYS$SYSTEM` or `SYS$MANAGER` go. Its size is fixed until
`INITIALIZE` takes `/SIZE` (PRD-0003 tier 3 #36).
