+++
title = "LBR$ librarian routines"
summary = "Read and write object, macro, text and help libraries from programs"
aka = ["LBR$", "LBRSHR", "LBR$OUTPUT_HELP", "HLB", "TLB", "MLB", "OLB"]
kind = "core"
intent = "goal"
research = "outline"
status = "done"
refs = ["vms/sysexe/lbrshr/lbr.mar", "vms/sysexe/lbrshr/outhelp.mar", "vms/sysexe/lbrtest.mar", "crosstools/vtools/docs/library-format.md", "docs/prd/0003-multi-user-vms.md"]

[[needs]]
on = "/rms/core"
why = "A library is a file of 512-byte blocks read through RMS"

[[needs]]
on = "/host/vlib"
why = "vlib writes the same library format on the host"

[[needs]]
on = "/exec/shareable-images"
why = "LBRSHR.EXE"
soft = true
+++

`LBR$` is the librarian as a library, `LBRSHR.EXE`: a program opens an
object, macro, text, help or shareable image symbol library, looks up
and reads modules by name, and inserts, replaces and deletes them.
`LBR$OUTPUT_HELP` is all of `HELP`: it finds a topic and its subtopics
in a help library and writes them, prompting for more. LINK, MACRO,
LIBRARY and HELP call it.

vaxpunk's `LBRSHR.EXE`, in `SYS$SHARE:`, is built from
`vms/sysexe/lbrshr/`: `LBR$INI_CONTROL`, `LBR$OPEN` (which creates a
library from the `CRE$` options), `LBR$CLOSE`, `LBR$GET_HEADER`,
`LBR$SET_INDEX`, `LBR$LOOKUP_KEY`, `LBR$GET_RECORD`, `LBR$PUT_RECORD`,
`LBR$PUT_END`, `LBR$INSERT_KEY`, `LBR$REPLACE_KEY`, `LBR$DELETE_KEY`,
`LBR$DELETE_DATA`, `LBR$GET_INDEX` (with `*` and `%` wildcards) and
`LBR$OUTPUT_HELP` (topics, subtopics, and the `Topic?` and `Subtopic?`
prompts). They return VMS's `LBR$_` codes, which `$GETMSG` knows. The
format is Alpha's, the one vlib and vlink use on the host
(`crosstools/vtools/docs/library-format.md`): the boot test reads
the object library LBRTEST writes with `vms-obj`.

`LBR$OPEN` reads every index into memory and `LBR$CLOSE` writes the
changed ones again at the end of the file. There are no free block
lists, cache or update history, so deleted data and old index blocks
stay in the file unused. `LBR$OUTPUT_HELP` reads one library: there is no
`HLP$LIBRARY` search list, `@library`, wildcard topics or `...`. `HELP`
still has no text; that needs `HELPLIB.HLB` (`/dcl/help#helplib`).
