+++
title = "Bugchecks and crash dumps"
summary = "Stopping the system on a fatal error, and saving memory for SDA to analyze"
aka = ["BUG_CHECK", "bugcheck", "fatal bugcheck", "nonfatal bugcheck", "BUGCHECKFATAL", "SYSDUMP.DMP", "DUMPSTYLE", "selective dump", "compressed dump", "DOSD", "dump off system disk", "SAVEDUMP", "ANALYZE/CRASH_DUMP", "CLUE", "CLUE$HISTORY", "OPCCRASH", "OPERATOR bugcheck", "dump kernel", "%PAL-F-", "%EXEC-F-"]
kind = "core"
intent = "goal"
research = "outline"
refs = ["docs/adr/0008-host-powers-qemu-off-on-halt.md", "docs/adr/0029-loginout-and-sysuaf.md", "docs/boot.md", "vms/exec/exec.mar", "vms/sysexe/opccrash.mar", "vms/sysexe/shutdown.com", "pal/src/main.c", "scripts/run-qemu.sh"]

[[feature]]
id = "fatal-stop"
title = "A fatal error stops the system with a message: %EXEC-F-, %PAL-F-; the host powers QEMU off"
status = "done"
milestone = "m0"
note = "ADR-0008; exec.mar's FAIL halts; the PAL stops the executive on a kernel-mode exception, \"as a bugcheck\""

[[feature]]
id = "opccrash"
title = "MCR OPCCRASH, the last step of SHUTDOWN.COM, halts the system with CMKRNL"
status = "done"
note = "ADR-0029 decision 7; opccrash.mar, shutdown.com"

[[feature]]
id = "bug-check"
title = "BUG_CHECK with a code: the bugcheck banner, the code's text, PC, PSL, process and image"
status = "todo"

[[feature]]
id = "nonfatal"
title = "Nonfatal bugchecks: the process or the request ends, the system goes on; BUGCHECKFATAL"
status = "todo"
note = "Logged by //exec/error-logging"

[[feature]]
id = "dump-file"
title = "Physical memory written to SYS$SYSTEM:SYSDUMP.DMP, or the page file, on the way down"
status = "todo"

[[feature]]
id = "selective"
title = "Selective and compressed dumps, and dumps off the system disk: DUMPSTYLE, DOSD"
status = "todo"
needs = [":dump-file"]

[[feature]]
id = "sda"
title = "ANALYZE/CRASH_DUMP: SDA reads the dump; SAVEDUMP copies it out of the page file"
status = "todo"
needs = [":dump-file"]
note = "//utils/sda"

[[feature]]
id = "clue"
title = "CLUE: each crash's summary written at the next boot to CLUE$HISTORY"
status = "todo"
needs = [":sda"]

[[feature]]
id = "reboot"
title = "Reboot after the dump, instead of powering off"
status = "todo"

[[needs]]
on = "//platform/pal"
why = "Stops the machine, and would write the dump when the executive can't be trusted"

[[needs]]
on = "//utils/sda"
why = "Analyzes the dump"
soft = true

[[needs]]
on = "//exec/error-logging"
why = "Nonfatal bugchecks and the crash are logged"
soft = true

[[needs]]
fuzzy = "A dump writer that works when the executive is broken: the PAL writing blocks by LBN, or a separate dump kernel as on VSI's x86-64"
why = "VMS writes the dump through a boot-time driver; x86-64 boots a second instance from the memory disk"
+++

When the VMS executive finds something it can't go on from, a
corrupted structure or an exception in kernel mode at a high IPL, it
calls `BUG_CHECK` with a code. A fatal bugcheck stops the system: it
writes a banner naming the bugcheck, the PC, the process and the image
on the console, writes physical memory to the dump file,
`SYS$SYSTEM:SYSDUMP.DMP` or the page file, and reboots. A nonfatal one
ends what went wrong and logs it, unless `BUGCHECKFATAL` says to stop.
`OPCCRASH`, which `SHUTDOWN.COM` runs last, crashes on purpose with an
`OPERATOR` bugcheck, so a clean shutdown leaves a dump too. After the
reboot, SDA (`ANALYZE/CRASH_DUMP`) reads the dump, and CLUE writes a
summary of each crash. Alpha added selective dumps (only the memory
that matters), compressed dumps and dumps off the system disk; VSI's
x86-64 port writes the dump from a second, minimal instance, the dump
kernel, booted from the memory disk.

vaxpunk stops but saves nothing. `EXEC$START` halts with
`%EXEC-F-NOMOUNT` or `NOSTART` and a status (`vms/exec/exec.mar`); an
exception in kernel mode makes the PAL stop the executive, "as a
bugcheck", with a `%PAL-F-` line (`pal/src/main.c`); `MCR OPCCRASH`
halts with `CMKRNL`. On a halt the root task prints a line and the host
powers QEMU off ([ADR-0008](../../docs/adr/0008-host-powers-qemu-off-on-halt.md)).
`run-qemu.sh --gdb` lets a debugger look at the system instead. There
is no `BUG_CHECK`, dump file or SDA.
