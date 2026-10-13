+++
title = "Capacity"
summary = "A real-size system: all of the 32-bit space and all the RAM, no tables sized for the demo"
aka = ["limits", "PFN_COUNT", "CTX_MAX", "XMAP_MAX", "POOL_PAGES", "MAXPROCESSCNT", "CHANNELCNT", "FILLM", "32-bit address space", "very large programs"]
kind = "core"
intent = "goal"
research = "outline"
milestone = "m1"
refs = ["docs/prd/0003-multi-user-vms.md", "docs/design/0001-pal-interface.md", "docs/adr/0005-access-modes-are-threads.md", "pal/src/main.c", "vms/exec/memory.mar"]

[[feature]]
id = "all-ram"
title = "Every page of RAM QEMU gives is a PFN, not 1,024 (4 MB)"
status = "todo"
milestone = "m1"
needs = ["/platform/pal#frame-caps"]
note = "PRD-0003 tier 1 #39; main.c PFN_COUNT"

[[feature]]
id = "page-tables"
title = "Page tables made as a process needs them, not 96 for the whole system"
status = "todo"
milestone = "m1"
needs = ["/platform/pal#frame-caps"]
note = "PRD-0003 tier 1 #39; main.c pte_pool: S0 and two 4 MB tables per process"

[[feature]]
id = "address-space-32"
title = "All of the 32-bit space: a program's P0 and P1 grow to their regions' ends"
status = "todo"
milestone = "m1"
needs = ["#page-tables", "/exec/memory#p1-expreg"]
note = "P0 1 GB, S0 and P1 512 MB each below 2 GB (ADR-0005); Alpha has P0 and P1 1 GB each and S0/S1 at the sign-extended top, seL4's half: an ADR decides whether to follow it"

[[feature]]
id = "processes"
title = "Processes up to MAXPROCESSCNT, not 32 contexts"
status = "todo"
milestone = "m1"
needs = ["/platform/pal#frame-caps"]
note = "main.c CTX_MAX; each context's threads and address spaces take root CNode slots"

[[feature]]
id = "shared-mappings"
title = "A page mapped by any number of processes, not 256 extra mappings"
status = "todo"
milestone = "m1"
needs = ["/platform/pal#frame-caps"]
note = "main.c XMAP_MAX; global sections and, later, installed images need it"

[[feature]]
id = "pool"
title = "Pool sized from memory and SYSGEN, not 512 KB"
status = "todo"
milestone = "m1"
needs = ["/exec/pool#sizing", "#all-ram"]

[[feature]]
id = "exec-image"
title = "EXEC.EXE of any size, not 256 KB"
status = "todo"
milestone = "m1"
note = "main.c exe_file: the PAL's load buffer; 'no [SYSEXE]EXEC.EXE ... (262144)' when it outgrows it"

[[feature]]
id = "per-process-tables"
title = "Open files, image headers and terminals limited by quotas and SYSGEN, not small constants"
status = "todo"
milestone = "m1"
note = "rms.mar MAXIFI 15 files a process, f11.mar MAXHDR 16 header blocks, ttdriver.mar MAXUNIT 9 TNAn: units, mddriver.mar MD$GL_BLOCKS"

[[needs]]
on = "/platform/pal"
why = "Frames, page tables and threads are seL4 objects the PAL holds"

[[needs]]
on = "/exec/memory"
why = "The PFN list and the regions grow"

[[needs]]
on = "/exec/sysgen-parameters"
why = "MAXPROCESSCNT, NPAGEDYN, CHANNELCNT size the tables"
soft = true
+++

A VMS system is sized by its memory and its system parameters: a program
can use its whole P0 and P1, the system as many processes as
`MAXPROCESSCNT` allows, and pool, channels and files come from SYSGEN and
the process's quotas. Real programs (compilers, editors, databases)
expect that.

vaxpunk started as a demo and still has its sizes: 1,024 PFNs (4 MB) and
96 page tables, as much as the PAL's root CNode leaves room for; 32
process contexts; 256 extra mappings of a page; 512 KB of pool; a 256 KB
buffer for `EXEC.EXE`; 15 open files a process. This component tracks
lifting them, so that no component is held back by one. Most need the
PAL's frame capabilities in a CNode of their own
(`t8y:/platform/pal#frame-caps`). Each limit lifted becomes a SYSGEN
parameter or a quota where VMS has one.

This is the 32-bit space, VAX's regions as OpenVMS Alpha kept them. The
64-bit space above it, P2 and S2, is a goal of its own,
`t8y:/exec/64-bit-addressing`, which comes after this.
