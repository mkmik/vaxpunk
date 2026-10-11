+++
title = "Files-11 ODS-2 (XQP)"
summary = "The disk structure, and the XQP that reads and writes it in the executive"
aka = ["XQP", "F11BXQP", "F11BACP", "ACP", "Files-11 structure level 2", "f11.mar", "f11wrt.mar"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["docs/adr/0007-system-disk-files-11-and-rms.md", "docs/adr/0009-ramdisk-writable-files-11.md", "docs/adr/0012-data-disk-writable-files-11.md", "docs/adr/0015-file-system-io-through-the-disk-driver.md", "docs/adr/0020-file-system-lock-below-ipl-synch.md", "docs/design/0002-executive-processes.md", "vms/exec/f11.mar", "vms/exec/f11wrt.mar", "crosstools/ods/docs/", "docs/prd/0003-multi-user-vms.md"]

[[feature]]
id = "read"
title = "Mount a volume; read headers, maps and directories"
status = "done"
milestone = "m0"
refs = ["docs/adr/0007-system-disk-files-11-and-rms.md"]

[[feature]]
id = "write"
title = "Create, extend and delete files and directories"
status = "done"
milestone = "m0"
refs = ["docs/adr/0009-ramdisk-writable-files-11.md", "docs/adr/0012-data-disk-writable-files-11.md"]

[[feature]]
id = "driver-irp"
title = "Block I/O as an IRP to the disk's start I/O routine"
status = "done"
milestone = "m0"
refs = ["docs/adr/0015-file-system-io-through-the-disk-driver.md"]

[[feature]]
id = "protection"
title = "Owner and protection mask checked on every access (FIL$CHKPRO)"
status = "done"
milestone = "m1"
note = "PRD-0003 step 2"

[[feature]]
id = "process-default-protection"
title = "New files take the process's default protection"
status = "todo"
milestone = "m1"
note = "PRD-0003 tier 1 #8; f11wrt.mar uses VMS's default, FILEPROT"
needs = ["//exec/uic-protection"]

[[feature]]
id = "version-limits"
title = "Version limits on directories and files"
status = "todo"
milestone = "m1"
note = "PRD-0003 tier 1 #8; rms.mar's $CREATE_DIR takes none"

[[feature]]
id = "spilled-versions"
title = "A name whose versions fill more than one directory record"
status = "todo"
milestone = "m1"
note = "PRD-0003 tier 2 #15: FIL$SEARCHDIR matches it twice"

[[feature]]
id = "dir-insert"
title = "Directory records inserted in place, not the directory rewritten"
status = "todo"
milestone = "m1"
note = "PRD-0003 tier 2 #15; f11wrt.mar rewrites the whole directory"

[[feature]]
id = "big-volumes"
title = "Clusters, multi-block bitmaps: more than 4,096 blocks and files"
status = "todo"
note = "PRD-0003 tier 3 #22; ADR-0009, ADR-0012"

[[feature]]
id = "extension-headers"
title = "Extension headers when a file's map fills"
status = "todo"
note = "PRD-0003 tier 3 #22; PRD-0008 open question"

[[feature]]
id = "free-count"
title = "Free block count kept in the VCB, not counted each time"
status = "todo"
note = "PRD-0003 tier 3 #22; FIL$FREEBLOCKS"

[[feature]]
id = "qio-interface"
title = "IO$_ACCESS, IO$_CREATE, IO$_MODIFY, IO$_DELETE with a FIB on disk channels"
status = "todo"
note = "ADR-0007 follow-up; PRD-0003 tier 3 #23 (SET PROTECTION should ask the XQP)"

[[feature]]
id = "queued-io"
title = "Waits for a queued IRP from an interrupt-driven disk"
status = "todo"
note = "PRD-0003 tier 4 #37; ADR-0015, ADR-0020"
needs = ["//drivers/interrupt-io"]

[[feature]]
id = "system-disk-writes"
title = "Writing the system disk, DKA0:"
status = "todo"
note = "ADR-0007 and ADR-0012 keep it read only"

[[feature]]
id = "highwater"
title = "High-water marking and erase on delete"
status = "todo"

[[needs]]
on = "//drivers/disk"
why = "Reads and writes blocks through its start I/O routine"

[[needs]]
on = "//drivers/ramdisk"
why = "MDA0: holds a volume it writes"

[[needs]]
on = "//exec/qio"
why = "Its block I/O is an IRP of its own, F11$AB_IRP"

[[needs]]
on = "//exec/ipl-sync"
why = "Raises IPL to IPL$_SYNCH around the driver call"

[[needs]]
on = "//files/fs-locking"
why = "Every caller holds the file system's lock"

[[needs]]
on = "//exec/uic-protection"
why = "FIL$CHKPRO applies UIC-based protection"

[[needs]]
on = "//exec/privileges"
why = "SYSPRV, GRPPRV, READALL and BYPASS change the checks"

[[needs]]
on = "//exec/pool"
why = "Directories and image headers are read into pool"

[[needs]]
on = "//host/ods"
why = "The model its layouts and checks come from; ods verify checks its volumes"
soft = true
+++

Files-11 On-Disk Structure level 2 is the disk format VMS has used since
VAX/VMS V1: a home block, an index file (`INDEXF.SYS`) of file headers, a
storage bitmap (`BITMAP.SYS`), directories that are files of name records,
and nine reserved files in the MFD, `[000000]`. Each header holds the
file's map (retrieval pointers to runs of blocks), owner, protection,
dates and RMS record attributes. Up to VAX/VMS V3 an ancillary control
process, `F11BACP`, did the file system's work for other processes;
V4 replaced it with the XQP, the Extended QIO Processor, which runs in
kernel mode in the process that asks, behind the `$QIO` functions
`IO$_ACCESS`, `IO$_CREATE`, `IO$_MODIFY` and `IO$_DELETE` with a file
information block (FIB). RMS calls it; so do `BACKUP`, `ANALYZE/DISK` and
`DIRECTORY`. Tapes and DECnet kept ACP processes (`MTAACP`, `NETACP`).

vaxpunk's XQP is `vms/exec/f11.mar` (mount, headers, maps, directory
search with wildcards, `FIL$CHKPRO`) and `f11wrt.mar` (allocation,
directories, `FIL$INIT`). It writes real ODS-2 that `ods verify` and
OpenVMS accept ([ADR-0007](../../docs/adr/0007-system-disk-files-11-and-rms.md),
[ADR-0009](../../docs/adr/0009-ramdisk-writable-files-11.md),
[ADR-0012](../../docs/adr/0012-data-disk-writable-files-11.md)). RMS calls its
`FIL$` routines directly instead of through `$QIO`, so there are no XQP
functions on disk channels yet. Its I/O goes to the disk driver's start
I/O routine in an IRP of its own
([ADR-0015](../../docs/adr/0015-file-system-io-through-the-disk-driver.md)).
The corners cut are in PRD-0003's backlog: a cluster is a block and each
bitmap one block, so a volume holds at most 4,096 blocks and files; no
extension headers; directories rewritten whole; no version limits. ACLs
in headers wait for `//exec/acls`, a non-goal of Milestone 1.
