+++
title = "Indexed files"
summary = "Prologue 3 files: records in key order, a primary and up to 254 alternate keys"
aka = ["IDX", "ISAM", "prologue 3", "XABKEY", "SIDR", "RRV", "bucket split", "key compression", "rmsidx.mar", "rmsidxw.mar"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m1"
refs = ["docs/prd/0008-rms-record-and-indexed-files.md", "docs/prd/0003-multi-user-vms.md", "docs/adr/0029-loginout-and-sysuaf.md", "vms/exec/rmsidx.mar", "vms/exec/rmsidxw.mar", "vms/sysexe/rmsrand.mar", "crosstools/ods/docs/indexed.md", "boot/tests/rms.rs"]

[[feature]]
id = "read"
title = "$OPEN of a prologue 3 file; $GET and $FIND by any key, generic, KGE, KGT; RFA"
status = "done"
milestone = "m1"
note = "PRD-0008 step 5, which PRD-0003 step 4's SYSUAF.DAT needed"

[[feature]]
id = "write"
title = "$CREATE with XABKEY and XABALL; $PUT with splits and RRVs; $UPDATE; $DELETE"
status = "done"
milestone = "m1"
note = "PRD-0008 step 6"

[[feature]]
id = "compression"
title = "Key, index and data compression, read and written"
status = "done"
note = "PRD-0008 step 6"

[[feature]]
id = "bucket-cache"
title = "Buckets kept in buffers between services, not read each time"
status = "todo"
note = "rmsidx.mar ponytail; PRD-0008 open question on buffers"

[[feature]]
id = "duplicates"
title = "A $GET along duplicates that doesn't walk every one before the current"
status = "todo"
note = "rmsidx.mar ponytail"

[[feature]]
id = "reclaim"
title = "Emptied buckets reclaimed and record IDs reused; CONVERT/RECLAIM"
status = "todo"
note = "rmsidxw.mar ponytail"

[[feature]]
id = "big-buckets"
title = "Buckets up to the prologue's 63 blocks, not 32"
status = "todo"
note = "PRD-0008 Limits: RMS$_BKS past what fits in pool"

[[feature]]
id = "key-types"
title = "Collated and packed decimal keys"
status = "todo"
note = "PRD-0008 non-goal at first: RMS$_DTP"

[[feature]]
id = "old-prologues"
title = "Prologue 1 and 2 files"
status = "todo"
note = "PRD-0008 non-goal: refused with RMS$_PLV; CONVERT on OpenVMS makes them prologue 3"

[[feature]]
id = "shared"
title = "Shared access with bucket and record locks"
status = "todo"
needs = ["/rms/sharing#record-locks"]
note = "PRD-0008 step 9"

[[needs]]
on = "/rms/core"
why = "Opened, created and closed through it; XABKEY and XABALL"

[[needs]]
on = "/rms/relative"
why = "Reads and writes buckets with rmsrel.mar's RMS$READBKT and RMS$WRITEBKT"

[[needs]]
on = "/files/ods2"
why = "Buckets are virtual blocks; the file extends as buckets split"

[[needs]]
on = "/files/ods2#extension-headers"
why = "An indexed file extends often, in small pieces; one header's map runs out"
soft = true

[[needs]]
on = "/host/ods"
why = "The host reader and checker of PRD-0008 step 2; ods loads SYSUAF.DAT at build"
soft = true
+++

An indexed file keeps its records in data buckets in primary key order,
with an index of buckets above them; each alternate key has its own
index, down to secondary index data records (SIDRs) that list the RFAs of
the records with each key value. A record is found by key, exactly, by a
leading part of it (generic), or the first greater or equal, and read in
order along any key. A full bucket splits, and a record that moves leaves
a record reference vector (RRV), so its RFA still finds it. Keys may
allow duplicates, and alternate keys may change on `$UPDATE`. Prologue 3,
from VAX/VMS V4, adds key and data compression; 255 keys of up to 8
segments are allowed. Most VMS applications keep their data in indexed
files, and so does VMS: `SYSUAF.DAT`, `RIGHTSLIST.DAT`, the queue file,
mail folders.

vaxpunk's are prologue 3 as OpenVMS writes them, with every key type but
collated and packed decimal: `rmsidx.mar` finds records, `rmsidxw.mar`
puts, updates and deletes them
([PRD-0008](../../prd/0008-rms-record-and-indexed-files.md) steps
5 and 6). The boot test reads files `CONVERT` made on OpenVMS Alpha by
every key and prints the same dumps, and `RMSRAND` runs a long random mix
of operations against a model, after which the file passes the `ods`
checker and `ANALYZE/RMS_FILE/CHECK`. `SYSUAF.DAT` is one, with
OpenVMS's four keys
([ADR-0029](../../adr/0029-loginout-and-sysuaf.md)). Buckets are read
again each time they are wanted.
