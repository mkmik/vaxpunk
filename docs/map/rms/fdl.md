+++
title = "FDL"
summary = "File Definition Language: a text description of a file's RMS attributes"
aka = ["FDL", "File Definition Language", "FDL$PARSE", "FDL$CREATE", "FDL$GENERATE", "FDL$RELEASE", "FDLSHR", "CREATE/FDL", "EDIT/FDL"]
kind = "core"
intent = "goal"
research = "done"
refs = ["docs/prd/0008-rms-record-and-indexed-files.md", "vms/sysexe/rms/fdl.mar", "vms/sysexe/rms/line.mar", "vms/sysexe/create.mar", "vms/sysexe/convert.mar", "vms/sysexe/analyzrms.mar", "boot/tests/rms.rs"]

[[feature]]
id = "parse"
title = "FDL$PARSE: an FDL file into a FAB, a RAB, XABALLs and XABKEYs, with VMS's warnings"
status = "done"
note = "PRD-0008 step 7"

[[feature]]
id = "create-fdl"
title = "CREATE/FDL, and CONVERT/FDL's output made from an FDL file"
status = "done"
note = "PRD-0008 step 7"

[[feature]]
id = "write-fdl"
title = "ANALYZE/RMS_FILE/FDL writes the FDL that makes a file again"
status = "done"
note = "PRD-0008 step 7; analyzrms.mar"

[[feature]]
id = "callable"
title = "FDL$CREATE, FDL$GENERATE and FDL$RELEASE, callable from any language"
status = "todo"
note = "PRD-0008 goals name FDL$CREATE; fdl.mar has FDL$PARSE alone, linked into each program"

[[feature]]
id = "all-attributes"
title = "Every primary and secondary applied: PROTECTION, OWNER, ACCESS, SHARING, DATE"
status = "todo"
note = "fdl.mar ponytail: taken and left alone"

[[feature]]
id = "edit-fdl"
title = "EDIT/FDL, the interactive editor that designs a file from its expected load"
status = "todo"
note = "PRD-0008 non-goal"

[[needs]]
on = "/rms/core"
why = "Makes the FAB, RAB and XAB chain $CREATE takes"

[[needs]]
on = "/rms/indexed"
why = "Most of FDL describes indexed files' keys and areas"
+++

FDL describes a file's RMS attributes in text: `FILE`, `RECORD`, `AREA n`
and `KEY n` sections with keywords for organization, record format, bucket
sizes, keys, fill factors, protection and sharing. `CREATE/FDL` makes an
empty file from one, `CONVERT/FDL` reorganizes a file into one,
`ANALYZE/RMS_FILE/FDL` writes the FDL of an existing file with its
statistics, and `EDIT/FDL` designs one interactively from the record size
and count the user expects, or optimizes one from those statistics. The
callable routines `FDL$PARSE`, `FDL$CREATE`, `FDL$GENERATE` and
`FDL$RELEASE` do the same for programs.

vaxpunk's `FDL$PARSE` (`vms/sysexe/rms/fdl.mar`) reads FDL into a FAB, a
RAB and their XABs, with VMS's `%FDL-W-` warnings; `CREATE/FDL`,
`CONVERT/FDL` and `ANALYZE/RMS_FILE/FDL` use it
([PRD-0008](../../prd/0008-rms-record-and-indexed-files.md) step 7).
It is linked into those programs, not a shareable image, and sets only
what `$CREATE` uses. `FDL$CREATE` and `EDIT/FDL` aren't there.
