+++
title = "vdump"
summary = "Decoded dump of objects, libraries and images, like ANALYZE/OBJECT and /IMAGE"
aka = ["vdump", "ANALYZE/OBJECT", "ANALYZE/IMAGE"]
kind = "tool"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["docs/prd/0001-vtools.md", "crosstools/vtools/crates/vdump"]

[[feature]]
id = "basic"
title = "Every record of an OBJ, OLB or EXE decoded, code disassembled"
status = "done"
milestone = "m0"
note = "PRD-0001 step 4; yaxpeax-arm decodes"

[[feature]]
id = "map"
title = "--map names an image's psects; --weights shows what each store needs to move"
status = "done"

[[feature]]
id = "debug-symbols"
title = "Debug symbol records decoded"
status = "todo"
needs = ["//host/vms-obj:debug-symbols"]

[[needs]]
on = "//host/vms-obj"
why = "Parses the formats it prints"
+++

On VMS, `ANALYZE/OBJECT` and `ANALYZE/IMAGE` print an object module's or
an image's records, checked and decoded. `vdump` does that on the host
for vaxpunk's formats, with the ARM64 in text sections disassembled. Its
output for reference programs is checked in, so format changes show in
review. A native `ANALYZE/IMAGE` is `//utils/analyze-image`.
