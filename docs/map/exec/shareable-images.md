+++
title = "Shareable images"
summary = "Libraries linked against a symbol vector and mapped at activation"
aka = ["shareable image", "symbol vector", "GSMATCH", "SYS$SHARE", "LIBRTL.EXE", "SHRACT", "veneer", "transfer vector", "LIB$FIND_IMAGE_SYMBOL", "IMAGELIB.OLB"]
kind = "core"
intent = "goal"
research = "done"
refs = ["docs/adr/0028-shareable-images.md", "crosstools/vtools/docs/image-format.md", "crosstools/vtools/docs/linker.md", "vms/exec/process.mar", "vms/sysexe/librtl/librtl.opt", "docs/prd/0003-multi-user-vms.md"]

[[feature]]
id = "basic"
title = "Alpha's format: a symbol vector, linked against, called through veneers"
status = "done"
note = "ADR-0028, PRD-0003 tier 3 #20"

[[feature]]
id = "private-copy"
title = "Found in SYS$SHARE:, mapped a copy in each process past P0, moved and fixed up"
status = "done"
note = "ADR-0028, as VMS maps one that isn't installed"

[[feature]]
id = "data-entries"
title = "DATA entries in the symbol vector"
status = "done"
note = "ADR-0028 follow-up; t8y:/rtl/librtl#data-vector, which the C RTL's shareable image waits on"

[[feature]]
id = "chained"
title = "A shareable image that calls another"
status = "todo"
note = "ADR-0028; DECC$SHR calls LIBRTL"

[[feature]]
id = "gsmatch"
title = "GSMATCH and the image ident checked at activation"
status = "todo"
note = "ADR-0028 consequences and follow-ups"

[[feature]]
id = "logical-name"
title = "The shareable image's name translated as a logical name first"
status = "todo"
note = "ADR-0028 follow-up; f11.mar ponytail"

[[feature]]
id = "from-p1"
title = "Called from an image in P1: DCL calling LIBRTL.EXE"
status = "todo"
note = "ADR-0028: DCL links LIBRTL's modules in"

[[feature]]
id = "run-time"
title = "Activated at run time, for LIB$FIND_IMAGE_SYMBOL"
status = "todo"
note = "ADR-0028 follow-up (t8y:/rtl/librtl#find-image-symbol)"

[[feature]]
id = "shared-pages"
title = "Its pages shared by every process that maps it, once installed /SHARED"
status = "todo"
needs = ["/exec/installed-images#shared"]
note = "PRD-0003 tier 3 #20; a non-goal of Milestone 1"

[[needs]]
on = "/exec/image-activator"
why = "SHRACT and SHRLIST are part of it"

[[needs]]
on = "/host/vlink"
why = "vlink makes them with /SHAREABLE, and the veneers that call them"

[[needs]]
on = "/exec/logical-names"
why = "SYS$SHARE: says where they are"
+++

A VMS shareable image is a library linked into an image of its own,
which programs link against and the image activator maps when they run.
Programs reach it only through its symbol vector, so a new version that
keeps its vector works with programs linked against the old one, and
`GSMATCH` says which versions are compatible. Installed `/SHARED`, its
read-only pages are one copy for every process. Most of VMS's run-time
libraries are shareable images (`LIBRTL`, `DECC$SHR`, `SMGSHR`,
`DECW$XLIBSHR`), and `LIB$FIND_IMAGE_SYMBOL` activates one at run time.

vaxpunk has them in OpenVMS Alpha's format
([ADR-0028](../../adr/0028-shareable-images.md)): vlink makes one
with `/SHAREABLE` and a `SYMBOL_VECTOR` option, and an image calls its
procedures through veneers that jump through a quadword the activator
sets, and holds the addresses of its data in quadwords the activator sets
too. The activator finds each one in `SYS$SHARE:`, maps a copy past the
end of P0, moves it with its own fixups and fills in the image's slots,
as VMS does for a shareable image that isn't installed. `LIBRTL.EXE` is
the first. Missing: one shareable image calling another,
`GSMATCH`, and shared pages, which wait for global sections and
`INSTALL` (PRD-0003 tier 3 #20).
