+++
title = "TPU"
summary = "The Text Processing Utility: a language for writing editors, in BLISS-64"
aka = ["TPU", "DECTPU", "EDIT/TPU", "TPU.EXE", "TPU$SECTION", "TPU$COMMAND", "section file", "vms/sysexe/tpu"]
kind = "core"
intent = "goal"
research = "done"
refs = ["docs/prd/0006-tpu-and-eve.md", "vms/sysexe/tpu/subset.md", "vms/sysexe/tpu/main.b64", "vms/sysexe/tpu/fio.mar", "crosstools/vtools/crates/vrun/tests/tpu.rs", "crosstools/ods/vms/tpu-oracle.py", "vms/cld/edit.cld"]

[[feature]]
id = "language"
title = "The language: procedures, variables, IF, LOOP, CASE, ON_ERROR, COMPILE, EXECUTE"
status = "done"
note = "PRD-0006 step 3; comp.b64, vm.b64: a one-pass compiler to bytecode"

[[feature]]
id = "buffers"
title = "Buffers, ranges and markers; READ_FILE and WRITE_FILE"
status = "done"
note = "PRD-0006 step 4; buffer.b64"

[[feature]]
id = "patterns"
title = "Patterns and SEARCH: ANY, SPAN, SCAN, ARB, MATCH, anchors"
status = "done"
note = "PRD-0006 step 5; pattern.b64"

[[feature]]
id = "on-vaxpunk"
title = "TPU.EXE on vaxpunk: EDIT/TPU/NODISPLAY runs a command file"
status = "done"
note = "PRD-0006 step 6; fio.mar, the RMS and $QIO I/O module"

[[feature]]
id = "screen-keys"
title = "Windows, UPDATE, key maps, READ_KEY on the terminal"
status = "done"
note = "PRD-0006 step 7; screen.b64, keys.b64"

[[feature]]
id = "sections"
title = "Section files: SAVE, and EVE loaded from SYS$SHARE:EVE$SECTION"
status = "done"
note = "PRD-0006 step 9; our own format, section.b64"

[[feature]]
id = "subprocesses"
title = "CREATE_PROCESS, SEND, SPAWN and ATTACH from the editor"
status = "todo"
note = "PRD-0006 non-goal until DCL can run a subprocess (t8y:/dcl/spawn)"

[[feature]]
id = "journal"
title = "Journaling and recovery: /JOURNAL, /RECOVER, JOURNAL_OPEN"
status = "todo"
note = "PRD-0006 non-goal at first"

[[feature]]
id = "help-text"
title = "HELP_TEXT from a help library"
status = "todo"
needs = ["/rtl/lbr"]
note = "PRD-0006 open question; PRD-0003 tier 3 #27"

[[feature]]
id = "wildcards"
title = "FILE_SEARCH with wildcards, and more than two files open at once"
status = "todo"
note = "buffer.b64 and fio.mar ponytails"

[[feature]]
id = "scrolling"
title = "Scrolling regions, SCROLLING margins and PAD acted on"
status = "todo"
note = "screen.b64 ponytail"

[[feature]]
id = "widgets"
title = "DECwindows: /DISPLAY=DECWINDOWS, widgets, the mouse and the clipboard"
status = "todo"
note = "PRD-0006 non-goal; DECwindows Motif (t8y:/gui/motif)"

[[needs]]
on = "/host/vbliss"
why = "Its BLISS-64 is compiled by vbliss's stage 0"

[[needs]]
on = "/rms/sequential"
why = "fio.mar reads and writes files through RMS"

[[needs]]
on = "/drivers/terminal"
why = "Keys with IO$M_ESCAPE and IO$M_NOFILTR; the screen's size from IO$_SENSEMODE"

[[needs]]
on = "/rtl/librtl"
why = "LIB$PUT_OUTPUT for lines without the screen"

[[needs]]
on = "/rtl/cli"
why = "CLI$GET_VALUE gives it EDIT/TPU's command line"

[[needs]]
on = "/exec/memory"
why = "Bytecode, buffers and the screen live in $EXPREG pages"

[[needs]]
on = "/exec/image-activator"
why = "TPU.EXE is too big to be read whole into pool (PRD-0003 tier 3 #21)"

[[needs]]
on = "/host/vrun"
why = "Its tests run under vrun against the oracle's outputs"
soft = true
+++

DEC's Text Processing Utility, from VMS V4.2, is an interpreter for a
small block-structured language whose data types are made for editing:
buffers, ranges, markers, windows, patterns, key maps. An editor is a
program in it. EVE is the one VMS ships, LSE and others are built on it,
and users change their editor with TPU procedures in an initialization
file or a section file of their own. On VMS it is also a shareable
image, `TPU$TPUSHR`, that other programs call (`t8y:/rtl/tpushr`).

vaxpunk's TPU is [PRD-0006](../../prd/0006-tpu-and-eve.md), done on
Oct 9, 2026: BLISS-64 in `vms/sysexe/tpu/`, a one-pass compiler to a
bytecode and a loop that runs it, with an I/O module for vrun on the
host and one for RMS and `$QIO` on vaxpunk (`fio.mar`). It is one image,
`TPU.EXE`; the subset of built-ins EVE needs is in
[subset.md](../../../vms/sysexe/tpu/subset.md), and the rest answer
`%TPU-W-NOTYET`. Section files are vaxpunk's own format, not DEC's.
Left out: subprocesses, journaling, help libraries, DECwindows, and the
callable interface.
