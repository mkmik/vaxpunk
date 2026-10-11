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
status = "todo"
note = "PRD-0003 tier 3 #23 and #24; directory.mar ponytail"

[[feature]]
id = "context"
title = "$SEARCH keeps its place in the directory, not by counting the matches it skips"
status = "todo"
note = "rms.mar ponytail: NAM$L_WCC counts them"

[[feature]]
id = "spilled-versions"
title = "A name whose versions fill several directory records found once"
status = "todo"
milestone = "m1"
needs = ["//files/ods2:spilled-versions"]
note = "PRD-0003 tier 2 #15"

[[feature]]
id = "streams"
title = "Several searches at once in one image, as F$SEARCH's context argument needs"
status = "todo"
note = "PRD-0003 tier 1 #6: F$SEARCH has one stream (dcl/lexical.mar)"

[[needs]]
on = "//rms/core"
why = "Parses into the FAB's NAM block, with its defaults and logical names"

[[needs]]
on = "//files/ods2"
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
types and versions with `f11.mar`'s `FIL$SEARCHDIR`, and step through
search lists as OpenVMS does; `DIRECTORY` and `DELETE` use them. There
are no wildcard directories, and `$SEARCH` finds its place again by
counting the matches it skips. `COPY` and `TYPE` take one file
(PRD-0003 tier 3 #23 and #24). WASMRUN's directory walks need wildcard
directories (PRD-0007 M6).
