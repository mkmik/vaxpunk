+++
title = "DIRECTORY"
summary = "Lists files: names, sizes, dates, owners and protection"
aka = ["DIR", "DIRECTORY.EXE", "DIRECTORY/FULL", "DIRECTORY/OWNER", "DIRECTORY/PROTECTION"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["vms/sysexe/directory.mar", "vms/cld/directory.cld", "docs/adr/0007-system-disk-files-11-and-rms.md", "docs/prd/0003-multi-user-vms.md", "docs/prd/0008-rms-record-and-indexed-files.md", "https://docs.vmssoftware.com/vsi-openvms-dcl-dictionary-a-m/"]

[[feature]]
id = "basic"
title = "A directory's files, four names to a line, and a total"
status = "done"
milestone = "m0"
note = "ADR-0007 decision 6: DIRECTORY uses RMS from user mode"

[[feature]]
id = "wildcards"
title = "* and % in names, types and versions, through $PARSE and $SEARCH"
status = "done"
milestone = "m0"

[[feature]]
id = "search-lists"
title = "Search lists: a heading and a total per directory, then a grand total"
status = "done"
note = "PR #119"

[[feature]]
id = "owner-protection"
title = "/OWNER and /PROTECTION, from a protection XAB"
status = "done"
milestone = "m1"
note = "PRD-0003 step 2"

[[feature]]
id = "full"
title = "/FULL: organization, record attributes, keys, dates and blocks, from the XABs"
status = "done"
note = "PRD-0008 step 8; matches the oracle's report"

[[feature]]
id = "wildcard-dirs"
title = "Wildcard directories, [...] and [*], and lists of files"
status = "todo"
needs = ["/rms/wildcards#directories"]
note = "PRD-0003 tier 3 #23 and #24; directory.mar ponytail"

[[feature]]
id = "xqp-attributes"
title = "Owner and protection of files the user may not read, from the XQP"
status = "todo"
needs = ["/files/ods2"]
note = "PRD-0003 tier 3 #23; directory.mar ponytail"

[[feature]]
id = "columns"
title = "/SIZE, /DATE, /COLUMNS, /BRIEF, /TOTAL, /VERSIONS"
status = "todo"

[[feature]]
id = "selection"
title = "/SINCE, /BEFORE, /BY_OWNER, /EXCLUDE, /SELECT=SIZE"
status = "todo"

[[feature]]
id = "remote"
title = "DIRECTORY node::spec on another VMS"
status = "todo"
needs = ["/net/remote-files"]

[[feature]]
id = "security"
title = "/SECURITY: a file's ACL"
status = "todo"
note = "ACLs (t8y:/exec/acls) are a non-goal of Milestone 1"

[[needs]]
on = "/rms/core"
why = "$PARSE, $SEARCH, and $OPEN with XABs for /FULL and /PROTECTION"

[[needs]]
on = "/rms/wildcards"
why = "$SEARCH finds the files a wildcard names"

[[needs]]
on = "/exec/logical-names"
why = "A search list's directories are listed one after another"

[[needs]]
on = "/rtl/cli"
why = "Its parameter and qualifiers through CLI$GET_VALUE and CLI$PRESENT"

[[needs]]
on = "/rtl/librtl"
why = "Writes its lines with LIB$PUT_OUTPUT"
+++

`DIRECTORY` lists the files a specification names, a heading per
directory, the names in columns, and a total. Qualifiers add what VMS
keeps about each file: `/SIZE`, `/DATE`, `/OWNER`, `/PROTECTION`,
`/SECURITY` for its ACL, and `/FULL` for all of it, from the file header
by way of RMS's XABs. `/SINCE`, `/BEFORE`, `/BY_OWNER` and `/EXCLUDE`
choose files; `[...]` walks a directory tree; `node::` lists a remote
directory over DECnet. Where the user may not read a file, VMS's
`DIRECTORY` still shows its owner and protection, by asking the XQP.

vaxpunk's is `vms/sysexe/directory.mar`, an image DCL runs with
`directory.cld`'s table. It expands the specification with `$PARSE`,
`*.*;*` for what it leaves out, finds files with `$SEARCH`, walks search
lists such as `SYS$SYSTEM:`, and with `/OWNER`, `/PROTECTION` and
`/FULL` opens each file for its XABs. `/FULL`'s report matched OpenVMS
Alpha's on PRD-0008's fixtures. Missing: wildcard directories, which RMS
lacks, one file spec only (PRD-0003 tier 3 #24), and the attributes of
files the user can't read, which wait for the XQP's `$QIO` interface.
