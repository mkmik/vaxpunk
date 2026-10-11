+++
title = "vms-obj (object, library and image formats)"
summary = "OpenVMS Alpha's object language, OLB and EXE formats, transliterated to ARM64"
aka = ["vms-obj", "EOBJ", "EGSD", "ETIR", "EEOM", "EIHD", "EISD", "EIAF", "OLB", "LHD", "object-format.md", "image-format.md", "library-format.md"]
kind = "tool"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["docs/prd/0001-vtools.md", "crosstools/vtools/crates/vms-obj", "crosstools/vtools/docs/object-format.md", "crosstools/vtools/docs/library-format.md", "crosstools/vtools/docs/image-format.md", "docs/adr/0028-shareable-images.md"]

[[feature]]
id = "obj"
title = "Object modules: header, GSD, TIR with ARM64 relocation stores, EOM"
status = "done"
milestone = "m0"
note = "PRD-0001 steps 1 and 4"

[[feature]]
id = "exe"
title = "Executable images: header, image section descriptors, 64 KB aligned sections"
status = "done"
milestone = "m0"
note = "PRD-0001 step 2"

[[feature]]
id = "olb"
title = "Object libraries: header, global symbol index, modules"
status = "done"
milestone = "m0"
note = "PRD-0001 step 9; no update history, free block lists or compression"

[[feature]]
id = "fixups"
title = "Fixup sections, so an image linked /RELOCATABLE can be moved"
status = "done"
note = "image-format.md"

[[feature]]
id = "shareable"
title = "Shareable images: symbol vector, global symbol table, the images an image calls"
status = "done"
note = "ADR-0028"

[[feature]]
id = "other-libraries"
title = "Macro, help and text libraries"
status = "todo"
note = "library-format.md: only object libraries"

[[feature]]
id = "debug-symbols"
title = "Debug symbol table (DST) records and the image's debug section"
status = "todo"
note = "PRD-0001 non-goal and open question; image-format.md keeps the Alpha slots unused"
+++

Not a VMS piece but the definition of VMS ones: the object module, object
library and executable image formats every vaxpunk tool reads and writes.
The rule is "transliterate, don't invent": record framing, record types,
symbol flags and psect attributes are OpenVMS Alpha's, and only what ARM64
forces changes, chiefly new TIR store commands for ARM64 instruction
relocations (`B`/`BL`, `ADRP`, low-12 immediates, `MOVZ`/`MOVK`). Each
change is listed beside the Alpha original in `object-format.md`, which is
the only place the ARM64 VMS object format is defined.

The crate is `no_std` with `alloc`, and CI builds it for
`aarch64-unknown-none` so a loader could use it. Parsing and writing come
from one definition per record, with round-trip tests. Debug symbol
tables, and libraries other than object libraries, are missing.
