+++
title = "SOR$ callable sort"
summary = "The sort and merge routines behind SORT and MERGE, callable from programs"
aka = ["SOR$", "SORTSHR", "SOR$BEGIN_SORT", "SOR$RELEASE_REC", "Hypersort"]
kind = "core"
intent = "aspiration"
research = "outline"
status = "todo"

[[needs]]
on = "/rms/core"
why = "Reads and writes files, and its work files, through RMS"

[[needs]]
on = "/exec/calling-standard"
why = "Called from any language by the standard"
+++

`SOR$` is VMS's sort as a library, `SORTSHR.EXE`: a program passes
files or records one at a time (`SOR$BEGIN_SORT`, `SOR$RELEASE_REC`,
`SOR$SORT_MERGE`, `SOR$RETURN_REC`), with keys and options, and gets
them back in order, using work files on disk when they don't fit in
memory. The SORT and MERGE commands, and COBOL's `SORT` verb, call it.
Alpha has two engines, the original and Hypersort.

vaxpunk has none; CONVERT orders an indexed file's records through an
indexed work file instead.
