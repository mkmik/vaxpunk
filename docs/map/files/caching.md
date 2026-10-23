+++
title = "File system caches"
summary = "XQP caches of headers, directories and extents, and XFC's data cache"
aka = ["XFC", "Extended File Cache", "VIOC", "virtual I/O cache", "ACP_HDRCACHE", "ACP_DIRCACHE", "ACP_MAPCACHE", "MOUNT/CACHE", "SET VOLUME/CACHE"]
kind = "core"
intent = "aspiration"
research = "outline"
refs = ["https://wasd.vsm.com.au/htbin/helpgate/HELP/RMS/EXTENDED_FILE_CACHE_", "https://de.openvms.org/TUD2010/File_System_Changes.pdf", "vms/exec/f11.mar"]

[[feature]]
id = "metadata"
title = "Caches of file headers, directories, extents and free file numbers, per volume"
status = "todo"
needs = ["/files/ods2#free-count"]

[[feature]]
id = "data"
title = "A data cache of file blocks, write-through, with per-file caching options"
status = "todo"
needs = ["/exec/memory"]

[[feature]]
id = "coherent"
title = "Caches kept the same across processes and nodes by locks"
status = "todo"
needs = ["#metadata", "/exec/lock-manager"]

[[needs]]
on = "/files/ods2"
why = "What it caches is the XQP's reads"

[[needs]]
on = "/exec/pool"
why = "The XQP's caches live in nonpaged pool, sized by ACP_ parameters"
+++

The XQP caches what it reads most: file headers, directory blocks, a
volume's free extents and free file numbers, and quota entries, sized by
the `ACP_` system parameters and kept the same in every process, and on
every node of a cluster, by locks. File data has a cache of its own: the
virtual I/O cache (VIOC), then from OpenVMS Alpha V7.3 the Extended File
Cache (XFC), which caches blocks of files and images in memory, writes
through to the disk, and lets a file or a volume turn caching off
(`MOUNT/CACHE`, `SET VOLUME/CACHE`, and a caching option in an RMS
`XABITM`).

vaxpunk caches nothing but a volume's free block count, in its VCB:
`f11.mar` reads a header or a directory from the disk each time it is
wanted, and RMS reads an indexed file's buckets each time
(`t8y:/rms/indexed`). QEMU and the host's page cache stand in for now.
