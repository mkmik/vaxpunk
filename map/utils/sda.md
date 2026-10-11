+++
title = "SDA"
summary = "The System Dump Analyzer: a crash dump or the running system's structures"
aka = ["SDA", "ANALYZE/SYSTEM", "ANALYZE/CRASH_DUMP", "SYSDUMP.DMP", "CLUE", "SHOW LOCKS", "SHOW PROCESS/LOCKS", "SHOW SUMMARY", "SHOW CRASH", "SDA extensions"]
kind = "core"
intent = "aspiration"
research = "outline"
refs = ["docs/adr/0030-lock-manager-in-the-executive.md", "docs/prd/0003-multi-user-vms.md", "https://h41379.www4.hpe.com/doc/82final/6549/6549pro_contents.html"]

[[feature]]
id = "running"
title = "ANALYZE/SYSTEM: SHOW SUMMARY, SHOW PROCESS, SHOW DEVICE, EXAMINE on the live system"
status = "todo"
needs = ["//exec/privileges"]

[[feature]]
id = "locks"
title = "SHOW LOCKS, SHOW RESOURCES, SHOW PROCESS/LOCKS"
status = "todo"
needs = [":running", "//exec/lock-manager"]
note = "ADR-0030 and PRD-0003: no DCL command lists locks on VMS; SDA does, and there is no SDA yet"

[[feature]]
id = "symbols"
title = "The executive's symbols, READ SYS$SYSTEM:SYSDEF.STB, FORMAT of a structure"
status = "todo"
needs = [":running"]

[[feature]]
id = "crash"
title = "ANALYZE/CRASH_DUMP: SHOW CRASH, SHOW STACK, the state at the bugcheck"
status = "todo"
needs = [":running", "//exec/crash-dump:dump-file"]

[[feature]]
id = "clue"
title = "CLUE: a report from the dump written at the next boot"
status = "todo"
needs = [":crash"]

[[feature]]
id = "extensions"
title = "SDA extensions and the System Code Debugger"
status = "todo"
needs = [":running"]

[[needs]]
on = "//exec/privileges"
why = "Reading the executive's memory takes CMKRNL"

[[needs]]
on = "//exec/crash-dump"
why = "A dump file to read after a bugcheck"
soft = true

[[needs]]
fuzzy = "A symbol table of the executive (SYSDEF.STB), from the link of EXEC.EXE"
why = "SDA names structures and fields; vaxpunk's link map names only routines and data"
+++

The System Dump Analyzer reads the executive's data structures, in a
crash dump after a bugcheck (`ANALYZE/CRASH_DUMP SYS$SYSTEM:SYSDUMP.DMP`)
or in the running system (`ANALYZE/SYSTEM`, with `CMKRNL`). It knows
the PCB, JIB, UCB, IRP, lock blocks and the rest by name: `SHOW
SUMMARY` lists processes, `SHOW PROCESS` one process's registers,
channels and locks, `SHOW DEVICE` the I/O database, `SHOW LOCKS` the
lock manager's resources, `SHOW CRASH` the bugcheck and where it
happened; `EXAMINE` and `FORMAT` decode memory. On Alpha, CLUE writes a
report from each new dump at boot, and extensions add commands for
subsystems such as XFC and spinlock tracing.

vaxpunk has none. A bugcheck today is the PAL printing a line such as
`%PAL-F-NOVEC` and stopping, with no dump. ADR-0030 and PRD-0003 note
that on VMS only SDA lists locks, and that there is no SDA yet.
