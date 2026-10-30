+++
title = "Volumes and mounting"
summary = "VCBs, $MOUNT, $DISMOU and $INIT_VOL: the disks that hold a volume"
aka = ["VCB", "$MOUNT", "$DISMOU", "$INIT_VOL", "MOUNT", "INITIALIZE", "DISMOUNT", "DISK$", "mount verification", "/FOREIGN", "SET VOLUME"]
kind = "core"
intent = "goal"
research = "outline"
milestone = "m0"
refs = ["docs/adr/0007-system-disk-files-11-and-rms.md", "docs/adr/0009-ramdisk-writable-files-11.md", "docs/adr/0012-data-disk-writable-files-11.md", "docs/design/0002-executive-processes.md", "vms/exec/f11.mar", "vms/exec/f11wrt.mar", "vms/sysexe/mount.mar", "vms/sysexe/init.mar", "vms/sysexe/dismount.mar", "vms/sysmgr/systartup_vms.com", "https://wasd.vsm.com.au/htbin/helpgate/HELP/MOUNT/QUALIFIERS", "https://de.openvms.org/TUD2010/File_System_Changes.pdf"]

[[feature]]
id = "mount"
title = "A VCB per disk; $MOUNT reads the home block and checks the label"
status = "done"
milestone = "m0"
refs = ["docs/adr/0007-system-disk-files-11-and-rms.md", "docs/adr/0009-ramdisk-writable-files-11.md"]

[[feature]]
id = "init"
title = "$INIT_VOL writes an empty ODS-2 volume, laid out as INITIALIZE does"
status = "done"
milestone = "m0"
note = "ADR-0009, ADR-0012; FIL$INIT in f11wrt.mar"

[[feature]]
id = "boot"
title = "DKA0: mounted write-locked at boot; SYSTARTUP_VMS.COM mounts MDA0: and DKB0:"
status = "done"
milestone = "m0"
note = "ADR-0007, ADR-0012"

[[feature]]
id = "dismount"
title = "$DISMOU, refused for the system disk and while a file is open"
status = "done"
milestone = "m0"
note = "f11.mar EXE$DISMOU, RMS$VOLIDLE"

[[feature]]
id = "protection"
title = "A volume's owner and protection; /OWNER_UIC and /PROTECTION; SYSNAM and VOLPRO"
status = "done"
milestone = "m1"
note = "PRD-0003 step 2"

[[feature]]
id = "size"
title = "INITIALIZE/SIZE, and a volume as big as its disk"
status = "todo"
needs = ["/files/ods2#big-volumes"]
note = "PRD-0003 tier 3 #36; ADR-0012: DKB0:'s volume is 4,096 blocks whatever the disk's size"

[[feature]]
id = "more-disks"
title = "Any number of disks, found from the devices, not three fixed VCBs"
status = "todo"
needs = ["/drivers/disk"]
note = "ADR-0012 follow-up; FIL$SELECT knows DKA0:, DKB0: and MDA0: only"

[[feature]]
id = "qualifiers"
title = "MOUNT /GROUP, /SHARE; INITIALIZE /CLUSTER_SIZE, /HEADERS; DISMOUNT /ABORT"
status = "todo"
note = "PRD-0003 tier 3 #24; f11.mar: $MOUNT takes no MNT$M_GROUP or MNT$M_SHARE, $DISMOU no flags"

[[feature]]
id = "private"
title = "Private mounts: the disk allocated to the job, dismounted when it ends; MNT$M_SYSTEM, MNT$M_NOWRITE, MNT$M_OVR_IDENT"
status = "done"
note = "VCB$L_PID, the job's master process, which FIL$CHKALLOC checks for $ASSIGN and RMS and FIL$RUNDOWN dismounts; VMS keeps it in UCB$L_PID (t8y:/exec/qio#allocate)"

[[feature]]
id = "logical-names"
title = "MOUNT defines DISK$label, and the logical name it is given"
status = "done"
note = "MNT$_LOGNAM, both in the system table for MOUNT/SYSTEM, else the process's; the system disk's DISK$label at boot; $DISMOU deletes them"

[[feature]]
id = "foreign"
title = "Foreign mounts: a disk with no Files-11 structure, read and written by LBN"
status = "todo"
needs = ["/exec/qio"]

[[feature]]
id = "mount-verification"
title = "Mount verification: a disk's I/O held and retried while it is offline"
status = "todo"
needs = ["/drivers/interrupt-io"]

[[feature]]
id = "set-volume"
title = "SET VOLUME: label, owner, protection, default file protection"
status = "done"
note = "FIL$SETVOLUME, in kernel mode through $CMKRNL, rewrites each home block (ADR-0039); SET.EXE is a known image with CMKRNL, as VMS installs it: t8y:/utils/set-show#set-privileged"

[[feature]]
id = "volume-characteristics"
title = "SET VOLUME/[NO]ERASE_ON_DELETE and /[NO]HIGHWATER_MARKING, and what they do"
status = "todo"

[[feature]]
id = "expansion"
title = "Dynamic volume expansion: SET VOLUME/SIZE up to INITIALIZE/LIMIT"
status = "todo"

[[needs]]
on = "/files/ods2"
why = "Mounting reads the home block and INDEXF.SYS; INITIALIZE writes them"

[[needs]]
on = "/drivers/disk"
why = "Each VCB names its disk's UCB and start I/O routine"

[[needs]]
on = "/drivers/ramdisk"
why = "MDA0:, whose pages $INIT_VOL takes"

[[needs]]
on = "/files/fs-locking"
why = "$MOUNT, $DISMOU and $INIT_VOL take the file system's lock"

[[needs]]
on = "/exec/logical-names"
why = "MOUNT defines DISK$label, and DISMOUNT deletes it"

[[needs]]
on = "/exec/privileges"
why = "SYSNAM to mount for everyone, VOLPRO for another's volume"

[[needs]]
on = "/exec/uic-protection"
why = "A volume has an owner UIC and a protection mask"

[[needs]]
on = "/host/ods"
why = "ods init lays volumes out as VMS does; initialize.md is FIL$INIT's model"
soft = true
+++

A volume is a disk with a file structure on it. In VMS each mounted
volume has a volume control block (VCB) in nonpaged pool: its label, its
free block count, its index file's map, and the XQP's caches. `MOUNT`
checks the label in the home block and makes the VCB, for the process,
its group (`/GROUP`) or everyone (`/SYSTEM`), and defines `DISK$label`;
`INITIALIZE` writes an empty volume with a chosen cluster size, header
count and protection; `DISMOUNT` takes the volume away once no file on it
is open. Mount verification holds a disk's I/O while it is offline and
resumes it when the disk comes back. `/FOREIGN` mounts a disk or tape
without reading a structure, for programs that use LBNs, and
`MOUNT/CLUSTER` mounts a volume on every node of a cluster. A volume could
hold 1 TB until V8.4, then 2 TB, with 64-bit LBNs in the drivers.

vaxpunk has three disks, each with a fixed VCB in `f11.mar`: `DKA0:`, the
system disk, mounted at boot and write-locked; `DKB0:`, a disk image on
the host that keeps its volume from boot to boot; and `MDA0:`, the
ramdisk ([ADR-0007](../../adr/0007-system-disk-files-11-and-rms.md),
[ADR-0009](../../adr/0009-ramdisk-writable-files-11.md),
[ADR-0012](../../adr/0012-data-disk-writable-files-11.md)).
`$MOUNT`, `$DISMOU` and `$INIT_VOL` are system services, which
`MOUNT.EXE`, `INIT.EXE` and `DISMOUNT.EXE` (`t8y:/utils/mount`) call, and
`SYSTARTUP_VMS.COM` mounts the ramdisk and the data disk. Each mount
defines `DISK$label`, and the logical name `MOUNT`'s third parameter
gives, as the device, `_DKB0:`, in the system table for `MOUNT/SYSTEM`,
and `DISMOUNT` deletes them. A mount without `/SYSTEM` is private: the
VCB holds the job's master process's PID, as VMS's UCB holds the
allocating process's, another job can't assign a channel to the disk or
open its files, the names go in that process's table, and process
deletion dismounts it. `/GROUP` and `/SHARE` are still missing (PRD-0003
tier 3 #24).
