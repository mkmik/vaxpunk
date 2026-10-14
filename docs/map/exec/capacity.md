+++
title = "Capacity"
summary = "A real-size system: all of the 32-bit space and all the RAM, no tables sized for the demo"
aka = ["limits", "PFN_COUNT", "CTX_MAX", "XMAP_MAX", "POOL_PAGES", "MAXPROCESSCNT", "CHANNELCNT", "FILLM", "32-bit address space", "very large programs"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m1"
refs = ["docs/adr/0037-all-ram-and-address-spaces-filled-on-faults.md", "docs/prd/0003-multi-user-vms.md", "docs/design/0001-pal-interface.md", "docs/adr/0005-access-modes-are-threads.md", "pal/src/main.c", "vms/exec/memory.mar"]

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
note = "P0 1 GB, S0 and P1 512 MB each below 2 GB (ADR-0005); Alpha has P0 and P1 1 GB each and S0/S1 at the sign-extended top, seL4's half: ADR-0037 keeps ADR-0005's layout"

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
title = "EXEC.EXE of any size, not 512 KB"
status = "todo"
milestone = "m1"
note = "main.c exe_file: the PAL's load buffer, raised from 256 KB once already; 'no [SYSEXE]EXEC.EXE ... (524288)' when it outgrows it"

[[feature]]
id = "per-process-tables"
title = "Open files, image headers and terminals limited by quotas and SYSGEN, not small constants"
status = "todo"
milestone = "m1"
note = "rms.mar MAXIFI 15 files a process, f11.mar MAXHDR 16 header blocks, ttdriver.mar MAXUNIT 9 TNAn: units, mddriver.mar MD$GL_BLOCKS"

[[feature]]
id = "ram-size"
title = "As much RAM as QEMU is given: one setting, from the seL4 build to the PAL's untypeds"
status = "todo"
milestone = "m1"
note = "ADR-0037 decisions 1 and 2: QEMU_MEM 8,192 MB. seL4 compiles QEMU's memory map in (pal/kernel/qemu.env QEMU_MEM=1024); the PAL retypes from the largest RAM untyped only. QEMU commits host pages only as they are touched, so a large default costs nothing"

[[feature]]
id = "pte-pfn"
title = "A PFN wider than the PTE's 21 bits, which stop at 8 GB of 4 KB pages"
status = "todo"
milestone = "m1"
note = "ADR-0037 decision 6: Alpha's quadword PTE, the PFN in bits 63:32; lib.mlb $PTEDEF PTE$M_PFN ^X001FFFFF today, used in memory.mar, process.mar and main.c only"

[[feature]]
id = "pfn-database"
title = "The PFN database sized from RPB$L_PFNCNT at boot, not 4,096 longwords in EXEC.EXE"
status = "todo"
milestone = "m1"
needs = ["#ram-size"]
note = "memory.mar MMG$C_MAXPFN, MMG$AL_FREEPFN; MMG$GL_PFNGSD is in pool. 8 bytes a PFN is 2 MB of S0 a GB of RAM; VMS's entry for paging is larger. ADR-0037 decision 7: at most 128 MB of S0, 64 GB of RAM, until it moves to S2"

[[feature]]
id = "switch-cost"
title = "A process switch that costs the same whatever the processes' sizes"
status = "todo"
milestone = "m1"
needs = ["/platform/pal#tlb-fill"]
note = "main.c kernel_space: SWPCTX unmaps and maps every P0 and P1 page of both processes, a seL4 call each: 500,000 calls for two 1 GB programs. ADR-0037 decision 5: a kernel address space per process"

[[feature]]
id = "pal-tables"
title = "The PAL's PTE tables, mappings and contexts made from memory as needed, not static arrays"
status = "todo"
milestone = "m1"
needs = ["/platform/pal#frame-caps"]
note = "main.c pte_pool[96], pfn_map[PFN_COUNT], xmap[XMAP_MAX], ctx[CTX_MAX], exe_file[512 KB]; sched.mar SCH$C_MAXPIX"

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
