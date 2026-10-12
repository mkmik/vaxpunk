+++
title = "Timers and system time"
summary = "The interval timer, the system time, the timer queue, $SETIMR and $SCHDWK"
aka = ["$GETTIM", "$SETIMR", "$CANTIM", "$SCHDWK", "$CANWAK", "$SETIME", "$GETUTC", "$GETTIM_PREC", "TQE", "EXE$GQ_SYSTIME", "TQELM", "SYS$TIMEZONE_DIFFERENTIAL", "TDF", "timeschdl.mar"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["docs/adr/0004-interval-timer-is-a-pal-thread.md", "docs/design/0002-executive-processes.md", "vms/exec/timeschdl.mar", "docs/prd/0003-multi-user-vms.md", "docs/prd/0007-wasmrun.md"]

[[feature]]
id = "clock"
title = "A 10 ms interval timer; the system time from the RTC at boot"
status = "done"
milestone = "m0"
refs = ["docs/adr/0004-interval-timer-is-a-pal-thread.md"]

[[feature]]
id = "timer-queue"
title = "The timer queue: $GETTIM, $SETIMR and $CANTIM, $SCHDWK and $CANWAK"
status = "done"
milestone = "m0"

[[feature]]
id = "driver-timers"
title = "Timer queue entries for drivers: a terminal's timed read"
status = "done"
milestone = "m0"
note = "TQE$C_SSSNGL, ADR-0027"

[[feature]]
id = "tqelm"
title = "TQELM: the timer queue entry quota"
status = "todo"
milestone = "m1"
note = "PRD-0003 tier 1 #4; with t8y:/exec/quotas#asts-timers, which needs this component, so no need back"

[[feature]]
id = "utc"
title = "$GETUTC and a time zone differential, SYS$TIMEZONE_DIFFERENTIAL"
status = "todo"
note = "PRD-0007 M0 wants $GETUTC; the C RTL's timezone feature wants the differential"

[[feature]]
id = "set-time"
title = "$SETIME and SET TIME, with OPER and LOG_IO; the RTC written back"
status = "todo"

[[feature]]
id = "y2038"
title = "The RTC's seconds past 2038"
status = "todo"
note = "PRD-0003 tier 4 #43: EMUL is signed"

[[needs]]
on = "/platform/pal"
why = "The PAL's clock thread is the interval timer; it reads the RTC"

[[needs]]
on = "/exec/ipl-sync"
why = "The clock at IPL$_HWCLK, the timer queue at IPL$_SYNCH"

[[needs]]
on = "/exec/event-flags"
why = "$SETIMR sets an event flag"

[[needs]]
on = "/exec/asts"
why = "$SETIMR's AST is its timer queue entry"

[[needs]]
on = "/exec/pool"
why = "Timer queue entries come from pool"

[[needs]]
on = "/exec/scheduler"
why = "The clock ends quanta; $SCHDWK wakes processes"
+++

VMS keeps the system time as a quadword of 100 ns units since
17-Nov-1858, advanced by the interval timer's interrupt, and reads the
battery-backed clock at boot. The timer queue holds what is due later:
`$SETIMR` sets an event flag or queues an AST at a time or after a delta,
`$SCHDWK` wakes a hibernating process, once or repeatedly, and drivers
time out their requests. `$SETIME` changes the time and writes it to the
clock. VMS V6 and later add UTC: `$GETUTC` and a time zone differential,
`SYS$TIMEZONE_DIFFERENTIAL`, that the system time is local time by.
Processes are limited by `TQELM`.

vaxpunk's are in `vms/exec/timeschdl.mar`. The PAL's clock thread
interrupts every 10 ms
([ADR-0004](../../adr/0004-interval-timer-is-a-pal-thread.md)),
`EXE$HWCLKINT` advances the time and charges the quantum, and
`EXE$SWTIMINT` serves the timer queue at a lower IPL, as VMS splits
them. The time starts from the PL031 RTC's seconds, which the PAL puts in
the RPB. Missing: `TQELM`, UTC and time zones, setting the time, and the
seconds past 2038 (PRD-0003 tier 4 #43).
