+++
title = "Symbols and foreign commands"
summary = "Local and global symbols, verbs made of symbols, name := $image"
aka = ["=", "==", ":=", ":==", "SHOW SYMBOL", "DELETE/SYMBOL", "foreign command", "LIB$GET_FOREIGN", "DCL$PATH", "automatic foreign command", "symbol abbreviation"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["vms/sysexe/dcl/symbol.mar", "vms/sysexe/dcl.mlb", "docs/adr/0018-set-command-and-foreign-commands.md", "vms/sysexe/lib/getforeign.mar"]

[[feature]]
id = "local-global"
title = "=, ==, := and :==; local symbols per procedure level, global ones"
status = "done"
milestone = "m0"

[[feature]]
id = "show-delete"
title = "SHOW SYMBOL and DELETE/SYMBOL, with /LOCAL, /GLOBAL, /ALL"
status = "done"
milestone = "m0"

[[feature]]
id = "verb-symbols"
title = "A symbol as the verb: HOME == \"SET DEFAULT [SYSMGR]\""
status = "done"
milestone = "m0"

[[feature]]
id = "foreign"
title = "name := $image, a foreign command; LIB$GET_FOREIGN reads its line"
status = "done"
milestone = "m0"
note = "ADR-0018"

[[feature]]
id = "abbreviations"
title = "Abbreviated symbol names: DIR*ECTORY := ..."
status = "todo"

[[feature]]
id = "substrings"
title = "Substring and bit field assignment: S[0,3] := abc, N[4,2] = 3"
status = "todo"
note = "dcl.mar ponytail"

[[feature]]
id = "dcl-path"
title = "DCL$PATH: a verb not defined runs a .COM or .EXE found on the path"
status = "todo"
note = "VMS V6.2's automatic foreign commands"

[[feature]]
id = "limits"
title = "As many symbols as memory allows, values longer than 255 bytes"
status = "todo"
note = "dcl.mlb: 64 symbols of 255 bytes"

[[feature]]
id = "lib-symbols"
title = "LIB$GET_SYMBOL, LIB$SET_SYMBOL, LIB$DELETE_SYMBOL: an image reads and sets DCL's symbols"
status = "todo"
note = "ADR-0018: an image has no way to ask DCL; the CLI$ routines asking DCL by callback (ADR-0028) would give one"

[[needs]]
on = "/dcl/interpreter"
why = "The symbol table is DCL's P1 data"

[[needs]]
on = "/rtl/librtl"
why = "LIB$GET_FOREIGN gives a foreign command's image its line"
soft = true
+++

DCL symbols are named values, integers or strings. `name = expression`
and `name := string` make a local symbol, belonging to the procedure
level that made it and seen by the procedures it calls; `==` and `:==`
make a global one, everyone's. A symbol whose name is the first word of
a command stands for its value, so sites define short commands, and an
abbreviated name, `DIR*ECTORY`, matches `DIR` to `DIRECTORY`. A symbol
whose value is `$image` is a foreign command: DCL runs the image without
parsing the rest of the line, which the image reads with
`LIB$GET_FOREIGN`; this is how ported programs and MCR-style tools take
their arguments. Since V6.2, with `DCL$PATH` defined, a verb DCL doesn't
know runs a `.COM` or `.EXE` of that name found on the path.

vaxpunk's symbol table is in `vms/sysexe/dcl/symbol.mar`: 64 entries of
up to 255 bytes. Foreign commands give the image a result block whose
`$LINE` is the command
([ADR-0018](../../adr/0018-set-command-and-foreign-commands.md));
`MCR image` works the same way.
