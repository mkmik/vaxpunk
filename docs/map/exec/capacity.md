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
status = "done"
milestone = "m1"
needs = ["/platform/pal#frame-caps"]
note = "ADR-0037 decision 2: all RAM but the PAL's share, 1/16 once measured: all of RAM mapped in two address spaces fits"

[[feature]]
id = "page-tables"
title = "Page tables made as a process needs them, not 96 for the whole system"
status = "done"
milestone = "m1"
needs = ["/platform/pal#frame-caps"]
note = "The PAL's PTE tables are pages of its share, made as needed"

[[feature]]
id = "address-space-32"
title = "All of the 32-bit space: a program's P0 and P1 grow to their regions' ends"
status = "done"
milestone = "m1"
needs = ["#page-tables", "/exec/memory#p1-expreg"]
note = "MEMTEST: P0 to its end, 1,008 MB, and P1 down to its start below DCL, with $EXPREG region 1. P0 1 GB, S0 and P1 512 MB each below 2 GB (ADR-0005); ADR-0037 keeps that layout"

[[feature]]
id = "processes"
title = "Processes up to MAXPROCESSCNT, not 32 contexts"
status = "done"
milestone = "m1"
needs = ["/platform/pal#frame-caps"]
note = "sched.mar: the PCB vector in pool, SGN$GW_MAXPRCCT, 1,024, slots; the PAL's contexts and ASID pools grow as needed"

[[feature]]
id = "shared-mappings"
title = "A page mapped by any number of processes, not 256 extra mappings"
status = "done"
milestone = "m1"
needs = ["/platform/pal#frame-caps"]
note = "A mapping's capabilities come from the PAL's CSpace of two levels, made as needed"

[[feature]]
id = "pool"
title = "Pool sized from memory and SYSGEN, not 512 KB"
status = "done"
milestone = "m1"
needs = ["/exec/pool#sizing", "#all-ram"]
note = "memory.mar: NPAGEDYN, 1 MB, at boot, past the PFN database, growing by 256 KB or the block up to NPAGEVIR, by default 0x4FF00000"

[[feature]]
id = "exec-image"
title = "EXEC.EXE of any size, not 512 KB"
status = "done"
milestone = "m1"
note = "The PAL reads EXEC.EXE into a table of its own as large as the file"

[[feature]]
id = "per-process-tables"
title = "Open files, image headers and terminals limited by quotas and SYSGEN, not small constants"
status = "todo"
milestone = "m1"
note = "rms.mar MAXIFI 15 files a process, f11.mar MAXHDR 16 header blocks, mddriver.mar MD$GL_BLOCKS; terminals and the other cloned units go to 9999 since ADR-0044"

[[feature]]
id = "ram-size"
title = "As much RAM as QEMU is given: one setting, from the seL4 build to the PAL's untypeds"
status = "done"
milestone = "m1"
note = "ADR-0037 decision 1: QEMU_MEM 8,192 MB in qemu.env, overridden by QEMU_MEM in the environment; the browser demo's kernel is built for 1 GB"

[[feature]]
id = "pte-pfn"
title = "A PFN wider than the PTE's 21 bits, which stop at 8 GB of 4 KB pages"
status = "done"
milestone = "m1"
note = "ADR-0037 decision 6: Alpha's quadword PTE, the PFN in bits 63:32, 16 TB; lib.mlb $PTEDEF, memory.mar, process.mar, main.c"

[[feature]]
id = "pfn-database"
title = "The PFN database sized from RPB$L_PFNCNT at boot, not 4,096 longwords in EXEC.EXE"
status = "done"
milestone = "m1"
needs = ["#ram-size"]
note = "MMG$INIT makes it in S0 from RPB$L_FRESVA, 8 bytes a PFN, at most 127 MB, 64 GB of RAM; %EXEC-W-PFNLIMIT past that, until it moves to S2"

[[feature]]
id = "switch-cost"
title = "A process switch that costs the same whatever the processes' sizes"
status = "done"
milestone = "m1"
needs = ["/platform/pal#tlb-fill"]
note = "ADR-0037 decision 5: each process's kernel mode has an address space of its own, filled on faults; SWPCTX maps nothing"

[[feature]]
id = "pal-tables"
title = "The PAL's PTE tables, mappings and contexts made from memory as needed, not static arrays"
status = "done"
milestone = "m1"
needs = ["/platform/pal#frame-caps"]
note = "main.c: the PFNs' mappings, PTE tables, contexts and EXEC.EXE's buffer are tables and pages of the PAL's share"

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

vaxpunk started as a demo, with 1,024 PFNs (4 MB), 96 page tables, 32
process contexts and 256 extra mappings of a page, as much as the PAL's
root CNode left room for.
[ADR-0037](../../adr/0037-all-ram-and-address-spaces-filled-on-faults.md)
lifted those: the PAL takes all the RAM seL4 gives it, keeps a share for
its own objects in a CSpace of two levels, and the executive sizes its
PFN database at boot, a PTE is Alpha's quadword, its PFN 32 bits, and the
address spaces are caches filled on faults, so a process switch costs the
same whatever the processes' sizes. Pool grows from `NPAGEDYN` to
`NPAGEVIR`, the processes are `MAXPROCESSCNT`'s, and P0 and P1 grow to
their ends. What is left: 15 open files a process and the other
per-process tables. This component tracks lifting them, so
that no component is held back by one. Each limit lifted becomes a
SYSGEN parameter or a quota where VMS has one.

This is the 32-bit space, VAX's regions as OpenVMS Alpha kept them. The
64-bit space above it, P2 and S2, is a goal of its own,
`t8y:/exec/64-bit-addressing`, which comes after this.
