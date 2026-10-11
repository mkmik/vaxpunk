+++
title = "ods (Files-11 library, CLI, FUSE)"
summary = "Files-11 ODS-2 and ODS-5 in Rust: reads, writes and verifies VMS disk images"
aka = ["ods", "ods-core", "ods-image", "ods-cli", "ods-fuse", "ods verify", "Files-11 in Rust"]
kind = "tool"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["crosstools/ods/README.md", "crosstools/ods/docs", "crosstools/ods/crates", "crosstools/ods/docs/vms-check.md", "crosstools/ods/docs/fuse.md", "crosstools/ods/docs/indexed.md", "docs/adr/0007-system-disk-files-11-and-rms.md"]

[[feature]]
id = "read"
title = "Home block, headers, directories, reading files, on fixtures from VMS V1.0 to 8.4-2L1"
status = "done"
milestone = "m0"
note = "ods README steps 1-5"

[[feature]]
id = "verify"
title = "The verifier, ods verify, as ANALYZE/DISK_STRUCTURE checks"
status = "done"
milestone = "m0"
note = "ods README step 6"

[[feature]]
id = "init"
title = "INITIALIZE, modelled on VMS 7.1"
status = "done"
milestone = "m0"
note = "ods README step 7"

[[feature]]
id = "write"
title = "Create, write, extend, rename, delete, purge; model and power-loss tests"
status = "done"
milestone = "m0"
note = "ods README step 8"

[[feature]]
id = "vms-check"
title = "OpenVMS Alpha finds its ODS-2 and ODS-5 volumes clean, and BACKUP copies them"
status = "done"
note = "ods README step 9; vms/check.py"

[[feature]]
id = "fuse-read"
title = "ods-fuse: a read-only FUSE mount, VMS attributes as extended attributes"
status = "done"
note = "ods README step 10; the mapping in docs/fuse.md waits to be confirmed"

[[feature]]
id = "rms-files"
title = "RMS relative and indexed files: read along any key, check, load from FDL and records"
status = "done"
note = "ods-core src/rms; PRD-0008"

[[feature]]
id = "fuse-write"
title = "A read-write FUSE mount"
status = "todo"
note = "ods README step 11: waits for the mapping to be settled"

[[feature]]
id = "volume-sets"
title = "Volume sets, sparse files, UCS-2 names on write, new hard links"
status = "todo"
note = "ods README known limits"

[[feature]]
id = "acl-editing"
title = "Editing ACLs (kept as they are today)"
status = "todo"
note = "ods README known limits"

[[needs]]
on = "//host/axpbox-oracle"
why = "Real VMS checks its volumes and made its RMS fixtures"
soft = true
+++

Not a VMS tool: a from-scratch implementation of Files-11 ODS-2 and
ODS-5 that reads and writes VMS disk images on macOS and Linux.
`ods-core` holds all the logic, `no_std` with no `unsafe`, over a block
device; `ods-image` puts it over image files, with VMS and Unix path
forms and record conversion; `ods` is the command line (`init`, `dir`,
`copy-in`, `verify`, `load`, `records` and more) and `ods-fuse` a
read-only mount.

It writes vaxpunk's system disk (`vms/build.rs`) and checks the data
disk after the boot tests. It was the model the MACRO-32 XQP was written
from, and its verifier is the check on what the XQP writes
(`//files/ods2`). It is tested against real VMS disks from 1978 to 2016,
an in-memory model, cuts at every write, and fuzzing, and OpenVMS Alpha
8.4 accepts its volumes.
