+++
title = "vbliss (BLISS-64 bootstrap compiler)"
summary = "Stage 0 of the BLISS-64 compiler, in Rust, throwaway once BLISS compiles itself"
aka = ["vbliss", "stage 0", "BLISS-64", "BLISS-32", "/A64", "/A32", "vbliss.md", "bliss64.md"]
kind = "tool"
intent = "goal"
research = "done"
refs = ["docs/prd/0004-bliss64-compiler.md", "crosstools/vtools/crates/vbliss", "crosstools/vtools/docs/vbliss.md", "crosstools/vtools/docs/bliss64.md", "crosstools/ods/vms/bliss-oracle.py"]

[[feature]]
id = "core"
title = "Routines, data, expressions and control; a small IR and an ARM64 back end"
status = "done"
note = "PRD-0004 steps 3 and 4, PR #97"

[[feature]]
id = "macros"
title = "Macros, lexical functions, REQUIRE, the listing with expansions"
status = "done"
note = "PRD-0004 step 5"

[[feature]]
id = "structures"
title = "STRUCTURE, BLOCK, VECTOR, FIELD, BIND, PLIT"
status = "done"
note = "PRD-0004 step 6"

[[feature]]
id = "linkages"
title = "LINKAGE, BUILTIN, ENABLE, SIGNAL"
status = "done"
note = "PRD-0004 step 8"

[[feature]]
id = "bliss32"
title = "/A32 and the _LONG switches"
status = "done"
note = "PRD-0004 step 9"

[[feature]]
id = "dot-lint"
title = "The dot lint, rules 1 to 6"
status = "done"
note = "PRD-0004 step 10; rules 5 and 6 with t8y:/lang/bliss#lint-5-6"

[[feature]]
id = "missing"
title = "GLOBAL BIND, GLOBAL registers, default structure references, PAL and atomic built-ins"
status = "done"
note = "t8y:/lang/bliss#missing-constructs"

[[feature]]
id = "psects"
title = "PSECT attributes honoured, PLITs in the psect asked for, directories in REQUIRE"
status = "done"
note = "and EXTERNAL_NAME; crates/vbliss/tests/asm.rs"

[[feature]]
id = "debug-symbols"
title = "Debug symbol table records, for a future DEBUG"
status = "todo"
needs = ["/host/vms-obj#debug-symbols"]
note = "PRD-0004 non-goal: the listing is the debugging aid"

[[feature]]
id = "frozen"
title = "Frozen once stage 1 replaces it: bug fixes only, the bootstrap's first stage"
status = "todo"
note = "PRD-0004 step 15; t8y:/lang/bliss stage1 and self-hosting"

[[needs]]
on = "/host/vasm"
why = "Writes vasm assembly, which vasm assembles"

[[needs]]
on = "/host/vdefs"
why = "STARLET.R64 and LIB.R64, and services.txt for the dot lint"

[[needs]]
on = "/exec/calling-standard"
why = "BLISS-64's default linkage is the standard"
soft = true

[[needs]]
on = "/host/axpbox-oracle"
why = "BLISSA64's listings and behaviour are the expected outputs"
soft = true
+++

`vbliss` is the BLISS-64 compiler's stage 0, written in Rust: the
compiler that compiles the real one ([PRD-0004](../../prd/0004-bliss64-compiler.md)).
It accepts the dialect DEC's BLISS-64EN accepted on OpenVMS Alpha, and
BLISS-32 with `/A32`, and writes vasm assembly through a small IR, with
no optimizer and no LLVM. DEC's BLISSA64 in AXPbox is the oracle for
listings and behaviour.

It is the system's BLISS-64 compiler today: `vms/build.rs` compiles
`$NUMTIM`, TPU and EVE, the DNS resolver and other programs with it, and
fails on any warning. It is throwaway by design. Stage 1, the compiler
written in BLISS-64, and the self-hosting stages belong to
`t8y:/lang/bliss`; once they pass, stage 0 is frozen and only starts the
bootstrap.
