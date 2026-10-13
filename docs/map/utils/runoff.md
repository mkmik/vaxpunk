+++
title = "RUNOFF"
summary = "DIGITAL Standard Runoff: formats .RNO text files into documents"
aka = ["RUNOFF", "DSR", "DIGITAL Standard Runoff", ".RNO", ".MEM", ".RNH", "RUNOFF/CONTENTS", "RUNOFF/INDEX"]
kind = "core"
intent = "aspiration"
research = "deferred"
refs = ["https://typewritten.org/Manual/DEC/VMS/5.2/helplib.hlb/RUNOFF.html"]

[[feature]]
id = "format"
title = "RUNOFF file.RNO: fill, justify, pages, headers, lists, notes into file.MEM"
status = "todo"
needs = ["/rms/sequential"]

[[feature]]
id = "toc-index"
title = "RUNOFF/CONTENTS and RUNOFF/INDEX for a table of contents and an index"
status = "todo"
needs = ["#format"]

[[feature]]
id = "help-sources"
title = "Help text kept as RUNOFF source and turned into help library modules"
status = "todo"
needs = ["#format", "/utils/librarian#help"]
note = "With t8y:/utils/librarian's help libraries"

[[needs]]
on = "/rms/sequential"
why = "Reads the source and writes the output file"
+++

DIGITAL Standard Runoff is the text formatter that came with VMS. A
`.RNO` file is text with commands on lines that start with a period
(`.PARAGRAPH`, `.CHAPTER`, `.LIST`, `.NOTE`, `.PAGE`, `.FILL`); `RUNOFF`
fills and justifies it into pages for a line printer or terminal, in a
`.MEM` file, and `RUNOFF/CONTENTS` and `/INDEX` make a table of
contents and an index. DEC and sites wrote release notes, internal
memos and some help text with it, before VAX DOCUMENT
(`t8y:/layered/vax-document`) took over typeset manuals.

vaxpunk has no RUNOFF and nothing plans one.
