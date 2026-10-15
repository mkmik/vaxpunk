+++
title = "OPCOM"
summary = "The operator communication process: requests, replies, OPERATOR.LOG"
aka = ["OPCOM", "OPCOM.EXE", "$SNDOPR", "OPERATOR.LOG", "REQUEST", "REPLY/ENABLE", "operator terminal", "%%%%%%%%%%%  OPCOM"]
kind = "core"
intent = "aspiration"
research = "outline"
refs = ["docs/prd/0003-multi-user-vms.md", "vms/exec/f11.mar", "https://docs.vmssoftware.com/vsi-openvms-system-manager-s-manual-volume-1-essentials/", "https://wasd.vsm.com.au/htbin/helpgate/HELP/REQUEST", "https://wasd.vsm.com.au/htbin/helpgate/HELP/REPLY"]

[[feature]]
id = "process"
title = "The OPCOM process, its mailbox, and $SNDOPR to reach it"
status = "todo"
needs = ["/exec/mailboxes"]

[[feature]]
id = "operator-terminals"
title = "Operator terminals and classes: REPLY/ENABLE=(CENTRAL, DISKS, TAPES, SECURITY, ...)"
status = "todo"
needs = ["#process", "/drivers/terminal#brkthru"]

[[feature]]
id = "log"
title = "SYS$MANAGER:OPERATOR.LOG, and REPLY/LOG for a new one"
status = "todo"
needs = ["#process", "/rms/sequential"]

[[feature]]
id = "requests"
title = "REQUEST/REPLY from users, REPLY/TO and /ABORT from operators"
status = "todo"
needs = ["#operator-terminals"]

[[feature]]
id = "system-messages"
title = "The system's events through OPCOM: mounts, dismounts, device errors, queues"
status = "todo"
needs = ["#process"]
note = "The executive writes %MOUNT-I-MOUNTED and the like on the console itself (f11.mar)"

[[feature]]
id = "volume-requests"
title = "MOUNT and BACKUP ask an operator to load the next volume"
status = "todo"
needs = ["#requests", "/utils/mount"]

[[feature]]
id = "security-alarms"
title = "Security alarms to SECURITY operators"
status = "todo"
needs = ["#operator-terminals"]
note = "Auditing is a non-goal of Milestone 1"

[[feature]]
id = "cluster"
title = "One OPCOM across a cluster's nodes"
status = "todo"
note = "Clusters are a non-goal of Milestone 1"

[[needs]]
on = "/exec/mailboxes"
why = "Messages arrive in its mailbox"

[[needs]]
on = "/exec/processes"
why = "A detached process started at boot"

[[needs]]
on = "/drivers/terminal#brkthru"
why = "Broadcasts to operator terminals with $BRKTHRU"

[[needs]]
fuzzy = "$SNDOPR, the system service that sends a message to OPCOM"
why = "How MOUNT, the job controller and REQUEST reach it"

[[needs]]
on = "/utils/audit-server"
why = "Sends security alarms to SECURITY operators"
soft = true

[[needs]]
on = "/exec/cluster"
why = "One OPCOM per node, sharing messages"
soft = true

[[needs]]
on = "/utils/reply"
why = "REPLY/ENABLE makes a terminal an operator's, REPLY/TO answers"
soft = true

[[needs]]
on = "/exec/mailboxes#attention-asts"
why = "Wakes on a message in its mailbox instead of polling"
soft = true
+++

OPCOM, the operator communication manager, is a process that the
startup procedure starts. Programs send it messages with the `$SNDOPR`
system service: `MOUNT` asking for a volume, the job controller about
queues, devices about errors, the audit server about breakins, and
users with `REQUEST`. OPCOM writes each in a banner on the terminals
enabled as operators for that class (`REPLY/ENABLE=TAPES`, `CENTRAL`,
`SECURITY`, and so on; `OPA0:` by default) and appends it to
`SYS$MANAGER:OPERATOR.LOG`. An operator answers a request with
`REPLY/TO=n` or `REPLY/ABORT=n`, which wakes the program waiting for it.
In a cluster one OPCOM per node shares the messages.

vaxpunk has no OPCOM. The executive writes its events, such as
`%MOUNT-I-MOUNTED`, straight on the console. PRD-0003 wanted `$BRKTHRU`
for `REPLY` and the job controller's notices, and `ttdriver.mar` has it
now; OPCOM would use it too, but PRD-0003 doesn't name OPCOM.
