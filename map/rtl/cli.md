+++
title = "CLI$ routines"
summary = "Command parsing against CLD tables: CLI$DCL_PARSE, CLI$GET_VALUE, CLI$PRESENT"
aka = ["CLI$", "CLI$DCL_PARSE", "CLI$DISPATCH", "CLI$GET_VALUE", "CLI$PRESENT"]
kind = "core"
intent = "goal"
research = "done"
refs = ["docs/adr/0017-command-tables-from-cld-with-vcdu.md", "docs/adr/0018-set-command-and-foreign-commands.md", "docs/adr/0028-shareable-images.md", "vms/sysexe/lib/cli.mar"]

[[feature]]
id = "parse"
title = "CLI$DCL_PARSE, CLI$PRESENT, CLI$GET_VALUE, CLI$DISPATCH"
status = "done"
milestone = "m0"
note = "ADR-0017"

[[feature]]
id = "image-tables"
title = "Utilities with tables of their own: TCPIP, AUTHORIZE"
status = "done"
note = "ADR-0022"

[[feature]]
id = "signal"
title = "Errors signalled with LIB$SIGNAL, not written and returned"
status = "todo"
note = "cli.mar ponytail; PRD-0003 tier 3 #26"

[[feature]]
id = "limits"
title = "Parse limits lifted; a default-present entity counts"
status = "todo"
note = "PRD-0003 tier 3 #32"

[[feature]]
id = "ask-dcl"
title = "The parse moves into DCL; the routines ask the CLI by callback"
status = "todo"
note = "ADR-0028: then LIB$GET_FOREIGN can join LIBRTL.EXE"

[[needs]]
on = "//dcl/cld"
why = "Parses against command tables compiled from CLD"

[[needs]]
on = "//dcl/interpreter"
why = "DCL passes the image its parse"
+++

The `CLI$` routines let an image read its command: `CLI$PRESENT` asks
whether a qualifier or parameter was given, `CLI$GET_VALUE` returns its
values one at a time, and `CLI$DCL_PARSE` with `CLI$DISPATCH` parse a
line against a utility's own command tables, as MAIL, AUTHORIZE and
TCPIP do at their prompts. On VMS they call back into DCL, which holds
the parse; they are object modules in `STARLET.OLB`, linked into each
image.

vaxpunk's are in `vms/sysexe/lib/cli.mar`, linked into every image: the
parser itself is there, and DCL hands the image its result block
([ADR-0017](../../docs/adr/0017-command-tables-from-cld-with-vcdu.md)).
`TCPIP` and `AUTHORIZE` parse their own commands with it.
