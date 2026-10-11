+++
title = "MACRO-32 compiler"
summary = "VAX assembly language compiled to native code, as AMACRO did on Alpha"
aka = ["MACRO-32", "MACRO/MIGRATION", "AMACRO", "IMACRO", "XMACRO", "VAX MACRO", "MACRO.EXE", ".MAR"]
kind = "core"
intent = "goal"
research = "done"
refs = ["docs/prd/0005-macro32-on-the-calling-standard.md", "docs/adr/0023-calling-standard.md", "docs/design/0004-calling-standard.md", "crosstools/vtools/docs/macro32.md", "crosstools/vtools/docs/amacro.md", "crosstools/vtools/crates/vmacro", "docs/prd/0007-wasmrun.md"]

[[feature]]
id = "cross"
title = "Compiled to ARM64 on the host by vmacro: the executive, drivers, utilities"
status = "done"
milestone = "m0"
needs = ["//host/vmacro"]
note = "About 100 .mar files under vms/"

[[feature]]
id = "calling-standard"
title = "Routines declared and compiled onto the calling standard, as AMACRO did"
status = "done"
needs = ["//exec/calling-standard"]
note = "PRD-0005 steps 1-6, ADR-0023: .CALL_ENTRY, .JSB_ENTRY, 64-bit preservation, porting messages"

[[feature]]
id = "quadword"
title = "DEC's 64-bit pieces: EVAX_ built-ins, QUAD_ARGS, quadword mode, $CALL64"
status = "done"
note = "PRD-0005 step 7"

[[feature]]
id = "porting-idioms"
title = "AMACRO's other flagged idioms, the page macros' quad=YES, and $RAB64"
status = "todo"
note = "amacro.md: not yet"

[[feature]]
id = "floating-point"
title = "F, D, G and IEEE floating point instructions and built-ins"
status = "todo"
note = "macro32.md: not yet. PRD-0007 M0 and M3 need IEEE built-ins, after FP encodings in vasm"

[[feature]]
id = "decimal"
title = "Packed decimal instructions: ADDP, CVTLP, EDITPC and the rest"
status = "todo"
note = "macro32.md: not yet"

[[feature]]
id = "interlocked"
title = "Interlocked queue instructions, and the other string and privileged ones"
status = "todo"
note = "macro32.md: not yet; INSQUE and REMQUE aren't interlocked"

[[feature]]
id = "listing"
title = "/LIST: a listing with the machine code each line became"
status = "todo"
note = "macro32.md: not yet"

[[feature]]
id = "native"
title = "MACRO/MIGRATION at the DCL prompt: the compiler rewritten in BLISS"
status = "todo"
needs = ["//lang/bliss:native", "//rms/core"]
note = "Marko's stated goal; PRD-0004 leaves the vmacro port out for now"

[[needs]]
on = "//host/vmacro"
why = "The compiler today, in Rust, on the host"

[[needs]]
on = "//exec/calling-standard"
why = "Its routines follow DESIGN-0004"
+++

MACRO-32 is the VAX's assembly language. VAX/VMS was written mostly in
it, assembled by the VAX MACRO assembler that came with the system. For
Alpha, DEC wrote a compiler, AMACRO, run as `MACRO/MIGRATION`, that
treats MACRO-32 as a source language: each VAX instruction becomes a few
Alpha ones, VAX registers and condition codes are kept as the source
expects, and routines are declared (`.CALL_ENTRY`, `.JSB_ENTRY`) so the
compiler knows what they take and keep. The executive stayed MACRO-32
on Alpha, and again on Itanium (IMACRO) and x86-64 (XMACRO). The
compiler ships with the operating system.

MACRO-32 is vaxpunk's main kernel language. `vmacro` compiles it on the
host (`//host/vmacro`): the executive, the drivers and most utilities,
about 100 files under `vms/`.
[PRD-0005](../../docs/prd/0005-macro32-on-the-calling-standard.md) put
it on the calling standard
([ADR-0023](../../docs/adr/0023-calling-standard.md)) in AMACRO's
dialect: declarations, every register kept in full, the argument list at
`32(FP)`, and DEC's 64-bit pieces under DEC's names. Missing: floating
point, packed decimal, the interlocked queue instructions and listings.
The compiler's own bugs (PRD-0003 tier 2 #16 and #17, tier 4 #45)
belong to `//host/vmacro`. A MACRO-32 compiler written in BLISS that
runs on vaxpunk is a stated goal.
