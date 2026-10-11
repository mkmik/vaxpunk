+++
title = "CONVERT"
summary = "Copies records into a file of another organization or layout, from FDL"
aka = ["CONVERT", "CONVERT.EXE", "CONVERT/FDL", "CONVERT/RECLAIM", "CONVERT/FAST_LOAD", "CONVWORK"]
kind = "core"
intent = "goal"
research = "done"
refs = ["vms/sysexe/convert.mar", "vms/cld/convert.cld", "docs/prd/0008-rms-record-and-indexed-files.md", "https://docs.vmssoftware.com/vsi-openvms-record-management-utilities-reference-manual/"]

[[feature]]
id = "basic"
title = "CONVERT in[,...] out: the first input's organization, attributes and keys, or /FDL's"
status = "done"
note = "PRD-0008 step 7"

[[feature]]
id = "indexed-load"
title = "An indexed output loaded in primary key order, through a sorted work file; /NOSORT"
status = "done"
note = "PRD-0008 step 7: SYS$SCRATCH:CONVWORK, not SORT"

[[feature]]
id = "exceptions"
title = "/EXCEPTIONS_FILE, /PAD, /TRUNCATE; duplicate keys and bad sizes reported"
status = "done"
note = "PRD-0008 step 7"

[[feature]]
id = "append-merge"
title = "/APPEND to a sequential output, /MERGE into an indexed one, /KEY on input"
status = "done"
note = "PRD-0008 step 7"

[[feature]]
id = "statistics"
title = "/STATISTICS with I/O counts, page faults and CPU time"
status = "todo"
needs = ["//exec/information-services"]
note = "PRD-0003 step 5 keeps the counts; convert.mar ponytail: the executive doesn't count them; elapsed time wraps at 429 s"

[[feature]]
id = "long-records"
title = "Records longer than its buffer, RECSIZE"
status = "todo"
note = "convert.mar ponytail"

[[feature]]
id = "reclaim"
title = "CONVERT/RECLAIM: free a prologue 3 file's empty buckets in place"
status = "todo"
needs = ["//rms/indexed:reclaim"]
note = "PRD-0008 lists it"

[[feature]]
id = "callable"
title = "The CONV$ routines, so programs can convert"
status = "todo"

[[feature]]
id = "sort-utility"
title = "Sorting through SOR$, with work files when records don't fit"
status = "todo"
note = "PRD-0008: SORT/MERGE is another PRD"

[[feature]]
id = "shared-input"
title = "/SHARE: convert a file others have open"
status = "todo"
needs = ["//rms/sharing"]

[[needs]]
on = "//rms/indexed"
why = "Builds indexed outputs and reads them along any key"

[[needs]]
on = "//rms/relative"
why = "Relative files in and out"

[[needs]]
on = "//rms/sequential"
why = "Sequential files in and out"

[[needs]]
on = "//rms/fdl"
why = "/FDL: FDL$PARSE makes the output's FAB and XABs"

[[needs]]
on = "//rtl/conv"
why = "VMS's CONVERT is a front end on the CONV$ routines"
soft = true

[[needs]]
on = "//rtl/sor"
why = "VMS's CONVERT sorts with SOR$, spilling to work files"
soft = true

[[needs]]
on = "//rtl/cli"
why = "Its parameters and qualifiers"
+++

`CONVERT` copies records from one or more files into a new or existing
file, changing organization, record format, keys, bucket sizes or areas
on the way, as an FDL file or the first input describes. It is how a VMS
user makes an indexed file from a text file, reorganizes a fragmented
one, or changes a record format. An indexed output is loaded in key
order, sorted first, with `/FAST_LOAD` building the indexes after;
records that don't fit go to an exceptions file. `CONVERT/RECLAIM`
frees empty buckets without copying. The `CONV$` routines do the same
from a program.

vaxpunk's is `vms/sysexe/convert.mar` (PRD-0008 step 7): the output from
`/FDL` or the first input, records copied along `/KEY`, an indexed
output loaded in primary key order through an indexed work file,
`SYS$SCRATCH:CONVWORK`, rather than SORT; `/APPEND`, `/MERGE`, `/PAD`,
`/TRUNCATE`, `/EXCEPTIONS_FILE` and `/STATISTICS`. The boot test
converts a text file into an indexed file that `ANALYZE/RMS_FILE`
passes. Missing: `/RECLAIM`, the callable `CONV$` routines, and the
counts `/STATISTICS` needs from the executive.
