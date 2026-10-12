+++
title = "Calling standard"
summary = "AAPCS64 plus VMS's argument count, sign extension and self-describing frames"
aka = ["OpenVMS Calling Standard", "vaxpunk calling standard", "AAPCS64", "x9 argument information", "frame descriptor", "$FDSCDEF", "$CHFDEF", "signal array", "mechanism array", "AMACRO", ".CALL_ENTRY", ".JSB_ENTRY", ".JSB32_ENTRY", "QUAD_ARGS", "$CALL64", "EVAX_CALLG_64", "descriptors", "DSC64$", "item lists"]
kind = "core"
intent = "goal"
research = "done"
refs = ["docs/adr/0023-calling-standard.md", "docs/design/0004-calling-standard.md", "docs/prd/0005-macro32-on-the-calling-standard.md", "docs/adr/0021-condition-handlers-run-in-the-mode-that-signals.md", "crosstools/vtools/docs/amacro.md", "crosstools/vtools/lib/call.mlb", "crosstools/vtools/lib/lib.mlb", "vms/exec/syssrv.mar", "vms/exec/sysunwind.mar", "pal/src/main.c"]

[[feature]]
id = "pal-frame"
title = "Interrupts and exceptions keep every register: the PAL's frame holds x0-x30 and sp"
status = "done"
needs = ["/platform/pal"]
note = "PRD-0005 step 1; ADR-0023 decision 8"

[[feature]]
id = "registers"
title = "R0-R1 in x0-x1, R2-R11 in x19-x28, SP in x18, longwords kept sign-extended"
status = "done"
note = "PRD-0005 step 2"

[[feature]]
id = "declarations"
title = "AMACRO's .CALL_ENTRY, .JSB_ENTRY and .JSB32_ENTRY, and full 64-bit preservation"
status = "done"
note = "PRD-0005 step 3: the tree's JSB routines declared"

[[feature]]
id = "frames"
title = "Frames name their descriptor at 24(FP), handler at 16(FP); 64-bit signal and mechanism arrays"
status = "done"
note = "PRD-0005 step 4; $FDSCDEF, $CHFDEF, sysunwind.mar, call.mlb"

[[feature]]
id = "arguments"
title = "CALLS and CALLG in x0-x7 and the stack, count in x9; AP's list homed at 32(FP)"
status = "done"
note = "PRD-0005 step 5"

[[feature]]
id = "services"
title = "CHMx carries its code in x7; frameless SYS$name; the sign-extension check"
status = "done"
note = "PRD-0005 step 6"

[[feature]]
id = "pieces-64"
title = "DEC's 64-bit pieces: quad_args, quadword arithmetic, EVAX_ built-ins, $CALL64, EVAX_CALLG_64"
status = "done"
note = "PRD-0005 step 7"

[[feature]]
id = "traceback"
title = "vrun prints a traceback through standard frames, from their descriptors"
status = "done"
note = "PRD-0005 step 4; vrun's -VRUN-I-FRAME lines"

[[feature]]
id = "bliss"
title = "BLISS-64's default linkage is the standard; JSB linkages match the declarations"
status = "done"
note = "PRD-0004; DESIGN-0004 BLISS-64"

[[feature]]
id = "c-jackets"
title = "C from a stock gcc reaches VMS routines through jackets"
status = "done"
note = "DESIGN-0004 C and Fortran, ADR-0032: gcc sets no x9 and zero-extends int results"

[[feature]]
id = "c-compiler"
title = "A C compiler that sets x9, sign-extends and builds standard frames"
status = "todo"
note = "DESIGN-0004 C and Fortran; t8y:/lang/c"

[[feature]]
id = "unwind-tables"
title = "Unwind tables keyed by PC for frames other compilers build"
status = "todo"
note = "ADR-0023 follow-up; PRD-0005 non-goal"

[[feature]]
id = "floating-point"
title = "Floating point arguments in v0-v7, their classes in x9; d8-d15 kept by the unwinder"
status = "todo"
note = "PRD-0007's feasibility: no floating point in vasm and vmacro yet"

[[feature]]
id = "address-space-64"
title = "64-bit addresses in arguments: P2 and S2, _64 services, 64-bit pool"
status = "todo"
note = "ADR-0023 follow-up and PRD-0005 non-goal: an ADR of its own; t8y:/exec/64-bit-addressing"

[[feature]]
id = "invocation-context"
title = "LIB$GET_CURR_INVO_CONTEXT and the invocation context routines"
status = "todo"
note = "ADR-0023 follow-up"

[[feature]]
id = "frame-flags"
title = "AST and exception frame flags; the target frame's handler on unwind; SYS$SET_RETURN_VALUE"
status = "todo"
note = "ADR-0023 follow-up; DESIGN-0004 Condition handling"

[[feature]]
id = "trampolines"
title = "Code above 2 GB reached through linker trampolines"
status = "todo"
note = "DESIGN-0004 Procedure values: nothing is linked there yet"

[[needs]]
on = "/host/vmacro"
why = "Compiles MACRO-32 onto it, as AMACRO did onto Alpha's"
soft = true

[[needs]]
on = "/host/vbliss"
why = "BLISS-64's default linkage is the standard"
soft = true
+++

The OpenVMS Calling Standard is what lets any VMS language call any
other: where arguments and results go, which registers survive a call,
how a routine finds its argument count, how strings and arrays travel
(by descriptor), and what a stack frame looks like, so that condition
handlers can be found and frames unwound in any language. VAX had
`CALLS`, `CALLG` and the argument list in memory; Alpha a standard DEC
wrote to fit VAX code, with procedure descriptors and the count in
R25; Itanium and x86-64 took the platform's conventions and added
VMS's argument count, sign extension of 32-bit values and the frame
information in unwind tables. DEC's MACRO-32 compiler, AMACRO, put the
VAX executive's MACRO-32 onto each.

vaxpunk's is AAPCS64 with what 64-bit VMS adds: argument information
in x9, 32-bit values sign-extended, x15 for a bound procedure's
environment, x18 as the VAX stack pointer, and frames that describe
themselves through a descriptor address at `24(FP)` next to the handler
at `16(FP)` ([ADR-0023](../../adr/0023-calling-standard.md),
[DESIGN-0004](../../design/0004-calling-standard.md)). PRD-0005
put `vmacro`, the PAL, the change mode dispatchers, `sysunwind.mar` and
`vrun` on it, and BLISS-64 calls MACRO-32 through it. C from gcc goes
through jackets, as gcc doesn't set x9 or sign-extend. Left: a
conforming C compiler or unwind tables for other compilers, floating
point arguments, and the 64-bit address space.
