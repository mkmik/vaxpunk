+++
title = "Sequential files"
summary = "Records one after another, in any of RMS's seven record formats"
aka = ["SEQ", "VAR", "FIX", "VFC", "STM", "STMLF", "STMCR", "UDF", "Stream_LF", "$GET", "$PUT", "$FIND", "$UPDATE", "$TRUNCATE", "$REWIND", "$FLUSH", "RFA", "rmsseq.mar"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["docs/adr/0007-system-disk-files-11-and-rms.md", "docs/adr/0009-ramdisk-writable-files-11.md", "docs/prd/0008-rms-record-and-indexed-files.md", "vms/exec/rmsseq.mar", "crosstools/ods/docs/records.md"]

[[feature]]
id = "basic"
title = "VAR and FIX records: $GET in order, $PUT at the end"
status = "done"
milestone = "m0"
refs = ["docs/adr/0007-system-disk-files-11-and-rms.md", "docs/adr/0009-ramdisk-writable-files-11.md"]

[[feature]]
id = "formats"
title = "VFC, STM, STMLF, STMCR and UDF; BLK, CR, FTN and PRN kept and given back"
status = "done"
note = "PRD-0008 step 3"

[[feature]]
id = "random"
title = "$FIND, $UPDATE, $TRUNCATE, $REWIND, $FLUSH, access by RFA, $PUT with RAB$V_TPT"
status = "done"
note = "PRD-0008 step 3"

[[feature]]
id = "fixed-by-key"
title = "FIX records by record number, RAB$C_KEY, and FAB$M_MSB"
status = "todo"
note = "rmsseq.mar ponytail"

[[feature]]
id = "multibuffer"
title = "Multiblock and multibuffer I/O, read-ahead and write-behind"
status = "todo"
note = "The IFAB holds one block; SET RMS_DEFAULT's counts"

[[needs]]
on = "/rms/core"
why = "Opened, created and closed through it"

[[needs]]
on = "/files/ods2"
why = "Reads and writes virtual blocks; extends the file as it fills"
+++

A sequential file's records follow one another from the first byte to
the end of file. RMS has seven record formats: fixed (`FIX`), variable
with a size word (`VAR`), variable with a fixed control area (`VFC`,
which `PRN` carriage control uses), three stream formats ended by CR LF,
LF or CR (`STM`, `STMLF`, `STMCR`), and undefined (`UDF`), read only by
blocks. Record attributes say how to print them (`CR`, `FTN`, `PRN`) and
whether a record may cross a block (`BLK`). Records are read in order or
by record file address (RFA), rewritten in place with `$UPDATE`, and the
file cut with `$TRUNCATE`. Most files on a VMS system are sequential:
text, listings, logs, object files.

vaxpunk has every format and attribute, laid out as OpenVMS lays them
out, in `rmsseq.mar`: `$GET`, `$FIND`, `$PUT` at the end or in the middle
with `RAB$V_TPT`, `$UPDATE`, `$TRUNCATE`, `$REWIND`, `$FLUSH` and RFA
access ([PRD-0008](../../prd/0008-rms-record-and-indexed-files.md)
step 3). `COPY` keeps a `STMLF` file's format, and the C library's stdio
reads records through it. The IFAB holds one block, so there is no
read-ahead or write-behind.
