+++
title = "Pool"
summary = "Nonpaged pool: the executive's heap for PCBs, IRPs, ACBs and the rest"
aka = ["nonpaged pool", "paged pool", "EXE$ALONONPAGED", "EXE$DEANONPGDSIZ", "lookaside lists", "NPAGEDYN", "NPAGEVIR", "PAGEDYN", "POOLCHECK"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["docs/design/0002-executive-processes.md", "vms/exec/memory.mar"]

[[feature]]
id = "nonpaged"
title = "Nonpaged pool, 512 KB in S0, first fit in 16-byte units"
status = "done"
milestone = "m0"

[[feature]]
id = "sizing"
title = "Pool sized at boot from NPAGEDYN and grown up to NPAGEVIR"
status = "todo"
note = "memory.mar: POOL_PAGES, a constant"

[[feature]]
id = "lookaside"
title = "Lookaside lists for the common sizes: IRPs, ACBs, TQEs"
status = "todo"

[[feature]]
id = "paged"
title = "Paged pool, for logical names, the known file list and other data used at low IPL"
status = "todo"

[[feature]]
id = "poolcheck"
title = "Pool checking: freed blocks filled and checked (POOLCHECK)"
status = "todo"

[[needs]]
on = "/exec/memory"
why = "Its pages, made at boot"

[[needs]]
on = "/exec/ipl-sync"
why = "Allocated and freed at IPL$_SYNCH"

[[needs]]
on = "/exec/sysgen-parameters"
why = "NPAGEDYN, NPAGEVIR and PAGEDYN size it"
soft = true
+++

VMS's executive allocates its data structures from pool. Nonpaged pool,
which stays in memory, holds what code at raised IPL touches: PCBs,
UCBs, IRPs, ACBs, timer queue entries, lock blocks. Paged pool holds what
is touched only at low IPL, such as logical names and the known file
list. Lookaside lists serve the common sizes quickly. SYSGEN's
`NPAGEDYN` and `NPAGEVIR` size nonpaged pool and `PAGEDYN` paged pool,
and process quotas (`BYTLM` and others) keep one user from taking it all.

vaxpunk has nonpaged pool only: 512 KB at `0x48000000`, first fit from a
list of free blocks sorted by address, in 16-byte units, at
`IPL$_SYNCH` (`EXE$ALONONPAGED` and `EXE$DEANONPGDSIZ` in
`vms/exec/memory.mar`). Everything comes from it: PCBs and kernel
stacks, IRPs with their buffers, ACBs, TQEs, logical names, open files,
mailbox messages, and images while they are activated. No quota limits
it yet (PRD-0003 tier 1 #4, in `t8y:/exec/quotas`), so one process can
take the pool with reads or mailbox messages.
