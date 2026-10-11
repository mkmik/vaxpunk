+++
title = "ACCOUNTING"
summary = "Resource use per user and job: ACCOUNTNG.DAT, SET ACCOUNTING and its reports"
aka = ["ACCOUNTING", "ACCOUNTNG.DAT", "SET ACCOUNTING", "ACC$", "accounting records"]
kind = "core"
intent = "aspiration"
research = "outline"
refs = ["docs/adr/0019-mailboxes.md", "docs/prd/0003-multi-user-vms.md", "https://h41379.www4.hpe.com/doc/84final/9996/9996pro_171.html"]

[[feature]]
id = "file"
title = "SYS$MANAGER:ACCOUNTNG.DAT: a record for each process, image, login failure and print job"
status = "todo"
needs = ["//utils/job-controller:accounting"]

[[feature]]
id = "set-accounting"
title = "SET ACCOUNTING /ENABLE, /DISABLE and /NEW_FILE, by kind of resource and process"
status = "todo"
needs = [":file"]

[[feature]]
id = "reports"
title = "The ACCOUNTING command: /BRIEF, /FULL, /SUMMARY, /REPORT, selected by user, type and time"
status = "todo"
needs = [":file"]

[[feature]]
id = "user-records"
title = "Records a program writes with $SNDJBC"
status = "todo"
needs = [":file", "//utils/job-controller:sndjbc"]

[[needs]]
on = "//utils/job-controller"
why = "The job controller writes the accounting file, with the counts //exec/accounting keeps"

[[needs]]
on = "//rms/sequential"
why = "ACCOUNTNG.DAT is a file of variable length records"
+++

VMS keeps an accounting file, `SYS$MANAGER:ACCOUNTNG.DAT` on each node,
with a record when a process ends (interactive, batch, detached,
network or subprocess), optionally when each image ends, for each
failed login, for each print job, and for records programs send with
`$SNDJBC`. Each says who, when, and how much CPU time, I/O, pages and
paper went. The job controller writes it, from what the executive tells
it when a process ends. `SET ACCOUNTING` chooses what is recorded and
starts a new file; the `ACCOUNTING` command reads one and prints
records, summaries or reports, by user, account, image or queue. Sites
used it to bill departments for computer time.

vaxpunk has no accounting file and no job controller. PRD-0003's step 5
keeps CPU time and I/O counts per process, for `SHOW
PROCESS/ACCOUNTING` (`//exec/accounting`); ADR-0019's termination
message, which process deletion writes to a mailbox, already has an
accounting part.
