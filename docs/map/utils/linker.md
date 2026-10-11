+++
title = "LINK"
summary = "The linker on VMS itself: objects and libraries into images, natively"
aka = ["LINK", "LINK.EXE", "linker", "/SHAREABLE", "options file", ".OPT", "SYMBOL_VECTOR", "GSMATCH", "link map", "/DEBUG", "/TRACEBACK"]
kind = "core"
intent = "aspiration"
research = "outline"
refs = ["docs/prd/0004-bliss64-compiler.md", "crosstools/vtools/docs/linker.md", "crosstools/vtools/docs/image-format.md", "docs/adr/0028-shareable-images.md"]

[[feature]]
id = "basic"
title = "LINK file[,...] on vaxpunk: an executable image with /EXECUTABLE and /MAP"
status = "todo"
note = "PRD-0004: vlink stays on the host until a PRD of its own ports it to BLISS-64"

[[feature]]
id = "libraries"
title = "Object libraries searched, /LIBRARY and /INCLUDE, and the default system libraries"
status = "todo"
needs = ["#basic", "/rtl/lbr"]

[[feature]]
id = "shareable"
title = "/SHAREABLE with an options file's SYMBOL_VECTOR; linking against LIBRTL.EXE"
status = "todo"
needs = ["#basic"]
note = "vlink does this on the host (ADR-0028)"

[[feature]]
id = "gsmatch"
title = "GSMATCH, DATA entries, and a shareable image that calls another"
status = "todo"
needs = ["#shareable"]
note = "PRD-0003 tier 3 #20"

[[feature]]
id = "debug"
title = "/DEBUG and /TRACEBACK: debug symbol records in the image"
status = "todo"
note = "PRD-0001 and PRD-0004 leave DST out: the link map is the debugging aid"

[[feature]]
id = "options"
title = "Options files: CLUSTER, COLLECT, PSECT_ATTR, IDENTIFICATION, STACK"
status = "todo"
needs = ["#basic"]

[[feature]]
id = "p2-images"
title = "64-bit images: sections in P2 space"
status = "todo"

[[needs]]
on = "/host/vlink"
why = "The Rust linker it would be ported from, and which checks it"

[[needs]]
on = "/lang/bliss"
why = "Written in BLISS-64 and built on vaxpunk, as Marko wants the toolchain"

[[needs]]
on = "/rms/core"
why = "Reads objects and writes the image"

[[needs]]
on = "/rtl/cli"
why = "LINK's qualifiers and the options file"
+++

On VMS, `LINK` turns the object modules compilers write into an
executable or shareable image: it collects program sections by name and
attribute, resolves symbols against object libraries and shareable
images, lays the image out in sections, and writes the image header the
image activator reads. Options files give a shareable image its symbol
vector and `GSMATCH`, which says when a new version can replace an old
one without relinking its callers; `/DEBUG` and `/TRACEBACK` put symbol
tables in the image for the debugger and for traceback on an error. A
map lists where everything went.

vaxpunk's linker is `vlink`, a Rust host tool (`t8y:/host/vlink`), which
links every image on the system disk, shareable images included. A
native `LINK.EXE` is an aspiration: PRD-0004 keeps `vlink` and `vlib` on
the host and says porting them to BLISS-64 "makes the toolchain whole on
vaxpunk", in a PRD of its own once the `vasm` port shows the cost. With
it and `BLISS.EXE`, a program could be compiled and linked on vaxpunk,
which Marko's self-hosting goal asks for.
