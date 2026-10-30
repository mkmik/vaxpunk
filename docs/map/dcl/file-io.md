+++
title = "OPEN, READ, WRITE, CLOSE"
summary = "A procedure's file I/O: records read into symbols and written from them"
aka = ["OPEN", "READ", "WRITE", "CLOSE", "READ/KEY", "WRITE/UPDATE", "READ/PROMPT", "WRITE SYS$OUTPUT"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m1"
refs = ["vms/sysexe/dcl/file.mar", "vms/cld/dcl.cld", "docs/prd/0008-rms-record-and-indexed-files.md", "docs/prd/0003-multi-user-vms.md"]

[[feature]]
id = "write-output"
title = "WRITE SYS$OUTPUT and SYS$ERROR"
status = "done"
milestone = "m0"

[[feature]]
id = "open-close"
title = "OPEN /READ /WRITE /APPEND, READ, WRITE, CLOSE on files of any organization"
status = "done"
milestone = "m1"
note = "PRD-0008 step 8"

[[feature]]
id = "keyed"
title = "READ/KEY/INDEX/MATCH, READ/DELETE, WRITE/UPDATE, /END_OF_FILE and /ERROR"
status = "done"
milestone = "m1"
note = "PRD-0008 step 8"

[[feature]]
id = "read-input"
title = "READ SYS$INPUT: a procedure's data line, or the terminal with /PROMPT"
status = "done"
milestone = "m1"
note = "PRD-0003 step 8"

[[feature]]
id = "logical-name"
title = "OPEN defines the logical name, a process-permanent file images can use"
status = "todo"
needs = ["/rms/ppf#indirect"]
note = "file.mar ponytail: no ESC and IFI name in LNM$PROCESS"

[[feature]]
id = "vfc"
title = "OPEN/WRITE makes VFC records with PRN carriage control, as VMS"
status = "todo"
note = "file.mar ponytail: VAR with CR"

[[feature]]
id = "long-records"
title = "Records of 2,048 bytes, 8,192 with Extended DCL"
status = "todo"
note = "file.mar ponytail: a symbol's 255 bytes"

[[feature]]
id = "sharing"
title = "OPEN/SHARE, READ/NOLOCK, READ/WAIT"
status = "done"
needs = ["/rms/sharing#record-locks"]

[[feature]]
id = "time-out"
title = "READ/TIME_OUT, of a terminal or a record lock's wait"
status = "todo"
needs = ["/rms/sharing#wait-timeout"]
note = "dcl.cld ponytail"

[[feature]]
id = "dispositions"
title = "CLOSE/DISPOSITION, WRITE/SYMBOL"
status = "todo"
note = "dcl.cld ponytail"

[[needs]]
on = "/dcl/symbols"
why = "READ puts the record in a symbol; WRITE takes expressions"

[[needs]]
on = "/rms/sequential"
why = "OPEN, READ and WRITE are $OPEN, $GET and $PUT"

[[needs]]
on = "/rms/indexed"
why = "READ/KEY reads an indexed file by key"

[[needs]]
on = "/dcl/procedures"
why = "READ SYS$INPUT takes the procedure's next data line"
+++

`OPEN logical-name file` opens a file for a procedure, `READ
logical-name symbol` reads its next record into a symbol, `WRITE
logical-name expression,...` writes one, and `CLOSE` closes it. They work
on files of any RMS organization: `READ/KEY` reads an indexed file by
key, `READ/DELETE` deletes the record read, `WRITE/UPDATE` rewrites it.
`/END_OF_FILE=label` and `/ERROR=label` jump instead of failing. An open
file is a process-permanent file: it stays open across images, and its
logical name, which `OPEN` defines in the process table, lets an image
use it. `READ SYS$COMMAND/PROMPT=` asks the terminal; `WRITE
SYS$OUTPUT` is how procedures print.

vaxpunk's are in `vms/sysexe/dcl/file.mar`, done with PRD-0008 step 8:
up to 8 files, each a FAB and a RAB in DCL's P1 data, open until `CLOSE`
or the end of the process. `OPEN` defines no logical name, so only DCL's
own file commands find the file; records are at most 255 bytes, and a
new file has VAR records, not VMS's VFC.
