+++
title = "Write ordering and recovery"
summary = "Careful write order, flushes and backup copies, so a crash spares the volume"
aka = ["careful write", "VIRTIO_BLK_T_FLUSH", "backup home block", "MOUNT/REBUILD", "improper dismount"]
kind = "core"
intent = "goal"
research = "outline"
milestone = "m1"
refs = ["docs/prd/0003-multi-user-vms.md", "docs/adr/0012-data-disk-writable-files-11.md", "vms/exec/f11wrt.mar", "vms/exec/rms.mar", "crosstools/ods/docs/index-file.md", "crosstools/ods/docs/directory.md", "crosstools/ods/docs/file-header.md"]

[[feature]]
id = "header-first"
title = "A new file's header written before its directory entry, and the reverse on delete"
status = "todo"
milestone = "m1"
note = "PRD-0003 tier 2 #11; rms.mar's $CREATE enters the directory, then writes the header"

[[feature]]
id = "flush"
title = "A flush to the device on DISMOUNT and at shutdown"
status = "todo"
milestone = "m1"
needs = ["//drivers/disk:flush"]
note = "PRD-0003 tier 2 #11; ADR-0012 follow-up: VIRTIO_BLK_T_FLUSH"

[[feature]]
id = "backup-home"
title = "The backup home block and index file header INITIALIZE writes"
status = "todo"
milestone = "m1"
note = "PRD-0003 tier 2 #11; f11wrt.mar FIL$INIT: the backup LBNs are the primary ones"

[[feature]]
id = "directory-swap"
title = "A changed directory written to new blocks and switched in one write"
status = "todo"
note = "f11wrt.mar rewrites a directory in place; ods's writer swaps (crosstools/ods/docs/directory.md)"

[[feature]]
id = "rebuild"
title = "MOUNT/REBUILD: free counts and caches put right after an improper dismount"
status = "todo"

[[feature]]
id = "power-loss-tests"
title = "Tests that cut the executive's writes after each one, as ods's do"
status = "todo"
note = "crates/ods-core/tests/model.rs does it for the host writer"

[[needs]]
on = "//files/ods2"
why = "The order of its writes is what this is about"

[[needs]]
on = "//host/ods"
why = "ods verify finds what a crash left; its writer is the careful model"
soft = true
+++

VMS's XQP orders its writes so that a crash at any point leaves at worst
lost space, never a damaged volume: the index file's end and bitmap bit
first, then the header, then the directory entry; on delete, the entry
goes first and the blocks are freed after the map no longer points at
them. The home block and the index file's header have backup copies, so
a volume survives losing either. A volume that wasn't dismounted is
marked, and the next `MOUNT` rebuilds its free counts and caches;
`ANALYZE/DISK_STRUCTURE` (`//files/analyze-disk`) finds and repairs
what is left. Spiralog, a log-structured file system DEC offered for
OpenVMS Alpha in 1996, tried another way and was retired.

vaxpunk's writer, `f11wrt.mar`, writes as it goes: `$CREATE` enters the
directory before it writes the header, a directory is rewritten whole in
place, there is no flush, and `INITIALIZE` writes no backup home block or
index file header. QEMU writes `DKB0:`'s image as requests come, so a host
crash in the middle of a `COPY` can leave a header without its directory
entry, which `ods verify` reports
([ADR-0012](../../docs/adr/0012-data-disk-writable-files-11.md)).
PRD-0003's tier 2 #11 closes the first three gaps before Milestone 1
ends. The host's `ods` writer already follows the careful order and is
tested by cutting each of its write sequences after every write.
