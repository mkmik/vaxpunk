+++
title = "LBR$ librarian routines"
summary = "Read and write object, macro, text and help libraries from programs"
aka = ["LBR$", "LBRSHR", "LBR$OUTPUT_HELP", "HLB", "TLB", "MLB", "OLB"]
kind = "core"
intent = "goal"
research = "outline"
status = "todo"
refs = ["crosstools/vtools/docs/library-format.md", "docs/prd/0003-multi-user-vms.md"]

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

vaxpunk's library format is Alpha's, written and read on the host by
vlib and vlink (`crosstools/vtools/docs/library-format.md`). On the
running system nothing reads libraries yet; `HELP` with text needs a
help library, `HELPLIB.HLB` (PRD-0003 tier 3 #27), and so this.
