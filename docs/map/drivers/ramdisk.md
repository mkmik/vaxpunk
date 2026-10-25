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
title = "MDA0:, blocks of S0 pages, made afresh by each INITIALIZE"
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
status = "done"
note = "INIT$_SIZE; up to 255 MB of S0, as the RAM allows"

[[feature]]
id = "units"
title = "More than one unit, MDA1:, ..."
status = "todo"

[[needs]]
on = "/drivers/disk"
why = "DK$STARTIO calls MD$IO where it would call the PAL"

[[needs]]
on = "/exec/memory"
why = "Its blocks are pages of S0 that MMG$CREPAG maps"
+++

DECram made a disk of memory: `MDDRIVER`'s units, `MDA0:` and on, take
their size from `INITIALIZE/SIZE`, then are mounted and used like any
disk, and are lost at shutdown. It was a layered product until OpenVMS
V8.2 made it part of the system. Sites used it for scratch files and for
files read often.

vaxpunk's is `vms/exec/mddriver.mar`: blocks in pages of S0 from
`0x50000000`, which the first `INITIALIZE MDA0:` makes, zeroed;
`INITIALIZE/SIZE=n` (`$INIT_VOL`'s `INIT$_SIZE`) gives how many, and
without it there are as many as the last time, 1,024 at first. Until
then the device is offline
([ADR-0009](../../adr/0009-ramdisk-writable-files-11.md)). The
disks' start I/O routine copies blocks in and out. It was the first disk
the file system could write. `SYSTARTUP_VMS.COM` makes and mounts it at
each boot as `SYS$SPECIFIC`, where the UAF, TCP/IP's files and anything
written to `SYS$SYSTEM` or `SYS$MANAGER` go; it makes it 2,048 blocks.
