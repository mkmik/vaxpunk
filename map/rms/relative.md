+++
title = "Relative files"
summary = "Fixed cells numbered from 1, read and written by record number"
aka = ["REL", "relative record number", "RRN", "FAB$L_MRN", "rmsrel.mar"]
kind = "core"
intent = "goal"
research = "done"
refs = ["docs/prd/0008-rms-record-and-indexed-files.md", "vms/exec/rmsrel.mar", "crosstools/ods/docs/relative.md"]

[[feature]]
id = "basic"
title = "A prologue and buckets of cells; $GET, $FIND, $PUT, $UPDATE, $DELETE by number or in order"
status = "done"
note = "PRD-0008 step 4"

[[feature]]
id = "rfa"
title = "Access by RFA, the record number in RAB$L_BKT, FAB$L_MRN as a bound"
status = "done"
note = "PRD-0008 step 4"

[[feature]]
id = "shared"
title = "Shared access with bucket and record locks"
status = "todo"
needs = ["//rms/sharing:record-locks"]
note = "PRD-0008 step 9"

[[needs]]
on = "//rms/core"
why = "Opened, created and closed through it"

[[needs]]
on = "//files/ods2"
why = "Buckets are virtual blocks of the file"

[[needs]]
on = "//host/ods"
why = "ods reads and checks relative files: the second implementation"
soft = true
+++

A relative file is a prologue block and buckets of fixed-size cells; cell
n holds record number n, counted from 1, with a control byte that says
whether it holds a record and whether that was deleted. A program reads,
writes or deletes a record by its number, the key, or goes through the
records in order, skipping empty cells. `$PUT` to a cell that holds a
record fails with `RMS$_REX` unless the program asks to update instead.
The maximum record number bounds the file. Relative files suit tables
indexed by a small number: slots, terminals, accounts.

vaxpunk's are in `rmsrel.mar`, VMS's format bit for bit; a file `RMSTEST`
writes passes `ANALYZE/RMS_FILE/CHECK` on OpenVMS
([PRD-0008](../../docs/prd/0008-rms-record-and-indexed-files.md) step 4).
Its bucket routines, `RMS$READBKT` and `RMS$WRITEBKT`, serve the indexed
files too.
