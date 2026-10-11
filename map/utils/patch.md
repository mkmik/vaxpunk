+++
title = "PATCH"
summary = "Changes an executable image in place, by address or by symbol"
aka = ["PATCH", "PATCH/ABSOLUTE", "PATCH.EXE", "image patch", "ECO"]
kind = "core"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["crosstools/vtools/docs/image-format.md", "https://wasd.vsm.com.au/htbin/helpgate/HELP/PATCH"]

[[needs]]
on = "//rms/core"
why = "Reads and writes the image file's blocks"

[[needs]]
on = "//utils/analyze-image"
why = "Shares its knowledge of image headers and sections"
soft = true

[[needs]]
fuzzy = "vaxpunk's image format, from crosstools/vtools/docs/image-format.md"
why = "Mapping virtual addresses to blocks of the image"
+++

`PATCH` changes an executable or shareable image on disk without
relinking it. On VAX it understood symbols from the image's debug
symbol table and could assemble instructions, and DEC shipped fixes as
patch command files; it wrote a new version of the image and recorded
each patch in its header, which `ANALYZE/IMAGE` shows. On Alpha and
Integrity only `PATCH/ABSOLUTE` remains: it examines and deposits data
at virtual addresses or offsets, mapping them to the image's sections,
with `/JOURNAL` keeping a record.

vaxpunk has nothing like it; images are rebuilt from source on the
host. Its image format is Alpha's, documented in
`crosstools/vtools/docs/image-format.md`, so `PATCH/ABSOLUTE` would
know where sections lie.
