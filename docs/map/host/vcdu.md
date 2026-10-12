+++
title = "vcdu (CLD compiler)"
summary = "Compiles CLD into command tables, as SET COMMAND/OBJECT does"
aka = ["vcdu", "CDU", "SET COMMAND/OBJECT", ".CLD", "DCL$TABLES", "command-tables.md"]
kind = "tool"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["docs/adr/0017-command-tables-from-cld-with-vcdu.md", "crosstools/vtools/crates/vcdu", "crosstools/vtools/docs/command-tables.md", "vms/cld"]

[[feature]]
id = "basic"
title = "Verbs, syntaxes, types, parameters and qualifiers into one table object"
status = "done"
milestone = "m0"
note = "ADR-0017"

[[feature]]
id = "macro"
title = "/MACRO: the table as MACRO-32 source"
status = "done"
milestone = "m0"

[[feature]]
id = "keywords-after-value"
title = "Keywords for a qualifier placed after a value"
status = "todo"
note = "PRD-0003 tier 4 #44; lib.rs ponytail"

[[needs]]
on = "/host/vmacro"
why = "Its table is MACRO-32 source that vmacro compiles"
+++

On VMS the Command Definition Utility compiles `.CLD` files, which
define verbs, their parameters and qualifiers, into command tables:
`SET COMMAND` adds them to a process's DCL, and `SET COMMAND/OBJECT`
makes an object module a program links to parse its own commands.

`vcdu` is the `/OBJECT` path on the host
([ADR-0017](../../adr/0017-command-tables-from-cld-with-vcdu.md)):
`vms/build.rs` compiles `vms/cld/*.cld` into `DCL$TABLES` and each
utility's `.CLD` into its own table, which `CLI$DCL_PARSE` reads. On
vaxpunk, DCL's `SET COMMAND` compiles the same language into the same
format with `CDU$COMPILE` (`t8y:/dcl/cld`).
