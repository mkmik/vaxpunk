+++
title = "Sequential files"
summary = "Records one after another, in any of RMS's seven record formats"
aka = ["SEQ", "VAR", "FIX", "VFC", "STM", "STMLF", "STMCR", "UDF", "Stream_LF", "$GET", "$PUT", "$FIND", "$UPDATE", "$TRUNCATE", "$REWIND", "$FLUSH", "RFA", "MSB", "MBC", "MBF", "RAH", "WBH", "DEQ", "rmsseq.mar"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["docs/adr/0007-system-disk-files-11-and-rms.md", "docs/adr/0009-ramdisk-writable-files-11.md", "docs/prd/0008-rms-record-and-indexed-files.md", "vms/exec/rmsseq.mar", "crosstools/ods/docs/records.md", "vms/sysexe/rmstest.mar"]

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
status = "done"
note = "rmsseq.mar KEYPOS; RMSTEST's FIXK, FIXKB and VARMSB against OpenVMS Alpha V8.4-2L1: a keyed $PUT past the end only, RMS$_RFM for MSB without VAR"

[[feature]]
id = "multibuffer"
title = "Multiblock and multibuffer I/O"
status = "done"
needs = ["/utils/set-show#rms-default"]
note = "RAB$B_MBC blocks a buffer, RAB$B_MBF buffers, or SET RMS_DEFAULT's; the one used least recently goes; RMSTEST's MB.SEQ"

[[feature]]
id = "read-ahead"
title = "Read-ahead and write-behind, RAB$V_RAH and RAB$V_WBH: a buffer's transfer while the program goes on"
status = "todo"
needs = ["/rms/async#asynchronous"]
note = "They only give the stream a second buffer: rmsseq.mar waits for every transfer"

[[feature]]
id = "default-extension"
title = "The default extension: FAB$W_DEQ, the header's, SET RMS_DEFAULT's, the system's"
status = "done"
note = "With none, 8 blocks: VMS's twice a buffer, given back at $CLOSE, wants the file system to truncate"

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
with `RAB$V_TPT`, `$UPDATE`, `$TRUNCATE`, `$REWIND`, `$FLUSH`, RFA
access, `FIX` records by number with `RAB$C_KEY`, and `VAR` sizes high
byte first with `FAB$M_MSB`
([PRD-0008](../../prd/0008-rms-record-and-indexed-files.md) step 3).
`$CONNECT` gives a stream `RAB$B_MBF` buffers of `RAB$B_MBC` blocks, or
the process's or the system's defaults, which `SET RMS_DEFAULT` sets, and
each is read and written in one transfer; the system's are VMS's, one
buffer of 32 blocks. A file grows by `FAB$W_DEQ`, the header's default
extension or `SET RMS_DEFAULT`'s. `COPY` keeps a `STMLF` file's format,
and the C library's stdio reads records through it. Every transfer is
done before a service returns, so read-ahead and write-behind only give
a stream a second buffer.
