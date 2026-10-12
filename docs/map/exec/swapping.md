+++
title = "Swapping"
summary = "The swapper writes idle processes' working sets to a swap file to free memory"
aka = ["swapper", "outswap", "inswap", "SWAPFILE.SYS", "FREELIM", "FREEGOAL", "SWPOUTPGCNT", "LONGWAIT", "DORMANTWAIT", "proactive memory reclamation", "MMG_CTLFLAGS", "$SETSWM", "PSWAPM", "working set trimming"]
kind = "core"
intent = "aspiration"
research = "outline"
refs = ["docs/prd/0003-multi-user-vms.md", "docs/design/0002-executive-processes.md", "vms/exec/exec.mar", "https://docs.vmssoftware.com/vsi-openvms-performance-management-manual"]

[[feature]]
id = "trimming"
title = "Working sets trimmed when free pages fall below FREELIM, back toward FREEGOAL"
status = "todo"
needs = ["/exec/paging#working-sets"]

[[feature]]
id = "outswap"
title = "Outswap and inswap: an idle process's whole working set to a swap file and back"
status = "todo"
needs = ["#trimming"]

[[feature]]
id = "swap-files"
title = "Swap files, SWAPFILE.SYS, made by SYSGEN CREATE or AUTOGEN"
status = "todo"
needs = ["#outswap"]

[[feature]]
id = "proactive"
title = "Proactive reclamation from processes that wake now and then but are mostly idle"
status = "todo"
needs = ["#outswap"]

[[feature]]
id = "noswap"
title = "$SETSWM and SET PROCESS/NOSWAPPING, with PSWAPM"
status = "todo"
needs = ["#outswap", "/exec/privileges"]

[[needs]]
on = "/exec/paging"
why = "Trims working sets and shares the page lists"

[[needs]]
on = "/exec/processes"
why = "The swapper is a process; outswapped processes stay in the balance set's tables"

[[needs]]
on = "/exec/scheduler"
why = "Outswapped states, and inswap when a process becomes computable"

[[needs]]
on = "/drivers/disk"
why = "Writes and reads working sets"

[[needs]]
on = "/exec/sysgen-parameters"
why = "FREELIM, FREEGOAL, SWPOUTPGCNT, LONGWAIT"
soft = true
+++

When free memory runs low, VMS's swapper, a process at priority 16,
first trims working sets back to their quotas, then picks idle
processes (suspended ones first, then those waiting longer than
`LONGWAIT`) and writes each one's whole working set to a swap file in a
few large writes. An outswapped process comes back into memory when it
becomes computable again. Later versions also take memory proactively
from processes that wake now and then but are idle most of the time.
`$SETSWM` keeps a privileged process in memory.

vaxpunk's process 1 is called the swapper, as on VMS, but it swaps
nothing: it frees what deleted processes left and starts `LOGINOUT` on
the console (`vms/exec/exec.mar`). Swapping is a non-goal of PRD-0003;
memory is allocated and kept, and paging comes first.
