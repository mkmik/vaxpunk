+++
title = "EVE"
summary = "The Extensible Versatile Editor, written in TPU; what EDIT runs"
aka = ["EVE", "EDIT", "EDIT/TPU", "EVE$SECTION", "eve.tpu", "TPU$INIT_PROCEDURE", "SET KEYPAD EDT"]
kind = "core"
intent = "goal"
research = "done"
refs = ["docs/prd/0006-tpu-and-eve.md", "vms/sysexe/tpu/eve/eve.tpu", "vms/sysexe/tpu/eve/tests", "vms/sysexe/tpu/subset.md", "vms/cld/edit.cld"]

[[feature]]
id = "commands"
title = "EVE's commands: moving, FIND, REPLACE, SELECT, buffers, windows, LEARN, DEFINE KEY, TPU"
status = "done"
note = "PRD-0006 step 8; fifteen scripted sessions match DEC's EVE"

[[feature]]
id = "keys"
title = "The editing keypad, Do, Help and F-key alternatives on a VT200 keyboard"
status = "done"
note = "PRD-0006 step 8"

[[feature]]
id = "edit-is-eve"
title = "EDIT runs EVE from its section file in SYS$SHARE"
status = "done"
note = "PRD-0006 steps 9 and 10"

[[feature]]
id = "word-wrap"
title = "Word wrap while typing"
status = "todo"
note = "eve.tpu ponytail"

[[feature]]
id = "command-recall"
title = "Command recall at the Command: prompt, and $CHOICES$ for an ambiguous command"
status = "todo"
note = "PRD-0006 status: left out"

[[feature]]
id = "replace-questions"
title = "REPLACE asks as DEC's EVE asks"
status = "todo"
note = "PRD-0006 status: left out"

[[feature]]
id = "box-select"
title = "Box selection"
status = "todo"
note = "eve.tpu ponytail"

[[feature]]
id = "help-library"
title = "HELP from EVE's help library rather than a list of commands"
status = "todo"
needs = ["//rtl/lbr"]
note = "PRD-0006 open question"

[[feature]]
id = "edt-keypad"
title = "SET KEYPAD EDT and WPS emulation"
status = "todo"
note = "PRD-0006 non-goal at first"

[[feature]]
id = "spawn-dcl"
title = "SPAWN, ATTACH and DCL commands from the editor"
status = "todo"
needs = ["//utils/tpu:subprocesses"]
note = "PRD-0006 non-goal"

[[feature]]
id = "journaling"
title = "Journaling and RECOVER"
status = "todo"
needs = ["//utils/tpu:journal"]

[[needs]]
on = "//utils/tpu"
why = "EVE is a TPU program, compiled into a section file"
+++

EVE, the Extensible Versatile Editor, is the editor written in TPU that
VMS runs for `EDIT` from V5 on. It has a text window with a status
line, a message window and a command line read with Do; commands are
words that can be cut short (`FIND`, `REPLACE`, `TWO WINDOWS`, `GET FILE`,
`LEARN`), and the editing keypad of a VT200 has Find, Insert Here,
Remove, Select, Prev and Next Screen. Users extend it with TPU
procedures, and `SET KEYPAD EDT` makes the keypad EDT's for those who
learned that first.

vaxpunk's EVE is ours, written in TPU from DEC's *Guide to the
Extensible Versatile Editor* and the oracle's behaviour, never from
DEC's EVE sources: `vms/sysexe/tpu/eve/eve.tpu`, compiled into
`[SYSLIB]EVE$SECTION.TPU$SECTION`. Its commands match fifteen scripted
sessions of DEC's EVE on AXPbox, screen and file
([PRD-0006](../../docs/prd/0006-tpu-and-eve.md), done Oct 9, 2026).
`eve.tpu`'s first comment lists what it leaves out: word wrap while
typing, command recall, box selection and a help library.
