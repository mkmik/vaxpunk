+++
title = "Paging and working sets"
summary = "Page faults, working sets, the free and modified lists, page files"
aka = ["page fault", "working set", "WSDEFAULT", "WSQUOTA", "WSEXTENT", "AWSA", "PFRATH", "PFRATL", "free page list", "modified page list", "modified page writer", "MPW", "PAGEFILE.SYS", "PGFLQUOTA", "secondary page cache", "$PURGWS", "$ADJWSL"]
kind = "core"
intent = "aspiration"
research = "outline"
refs = ["docs/prd/0003-multi-user-vms.md", "docs/design/0001-pal-interface.md", "https://docs.vmssoftware.com/vsi-openvms-performance-management-manual", "https://neilrieck.net/raxco/raxco-3.html"]

[[feature]]
id = "faults"
title = "Page faults: pages read from the image file or the page file, or made zero, when touched"
status = "todo"
needs = ["/exec/memory#page-faults"]

[[feature]]
id = "working-sets"
title = "Working sets: a limit per process from WSDEFAULT, WSQUOTA and WSEXTENT"
status = "todo"
needs = ["#faults"]

[[feature]]
id = "page-cache"
title = "The free and modified page lists: pages left a working set, found again without I/O"
status = "todo"
needs = ["#working-sets"]

[[feature]]
id = "page-files"
title = "Page files, PAGEFILE.SYS and more, and the modified page writer that fills them"
status = "todo"
needs = ["#page-cache"]

[[feature]]
id = "awsa"
title = "Automatic working set adjustment: grow on PFRATH, shrink on PFRATL"
status = "todo"
needs = ["#working-sets"]

[[feature]]
id = "pgflquota"
title = "PGFLQUOTA and the working set quotas charged per process"
status = "todo"
needs = ["#page-files", "/exec/quotas"]

[[needs]]
on = "/exec/memory"
why = "Pages, PTEs and the PFN database"

[[needs]]
on = "/exec/scheduler"
why = "Page fault and free page waits"

[[needs]]
on = "/drivers/disk"
why = "Reads and writes pages on disk"

[[needs]]
on = "/exec/sysgen-parameters"
why = "Working set and page list limits"
soft = true

[[needs]]
on = "/drivers/interrupt-io"
why = "A hard fault waits for its read while others run"
soft = true

[[needs]]
on = "/platform/pal#page-faults"
why = "Today the PAL owns the page tables and a missing page is an access violation"
+++

VMS is a paging system. Each process has a working set, the pages it may
have valid at once, limited by `WSDEFAULT`, `WSQUOTA` and `WSEXTENT` and
adjusted by the page fault rate. A page fault on a page not in the
working set finds it in the free or modified page list (a soft fault),
reads it from the image file, a mapped section file or the page file (a
hard fault), or makes it zero. Pages that leave a working set go to the
free list, or to the modified list if written, from which the modified
page writer writes them to the page file. `PGFLQUOTA` limits a process's
share of the page files, which `SYSGEN CREATE` and `AUTOGEN` make.

vaxpunk has no paging: memory is allocated and kept, a non-goal of
PRD-0003. Image sections are read in whole when the image is activated
(PRD-0003 tier 3 #21). The PAL owns the page tables and a touch of a page
that isn't mapped is an access violation
([DESIGN-0001](../../design/0001-pal-interface.md)), so faulting
pages in needs a PAL change first (`t8y:/exec/memory#page-faults`). With
4 MB of memory (tier 4 #39) the first need is more memory, not paging.
