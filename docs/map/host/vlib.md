+++
title = "vlib (librarian)"
summary = "Builds and lists object libraries, like LIBRARY/OBJECT"
aka = ["vlib", "LIBRARY", "LIBRARY/OBJECT", ".OLB"]
kind = "tool"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["docs/prd/0001-vtools.md", "crosstools/vtools/crates/vlib", "crosstools/vtools/docs/library-format.md"]

[[feature]]
id = "object"
title = "/CREATE, replace and /LIST [/NAMES] for object libraries"
status = "done"
milestone = "m0"
note = "PRD-0001 step 9; vms/c.rs builds DECC$CRTL.OLB and SSL3$LIBSSL.OLB with it"

[[feature]]
id = "help"
title = "Help libraries (.HLB) from .HLP text, for HELP"
status = "todo"
needs = ["/host/vms-obj#other-libraries"]
note = "t8y:/dcl/help wants HELPLIB.HLB built on the host"

[[feature]]
id = "macro-text"
title = "Macro (.MLB) and text (.TLB) libraries in VMS's format"
status = "todo"
needs = ["/host/vms-obj#other-libraries"]
note = "lib.mlb and starlet.mlb are plain text today"

[[needs]]
on = "/host/vms-obj"
why = "The library format"
+++

VMS's librarian, `LIBRARY`, keeps modules in indexed library files:
object libraries the linker searches, macro libraries MACRO reads with
`.LIBRARY`, help libraries `HELP` reads, and text libraries. `vlib` does
the object part on the host, with only what the linker needs: no update
history, no free block lists, no compression. The macro libraries in
`crosstools/vtools/lib` are text files that vasm reads directly. A
native `LIBRARY` is `t8y:/utils/librarian`, over the `LBR$` routines
(`t8y:/rtl/lbr`).
