+++
title = "Process-permanent files"
summary = "Files open for the process's life: SYS$INPUT, SYS$OUTPUT, DCL's OPEN files"
aka = ["PPF", "process permanent files", "indirect access", "SYS$INPUT", "SYS$OUTPUT", "SYS$ERROR", "SYS$COMMAND", "process I/O segment", "PIOPAGES", "$RMSRUNDWN"]
kind = "core"
intent = "goal"
research = "outline"
milestone = "m1"
refs = ["docs/prd/0003-multi-user-vms.md", "vms/exec/rms.mar", "vms/exec/process.mar", "vms/sysexe/dcl/file.mar", "vms/sysexe/dcl/proc.mar", "https://wasd.vsm.com.au/htbin/helpgate/HELP/SYS_PARAMETERS/PIOPAGES", "https://wasd.vsm.com.au/htbin/helpgate/HELP/SYSTEM_SERVICES/_RMSRUNDWN/ARGUMENTS"]

[[feature]]
id = "cli-files"
title = "Files the command interpreter opens stay open across images, closed with the process"
status = "done"
note = "PCB$L_CLIFILES, RMS$RUNDOWN; DCL's OPEN relies on it (dcl/file.mar, PRD-0008 step 8)"

[[feature]]
id = "procedure-input"
title = "A command procedure as an image's SYS$INPUT, read through RMS, the rest given back to DCL"
status = "todo"
milestone = "m1"
needs = ["//exec/logical-names"]
note = "PRD-0003 tier 1 #6: data lines go through a mailbox that replaces SYS$INPUT (dcl/proc.mar)"

[[feature]]
id = "indirect"
title = "An image's $OPEN of SYS$OUTPUT or an OPEN's logical name reaches the open file"
status = "todo"
milestone = "m1"
needs = [":cli-files", "//exec/logical-names"]
note = "dcl/file.mar ponytail: OPEN makes no ESC and IFI logical name"

[[feature]]
id = "log-file"
title = "SYS$OUTPUT and SYS$ERROR a disk file, as a batch job's log"
status = "todo"
milestone = "m1"
needs = [":indirect", "//rms/core:devices"]
note = "PRD-0003 Batch and print: SYS$OUTPUT a log file in SYS$LOGIN"

[[feature]]
id = "terminal"
title = "SYS$INPUT and SYS$OUTPUT on the terminal opened through RMS, not with $QIO"
status = "todo"
needs = [":indirect", "//rms/core:devices"]
note = "LIB$PUT_OUTPUT and the C library's stdio write SYS$OUTPUT with $QIOW"

[[needs]]
on = "//rms/core"
why = "Process-permanent files are RMS files that outlive the image"

[[needs]]
on = "//exec/logical-names"
why = "SYS$INPUT and the rest are executive-mode names whose value names the open file"

[[needs]]
on = "//exec/processes"
why = "They belong to the process and close at its rundown, not the image's"
+++

A process-permanent file is one RMS keeps open for the life of the
process rather than the image: DCL opens `SYS$INPUT`, `SYS$OUTPUT`,
`SYS$ERROR` and `SYS$COMMAND` at login, and the files a procedure opens
with `OPEN`. Their structures live in the process I/O segment of P1
(`PIOPAGES` sizes it), and their logical names are executive-mode names
whose value starts with an escape and holds the file's IFI. An image that
opens one of those names gets indirect access to the file DCL already has
open, so a batch job's output goes to its log file and an image reads its
data lines from the procedure that runs it. Image rundown closes the
image's own files and leaves these.

vaxpunk keeps the files DCL's `OPEN` opens across images, closing them
only when the process ends (`PCB$L_CLIFILES`), but makes no logical name
for them, so only DCL's file commands reach them. `SYS$INPUT` and
`SYS$OUTPUT` are devices programs write with `$QIO`, and a procedure's
data lines reach an image through a mailbox that takes `SYS$INPUT`'s
place, for want of logical names' access modes and RMS on
process-permanent files (PRD-0003 tier 1 #6). Batch log files
(PRD-0003 step 9) need `SYS$OUTPUT` to be a disk file.
