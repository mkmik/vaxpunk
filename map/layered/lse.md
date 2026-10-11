+++
title = "LSE"
summary = "DECset's Language-Sensitive Editor: templates and compiler review, on TPU"
aka = ["LSE", "LSEDIT", "Language-Sensitive Editor", "DECset", "placeholders", "templates", "REVIEW", "/DIAGNOSTICS"]
kind = "layered"
intent = "aspiration"
research = "outline"
status = "todo"
refs = ["https://docs.vmssoftware.com/vsi-decset-for-openvms-language-sensitive-editor-command-line-interface-and-callable-routines-reference-manual/", "https://docs.vmssoftware.com/vsi-decset-for-openvms-language-sensitive-editor-source-code-analyzer-reference-manual/", "docs/prd/0006-tpu-and-eve.md"]

[[needs]]
on = "//rtl/tpushr"
why = "LSE is a TPU section on callable TPU"

[[needs]]
on = "//utils/tpu"
why = "Its editing and its language definitions are TPU procedures"

[[needs]]
on = "//dcl/spawn"
why = "COMPILE and REVIEW run the compiler from inside the editor"

[[needs]]
fuzzy = "Compilers that write diagnostics files (/DIAGNOSTICS) for REVIEW"
why = "Stepping through compile errors next to the source"

[[needs]]
on = "//layered/sca"
why = "GOTO DECLARATION and FIND OCCURRENCES ask SCA"
soft = true

[[needs]]
on = "//layered/cms"
why = "RESERVE and REPLACE from the editor"
soft = true

[[needs]]
on = "//gui/motif"
why = "Its DECwindows interface"
soft = true
+++

LSE, the Language-Sensitive Editor, is DECset's programming editor,
built on TPU. For each language it knows templates: typing a construct
expands into its keywords with placeholders such as `{expression}` to
fill in or delete, and tokens and aliases expand too. `COMPILE` runs the
compiler from the editor and `REVIEW` steps through the errors next to
the source, from the diagnostics files the DEC compilers write with
`/DIAGNOSTICS`. With SCA it jumps to a symbol's declaration, and with
CMS it reserves and replaces files. It has a DECwindows interface.

On vaxpunk it is an aspiration. TPU and EVE are a goal
([PRD-0006](../../docs/prd/0006-tpu-and-eve.md)), which lists LSE as a
non-goal of that work; LSE would also need callable TPU, which waits for
a shareable image that may call another one.
