+++
title = "ANALYZE/DISK_STRUCTURE"
summary = "The Verify utility: checks a volume's structure, and repairs it"
aka = ["ANALYZE/DISK_STRUCTURE", "ANALYZE/DISK", "Verify utility", "VERIFY", "/REPAIR", "[SYSLOST]", "/USAGE"]
kind = "core"
intent = "aspiration"
research = "outline"
refs = ["crosstools/ods/docs/verify.md", "crosstools/ods/docs/vms-check.md", "docs/adr/0012-data-disk-writable-files-11.md"]

[[feature]]
id = "check"
title = "A report of what is wrong: headers, maps, bitmaps, directories, lost files"
status = "todo"

[[feature]]
id = "repair"
title = "/REPAIR: bitmaps put right, lost files entered in [SYSLOST]"
status = "todo"
needs = [":check"]

[[feature]]
id = "usage"
title = "/USAGE: a file of the blocks each owner uses"
status = "todo"
needs = [":check"]

[[needs]]
on = "//files/ods2"
why = "Reads every header, map and directory of the volume"

[[needs]]
on = "//files/volumes"
why = "Works on a mounted volume, holding off its writers while it looks"

[[needs]]
on = "//files/fs-locking"
why = "Locks the volume against allocation while it checks the bitmaps"

[[needs]]
on = "//host/ods"
why = "ods verify does the same checks on the host: the model"
soft = true
+++

`ANALYZE/DISK_STRUCTURE`, the Verify utility, reads a whole Files-11
volume and reports what doesn't agree: headers with bad checksums or
backlinks, blocks the bitmap calls free that a file maps, blocks mapped
twice, files no directory enters, directory entries that point nowhere,
quota usage that doesn't match. With `/REPAIR` it fixes the bitmaps and
counts and enters lost files in `[SYSLOST]`; `/USAGE` writes a file of the
space each owner uses. System managers run it after a crash and before
`BACKUP/IMAGE`.

vaxpunk has none on the VMS side. The host's `ods verify` does these
checks on a disk image, and the boot test runs it on `DKB0:`'s image once
QEMU has stopped ([ADR-0012](../../docs/adr/0012-data-disk-writable-files-11.md)).
