+++
title = "SORT and MERGE"
summary = "Sorts records by keys into a new file, or merges sorted files"
aka = ["SORT", "MERGE", "SORT/MERGE", "SORTMERGE", "Hypersort", "SORTSHR", "SORT/SPECIFICATION"]
kind = "core"
intent = "aspiration"
research = "outline"
status = "todo"
refs = ["docs/prd/0008-rms-record-and-indexed-files.md", "vms/sysexe/convert.mar", "https://docs.vmssoftware.com/vsi-openvms-user-s-manual/", "https://wasd.vsm.com.au/htbin/helpgate/HELP/SORT"]

[[needs]]
on = "//rtl/sor"
why = "The commands are a front end on SOR$ routines"

[[needs]]
on = "//rms/core"
why = "Input and output of any organization, and its work files"

[[needs]]
on = "//rms/indexed"
why = "An indexed output, and /KEY on an indexed input"
soft = true

[[needs]]
on = "//rtl/cli"
why = "Its parameters, and per-file qualifiers"
+++

`SORT in out /KEY=(POSITION:n,SIZE:m,...)` reads records from one or
more files, orders them by one or more keys of many data types, and
writes them to a file of any organization. `/PROCESS=RECORD|TAG|ADDRESS|
INDEX` chooses what is moved, `/STABLE` keeps equal records in order,
`/WORK_FILES` sets how many scratch files it spills to, and a
specification file holds keys, conditions and collating sequences too
long for a command. `MERGE` combines files already sorted on the same
keys. Alpha has two engines behind the same commands, the original and
Hypersort, chosen by the logical name `SORTSHR`. COBOL's `SORT` verb
and `CONVERT` call the same `SOR$` routines.

vaxpunk has no `SORT`. PRD-0008 lists it as a non-goal and "another
PRD"; `CONVERT` orders an indexed file's records through an indexed work
file instead.
