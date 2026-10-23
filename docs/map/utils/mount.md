+++
title = "INITIALIZE, MOUNT, DISMOUNT"
summary = "Write an empty volume on a disk, mount it for use, dismount it"
aka = ["INITIALIZE", "INIT", "MOUNT", "DISMOUNT", "INIT.EXE", "MOUNT.EXE", "DISMOUNT.EXE", "$INIT_VOL", "$MOUNT", "$DISMOU", "MOUNT/SYSTEM", "MOUNT/FOREIGN"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["vms/sysexe/init.mar", "vms/sysexe/mount.mar", "vms/sysexe/dismount.mar", "vms/cld/initialize.cld", "vms/cld/mount.cld", "vms/cld/dismount.cld", "docs/adr/0009-ramdisk-writable-files-11.md", "docs/adr/0012-data-disk-writable-files-11.md", "docs/prd/0003-multi-user-vms.md", "vms/sysmgr/systartup_vms.com", "https://docs.vmssoftware.com/vsi-openvms-dcl-dictionary-a-m/"]

[[feature]]
id = "init"
title = "INITIALIZE device label: an empty ODS-2 volume on DKB0: or MDA0:"
status = "done"
milestone = "m0"
note = "ADR-0009, ADR-0012"

[[feature]]
id = "mount"
title = "MOUNT device [label [logical-name]], seen by every process, as DISK$label without a name"
status = "done"
milestone = "m0"
note = "ADR-0009; mounted at boot by SYSTARTUP_VMS.COM"

[[feature]]
id = "dismount"
title = "DISMOUNT device"
status = "done"
milestone = "m0"
note = "ADR-0009 and ADR-0012 follow-up"

[[feature]]
id = "owner-protection"
title = "/OWNER_UIC and /PROTECTION on INITIALIZE and MOUNT"
status = "done"
milestone = "m1"
note = "PRD-0003 step 2"

[[feature]]
id = "init-size"
title = "INITIALIZE/SIZE for the ramdisk"
status = "todo"
needs = ["/files/ods2#big-volumes"]
note = "PRD-0003 tier 3 #36; a disk's volume is already its size (ADR-0043)"

[[feature]]
id = "init-qualifiers"
title = "/CLUSTER_SIZE, /HEADERS, /MAXIMUM_FILES, /INDEX, /STRUCTURE=5, /ERASE"
status = "todo"
note = "PRD-0003 tier 3 #24"

[[feature]]
id = "backup-home"
title = "The backup home block and index file header INITIALIZE writes"
status = "todo"
milestone = "m1"
needs = ["/files/write-ordering"]
note = "PRD-0003 tier 2 #11"

[[feature]]
id = "mount-qualifiers"
title = "MOUNT/SYSTEM, /GROUP, private mounts, /NOWRITE, /OVERRIDE"
status = "todo"
note = "PRD-0003 tier 3 #24; f11.mar ponytail: every mount is /SYSTEM"

[[feature]]
id = "dismount-qualifiers"
title = "DISMOUNT/UNLOAD, /NOUNLOAD, /ABORT; a flush before it dismounts"
status = "todo"
note = "dismount.mar ponytail; PRD-0003 tier 2 #11 wants the flush"

[[feature]]
id = "foreign"
title = "MOUNT/FOREIGN: a disk or tape read as blocks, with no file structure"
status = "todo"

[[feature]]
id = "tapes"
title = "Tapes: ANSI labels, INITIALIZE and MOUNT of a magnetic tape"
status = "todo"

[[feature]]
id = "cd"
title = "CD-ROMs: ISO 9660 and High Sierra volumes"
status = "todo"

[[feature]]
id = "volume-sets"
title = "MOUNT/BIND: a volume set across several disks"
status = "todo"

[[feature]]
id = "cluster"
title = "MOUNT/CLUSTER, and shadow sets"
status = "todo"
note = "Clusters are a non-goal of Milestone 1 (t8y:/exec/cluster)"

[[needs]]
on = "/files/volumes"
why = "$MOUNT, $DISMOU and $INIT_VOL: the volume control blocks they make and remove"

[[needs]]
on = "/files/ods2"
why = "FIL$INIT writes the empty volume; FIL$MOUNT reads its home block"

[[needs]]
on = "/drivers/disk"
why = "DKB0:, the data disk it writes"

[[needs]]
on = "/drivers/ramdisk"
why = "MDA0:, which INITIALIZE makes afresh"

[[needs]]
on = "/exec/privileges"
why = "MOUNT takes SYSNAM, and VOLPRO for another's volume"

[[needs]]
on = "/rtl/cli"
why = "Their parameters and qualifiers"

[[needs]]
on = "/drivers/tape"
why = "Tape volumes and their ANSI labels"
soft = true

[[needs]]
on = "/files/iso9660"
why = "Mounting a CD-ROM"
soft = true

[[needs]]
on = "/files/volume-sets"
why = "MOUNT/BIND"
soft = true

[[needs]]
on = "/exec/cluster"
why = "MOUNT/CLUSTER"
soft = true
+++

`INITIALIZE device label` writes an empty volume on a disk or tape: on
a disk, a Files-11 home block, index file, bitmap and MFD, with
qualifiers for the cluster size, the number of headers, the structure
level (ODS-2 or ODS-5), the owner and protection. `MOUNT` makes a
volume known to the system, for one process, a group, every process
(`/SYSTEM`), or every node of a cluster (`/CLUSTER`), and gives it a
logical name; `/FOREIGN` mounts it as raw blocks. `DISMOUNT` writes back
what is cached and removes it, unloading the drive. They also handle
tapes with ANSI labels, CD-ROMs, volume sets and shadow sets.

vaxpunk's are `vms/sysexe/init.mar`, `mount.mar` and `dismount.mar`,
images that call `$INIT_VOL`, `$MOUNT` and `$DISMOU` in the executive
([ADR-0009](../../adr/0009-ramdisk-writable-files-11.md),
[ADR-0012](../../adr/0012-data-disk-writable-files-11.md)). They
work on the data disk, `DKB0:`, and the ramdisk, `MDA0:`, which
`SYSTARTUP_VMS.COM` initializes and mounts at each boot; `DKA0:` is
mounted by the executive, write locked. `/OWNER_UIC` and `/PROTECTION`
are the only qualifiers (PRD-0003 tier 3 #24). Every mount is a system
mount, whose logical name, `DISK$label` or `MOUNT`'s third parameter,
goes in the system table, and a volume on `DKB0:` is the disk's size
([ADR-0043](../../adr/0043-volumes-the-size-of-their-disk.md)).
