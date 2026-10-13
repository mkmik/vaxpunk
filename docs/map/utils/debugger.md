+++
title = "DEBUG"
summary = "The symbolic debugger: breakpoints, steps and variables by name, any language"
aka = ["DEBUG", "OpenVMS Debugger", "DBG$", "RUN/DEBUG", "DEBUG/KEEP", "ANALYZE/PROCESS_DUMP", "SET BREAK", "STEP", "EXAMINE", "DST", "DSF", "traceback", "Heap Analyzer", "SYS$DEBUG"]
kind = "core"
intent = "aspiration"
research = "deferred"
refs = ["docs/prd/0001-vtools.md", "docs/prd/0004-bliss64-compiler.md", "https://h41379.www4.hpe.com/doc/84final/4538/4538pro_001.html"]

[[feature]]
id = "traceback"
title = "Traceback: an unhandled condition lists the routines and lines on the stack"
status = "todo"
needs = ["/exec/condition-handling"]

[[feature]]
id = "dst"
title = "Debug symbol records from vbliss, vmacro and vlink"
status = "todo"
needs = ["/host/vbliss#debug-symbols", "/host/vmacro#debug-symbols", "/host/vlink#debug-symbols"]
note = "PRD-0001 and PRD-0004: no DST yet; the listing and the link map are the debugging aids"

[[feature]]
id = "command"
title = "RUN/DEBUG: SET BREAK, STEP, GO, EXAMINE, DEPOSIT, SHOW CALLS, by symbol"
status = "todo"
needs = ["#dst"]

[[feature]]
id = "screen"
title = "Screen mode: source, instruction, register and output displays"
status = "todo"
needs = ["#command", "/rtl/smg"]

[[feature]]
id = "kept"
title = "DEBUG/KEEP: run and rerun programs in one session"
status = "todo"
needs = ["#command"]

[[feature]]
id = "multiprocess"
title = "Several processes and kernel threads in one session; client and server"
status = "todo"
needs = ["#command"]

[[feature]]
id = "process-dump"
title = "ANALYZE/PROCESS_DUMP: a process's dump file, after it dies, under the debugger"
status = "todo"
needs = ["#command"]

[[feature]]
id = "gui"
title = "The DECwindows Motif interface and the Heap Analyzer"
status = "todo"
needs = ["#command"]
note = "/gui/motif"

[[needs]]
on = "/exec/condition-handling"
why = "Breakpoints and steps arrive as conditions the debugger handles"

[[needs]]
on = "/exec/image-activator"
why = "The image activator maps the debugger in before the program's first instruction"

[[needs]]
on = "/host/vlink"
why = "The image has to carry symbols and a traceback table"

[[needs]]
on = "/host/vms-obj#debug-symbols"
why = "PRD-0001 and PRD-0004 have no DST; the debugger needs names, types and line numbers"
+++

The OpenVMS Debugger works the same way for every language. A program
compiled and linked with `/DEBUG` starts under it at `RUN`; the user
sets breakpoints and watchpoints by routine name or line, steps, shows
the call stack, and examines and changes variables by name and type,
in the program's own language. It has a line mode, a screen mode with
source and register windows, a DECwindows Motif interface and a
client/server mode; `DEBUG/KEEP` runs program after program in one
session, and the Heap Analyzer shows memory use. Programs linked with
only `/TRACEBACK` print the routines and line numbers on the stack when
an error ends them.

vaxpunk has no debugger on the system. Debugging today is on the host:
`vrun --gdb` and `cargo run -p boot -- --gdb` start QEMU waiting for
LLDB or gdb, link maps name addresses, and the PAL prints
`%PAL-F-ACCVIO` with the PC. PRD-0001 and PRD-0004 leave debug symbol
records out, so the first need is a symbol format the compilers and
`vlink` write.
