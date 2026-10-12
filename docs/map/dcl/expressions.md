+++
title = "Expressions and substitution"
summary = "DCL's integer and string expressions, and 'symbol' substitution"
aka = ["'symbol'", "''symbol'", "&symbol", ".EQS.", ".EQ.", ".AND.", ".OR.", ".NOT.", "%X", "%O", "%D", "expression"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["vms/sysexe/dcl/expr.mar", "vms/sysexe/dcl.mar", "docs/prd/0003-multi-user-vms.md"]

[[feature]]
id = "expressions"
title = "Integers and strings, + - * /, comparisons, .AND. .OR. .NOT., parentheses"
status = "done"
milestone = "m0"

[[feature]]
id = "substitution"
title = "'symbol' outside quotes and ''symbol' inside them; ! comments"
status = "done"
milestone = "m0"

[[feature]]
id = "radix-lexicals"
title = "%X, %O and %D numbers, and lexical function calls in an expression"
status = "done"
milestone = "m1"
note = "PRD-0003 step 8"

[[feature]]
id = "not-comparison"
title = ".NOT. in front of a comparison, not only an operand"
status = "todo"
note = "dcl.mar ponytail"

[[feature]]
id = "ampersand"
title = "&symbol substitution, done after the command is parsed"
status = "todo"
note = "dcl.mar ponytail"

[[feature]]
id = "string-minus"
title = "String subtraction: \"abc\" - \"b\" removes the first match"
status = "todo"

[[needs]]
on = "/dcl/symbols"
why = "A symbol's value is an operand, and substitution reads it"

[[needs]]
on = "/dcl/lexicals"
why = "F$name(...) calls are operands"
soft = true
+++

DCL evaluates expressions in assignments, `IF`, `WRITE`, `EXIT` and the
arguments of lexical functions. Values are 32-bit integers or strings,
converted as each operator needs: a string compared with `.EQ.` is read
as a number, and one used as a truth value is true if it is odd or starts
with `T` or `Y`. String operators have an `S` (`.EQS.`, `.GTS.`); `+`
joins strings and `-` removes the first occurrence of one string from
another. Before a command is parsed, `'symbol'` is replaced by the
symbol's value, `''symbol'` inside quotes; `&symbol` is replaced later,
after parsing, once per command.

vaxpunk's are in `vms/sysexe/dcl/expr.mar`: `DCL$SUBST` strips comments
and substitutes, `DCL$EXPR` works out an expression on a value stack, and
calls the lexical functions it meets. `%X`, `%O` and `%D` came with
PRD-0003 step 8. `&` substitution, string subtraction and `.NOT.` before
a comparison are missing.
