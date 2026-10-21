+++
title = "OPCOM"
summary = "The operator communication process: requests, replies, OPERATOR.LOG"
aka = ["OPCOM", "OPCOM.EXE", "$SNDOPR", "OPERATOR.LOG", "REQUEST", "REPLY/ENABLE", "operator terminal", "%%%%%%%%%%%  OPCOM"]
kind = "core"
intent = "goal"
research = "outline"
refs = ["docs/adr/0042-opcom-reads-the-operators-mailbox.md", "vms/sysexe/opcom.mar", "vms/sysexe/request.mar", "vms/exec/mbdriver.mar", "docs/prd/0003-multi-user-vms.md", "vms/exec/f11.mar", "https://docs.vmssoftware.com/vsi-openvms-system-manager-s-manual-volume-1-essentials/", "https://wasd.vsm.com.au/htbin/helpgate/HELP/REQUEST", "https://wasd.vsm.com.au/htbin/helpgate/HELP/REPLY"]

[[feature]]
id = "process"
title = "The OPCOM process, its mailbox, and $SNDOPR to reach it"
status = "done"
needs = ["/exec/mailboxes", "/dcl/interpreter#run-detach"]
note = "ADR-0042: MBA2:, which the executive makes; OPC$_RQ_RQST's messages on the console"

[[feature]]
id = "request"
title = "REQUEST \"text\" [/TO=(class,...)]: a user's message to the operators"
status = "done"
needs = ["#process"]
note = "request.mar; DCL checks /TO's classes, %DCL-W-IVKEYW where VMS says %OPCOM-F-VALUERR"

[[feature]]
id = "operator-terminals"
title = "Operator terminals and classes: REPLY/ENABLE=(CENTRAL, DISKS, TAPES, SECURITY, ...)"
status = "todo"
needs = ["#process", "/drivers/terminal#brkthru"]
note = "opcom.mar ponytail: the console is the one, for VMS's default classes"

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
note = "opcom.mar ponytail: a request with a reply mailbox shows as a message, and gets no answer"

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

vaxpunk's OPCOM ([ADR-0042](../../adr/0042-opcom-reads-the-operators-mailbox.md))
is `OPCOM.EXE`, which `SYSTARTUP_VMS.COM` starts with `RUN/DETACHED`. It
reads `MBA2:`, the operator's mailbox the executive makes at boot, where
`$SNDOPR` writes, and writes each message on the console with
`$BRKTHRU`, under VMS's banner; the console is enabled for VMS's default
classes when it starts, and is the only operator terminal. `REQUEST`
sends it a user's message. The executive still writes its own events,
such as `%MOUNT-I-MOUNTED`, straight on the console, and there is no
`OPERATOR.LOG` or `REPLY/ENABLE` yet.
