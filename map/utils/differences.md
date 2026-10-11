+++
title = "DIFFERENCES"
summary = "Compares two text files record by record and lists what differs"
aka = ["DIFFERENCES", "DIFF", "DIFFERENCES.EXE", "DIFF/SLP", "DIFF/PARALLEL"]
kind = "core"
intent = "aspiration"
research = "outline"
status = "todo"
refs = ["https://docs.vmssoftware.com/vsi-openvms-dcl-dictionary-a-m/", "https://wasd.vsm.com.au/htbin/helpgate/HELP/DIFFERENCES"]

[[needs]]
on = "//rms/sequential"
why = "Reads both files' records with $GET"

[[needs]]
on = "//rtl/cli"
why = "Its parameters and many qualifiers"

[[needs]]
on = "//rtl/librtl"
why = "Writes the listing; LIB$ string routines"
+++

`DIFFERENCES file1 [file2]` compares two files a record at a time and
lists the sections that don't match. With one file it compares it with
the version before it, which suits VMS's numbered versions: `DIFF
LOGIN.COM` shows the last edit. Qualifiers set what counts as a match
(`/IGNORE` blanks, case, comments; `/MATCH` records to resync), how the
output looks (`/PARALLEL` side by side, `/MERGED`, `/CHANGE_BAR`,
`/MODE` in hex or octal), and `/SLP` writes an editing script that the
SLP editor applies, an old way to ship patches to sources.

vaxpunk has no `DIFFERENCES`. It is a plain RMS program; nothing in a
PRD asks for it.
