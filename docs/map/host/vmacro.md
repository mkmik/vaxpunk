+++
title = "vmacro (MACRO-32 compiler)"
summary = "Compiles MACRO-32 to ARM64 on the host, as AMACRO compiled it for Alpha"
aka = ["vmacro", "AMACRO", "MACRO/MIGRATION", "MACRO-32 compiler", ".MAR", "macro32.md", "amacro.md"]
kind = "tool"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["crosstools/vtools/crates/vmacro", "crosstools/vtools/docs/macro32.md", "crosstools/vtools/docs/amacro.md", "docs/prd/0005-macro32-on-the-calling-standard.md", "docs/adr/0023-calling-standard.md", "docs/prd/0007-wasmrun.md", "docs/prd/0003-multi-user-vms.md"]

[[feature]]
id = "basic"
title = "VAX integer instructions, CASE, bit fields, strings, queues, privileged ones, CHMx"
status = "done"
milestone = "m0"
note = "macro32.md status"

[[feature]]
id = "calling-standard"
title = "AMACRO's declarations and preservation, onto the vaxpunk calling standard"
status = "done"
note = "PRD-0005 steps 1-6, ADR-0023"

[[feature]]
id = "quadword"
title = "EVAX_ built-ins, QUAD_ARGS, quadword mode, the $CALL64 macros"
status = "done"
note = "PRD-0005 step 7"

[[feature]]
id = "extzv-insv"
title = "EXTZV and INSV that don't touch 8 bytes around the field, atomic where shared"
status = "todo"
milestone = "m1"
note = "PRD-0003 tier 2 #16; insn.rs ponytail; tests/macro32-bugs/fields.mar"

[[feature]]
id = "shifts"
title = "Shift counts of 32 and up clear the result instead of wrapping"
status = "todo"
milestone = "m1"
note = "PRD-0003 tier 2 #17; insn.rs ponytail; tests/macro32-bugs/shifts.mar"

[[feature]]
id = "movc5"
title = "MOVC5 copies overlapping strings correctly, as MOVC3 does"
status = "todo"
note = "PRD-0003 tier 4 #45; insn.rs ponytail"

[[feature]]
id = "float"
title = "IEEE floating point: single and double built-ins on ARM64's FP instructions"
status = "todo"
needs = ["/host/vasm#fp"]
note = "PRD-0007 M0 and M3: add, sub, mul, div, sqrt, compare, convert, min, max, rounding"

[[feature]]
id = "vax-float"
title = "VAX F, D and G floating instructions"
status = "todo"
needs = ["#float"]
note = "macro32.md: not yet"

[[feature]]
id = "separate-names"
title = "Linkages solved per program, not in one name space for every program"
status = "todo"
note = "routine.rs ponytail"

[[feature]]
id = "listing"
title = "/LIST: source, addresses and the ARM64 each line became"
status = "todo"
needs = ["/host/vasm#listing"]
note = "macro32.md: not yet"

[[feature]]
id = "debug-symbols"
title = "Debug symbol table records, for a future DEBUG"
status = "todo"
needs = ["/host/vms-obj#debug-symbols"]
note = "PRD-0001 non-goal: the listing is the debugging aid"

[[needs]]
on = "/host/vasm"
why = "It is vasm with a dialect: its output is vasm's input"

[[needs]]
on = "/exec/calling-standard"
why = "Compiles routines onto it, as AMACRO did onto Alpha's"
soft = true
+++

On Alpha, Itanium and x86-64, DEC and its successors kept the VAX
executive in MACRO-32 by compiling it rather than assembling it: AMACRO
(`MACRO/MIGRATION`) turned VAX instructions into Alpha code, with
routine declarations, register preservation, and the `EVAX_` built-ins
for 64-bit work. `vmacro` does the same for ARM64, on the host.

It builds vaxpunk's executive, drivers and most utilities, about 100
`.mar` files under `vms/`. vasm reads the source, macros and directives;
vmacro turns each VAX instruction into ARM64 lines that vasm assembles.
Since [PRD-0005](../../prd/0005-macro32-on-the-calling-standard.md)
it compiles onto the calling standard of
[ADR-0023](../../adr/0023-calling-standard.md). Missing: floating
point (a blocker for WASMRUN's floats), packed decimal, the interlocked
queue instructions and listings, which `t8y:/lang/macro32` tracks as the
language; and three bugs in PRD-0003's backlog: `EXTZV` and `INSV` (#16),
long shifts (#17) and overlapping `MOVC5` (#45). The Rust compiler is a
bootstrap: a MACRO-32 compiler written in BLISS is the stated goal
(`t8y:/lang/macro32`).
