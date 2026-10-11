+++
title = "Command tables (CLD)"
summary = "Verbs defined in CLD, compiled into command tables; SET COMMAND"
aka = ["CLD", "CDU", "Command Definition Utility", "DCL$TABLES", "DCLTABLES.EXE", "SET COMMAND", "vcdu", "CDU$COMPILE", "DEFINE VERB", "DEFINE SYNTAX", "CLIROUTINE"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["docs/adr/0017-command-tables-from-cld-with-vcdu.md", "docs/adr/0018-set-command-and-foreign-commands.md", "crosstools/vtools/docs/command-tables.md", "vms/cld", "vms/sysexe/dcl/cdu.mar", "vms/sysmgr/dcltest.cld"]

[[feature]]
id = "host-tables"
title = "vms/cld/*.cld compiled by vcdu into DCL$TABLES, linked into DCL"
status = "done"
milestone = "m0"
needs = ["//host/vcdu"]
note = "ADR-0017"

[[feature]]
id = "every-verb"
title = "Every DCL verb in CLD: syntaxes, keywords, CLIROUTINE, DISALLOW"
status = "done"
milestone = "m0"
note = "ADR-0017 decision 3"

[[feature]]
id = "routines"
title = "ROUTINE, CLI$DISPATCH, PLACEMENT=LOCAL and POSITIONAL"
status = "done"
milestone = "m0"
note = "ADR-0017 follow-ups, done"

[[feature]]
id = "set-command"
title = "SET COMMAND file: CDU$COMPILE in DCL, tables looked in before DCL's"
status = "done"
milestone = "m0"
note = "ADR-0018"

[[feature]]
id = "cdu-limits"
title = "SET COMMAND reports every error, takes more than 8 files and 16 KB"
status = "todo"
note = "PRD-0003 tier 3 #32; cdu.mar ponytail"

[[feature]]
id = "set-command-options"
title = "SET COMMAND /DELETE, /TABLE, /OUTPUT, /OBJECT, /REPLACE"
status = "todo"
note = "ADR-0018 follow-up"

[[feature]]
id = "dcltables-image"
title = "DCLTABLES.EXE: tables in an image, kept for the next login"
status = "todo"
note = "ADR-0017 alternatives, ADR-0018 follow-up; VMS installs it shared"

[[needs]]
on = "//rms/sequential"
why = "SET COMMAND reads the .CLD file with RMS; DCL's own tables are linked in"
soft = true
+++

On VMS every DCL verb is defined in the Command Definition Language: a
`DEFINE VERB` names its image or routine, its parameters `P1` to `P8`,
its qualifiers with their values and defaults; `DEFINE SYNTAX` gives a
variant a keyword or qualifier switches to; `DEFINE TYPE` lists
keywords. The Command Definition Utility compiles CLD into command
tables: DCL's own are `SYS$LIBRARY:DCLTABLES.EXE`, which LOGINOUT maps
into each process. `SET COMMAND file` adds a site's verbs to the
process's tables, or writes them into a table image or, with `/OBJECT`,
an object module a utility links to parse its own commands. The same
tables give DCL its parsing rules: 4-character abbreviations, `/NO`,
values, lists and prompts for missing parameters.

vaxpunk's verbs are in `vms/cld/*.cld`. `vcdu` compiles them on the host
into `DCL$TABLES`, linked into `DCL.EXE`, in a format of its own
documented in `crosstools/vtools/docs/command-tables.md`
([ADR-0017](../../docs/adr/0017-command-tables-from-cld-with-vcdu.md)).
`SET COMMAND` compiles a `.CLD` file on vaxpunk with `CDU$COMPILE`,
`vms/sysexe/dcl/cdu.mar`, into tables DCL looks in first
([ADR-0018](../../docs/adr/0018-set-command-and-foreign-commands.md));
`SYS$MANAGER:DCLTEST.CLD` adds `GREET`. The parser itself is //rtl/cli.
