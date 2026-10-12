+++
title = "EDT"
summary = "DEC's line and keypad editor; EDIT/EDT on vaxpunk, in MACRO-32"
aka = ["EDT", "EDIT/EDT", "EDIT.EXE", "EDT$EDIT", "EDTINI.EDT", "keypad mode", "edit.mar"]
kind = "core"
intent = "goal"
research = "done"
refs = ["vms/sysexe/edit.mar", "vms/cld/edit.cld", "docs/design/0002-executive-processes.md", "docs/adr/0027-terminals-are-ucbs-and-telnet-is-in-the-driver.md", "docs/prd/0003-multi-user-vms.md", "docs/prd/0006-tpu-and-eve.md"]

[[feature]]
id = "line-mode"
title = "Line mode: ranges, TYPE, INSERT, DELETE, REPLACE, EXIT, QUIT at the * prompt"
status = "done"
milestone = "m0"
note = "edit.mar; DESIGN-0002's EDIT row"

[[feature]]
id = "keypad"
title = "Keypad mode (CHANGE) on a VT100: GOLD, FIND, DEL L, LINE, SECT and the arrows"
status = "done"
milestone = "m0"

[[feature]]
id = "any-terminal"
title = "Keypad mode on any terminal, its size from IO$_SENSEMODE, over SET HOST too"
status = "done"
milestone = "m1"
note = "PRD-0003 step 3, ADR-0027"

[[feature]]
id = "edit-edt"
title = "EDIT/EDT, now that EDIT is EVE"
status = "done"
note = "PRD-0006 step 10; edit.cld"

[[feature]]
id = "more-commands"
title = "SUBSTITUTE, MOVE, COPY, WRITE in line mode; WORD, PAGE, SELECT, CUT, UNDELETE on the keypad"
status = "todo"
note = "PRD-0003 tier 3 #29; edit.mar ponytail"

[[feature]]
id = "limits"
title = "Lines and buffers without fixed sizes; tabs shown as tabs"
status = "todo"
note = "edit.mar ponytail: LINESIZE, MAXLINES, TEXTSIZE"

[[feature]]
id = "buffers"
title = "Several buffers, =MAIN, =PASTE, INCLUDE and journaling (/RECOVER)"
status = "todo"

[[feature]]
id = "init-file"
title = "SET and DEFINE KEY in an EDTINI.EDT startup file"
status = "todo"

[[needs]]
on = "/rms/sequential"
why = "Reads the file and writes its next version with $GET and $PUT"

[[needs]]
on = "/drivers/terminal"
why = "Prompts with IO$_READPROMPT, reads keys with IO$M_NOFILTR, senses the size"

[[needs]]
on = "/dcl/cld"
why = "EDIT/EDT's verb and qualifiers in edit.cld"
+++

EDT was VMS's editor from V1 to V5: a line editor with a `*` prompt
(ranges, `TYPE`, `INSERT`, `SUBSTITUTE`, `MOVE`, `COPY`) and a keypad
mode on VT52 and VT100 terminals, where the numeric keypad does the
editing and PF1 is the GOLD key that gives each key a second function.
It has several buffers, journaling, a startup file (`EDTINI.EDT`) and a
callable interface, `EDT$EDIT`, which MAIL used. From VMS V5 `EDIT`
runs EVE, and EDT stays as `EDIT/EDT`; generations of VMS users kept
EDT's keypad in their fingers, which is why EVE can emulate it.

vaxpunk's EDT is `vms/sysexe/edit.mar`, about 1,500 lines of MACRO-32:
line mode with ranges, searches and the main commands, and keypad mode
with EDT's main keys, on a screen the size `SET TERMINAL` gives it, on
the console or a TELNET terminal. It was Milestone 0's editor; since
PRD-0006, `EDIT` runs TPU and `EDIT/EDT` runs this. Its gaps are
PRD-0003 tier 3 #29: several line and keypad commands, fixed line and
buffer sizes, one buffer, no startup file.
