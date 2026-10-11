+++
title = "Global buffers"
summary = "One cache of a shared file's buckets in a global section, for every process"
aka = ["SET FILE/GLOBAL_BUFFER", "FAB$W_GBC", "XAB$W_GBC", "global buffer section", "GBLPAGFIL", "deferred write"]
kind = "core"
intent = "aspiration"
research = "outline"
refs = ["docs/prd/0008-rms-record-and-indexed-files.md", "vms/exec/rms.mar", "https://wasd.vsm.com.au/htbin/helpgate/HELP/SYS_PARAMETERS/GBLPAGFIL"]

[[feature]]
id = "global-section"
title = "A file's global buffer count makes a global section of buckets its openers share"
status = "todo"
needs = ["//exec/global-sections:page-file-sections", "//rms/sharing:record-locks"]
note = "PRD-0008 non-goal"

[[feature]]
id = "set-file"
title = "SET FILE/GLOBAL_BUFFER, and a count given at $OPEN"
status = "todo"
needs = [":global-section"]

[[feature]]
id = "deferred-write"
title = "Deferred write across processes: a bucket written once, when it leaves the cache"
status = "todo"
needs = [":global-section"]
note = "PRD-0008 non-goal"

[[needs]]
on = "//rms/sharing"
why = "Only shared files have them; the bucket locks guard the buffers"

[[needs]]
on = "//exec/global-sections"
why = "The buffers are a global page-file section mapped by each opener"

[[needs]]
on = "//exec/lock-manager"
why = "Locks keep each buffer's contents current across processes"
+++

By default each process that opens a file has its own buffers, so ten
processes reading the same indexed file read the same buckets ten times.
A file with a global buffer count, set with `SET FILE/GLOBAL_BUFFER` or
at `$OPEN`, gets one cache of buckets in a global page-file section that
every process with the file open shares, kept consistent by the lock
manager. It suits files many processes read at once, such as
`SYSUAF.DAT`, `RIGHTSLIST.DAT` and an application's main indexed file;
the global pages come from `GBLPAGFIL`.

vaxpunk has none (a non-goal of
[PRD-0008](../../docs/prd/0008-rms-record-and-indexed-files.md)). The
header's global buffer count is kept and given back in `FAB$W_GBC` and
`XAB$W_GBC`, but nothing uses it, and RMS doesn't keep even a process's
own buckets between services yet.
