+++
title = "ANALYZE/DISK_STRUCTURE"
summary = "Checks a Files-11 volume's structure and repairs it, lost files to [SYSLOST]"
aka = ["ANALYZE/DISK_STRUCTURE", "ANALYZE/DISK", "VERIFY", "SYSLOST", "ANALYZE/DISK/REPAIR", "ANALYZE/DISK/USAGE", "ANALYZE/MEDIA"]
kind = "core"
intent = "aspiration"
research = "outline"
refs = ["crosstools/ods/docs/verify.md", "crosstools/ods/docs/vms-check.md", "docs/prd/0003-multi-user-vms.md", "https://docs.vmssoftware.com/vsi-openvms-system-management-utilities-reference-manual-volume-i-a-l/"]

[[feature]]
id = "check"
title = "Report what is wrong: bitmaps, headers, directories, lost files"
status = "todo"

[[feature]]
id = "repair"
title = "/REPAIR: fix the bitmaps, enter lost files in [SYSLOST]"
status = "todo"
needs = ["#check"]

[[feature]]
id = "usage"
title = "/USAGE: blocks used and allocated by each owner"
status = "todo"
needs = ["#check"]

[[feature]]
id = "read-check"
title = "/READ_CHECK: read every block of every file"
status = "todo"
needs = ["#check"]

[[needs]]
on = "/files/ods2"
why = "Reads the index file, bitmaps and directories it checks"

[[needs]]
on = "/files/ods2#qio-interface"
why = "Reads headers and changes them through the XQP's $QIO functions"

[[needs]]
on = "/drivers/disk"
why = "Reads logical blocks of the volume"

[[needs]]
on = "/files/fs-locking"
why = "/REPAIR locks the volume against other writers"

[[needs]]
on = "/exec/privileges"
why = "Takes SYSPRV or BYPASS to read every file"

[[needs]]
on = "/host/ods"
why = "ods verify already checks the same rules on the host, a model to follow"
soft = true
+++

`ANALYZE/DISK_STRUCTURE`, the Verify utility, reads a whole Files-11
volume and checks that its parts agree: every block a file maps is
marked in use in the storage bitmap and in no other file, every header
in use is marked in the index file bitmap, every directory entry points
at a live file, and every file is in some directory. It reports what it
finds; `/REPAIR` fixes the bitmaps and enters lost files in
`[SYSLOST]`, `/USAGE` writes a file of each owner's blocks, and
`/READ_CHECK` reads every block. It is run after a crash or before a
`BACKUP/IMAGE`. `ANALYZE/MEDIA`, the Bad Block Locator, is its
companion for the media itself.

vaxpunk has none on the running system. On the host, `ods verify`
walks a volume as `ANALYZE/DISK_STRUCTURE` does, sorting findings into
errors, leaks and warnings, and `--repair-bitmap` rebuilds the bitmaps;
OpenVMS's own `ANALYZE/DISK_STRUCTURE` checks volumes `ods` writes
(`crosstools/ods/docs/vms-check.md`). PRD-0003's tier 2 #11 makes the
need plain: a host crash mid-`COPY` can leave a header without its
directory entry.
