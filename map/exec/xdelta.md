+++
title = "XDELTA"
summary = "The executive's own debugger, on the console, and the System Code Debugger"
aka = ["XDELTA", "DELTA", "SYSTEM_DEBUG", "System Code Debugger", "SCD", "DEBUG/KEEP", "boot flag 2", "INI$BRK", "breakpoint 1"]
kind = "core"
intent = "aspiration"
research = "deferred"
refs = ["scripts/run-qemu.sh", "docs/prd/0005-macro32-on-the-calling-standard.md"]

[[feature]]
id = "xdelta"
title = "XDELTA loaded at boot: breakpoints, examine and deposit, step, on the console at IPL 31"
status = "todo"

[[feature]]
id = "initial-break"
title = "An initial breakpoint, INI$BRK, early in the executive's start"
status = "todo"
needs = [":xdelta"]

[[feature]]
id = "delta"
title = "DELTA: the same debugger for one process's code, in any mode"
status = "todo"

[[feature]]
id = "scd"
title = "The System Code Debugger: DEBUG's commands on executive code, from another node"
status = "todo"
needs = ["//utils/debugger", "//net/tcpip-qio"]

[[needs]]
on = "//drivers/terminal"
why = "XDELTA talks to the console directly, polled"

[[needs]]
on = "//platform/pal"
why = "Breakpoints and single steps arrive as exceptions the PAL delivers"

[[needs]]
on = "//exec/system-init"
why = "Loaded by a boot flag, before the executive's start"
soft = true
+++

XDELTA is a small debugger built into the VMS executive, loaded when
the system boots with the right flag. It takes over the console at IPL
31: a programmer sets breakpoints in the executive or a driver,
examines and deposits memory and registers, and steps instructions, in
the terse syntax of DEC's DDT family. `DELTA` is the same debugger for
one process's code. On Alpha and later, the System Code Debugger runs
DEBUG's full command language against the executive from another
node, over the network.

vaxpunk debugs the executive from the host instead:
`run-qemu.sh --gdb` makes QEMU wait for gdb or LLDB, and `vrun --gdb`
does the same for one image, with frames gdb can walk since PRD-0005.
An XDELTA of its own would matter on real hardware and once vaxpunk
is meant to be worked on from inside.
