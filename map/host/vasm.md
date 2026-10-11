+++
title = "vasm (ARM64 assembler)"
summary = "ARM64 assembler with VMS-style directives and macros, writing object modules"
aka = ["vasm", "VASM", "assembler.md", ".M64", "MACRO-64-style directives"]
kind = "tool"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["docs/prd/0001-vtools.md", "crosstools/vtools/crates/vasm", "crosstools/vtools/docs/assembler.md", "crosstools/vtools/bliss/vasm", "docs/prd/0007-wasmrun.md"]

[[feature]]
id = "instructions"
title = "Standard ARM64 integer instructions, each form checked bit for bit against GNU as"
status = "done"
milestone = "m0"
note = "PRD-0001 steps 5 and 7; loads and stores of B/H/S/D/Q registers too"

[[feature]]
id = "directives"
title = ".PSECT, .GLOBAL, data directives, .ASCID, .TITLE, .IDENT, .END"
status = "done"
milestone = "m0"
note = "PRD-0001 step 5"

[[feature]]
id = "macros"
title = ".MACRO, .IF, .IRP, .REPEAT, local labels, .LIBRARY macro libraries"
status = "done"
milestone = "m0"
note = "PRD-0001 step 8"

[[feature]]
id = "fp"
title = "Floating point and SIMD arithmetic encodings"
status = "todo"
note = "assembler.md: not yet; PRD-0007 feasibility blocker, before IEEE built-ins in vmacro"

[[feature]]
id = "literal-pools"
title = "Literal pools: ldr x0, =value"
status = "todo"
note = "assembler.md: not yet"

[[feature]]
id = "procedure-descriptors"
title = "Procedure descriptor directives, as MACRO-64's $ROUTINE"
status = "todo"
note = "PRD-0001 step 11; assembler.md: not yet. call.mlb's macros build standard frames meanwhile"

[[feature]]
id = "listing"
title = "/LIST: source, addresses, bytes and macro expansions"
status = "todo"
note = "assembler.md: not yet"

[[needs]]
on = "//host/vms-obj"
why = "Writes object modules in its format"
+++

OpenVMS Alpha had MACRO-64, DEC's native Alpha assembler; vasm is its
counterpart for ARM64. Mnemonics and operands are standard ARM64, as GNU
`as` and LLVM accept them; directives, program sections, macros and
symbol rules are MACRO-64's and MACRO-32's. Symbols are case-insensitive
and folded to upper case.

vasm is the back end of the other compilers: vmacro is vasm with a
MACRO-32 dialect, and vbliss and vcdu emit vasm source. It encodes from
one template per instruction class, checked against GNU `as`, since no
Rust crate encodes ARM64 correctly without LLVM. It has been ported to
BLISS-64 as VASM (`crosstools/vtools/bliss/vasm`, PRD-0004 step 13),
which writes the same objects. Missing: FP and SIMD arithmetic, which
WASMRUN's floats wait for, literal pools, procedure descriptor
directives and listings.
