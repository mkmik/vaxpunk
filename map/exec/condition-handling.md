+++
title = "Condition handling"
summary = "Exceptions and signals, handlers along the call chain, $UNWIND, the catch-all"
aka = ["CHF", "LIB$SIGNAL", "LIB$STOP", "LIB$ESTABLISH", "$UNWIND", "$SETEXV", "signal array", "mechanism array", "SS$_RESIGNAL", "SS$_CONTINUE", "last chance handler", "exception vectors", "EXE$CATCHALL", "sysunwind.mar"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["docs/adr/0021-condition-handlers-run-in-the-mode-that-signals.md", "docs/adr/0023-calling-standard.md", "docs/design/0004-calling-standard.md", "docs/design/0002-executive-processes.md", "vms/exec/sysunwind.mar", "vms/sysexe/chftest.mar", "docs/prd/0003-multi-user-vms.md"]

[[feature]]
id = "handlers"
title = "Handlers found along the FP chain and called in the mode that signaled"
status = "done"
milestone = "m0"
refs = ["docs/adr/0021-condition-handlers-run-in-the-mode-that-signals.md"]

[[feature]]
id = "exceptions"
title = "Access violations, reserved instructions and BPT reflected to the faulting mode"
status = "done"
milestone = "m0"

[[feature]]
id = "unwind"
title = "$UNWIND from any frame, by the frame descriptors; SS$_UNWIND to each handler"
status = "done"
milestone = "m0"

[[feature]]
id = "catchall"
title = "The catch-all: $PUTMSG, then $EXIT through the exit handlers if severe"
status = "done"
milestone = "m0"

[[feature]]
id = "last-chance"
title = "A fault with no room for the arrays still runs the exit handlers"
status = "todo"
milestone = "m1"
note = "PRD-0003 tier 2 #12: it ends the image past them (sysunwind.mar, process.mar)"

[[feature]]
id = "vectors"
title = "Primary, secondary and last chance exception vectors, $SETEXV"
status = "todo"
note = "ADR-0021 follow-up; DEBUG uses them"

[[feature]]
id = "nested"
title = "A fault in a handler skips the frames already searched"
status = "todo"
note = "ADR-0021 follow-up; sysunwind.mar ponytail"

[[feature]]
id = "arith-traps"
title = "Arithmetic traps: integer overflow, divide by zero, floating point"
status = "todo"
note = "ADR-0021 follow-up"

[[feature]]
id = "target-invo"
title = "The target frame's handler, FDSC$V_TARGET_INVO; JSB routines' saves restored"
status = "todo"
note = "ADR-0021 follow-up; sysunwind.mar ponytail"

[[needs]]
on = "//exec/calling-standard"
why = "Frames, the handler slot at 16(FP) and frame descriptors"

[[needs]]
on = "//exec/messages"
why = "The catch-all writes the condition's message with $PUTMSG"

[[needs]]
on = "//exec/exit-handlers"
why = "A severe condition exits the image through them"

[[needs]]
on = "//exec/system-services"
why = "The search, $UNWIND and $PUTMSG run in outer modes, from the vector"

[[needs]]
on = "//platform/pal"
why = "Faults in an outer mode come to the executive through the SCB"
+++

A VMS condition is a 32-bit status signaled to whoever can handle it. A
hardware exception, or a call to `LIB$SIGNAL` or `LIB$STOP`, builds a
signal array (the condition, its arguments, the PC and PSL) and a
mechanism array, and the system calls the condition handlers that
routines established in their frames, from the newest frame outward, in
the access mode that signaled. A handler continues, resignals or
unwinds the stack to its establisher with `$UNWIND`. Before the frames
come the primary and secondary exception vectors, which the debugger
uses, and after them the last chance vector and the catch-all, which
writes the message and exits if the condition is severe. Every language's
run-time library maps its errors onto this.

vaxpunk's is in `vms/exec/sysunwind.mar`
([ADR-0021](../../docs/adr/0021-condition-handlers-run-in-the-mode-that-signals.md)).
An exception in an outer mode is reflected to that mode with 64-bit VMS's
arrays, and the search, `$UNWIND` and the catch-all run there from the
system service vector. The calling standard puts the handler at `16(FP)`
and a frame descriptor at `24(FP)`, so `$UNWIND` can restore what any
frame saved ([ADR-0023](../../docs/adr/0023-calling-standard.md)).
`CHFTEST` checks it at boot. Missing: exception vectors, arithmetic
traps, and an exit through the exit handlers when the faulting mode's
stack has no room (PRD-0003 tier 2 #12).
