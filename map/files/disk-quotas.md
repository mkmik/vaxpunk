+++
title = "Disk quotas"
summary = "Blocks each UIC may own on a volume: QUOTA.SYS and SYSMAN DISKQUOTA"
aka = ["QUOTA.SYS", "DISKQUOTA", "SYSMAN DISKQUOTA", "SHOW QUOTA", "EXQUOTA", "SS$_EXDISKQUOTA", "MOUNT/QUOTA", "overdraft"]
kind = "core"
intent = "aspiration"
research = "outline"
refs = ["https://h41379.www4.hpe.com/doc/84final/6048/6048pro_077.html", "crosstools/ods/docs/vms-check.md"]

[[feature]]
id = "quota-file"
title = "[000000]QUOTA.SYS: usage, permanent quota and overdraft per UIC"
status = "todo"

[[feature]]
id = "enforce"
title = "Allocation charged to the file's owner; SS$_EXDISKQUOTA past quota and overdraft"
status = "todo"
needs = [":quota-file", "//exec/privileges"]
note = "EXQUOTA exempts; files marked NOCHARGE aren't charged"

[[feature]]
id = "sysman"
title = "SYSMAN DISKQUOTA: CREATE, ADD, MODIFY, REMOVE, SHOW, ENABLE, DISABLE, REBUILD"
status = "todo"
needs = [":quota-file"]
note = "The commands are //utils/sysman's; this is the XQP side they call"

[[feature]]
id = "show-quota"
title = "SHOW QUOTA, and the XQP's quota functions through $QIO"
status = "todo"
needs = [":quota-file", "//files/ods2:qio-interface"]

[[feature]]
id = "identifiers"
title = "Rights identifiers as quota holders, not only UICs"
status = "todo"
needs = [":quota-file", "//exec/rights-database"]

[[needs]]
on = "//files/ods2"
why = "Allocation and deallocation update the owner's entry"

[[needs]]
on = "//exec/uic-protection"
why = "Entries are kept per owner UIC"

[[needs]]
on = "//files/fs-locking"
why = "The quota entries are cached and locked across processes"
+++

A volume may carry a quota file, `[000000]QUOTA.SYS`, with an entry per
UIC or rights identifier: the blocks its files use, a permanent quota and
an overdraft. The XQP charges each allocation to the file's owner and
fails with `SS$_EXDISKQUOTA` once usage would pass quota plus overdraft;
`EXQUOTA` lets a process past it. The system manager makes and changes
the file with `SYSMAN DISKQUOTA` (`CREATE`, `ADD`, `MODIFY`, `REBUILD`,
which recounts usage from the headers, and the rest); `[0,0]`'s entry
gives the defaults for new owners. Users see their own with `SHOW
QUOTA`. `MOUNT/NOQUOTA` mounts a volume without enforcement.

vaxpunk has no disk quotas, and PRD-0003's quotas are the process quotas
of the UAF, not these. The volumes `ods` writes have no quota file;
VMS's `ANALYZE/DISK_STRUCTURE` only remarks on it.
