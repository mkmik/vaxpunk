+++
title = "IF, GOTO, ON, GOSUB, CALL"
summary = "A procedure's control flow: IF, GOTO, ON, EXIT, and subroutines"
aka = ["IF", "THEN", "ELSE", "ENDIF", "GOTO", "ON", "SET ON", "SET NOON", "EXIT", "GOSUB", "RETURN", "CALL", "SUBROUTINE", "ENDSUBROUTINE", "$STATUS", "$SEVERITY"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["vms/sysexe/dcl/proc.mar", "docs/prd/0003-multi-user-vms.md", "vms/sysmgr/dcltest.com"]

[[feature]]
id = "if-goto"
title = "IF expression THEN command, GOTO label, EXIT [status]"
status = "done"
milestone = "m0"

[[feature]]
id = "block-if"
title = "Block IF, THEN, ELSE, ENDIF, nested"
status = "done"
milestone = "m1"
note = "PRD-0003 step 8"

[[feature]]
id = "on"
title = "ON WARNING, ERROR, SEVERE_ERROR, CONTROL_Y; SET [NO]ON; $STATUS, $SEVERITY"
status = "done"
milestone = "m1"
note = "PRD-0003 step 8"

[[feature]]
id = "gosub"
title = "GOSUB and RETURN: a subroutine at the same level"
status = "todo"
note = "dcl.mar ponytail"

[[feature]]
id = "call"
title = "CALL, SUBROUTINE, ENDSUBROUTINE: a subroutine at a new level, with P1 to P8"
status = "todo"
note = "dcl.mar ponytail"

[[needs]]
on = "/dcl/procedures"
why = "Labels and levels are a procedure's"

[[needs]]
on = "/dcl/expressions"
why = "IF's condition and EXIT's status are expressions"
+++

DCL's control flow is for procedures. `IF expression THEN command` runs
the command when the expression is true, odd; the block form, `IF`,
`THEN`, `ELSE`, `ENDIF`, runs lines. `GOTO label` goes to a label,
`label:`, of the same procedure, and `EXIT [status]` ends it. After each
command `$STATUS` and `$SEVERITY` hold its status; by default an error
or worse ends the procedure, and `ON WARNING THEN command`, `ON ERROR`,
`ON SEVERE_ERROR` and `ON CONTROL_Y` say otherwise, once; `SET NOON`
turns the checks off. By VMS V5 DCL also had `GOSUB` and `RETURN`, a
subroutine at the same level, up to 16 deep, and `CALL label`, which
runs a `SUBROUTINE` to `ENDSUBROUTINE` block as a new level with its own
local symbols and parameters.

vaxpunk's are in `vms/sysexe/dcl/proc.mar`. Simple `IF`, `GOTO` and
`EXIT` came with Milestone 0; block `IF`, `ON`, `SET [NO]ON` and
`$SEVERITY` with PRD-0003 step 8. `GOSUB` and `CALL` are missing.
