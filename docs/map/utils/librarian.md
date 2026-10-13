+++
title = "LIBRARY"
summary = "The librarian: object, macro, text, help and shareable image libraries"
aka = ["LIBRARY", "LIBRARIAN", "LIBRAR.EXE", ".OLB", ".MLB", ".TLB", ".HLB", "HELPLIB.HLB", "STARLET.OLB", "IMAGELIB.OLB", "/DATA=REDUCE"]
kind = "core"
intent = "aspiration"
research = "outline"
refs = ["crosstools/vtools/docs/library-format.md", "docs/prd/0004-bliss64-compiler.md", "docs/prd/0003-multi-user-vms.md"]

[[feature]]
id = "object"
title = "LIBRARY/CREATE, /INSERT, /REPLACE, /LIST of object libraries on vaxpunk"
status = "todo"
needs = ["/rtl/lbr"]
note = "PRD-0004: vlib stays on the host until a PRD ports it"

[[feature]]
id = "help"
title = "Help libraries: LIBRARY/HELP, so HELPLIB.HLB has text"
status = "todo"
needs = ["#object"]
note = "PRD-0003 tier 3 #27; HELP's text can come from vlib on the host first"

[[feature]]
id = "macro-text"
title = "Macro and text libraries, /EXTRACT and /DELETE"
status = "todo"
needs = ["#object"]

[[feature]]
id = "shareable"
title = "Shareable image libraries, IMAGELIB.OLB, which LINK searches"
status = "todo"
needs = ["#object"]

[[feature]]
id = "compress"
title = "/COMPRESS and /DATA=REDUCE"
status = "todo"
needs = ["#macro-text", "/rtl/dcx"]

[[needs]]
on = "/rtl/lbr"
why = "LIBRARY is a front end on the LBR$ routines"

[[needs]]
on = "/host/vlib"
why = "The host librarian it would be ported from; same format"

[[needs]]
on = "/lang/bliss#native"
why = "Written in BLISS-64 and built on vaxpunk, with the rest of the native toolchain"

[[needs]]
on = "/rtl/cli"
why = "Its qualifiers"
+++

VMS keeps related modules in one indexed file, a library: object
modules for the linker (`STARLET.OLB`), macros for MACRO
(`STARLET.MLB`), help text for `HELP` (`HELPLIB.HLB`), text modules for
any program, and shareable image symbol tables (`IMAGELIB.OLB`). The
`LIBRARY` command creates them and inserts, replaces, extracts, lists and
deletes modules; it is a front end on the `LBR$` routines, which LINK,
the assemblers, `HELP` and TPU call too. `/DATA=REDUCE` compresses text
and help libraries.

vaxpunk's libraries are Alpha's format, written on the host by `vlib`
(object libraries only) and read by `vlink`
(`crosstools/vtools/docs/library-format.md`). On vaxpunk the `LBR$`
routines in `LBRSHR.EXE` read and write them, but no command does yet. A native `LIBRARY` is an aspiration with
`LINK`: PRD-0004 leaves both to a PRD of their own. A help library is
the nearer need: `HELP` has no text until there is one (PRD-0003 tier 3
#27), and EVE's and TPU's help wait on it too.
