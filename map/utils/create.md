+++
title = "CREATE"
summary = "Makes a text file from the terminal, a directory, or a file from FDL"
aka = ["CREATE", "CREATE.EXE", "CREATE/DIRECTORY", "CREATE/FDL", "CREATE/MAILBOX", "CREATE/NAME_TABLE", "CREATE/TERMINAL"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["vms/sysexe/create.mar", "vms/cld/create.cld", "docs/adr/0009-ramdisk-writable-files-11.md", "docs/prd/0008-rms-record-and-indexed-files.md", "docs/prd/0003-multi-user-vms.md", "https://docs.vmssoftware.com/vsi-openvms-dcl-dictionary-a-m/"]

[[feature]]
id = "directory"
title = "CREATE/DIRECTORY [A.B.C], and the directories above it that aren't there"
status = "done"
milestone = "m0"
note = "ADR-0009 follow-up, $CREATE_DIR"

[[feature]]
id = "fdl"
title = "CREATE/FDL=file: an empty file as an FDL file describes it"
status = "done"
needs = ["//rms/fdl:create-fdl", "//rms/indexed"]
note = "PRD-0008 step 7"

[[feature]]
id = "text-file"
title = "CREATE file: the lines typed up to CTRL/Z, with /LOG"
status = "done"
note = "PR #123"

[[feature]]
id = "procedure-data"
title = "In a command procedure, the data lines that follow it"
status = "done"
note = "PRD-0003 step 8 (dcltest.com); create.mar's ponytail predates it"

[[feature]]
id = "dir-qualifiers"
title = "/OWNER_UIC, /PROTECTION, /VERSION_LIMIT and /ALLOCATION on a directory"
status = "todo"
needs = ["//files/ods2:version-limits"]
note = "PRD-0003 tier 1 #8: $CREATE_DIR takes no version limit"

[[feature]]
id = "lists"
title = "Lists of files or directories"
status = "todo"
note = "create.mar ponytail"

[[feature]]
id = "mailbox-table"
title = "CREATE/MAILBOX and CREATE/NAME_TABLE"
status = "todo"
needs = ["//exec/mailboxes", "//exec/logical-names"]

[[feature]]
id = "terminal"
title = "CREATE/TERMINAL: a DECterm window"
status = "todo"
note = "Needs a DECterm, //gui/decterm"

[[needs]]
on = "//rms/core"
why = "$CREATE, $CREATE_DIR, and $PUT of each line"

[[needs]]
on = "//files/ods2"
why = "Writes the directory files and headers"

[[needs]]
on = "//gui/decterm"
why = "CREATE/TERMINAL opens a DECterm window"
soft = true

[[needs]]
on = "//rtl/cli"
why = "Its parameter and qualifiers"
+++

`CREATE file` makes a text file of the lines typed after it, up to
CTRL/Z, or in a command procedure of the data lines that follow it.
`CREATE/DIRECTORY` makes directories, every missing level at once, with
an owner, protection and version limit. `CREATE/FDL` makes an empty file
of any organization from an FDL description. Later VMS versions hang
other things off the verb: `/MAILBOX`, `/NAME_TABLE`, and `/TERMINAL`
for a DECterm.

vaxpunk's is `vms/sysexe/create.mar`. `CREATE/DIRECTORY` calls
`$CREATE_DIR`; `CREATE/FDL` calls `FDL$PARSE` in `rms/fdl.mar` (PRD-0008
step 7); `CREATE file` reads `SYS$INPUT` into variable-length records
with carriage-return control. A procedure's data lines reach it since
PRD-0003 step 8. Missing: directory qualifiers, a version limit, and
the newer `/MAILBOX`, `/NAME_TABLE` and `/TERMINAL`.
