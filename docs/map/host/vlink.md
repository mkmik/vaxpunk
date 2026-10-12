+++
title = "vlink (linker)"
summary = "Links object modules into images and shareable images, with a VMS-style map"
aka = ["vlink", "LINK", "LINK/MAP", "/SHAREABLE", "SYMBOL_VECTOR", "linker.md", ".OPT"]
kind = "tool"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["docs/prd/0001-vtools.md", "crosstools/vtools/crates/vlink", "crosstools/vtools/docs/linker.md", "docs/adr/0028-shareable-images.md", "docs/prd/0003-multi-user-vms.md"]

[[feature]]
id = "basic"
title = "Psects collected, symbols resolved, ARM64 relocations range-checked, image and map"
status = "done"
milestone = "m0"
note = "PRD-0001 steps 6 and 7"

[[feature]]
id = "libraries"
title = "Object libraries searched for what is still undefined, as on VMS"
status = "done"
milestone = "m0"
note = "PRD-0001 step 9"

[[feature]]
id = "relocatable"
title = "/RELOCATABLE: a fixup section, and a link error for what can't move"
status = "done"
note = "linker.md Movable images"

[[feature]]
id = "shareable"
title = "/SHAREABLE images with a symbol vector, and veneers to call them"
status = "done"
note = "ADR-0028"

[[feature]]
id = "data-vector"
title = "DATA entries in the symbol vector, and the rest of the options file"
status = "todo"
note = "linker.md and lib.rs ponytail: SYMBOL_VECTOR only, PROCEDURE entries only; PRD-0003 tier 3 #20"

[[feature]]
id = "shareable-chains"
title = "A shareable image that calls another, and GSMATCH"
status = "todo"
note = "PRD-0003 tier 3 #20; ADR-0032 decision 8 (DECC$SHR)"

[[feature]]
id = "procedure-addresses"
title = "One address for a procedure in another image, not each caller's veneer"
status = "todo"
note = "linker.md ponytail"

[[feature]]
id = "library-index"
title = "Library search from an index, not a rescan per module taken"
status = "todo"
note = "PRD-0003 tier 4 #44; lib.rs ponytail"

[[feature]]
id = "debug-symbols"
title = "Debug symbol table records and a traceback table in the image, for a future DEBUG"
status = "todo"
needs = ["/host/vms-obj#debug-symbols"]
note = "PRD-0001 non-goal: the link map only, for now"

[[needs]]
on = "/host/vms-obj"
why = "Reads object modules and libraries, writes images"
+++

VMS's linker, `LINK`, turns object modules into an executable or
shareable image: it gathers program sections by name and attribute,
resolves symbols across modules, object libraries and shareable images,
and writes the image with its image section descriptors and a map
(`LINK/MAP`). Options files give a shareable image its symbol vector.

`vlink` is that linker on the host, and links every image on vaxpunk's
system disk, `EXEC.EXE` and `LIBRTL.EXE` included. Its command line is
DCL-flavoured with a Unix alternative, and its messages are
`%VLINK-E-...`. It makes relocatable and shareable images
([ADR-0028](../../adr/0028-shareable-images.md)). Still missing:
`DATA` entries in a symbol vector, which the C run-time library's
shareable image needs, shareable images calling each other, and debug
symbols. A native `LINK` is `t8y:/utils/linker`.
