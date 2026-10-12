+++
title = "MONITOR"
summary = "Live and recorded system performance displays, by class"
aka = ["MONITOR", "MONITOR.EXE", "MONITOR PROCESSES/TOPCPU", "MONITOR SYSTEM", "MONITOR LOCK", "MONITOR MODES", "MONITOR IO", "MONITOR FCP", "/RECORD", "/INPUT", "SPM", "VPM"]
kind = "core"
intent = "aspiration"
research = "outline"
refs = ["https://wasd.vsm.com.au/htbin/helpgate/HELP/MONITOR", "docs/adr/0030-lock-manager-in-the-executive.md"]

[[feature]]
id = "basic"
title = "MONITOR SYSTEM and PROCESSES: CPU use, process states, a refreshing screen"
status = "todo"
needs = ["/exec/information-services", "/exec/accounting"]

[[feature]]
id = "classes"
title = "MODES, STATES, IO, FCP, LOCK, RMS, PAGE, DISK and the other classes"
status = "todo"
needs = ["#basic"]

[[feature]]
id = "topcpu"
title = "PROCESSES/TOPCPU, /TOPDIO and the other bar graphs"
status = "todo"
needs = ["#basic"]

[[feature]]
id = "record"
title = "Recording to a file (/RECORD), playback (/INPUT) and /SUMMARY"
status = "todo"
needs = ["#basic"]

[[feature]]
id = "cluster"
title = "MONITOR CLUSTER and remote nodes through a server process"
status = "todo"
needs = ["#basic"]
note = "Clusters are a PRD-0003 non-goal (t8y:/exec/cluster)"

[[needs]]
on = "/exec/information-services"
why = "$GETSYI, $GETJPI and $GETDVI for most of its numbers"

[[needs]]
on = "/drivers/terminal"
why = "Repaints a VT100 screen every few seconds"

[[needs]]
on = "/rtl/smg"
why = "Its displays are SMG$ screens"
soft = true

[[needs]]
fuzzy = "Executive counters that MONITOR reads directly: per-mode CPU time, page faults, lock and XQP counts"
why = "Many classes read data cells in the executive, with CMKRNL, rather than through services"
+++

`MONITOR` shows how the system is doing, refreshed every few seconds:
`MONITOR SYSTEM` the CPU busy and idle, processes, page faults and I/O
rates, `MONITOR PROCESSES/TOPCPU` a bar graph of who uses the CPU,
`MONITOR MODES` the time spent in each access mode and interrupt
state, and one class each for I/O, the file system (`FCP`), locks,
RMS, disks, paging and the cluster. It can record to a file and play a
recording back or summarise it, which system managers ran in batch
around the clock for capacity planning.

vaxpunk has none. What exists to build on: `SHOW SYSTEM` and `SHOW
PROCESS` from `$GETJPI` and `$GETSYI`, and PRD-0003's step 5, which adds
CPU time and I/O counts per process. ADR-0030 names `MONITOR LOCK` as
one of the places VMS shows locks.
