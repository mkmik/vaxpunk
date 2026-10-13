+++
title = "Wildcards and searching"
summary = "$PARSE and $SEARCH: *, % and [...] in specifications, over search lists"
aka = ["$PARSE", "$SEARCH", "wildcard", "[...]", "[*]", "NAM$L_WCC", "RMS$_NMF", "F$SEARCH", "search list"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["docs/adr/0007-system-disk-files-11-and-rms.md", "docs/prd/0003-multi-user-vms.md", "docs/design/0002-executive-processes.md", "vms/exec/rms.mar", "vms/exec/f11.mar", "vms/sysexe/directory.mar", "vms/sysexe/delete.mar"]

[[feature]]
id = "names"
title = "* and % in the name and type, ;* and ;-n versions; $SEARCH file by file"
status = "done"
milestone = "m0"
refs = ["docs/adr/0007-system-disk-files-11-and-rms.md"]

[[feature]]
id = "search-lists"
title = "$SEARCH through each string of a search list, depth first"
status = "done"
note = "PRs #119, #121; DESIGN-0002 Files"

[[feature]]
id = "directories"
title = "Wildcard directories: [*], [A.*], [A...] and [...]"
status = "done"
note = "Depth first, a directory's files before those of the directories in it; boot test's wildcards phase"

[[feature]]
id = "context"
title = "$SEARCH keeps its place in the directory, not by counting the matches it skips"
status = "done"
note = "It goes on after the resultant string's file, as VMS's XQP does with FIB$L_WCC"

[[feature]]
id = "spilled-versions"
title = "A name whose versions fill several directory records found once"
status = "todo"
milestone = "m1"
needs = ["/files/ods2#spilled-versions"]
note = "PRD-0003 tier 2 #15: FIL$SEARCHDIR goes on by name, so it should; untested until f11wrt.mar can make one"

[[feature]]
id = "streams"
title = "Several searches at once in one image, as F$SEARCH's context argument needs"
status = "done"
note = "A search's place is all in its NAM block; F$SEARCH's streams in dcl/lexical.mar"

[[needs]]
on = "/rms/core"
why = "Parses into the FAB's NAM block, with its defaults and logical names"

[[needs]]
on = "/files/ods2"
why = "FIL$SEARCHDIR walks a directory's records for matches"
+++

`$PARSE` expands a file specification, with its defaults, into a NAM
block and says whether it has wildcards; `$SEARCH` then returns the files
it names, one per call, until `RMS$_NMF`. In the name and type `*`
matches any characters and `%` one; `;*` asks for every version. In the
directory `[*]` matches one level, `[A...]` the tree under `A`, and
`[...]` every directory on the volume. A device that is a search list is
searched string by string. `DIRECTORY`, `DELETE`, `COPY`, `PURGE`, DCL's
`F$SEARCH` and every program that takes wildcards go through these two
services.

vaxpunk's `$PARSE` and `$SEARCH` in `rms.mar` match `*` and `%` in names,
types and versions with `f11.mar`'s `FIL$SEARCHDIR`, walk wildcard
directories depth first, each directory's own files before those of the
directories in it, and step through search lists as OpenVMS does;
`DIRECTORY`, `DELETE` and `F$SEARCH` use them. `$SEARCH` goes on after
the file in the NAM block's resultant string, by name, so a file `DELETE`
takes away doesn't make it skip one; a wildcard search therefore needs a
resultant string area, where VMS's RMS keeps a copy of its own. `COPY`
and `TYPE` take one file (PRD-0003 tier 3 #24).
