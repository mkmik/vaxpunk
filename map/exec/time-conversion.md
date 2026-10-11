+++
title = "Time conversion"
summary = "$ASCTIM, $BINTIM and $NUMTIM: times as text and as numbers"
aka = ["$ASCTIM", "$BINTIM", "$NUMTIM", "$ASCUTC", "$BINUTC", "$NUMUTC", "delta time", "numtim.b64"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m1"
refs = ["vms/exec/timeschdl.mar", "vms/exec/numtim.b64", "docs/prd/0003-multi-user-vms.md", "docs/prd/0004-bliss64-compiler.md"]

[[feature]]
id = "asctim"
title = "$ASCTIM: dd-MMM-yyyy hh:mm:ss.cc, delta times, the time alone"
status = "done"
milestone = "m1"
note = "PRD-0003 tier 1 #7"

[[feature]]
id = "numtim"
title = "$NUMTIM, in BLISS-64, run in the caller's mode from the vector"
status = "done"
milestone = "m1"
note = "PRD-0004 step 11, the first BLISS-64 in the system"

[[feature]]
id = "bintim"
title = "$BINTIM: text to a time, absolute or delta, with fields left out"
status = "todo"
milestone = "m1"
note = "PRD-0003 tier 1 #7: SUBMIT/AFTER needs it; SHOW TIME is left too"

[[feature]]
id = "utc-forms"
title = "$ASCUTC, $BINUTC and $NUMUTC, on 128-bit UTC times"
status = "todo"
needs = ["//exec/timers:utc"]

[[needs]]
on = "//exec/timers"
why = "The system time, when no time is given"

[[needs]]
on = "//exec/system-services"
why = "$NUMTIM runs from the vector, in the caller's mode"

[[needs]]
on = "//host/vbliss"
why = "numtim.b64 is BLISS-64"
+++

VMS converts its quadword times with three services: `$ASCTIM` writes
one as text, `17-NOV-1858 00:00:00.00` or a delta `dddd hh:mm:ss.cc`;
`$BINTIM` reads that text back, filling in what is left out from the
current time; `$NUMTIM` gives the year, month, day and the rest as seven
words. V6 added the UTC forms, `$ASCUTC`, `$BINUTC` and `$NUMUTC`, on
128-bit UTC times with a time zone. DCL's `F$TIME`, `F$CVTIME`, `SHOW
TIME`, `SUBMIT/AFTER` and the `LIB$` date routines are built on them.

vaxpunk has `$ASCTIM` in MACRO-32 (`vms/exec/timeschdl.mar`), which
`SHOW PROCESS`, `SHOW SYSTEM` and `F$TIME` use, and `$NUMTIM` in BLISS-64
(`vms/exec/numtim.b64`), the pilot of PRD-0004, which runs in the
caller's mode from the system service vector. `$BINTIM` is left for
PRD-0003's batch step (tier 1 #7).
