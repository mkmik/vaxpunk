+++
title = "ANALYZE/IMAGE and /OBJECT"
summary = "Decoded dumps of image headers and object modules, on VMS itself"
aka = ["ANALYZE/IMAGE", "ANALYZE/OBJECT", "ANALIMDMP", "ANALYZOBJ", "image header", "GSD", "TIR", "vdump"]
kind = "core"
intent = "aspiration"
research = "outline"
refs = ["crosstools/vtools/crates/vdump", "crosstools/vtools/docs/object-format.md", "crosstools/vtools/docs/image-format.md", "crosstools/vtools/docs/library-format.md", "docs/prd/0001-vtools.md", "vms/cld/analyze.cld"]

[[feature]]
id = "image"
title = "ANALYZE/IMAGE on vaxpunk: header, sections, fixups, shareable images it calls"
status = "todo"
note = "vdump does it on the host (PRD-0001); analyze.cld's ponytail lists /IMAGE and /OBJECT as missing"

[[feature]]
id = "object"
title = "ANALYZE/OBJECT: every record of an object module or library, checked"
status = "todo"

[[feature]]
id = "select"
title = "ANALYZE/IMAGE/SELECT for procedures: one field, such as the image's ident or link date"
status = "todo"
needs = ["#image"]

[[needs]]
on = "/host/vdump"
why = "The host tool whose decoding it would share"

[[needs]]
on = "/rms/core"
why = "Reads the file's blocks"

[[needs]]
on = "/dcl/cld"
why = "A keyword of the ANALYZE verb, next to /RMS_FILE"
+++

`ANALYZE/IMAGE` prints an executable or shareable image's header: its
image sections and their attributes, the shareable images it calls,
its fixups, transfer address, ident, link date and patch history, and
checks that they are consistent. `ANALYZE/OBJECT` does the same for an
object module or object library, record by record, and reports what
the linker would reject. System managers use `/IMAGE` to see which
version of a library an image was linked against; `/SELECT` gives one
field for a procedure to test.

vaxpunk does this on the host: `vdump` decodes vaxpunk's object
modules, libraries and images, with the ARM64 code disassembled
(PRD-0001). On the system, `ANALYZE` has only `/RMS_FILE`, and its CLD
file lists `/IMAGE` and `/OBJECT` among what isn't there.
