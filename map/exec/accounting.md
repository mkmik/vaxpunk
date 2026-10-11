+++
title = "Accounting"
summary = "CPU time and I/O counts per process, and the accounting file of who used what"
aka = ["ACCOUNTNG.DAT", "SET ACCOUNTING", "SHOW ACCOUNTING", "$SNDJBC SJC$_WRITE_ACCOUNTING", "ACC$", "$ACCDEF", "MSG$_DELPROC", "termination message", "CPUTIM", "BUFIO", "DIRIO", "PAGEFLTS", "SHOW PROCESS/ACCOUNTING", "NOACNT", "account name"]
kind = "core"
intent = "goal"
research = "outline"
milestone = "m1"
refs = ["docs/prd/0003-multi-user-vms.md", "docs/adr/0019-mailboxes.md", "vms/exec/process.mar", "vms/exec/getjpi.mar", "vms/sysexe/show.mar", "crosstools/vtools/lib/starlet.mlb"]

[[feature]]
id = "termination-message"
title = "A process's termination message: final status, PID, time, owner"
status = "done"
milestone = "m0"
note = "ADR-0019; TERMMSG in process.mar leaves the counts in $ACCDEF's layout zero"

[[feature]]
id = "cpu-time"
title = "CPU time kept by the scheduler, in 10 ms ticks"
status = "todo"
milestone = "m1"
needs = ["//exec/scheduler"]
note = "PRD-0003 tier 1 #4, step 5"

[[feature]]
id = "io-counts"
title = "Buffered and direct I/O counts kept by $QIO"
status = "todo"
milestone = "m1"
needs = ["//exec/qio"]
note = "PRD-0003 tier 1 #4, step 5"

[[feature]]
id = "show"
title = "$GETJPI's CPUTIM, BUFIO, DIRIO; SHOW PROCESS/ACCOUNTING; CTRL/T's line"
status = "todo"
milestone = "m1"
needs = [":cpu-time", ":io-counts", "//exec/information-services:jpi-quotas"]
note = "PRD-0003 step 5; show.mar ponytail. CTRL/T is PRD-0003 tier 3 #29"

[[feature]]
id = "full-message"
title = "The termination message carries the counts, username, account and login time"
status = "todo"
needs = [":cpu-time", ":io-counts"]

[[feature]]
id = "file"
title = "The job controller writes ACCOUNTNG.DAT: process, image, login failure, print and user records"
status = "todo"
needs = [":full-message"]
note = "Written by //utils/job-controller"

[[feature]]
id = "set-accounting"
title = "SET ACCOUNTING /ENABLE, /DISABLE, /NEW_FILE; SHOW ACCOUNTING"
status = "todo"
needs = [":file"]

[[feature]]
id = "report"
title = "The ACCOUNTING utility's reports and summaries"
status = "todo"
needs = [":file"]

[[feature]]
id = "page-counts"
title = "Page faults, peak working set and page file use"
status = "todo"
note = "With //exec/paging, a PRD-0003 non-goal"

[[needs]]
on = "//exec/processes"
why = "The counters are the process's"

[[needs]]
on = "//exec/mailboxes"
why = "The termination message goes to the creator's mailbox"

[[needs]]
on = "//utils/job-controller"
why = "Takes termination messages and writes ACCOUNTNG.DAT"
soft = true

[[needs]]
on = "//utils/accounting"
why = "Reports from the accounting file"
soft = true
+++

VMS counts what each process uses: CPU time, buffered and direct I/O,
page faults, peak working set and page file use, volumes mounted.
`SHOW PROCESS/ACCOUNTING` and CTRL/T show them while it runs. When a
process ends, the executive sends a termination message (`$ACCDEF`,
`MSG$_DELPROC`) holding them, with its final status, username,
account and login time, to its creator's termination mailbox; the job
controller takes them for the processes it created and writes an
accounting record to `SYS$MANAGER:ACCOUNTNG.DAT`. Images, print jobs,
login failures and programs (`$SNDJBC`'s `SJC$_WRITE_ACCOUNTING`) add
records too. `SET ACCOUNTING` chooses which; the `ACCOUNTING` utility
reports from the file, by user or by account, for charging users for
the machine. `NOACNT` keeps a process out of it.

vaxpunk writes the termination message with the final status, PID,
time and owner (`TERMMSG` in `vms/exec/process.mar`,
[ADR-0019](../../docs/adr/0019-mailboxes.md)); the counts in it are
zero because nothing keeps them. PRD-0003 step 5 makes the scheduler
keep CPU time and `$QIO` keep I/O counts, for `$GETJPI` and `SHOW
PROCESS/ACCOUNTING`. The accounting file waits on the job controller.
