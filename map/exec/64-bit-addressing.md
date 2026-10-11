+++
title = "64-bit addressing"
summary = "P2 and S2 space above 2 GB, the _64 services and 64-bit pool"
aka = ["P2", "S2", "_64 services", "$EXPREG_64", "$CRETVA_64", "$CREATE_REGION_64", "$DELETE_REGION_64", "64-bit pool", "VLM", "very large memory", "64-bit descriptors"]
kind = "core"
intent = "goal"
research = "outline"
refs = ["docs/adr/0023-calling-standard.md", "docs/adr/0005-access-modes-are-threads.md", "docs/prd/0004-bliss64-compiler.md", "docs/prd/0005-macro32-on-the-calling-standard.md", "docs/prd/0007-wasmrun.md", "docs/design/0001-pal-interface.md"]

[[feature]]
id = "calling-standard"
title = "64-bit arguments, descriptors and signal arrays in the calling standard"
status = "done"
milestone = "m0"
note = "ADR-0023; services check each argument is a sign-extended longword (SS$_ARG_GTR_32_BITS)"

[[feature]]
id = "p2-space"
title = "P2: a process's space above 2 GB, which the PAL maps"
status = "todo"
needs = ["//platform/pal"]
note = "ADR-0023 follow-up, an ADR of its own; DESIGN-0001: the PAL maps nothing at or above 2 GB"

[[feature]]
id = "regions"
title = "Regions and the _64 services: $CREATE_REGION_64, $EXPREG_64, $CRETVA_64, $DELTVA_64"
status = "todo"
needs = [":p2-space"]

[[feature]]
id = "services-64"
title = "64-bit buffer and IOSB addresses in $QIO and the other services that take them"
status = "todo"
needs = [":p2-space"]

[[feature]]
id = "s2-pool"
title = "S2 space and 64-bit pool for the executive"
status = "todo"
needs = [":p2-space"]

[[needs]]
on = "//exec/memory"
why = "P0, P1 and S0 stay; P2 and S2 are more of the same"

[[needs]]
on = "//exec/calling-standard"
why = "64-bit addresses passed as arguments"

[[needs]]
on = "//exec/system-services"
why = "The _64 services and their argument checks"

[[needs]]
on = "//platform/pal"
why = "Mappings above 2 GB"
+++

OpenVMS Alpha V7.0 added 64-bit virtual addressing beside the 32-bit
VAX-shaped space, which stayed as it was. P2 is a process's own space
above P1, S2 the system's above S0. Programs reach it through `_64`
system services (`$EXPREG_64`, `$CRETVA_64`, `$CRMPSC_GPFILE_64` and
the rest) on regions they create with `$CREATE_REGION_64`, and through
64-bit descriptors and item lists; existing 32-bit programs run
unchanged. Page tables moved to a space of their own at the same time,
and V7.1 added the very large memory (VLM) features for shared memory
of many gigabytes.

vaxpunk keeps everything below 2 GB, where MACRO-32's sign-extended
longwords reach ([ADR-0005](../../docs/adr/0005-access-modes-are-threads.md)).
Its calling standard already passes 64-bit values and has 64-bit VMS's
descriptors and signal arrays, and every service checks that its
arguments are 32-bit ([ADR-0023](../../docs/adr/0023-calling-standard.md)).
ADR-0023, PRD-0004 and PRD-0005 leave P2, S2, the `_64` services and
64-bit pool to an ADR of their own, and PRD-0007 wants WASMRUN's linear
memory in P2 later. Nothing is designed yet.
