+++
title = "Job controller and queues"
summary = "JOB_CONTROL and the queue manager: SUBMIT, PRINT, batch and print queues"
aka = ["JOB_CONTROL", "QUEUE_MANAGER", "QMAN$MASTER.DAT", "SYS$QUEUE_MANAGER", "$SNDJBC", "$GETQUI", "F$GETQUI", "SUBMIT", "PRINT", "SHOW QUEUE", "SYNCHRONIZE", "SYS$BATCH", "SYS$PRINT", "INITIALIZE/QUEUE", "START/QUEUE/MANAGER", "batch"]
kind = "core"
intent = "goal"
research = "outline"
refs = ["docs/prd/0003-multi-user-vms.md", "docs/adr/0029-loginout-and-sysuaf.md", "docs/prd/0008-rms-record-and-indexed-files.md"]

[[feature]]
id = "job-control"
title = "A JOB_CONTROL process that keeps queues in SYS$SYSTEM:QMAN$MASTER.DAT"
status = "todo"
milestone = "m1"
needs = ["/rms/indexed", "/exec/mailboxes"]
note = "PRD-0003 step 9"

[[feature]]
id = "sndjbc"
title = "$SNDJBC and $GETQUI, and F$GETQUI"
status = "todo"
milestone = "m1"
needs = ["/exec/system-services"]
note = "PRD-0003 step 9"

[[feature]]
id = "queue-commands"
title = "INITIALIZE/QUEUE/BATCH, START/QUEUE, STOP/QUEUE, DELETE/QUEUE, SHOW QUEUE"
status = "todo"
milestone = "m1"
needs = ["/dcl/cld"]
note = "PRD-0003 Batch and print"

[[feature]]
id = "submit"
title = "SUBMIT runs a procedure in a batch process as the submitter, with a log file"
status = "todo"
milestone = "m1"
needs = ["/utils/loginout#batch", "/exec/processes", "/rms/ppf#log-file"]
note = "PRD-0003 step 9: /AFTER, /HOLD, /PARAMETERS, /LOG, /NOTIFY"

[[feature]]
id = "after"
title = "/AFTER and the time in the log: $BINTIM, SHOW TIME"
status = "todo"
milestone = "m1"
needs = ["/exec/time-conversion#bintim", "/exec/timers"]
note = "PRD-0003 tier 1 #7"

[[feature]]
id = "entries"
title = "DELETE/ENTRY, SET ENTRY, SYNCHRONIZE"
status = "todo"
milestone = "m1"
note = "PRD-0003 step 9"

[[feature]]
id = "notify"
title = "/NOTIFY: a broadcast to the submitter when the job ends"
status = "todo"
milestone = "m1"
needs = ["/drivers/terminal#brkthru"]
note = "PRD-0003 step 9 and tier 1 #3: $BRKTHRU, which the terminal driver lacks"

[[feature]]
id = "print-queues"
title = "PRINT to a print queue that a symbiont serves"
status = "todo"
milestone = "m1"
note = "PRD-0003 step 10, if its open question says yes. The queue manager starts t8y:/utils/print-symbiont and sends it jobs; the need runs from the symbiont to here"

[[feature]]
id = "generic-queues"
title = "Generic queues (SYS$BATCH, SYS$PRINT) feeding execution queues"
status = "todo"

[[feature]]
id = "forms-characteristics"
title = "Forms, characteristics, job limits, priorities, retention"
status = "todo"

[[feature]]
id = "autostart"
title = "Autostart queues, ENABLE AUTOSTART, and the queue manager restarting with the system"
status = "todo"
note = "Started from the startup procedure (t8y:/utils/startup-shutdown)"

[[feature]]
id = "accounting"
title = "Writes the accounting file: process, image, print and batch records"
status = "todo"
note = "What t8y:/utils/accounting reports on"

[[feature]]
id = "unsolicited-login"
title = "Starts LOGINOUT on a terminal when a key is pressed there"
status = "todo"
needs = ["/drivers/terminal"]
note = "ADR-0029: the swapper creates the console's at once instead (exec.mar ponytail, PRD-0003 tier 1 #2)"

[[feature]]
id = "clusterwide"
title = "One queue database and queue manager for a cluster, with failover"
status = "todo"
note = "PRD-0003 non-goal: clusters (t8y:/exec/cluster)"

[[needs]]
on = "/utils/loginout"
why = "A batch job's process logs in through LOGINOUT, which runs its procedure"

[[needs]]
on = "/exec/processes"
why = "Creates the batch processes and symbionts with $CREPRC"

[[needs]]
on = "/rms/indexed"
why = "QMAN$MASTER.DAT and the queue and journal files are RMS indexed files"

[[needs]]
on = "/exec/mailboxes"
why = "Requests and termination messages reach it through mailboxes"

[[needs]]
on = "/exec/uic-protection"
why = "Queues have an owner and a protection mask; jobs run under their user"

[[needs]]
on = "/exec/mailboxes#attention-asts"
why = "Wakes on a request in its mailbox instead of polling"
soft = true

[[needs]]
on = "/exec/cluster"
why = "Queues shared across a cluster"
soft = true
+++

VMS's batch and print system is a set of queues and the processes that
work them. Programs ask for everything with `$SNDJBC` (make, start and
stop queues; submit, hold and delete jobs) and read with `$GETQUI`;
DCL's `SUBMIT`, `PRINT`, `SHOW QUEUE`, `SYNCHRONIZE` and
`INITIALIZE/QUEUE` are thin layers on those. Early VMS had one process,
`JOB_CONTROL`, for all of it. From V5.5 a queue manager, started with
`START/QUEUE/MANAGER` and on later versions a process of its own,
`QUEUE_MANAGER`, keeps the cluster-wide queue database:
`QMAN$MASTER.DAT` and a queue and journal file per manager.
`JOB_CONTROL` on each node creates batch processes, starts symbionts,
starts `LOGINOUT` on a terminal at its first key and writes the
accounting file. Generic queues such as `SYS$BATCH`
and `SYS$PRINT` hold nothing themselves and pass each job to an
execution queue. A batch job is a detached process that logs in as the
submitter, runs the procedure with `SYS$OUTPUT` a log file, and logs
out.

vaxpunk has none of it yet. Batch is PRD-0003's step 9: `JOB_CONTROL`,
`$SNDJBC`, `$GETQUI`, the queue commands, `SUBMIT` with a log file in
`SYS$LOGIN`, `SHOW QUEUE`, `SYNCHRONIZE` and `$BRKTHRU` for `/NOTIFY`;
print is step 10, if the open question "Is `PRINT` worth doing without a
printer?" says so. DCL's block `IF`, `ON` and lexical functions, which
batch procedures need, are done (step 8), and `STARTUP` already runs a
procedure in a process without a terminal and ends with `job
terminated`, as a batch job will.
