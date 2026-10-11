+++
title = "Virtual memory"
summary = "P0, P1 and S0, the PFN list, page protection, $CRETVA and $EXPREG"
aka = ["MMG$", "$CRETVA", "$DELTVA", "$EXPREG", "$CNTREG", "$SETPRT", "$LCKPAG", "$ULKPAG", "PFN database", "PTE", "P0", "P1", "S0", "WRPTE", "memory.mar"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["docs/design/0001-pal-interface.md", "docs/design/0002-executive-processes.md", "docs/adr/0005-access-modes-are-threads.md", "vms/exec/memory.mar", "docs/prd/0003-multi-user-vms.md"]

[[feature]]
id = "address-space"
title = "P0, S0 and P1 below 2 GB; P0 and P1 each process's own"
status = "done"
milestone = "m0"
refs = ["docs/adr/0005-access-modes-are-threads.md"]

[[feature]]
id = "pages"
title = "The PFN free list; pages made, deleted and protected with WRPTE"
status = "done"
milestone = "m0"
refs = ["docs/design/0001-pal-interface.md"]

[[feature]]
id = "services"
title = "$CRETVA, $DELTVA and $EXPREG"
status = "done"
milestone = "m0"

[[feature]]
id = "probe"
title = "PROBER and PROBEW: services check a caller's addresses for its mode"
status = "done"
milestone = "m0"
refs = ["docs/adr/0005-access-modes-are-threads.md"]

[[feature]]
id = "setprt"
title = "$SETPRT and $CNTREG"
status = "todo"
note = "stubs in syssrv.mar"

[[feature]]
id = "lock-pages"
title = "$LCKPAG, $ULKPAG, $LKWSET and $ULWSET"
status = "todo"
note = "stubs; PRD-0003 tier 2 #10 could lock a pending read's pages with $LCKPAG"

[[feature]]
id = "p1-expreg"
title = "$EXPREG of P1, and rundown freeing only the pages a process has"
status = "todo"
note = "PRD-0003 tier 3 #30"

[[feature]]
id = "more-memory"
title = "More than 1,024 PFNs (4 MB) and 96 page tables: frame caps in a CNode of their own"
status = "todo"
needs = ["//platform/pal"]
note = "PRD-0003 tier 4 #39; turns tier 1 when logged-in users and batch jobs run out of memory"

[[feature]]
id = "page-faults"
title = "Page faults: a page mapped when it is first touched, not an access violation"
status = "todo"
needs = ["//platform/pal"]
note = "PRD-0003 tier 3 #21; DESIGN-0001: a page fault is an access violation today"

[[needs]]
on = "//platform/pal"
why = "WRPTE maps a page; the PAL owns the page tables"

[[needs]]
on = "//exec/ipl-sync"
why = "The PFN list changes at IPL$_SYNCH"

[[needs]]
on = "//platform/access-mode-threads"
why = "What each mode may read is its thread's address space"
+++

VMS gives each process its own P0, the program region, growing up, and
P1, the control region: the command interpreter, stacks and per-process
data, growing down. Every process shares S0, the system region. Each
page's PTE has a protection code saying which access modes may read and
write it, and the executive keeps the PFN database of physical pages.
Programs make and delete pages with `$CRETVA`, `$DELTVA`, `$EXPREG` and
`$CNTREG`, change their protection with `$SETPRT`, and lock pages in
memory or in the working set with `$LCKPAG` and `$LKWSET`. VAX pages are
512 bytes, Alpha's 8 KB; OpenVMS Alpha V7.0 added 64-bit P2 and S2
space (`//exec/64-bit-addressing`). Paging, swapping and global sections
build on this.

vaxpunk's is in `vms/exec/memory.mar`, with 4 KB pages, in VAX's layout
below 2 GB, where MACRO-32's sign-extended longwords reach
([ADR-0005](../../docs/adr/0005-access-modes-are-threads.md)). The
executive keeps a free list of PFNs and no page tables: `WRPTE` asks the
PAL to map, unmap or protect a page and returns the old PTE
([DESIGN-0001](../../docs/design/0001-pal-interface.md)). Each outer
mode's seL4 thread has an address space holding exactly what that mode
may read. Pages are allocated and kept: no paging, swapping or sharing.
The PAL has 1,024 PFNs, 4 MB, as many as the root CNode leaves room for
(PRD-0003 tier 4 #39), and a page fault is an access violation, so image
sections are read in whole (tier 3 #21).
