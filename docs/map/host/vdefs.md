+++
title = "vdefs (BLISS require files)"
summary = "Writes STARLET.R64 and LIB.R64 from the macro libraries"
aka = ["vdefs", "STARLET.R64", "LIB.R64", "STARLET.REQ", "LIB.REQ", "services.txt", "just defs"]
kind = "tool"
intent = "goal"
research = "done"
refs = ["docs/prd/0004-bliss64-compiler.md", "crosstools/vtools/crates/vdefs", "crosstools/vtools/lib"]

[[feature]]
id = "require-files"
title = "lib.r64, starlet.r64, lib.req and starlet.req from lib.mlb and starlet.mlb"
status = "done"
note = "PRD-0004 step 7, PR #102"

[[feature]]
id = "services"
title = "services.txt: each system service's argument mechanisms, for the dot lint"
status = "done"
note = "PRD-0004 step 10"

[[feature]]
id = "stale-check"
title = "Its test fails when the files are stale"
status = "done"
+++

On VMS, definitions such as `$PCBDEF` or `SS$_NORMAL` come from one
source per structure in SDL (`t8y:/lang/sdl`), which writes a macro library
for MACRO, a require file for BLISS and headers for the other languages.
vaxpunk's single source is its macro libraries, so `vdefs` reads
`lib.mlb` and `starlet.mlb` and writes the BLISS require files from
them. A BLISS-64 program then reads a field of `$PCBDEF` at the offset
MACRO-32 does. `just defs` regenerates them, and the test catches a
library edited without them.
